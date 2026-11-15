//! Runs a program that gleam compiled to WebAssembly. The program calls the
//! seven WASI functions that `sgleam/wasi` and `gleam/io` import, and
//! `env.wasm_try`, which `failure.isolate` imports, and this module answers
//! them.

use std::hash::{BuildHasher, Hasher, RandomState};
use std::io::{ErrorKind, Read, Write};
use std::num::NonZeroUsize;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime};

use gleam_core::webassembly::Lines;
use wasmtime::{
    AnyRef, Caller, Config, Engine, Extern, Func, GcHeapOutOfMemory, Linker, Module, Rooted, Store,
    ThrownException, Trap, UpdateDeadline, Val, WasmBacktrace,
};

use crate::error::{SgleamError, show_error};
use crate::host::{self, check_interrupt};

/// The stack of a program, which fits about 500_000 calls of a small
/// function. wasmtime walks the whole stack for the backtrace of a stack
/// overflow, so a larger stack makes each overflow slower, and a test can
/// have many of them. Only the part that the calls touch takes memory.
const MAX_WASM_STACK: usize = 16 * 1024 * 1024;

/// The stack of the host under the program, for the compile and the calls of
/// WASI.
const HOST_STACK: usize = 16 * 1024 * 1024;

const WASI: &str = "wasi_snapshot_preview1";

const WASI_FUNCTIONS: [&str; 7] = [
    "args_sizes_get",
    "args_get",
    "random_get",
    "clock_time_get",
    "fd_read",
    "fd_write",
    "proc_exit",
];

/// The import of `failure.isolate`, which calls a function of the program
/// and returns 1 when a stack overflow ended the call.
const WASM_TRY: (&str, &str) = ("env", "wasm_try");

const SUCCESS: i32 = 0;
const EBADF: i32 = 8;
const EFAULT: i32 = 21;
const EINVAL: i32 = 28;
const EIO: i32 = 29;

/// The name of the program in `argv[0]`.
const PROGRAM: &str = "program.wasm";

/// The streams of a program.
pub struct Io {
    pub stdin: Box<dyn Read>,
    pub stdout: Box<dyn Write>,
    pub stderr: Box<dyn Write>,
}

impl Io {
    pub fn std() -> Io {
        Io {
            stdin: Box::new(std::io::stdin()),
            stdout: Box::new(std::io::stdout()),
            stderr: Box::new(std::io::stderr()),
        }
    }
}

struct Wasi {
    args: Vec<String>,
    io: Io,
    start: Instant,
}

/// The error that `proc_exit` ends the program with.
#[derive(Debug)]
struct Exit(i32);

impl std::fmt::Display for Exit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "exit with {}", self.0)
    }
}

impl std::error::Error for Exit {}

/// Runs `program` with `args` on the streams of the process, and returns the
/// code that the program exits with.
pub fn run(program: &[u8], args: &[String]) -> Result<i32, SgleamError> {
    run_with(program, args, Io::std())
}

/// Runs `program` with `args` on `io`. A failure of the program goes to the
/// stdout of `io`, as on the JavaScript target, and the result is
/// `UserProgramFailed`.
pub fn run_with(program: &[u8], args: &[String], io: Io) -> Result<i32, SgleamError> {
    stacker::grow(MAX_WASM_STACK + HOST_STACK, || run_here(program, args, io))
}

fn run_here(program: &[u8], args: &[String], io: Io) -> Result<i32, SgleamError> {
    host::init()?;
    let engine = Engine::new(&config()).map_err(other)?;
    let _ticker = Ticker::start(engine.clone());
    let module = Module::new(&engine, program).map_err(other)?;
    check_imports(&module)?;
    let mut linker = Linker::new(&engine);
    add_wasi(&mut linker).map_err(other)?;
    let wasi = Wasi {
        args: args.to_vec(),
        io,
        start: Instant::now(),
    };
    let mut store = Store::new(&engine, wasi);
    // The callback runs at each tick, so an interrupt that came before the
    // store also stops the program.
    store.set_epoch_deadline(1);
    store.epoch_deadline_callback(|_| {
        Ok(if check_interrupt() {
            UpdateDeadline::Interrupt
        } else {
            UpdateDeadline::Continue(1)
        })
    });
    let result = linker
        .instantiate(&mut store, &module)
        .and_then(|instance| instance.get_typed_func::<(), ()>(&mut store, "_start"))
        .and_then(|start| start.call(&mut store, ()));
    match result {
        Ok(()) => Ok(0),
        Err(err) => failure(err, program, &mut store.data_mut().io.stdout),
    }
}

fn config() -> Config {
    let mut config = Config::new();
    config
        .wasm_gc(true)
        .wasm_function_references(true)
        .wasm_tail_call(true)
        .wasm_exceptions(true)
        .epoch_interruption(true)
        .max_wasm_stack(MAX_WASM_STACK)
        // A cycle of n calls shows in 2n frames.
        .wasm_backtrace_max_frames(NonZeroUsize::new(100))
        // `Engine::new` turns down a wasm stack larger than this one, which
        // only a call with `async` takes.
        .async_stack_size(MAX_WASM_STACK + HOST_STACK);
    config
}

/// How often the program checks for an interrupt.
const TICK: Duration = Duration::from_millis(10);

/// How long a program has to stop after an interrupt. A program that waits
/// in a call of WASI, such as a read of stdin, does not see the interrupt.
const GRACE: Duration = Duration::from_millis(200);

/// A thread that moves the epoch of the engine on at each tick, while the
/// program runs, and ends the process when the program does not stop in
/// `GRACE` after an interrupt.
struct Ticker {
    done: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Ticker {
    fn start(engine: Engine) -> Ticker {
        let (done, until) = mpsc::channel();
        let thread = std::thread::spawn(move || tick(&engine, &until));
        Ticker {
            done: Some(done),
            thread: Some(thread),
        }
    }
}

impl Drop for Ticker {
    fn drop(&mut self) {
        drop(self.done.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn tick(engine: &Engine, until: &mpsc::Receiver<()>) {
    let mut interrupted = None;
    while let Err(RecvTimeoutError::Timeout) = until.recv_timeout(TICK) {
        engine.increment_epoch();
        if check_interrupt() && interrupted.get_or_insert_with(Instant::now).elapsed() >= GRACE {
            show_error(&SgleamError::Interrupted);
            std::process::exit(1);
        }
    }
}

fn other(err: wasmtime::Error) -> SgleamError {
    SgleamError::Other(err.into())
}

/// An import that sgleam does not provide comes from an `@external` of the
/// user, so it is an error of the user and not of sgleam.
fn check_imports(module: &Module) -> Result<(), SgleamError> {
    for import in module.imports() {
        let known = if import.module() == WASI {
            WASI_FUNCTIONS.contains(&import.name())
        } else {
            (import.module(), import.name()) == WASM_TRY
        };
        if !known {
            return Err(SgleamError::UnknownImport {
                module: import.module().into(),
                name: import.name().into(),
            });
        }
    }
    Ok(())
}

/// What a program that stopped with `err` ends with. A trap or an exception
/// that no one caught says so on `out`.
fn failure(err: wasmtime::Error, program: &[u8], out: &mut dyn Write) -> Result<i32, SgleamError> {
    let message = if let Some(Exit(code)) = err.downcast_ref::<Exit>() {
        return Ok(if (0..=255).contains(code) { *code } else { 1 });
    } else if let Some(trap) = err.downcast_ref::<Trap>() {
        match trap {
            Trap::Interrupt => return Err(SgleamError::Interrupted),
            Trap::StackOverflow => stack_overflow(&err, program),
            Trap::AllocationTooLarge => "Out of memory".into(),
            trap => format!("WebAssembly trap: {trap}"),
        }
    } else if err.is::<GcHeapOutOfMemory<()>>() {
        "Out of memory".into()
    } else if err.is::<ThrownException>() {
        "Uncaught WebAssembly exception".into()
    } else {
        return Err(other(err));
    };
    let _ = writeln!(out, "{message}");
    let _ = out.flush();
    Err(SgleamError::UserProgramFailed)
}

/// "Stack overflow", followed by the calls that repeat, as on the
/// JavaScript target, when `program` has the lines of its calls.
fn stack_overflow(err: &wasmtime::Error, program: &[u8]) -> String {
    let mut text = String::from("Stack overflow");
    let (Some(backtrace), Some(lines)) = (err.downcast_ref::<WasmBacktrace>(), Lines::of(program))
    else {
        return text;
    };
    let frames: Vec<String> = backtrace
        .frames()
        .iter()
        .filter_map(|frame| {
            // The frame where the stack ran out has no offset, and a function
            // of the runtime has no lines.
            let index = frame.func_index();
            let offset = u32::try_from(frame.func_offset()?).ok()?;
            let function = lines.functions.get(&index)?;
            let line = lines.calls.get(&(index, offset))?;
            // The path of a module starts at the root of the package, and the
            // user gave the path from `src`.
            let file = function.file.strip_prefix("src/").unwrap_or(&function.file);
            Some(format!("at {} ({file}:{line})", function.name))
        })
        .collect();
    if !frames.is_empty() {
        for frame in first_cycle(&frames) {
            text.push_str("\n  ");
            text.push_str(&frame);
        }
        text.push_str("\n  ...");
    }
    text
}

/// One round of the cycle that repeats until the end of `frames`, at its
/// smallest rotation, or every frame when no cycle repeats twice. This is
/// `first_cycle` of `sgleam_ffi.mjs`.
fn first_cycle(frames: &[String]) -> Vec<String> {
    for start in 0..frames.len() {
        for period in 1..=(frames.len() - start) / 2 {
            let mut rest = frames.iter().skip(start);
            if rest
                .clone()
                .zip(rest.clone().skip(period))
                .all(|(a, b)| a == b)
            {
                let round: Vec<String> = rest.by_ref().take(period).cloned().collect();
                return smallest_rotation(&round);
            }
        }
    }
    frames.to_vec()
}

/// The rotation of `round` that comes first in the order of the text, so the
/// trace does not depend on the frame where the stack ran out.
fn smallest_rotation(round: &[String]) -> Vec<String> {
    (0..round.len())
        .map(|i| {
            round
                .iter()
                .cycle()
                .skip(i)
                .take(round.len())
                .cloned()
                .collect::<Vec<_>>()
        })
        .min_by_key(|rotation| rotation.join("\n"))
        .unwrap_or_default()
}

fn add_wasi(linker: &mut Linker<Wasi>) -> wasmtime::Result<()> {
    linker.func_wrap(
        WASI,
        "args_sizes_get",
        |caller: Caller<'_, Wasi>, count: i32, size: i32| {
            errno(args_sizes_get(caller, count, size))
        },
    )?;
    linker.func_wrap(
        WASI,
        "args_get",
        |caller: Caller<'_, Wasi>, pointers: i32, buffer: i32| {
            errno(args_get(caller, pointers, buffer))
        },
    )?;
    linker.func_wrap(
        WASI,
        "random_get",
        |caller: Caller<'_, Wasi>, buffer: i32, size: i32| errno(random_get(caller, buffer, size)),
    )?;
    linker.func_wrap(
        WASI,
        "clock_time_get",
        |caller: Caller<'_, Wasi>, clock: i32, _precision: i64, time: i32| {
            errno(clock_time_get(caller, clock, time))
        },
    )?;
    linker.func_wrap(
        WASI,
        "fd_read",
        |caller: Caller<'_, Wasi>, fd: i32, iovecs: i32, count: i32, size: i32| {
            errno(fd_read(caller, fd, iovecs, count, size))
        },
    )?;
    linker.func_wrap(
        WASI,
        "fd_write",
        |caller: Caller<'_, Wasi>, fd: i32, iovecs: i32, count: i32, size: i32| {
            errno(fd_write(caller, fd, iovecs, count, size))
        },
    )?;
    linker.func_wrap(WASI, "proc_exit", |code: i32| -> wasmtime::Result<()> {
        Err(wasmtime::Error::new(Exit(code)))
    })?;
    let (module, name) = WASM_TRY;
    linker.func_wrap(module, name, wasm_try)?;
    Ok(())
}

/// Calls `f` with `x`, and returns 0, or 1 when a stack overflow ended the
/// call. Every other error goes on, as an exit, an interrupt or an exception
/// has to end the call of the program too.
fn wasm_try(
    mut caller: Caller<'_, Wasi>,
    f: Option<Func>,
    x: Option<Rooted<AnyRef>>,
) -> wasmtime::Result<i32> {
    let f = f.ok_or_else(|| wasmtime::Error::new(Trap::NullReference))?;
    match f.call(&mut caller, &[Val::AnyRef(x)], &mut []) {
        Ok(()) => Ok(0),
        Err(err) if err.downcast_ref::<Trap>() == Some(&Trap::StackOverflow) => Ok(1),
        Err(err) => Err(err),
    }
}

fn errno(result: Result<(), i32>) -> i32 {
    result.err().unwrap_or(SUCCESS)
}

/// The memory of the program, with the state of the store beside it.
fn memory<'a>(caller: &'a mut Caller<'_, Wasi>) -> Result<(&'a mut [u8], &'a mut Wasi), i32> {
    let memory = caller
        .get_export("memory")
        .and_then(Extern::into_memory)
        .ok_or(EFAULT)?;
    Ok(memory.data_and_store_mut(caller))
}

fn bytes(memory: &[u8], address: i32, size: usize) -> Result<&[u8], i32> {
    let start = address as u32 as usize;
    memory.get(start..start.saturating_add(size)).ok_or(EFAULT)
}

fn bytes_mut(memory: &mut [u8], address: i32, size: usize) -> Result<&mut [u8], i32> {
    let start = address as u32 as usize;
    memory
        .get_mut(start..start.saturating_add(size))
        .ok_or(EFAULT)
}

fn load_u32(memory: &[u8], address: i32) -> Result<u32, i32> {
    let mut word = [0; 4];
    word.copy_from_slice(bytes(memory, address, 4)?);
    Ok(u32::from_le_bytes(word))
}

fn store(memory: &mut [u8], address: i32, value: &[u8]) -> Result<(), i32> {
    bytes_mut(memory, address, value.len())?.copy_from_slice(value);
    Ok(())
}

fn store_u32(memory: &mut [u8], address: i32, value: usize) -> Result<(), i32> {
    let value = u32::try_from(value).map_err(|_| EINVAL)?;
    store(memory, address, &value.to_le_bytes())
}

fn argv(wasi: &Wasi) -> impl Iterator<Item = &str> {
    std::iter::once(PROGRAM).chain(wasi.args.iter().map(String::as_str))
}

fn args_sizes_get(mut caller: Caller<'_, Wasi>, count: i32, size: i32) -> Result<(), i32> {
    let (memory, wasi) = memory(&mut caller)?;
    let total = argv(wasi).map(|arg| arg.len() + 1).sum();
    store_u32(memory, count, argv(wasi).count())?;
    store_u32(memory, size, total)
}

/// Writes the address of each argument at `pointers`, and the arguments, each
/// ending in a 0, at `buffer`.
fn args_get(mut caller: Caller<'_, Wasi>, pointers: i32, buffer: i32) -> Result<(), i32> {
    let (memory, wasi) = memory(&mut caller)?;
    let mut address = buffer;
    for (i, arg) in argv(wasi).enumerate() {
        let pointer = pointers.wrapping_add(4 * i as i32);
        store_u32(memory, pointer, address as u32 as usize)?;
        store(memory, address, arg.as_bytes())?;
        store(memory, address.wrapping_add(arg.len() as i32), &[0])?;
        address = address.wrapping_add(arg.len() as i32 + 1);
    }
    Ok(())
}

/// Fills the buffer from the keys of `RandomState`, which the system draws at
/// random.
fn random_get(mut caller: Caller<'_, Wasi>, buffer: i32, size: i32) -> Result<(), i32> {
    let (memory, _) = memory(&mut caller)?;
    for chunk in bytes_mut(memory, buffer, size as u32 as usize)?.chunks_mut(8) {
        let random = RandomState::new().build_hasher().finish();
        for (byte, value) in chunk.iter_mut().zip(random.to_le_bytes()) {
            *byte = value;
        }
    }
    Ok(())
}

const REALTIME: i32 = 0;
const MONOTONIC: i32 = 1;

fn clock_time_get(mut caller: Caller<'_, Wasi>, clock: i32, time: i32) -> Result<(), i32> {
    let (memory, wasi) = memory(&mut caller)?;
    let elapsed = match clock {
        REALTIME => SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| EIO)?,
        MONOTONIC => wasi.start.elapsed(),
        _ => return Err(EINVAL),
    };
    let nanoseconds = u64::try_from(elapsed.as_nanos()).map_err(|_| EIO)?;
    store(memory, time, &nanoseconds.to_le_bytes())
}

/// Reads stdin into the first buffer that has room. A read may stop short of
/// filling it, as WASI allows.
fn fd_read(
    mut caller: Caller<'_, Wasi>,
    fd: i32,
    iovecs: i32,
    count: i32,
    size: i32,
) -> Result<(), i32> {
    if fd != 0 {
        return Err(EBADF);
    }
    let (memory, wasi) = memory(&mut caller)?;
    let mut read = 0;
    for i in 0..count {
        let iovec = iovecs.wrapping_add(8 * i);
        let buffer = load_u32(memory, iovec)? as i32;
        let length = load_u32(memory, iovec.wrapping_add(4))? as usize;
        if length > 0 {
            let target = bytes_mut(memory, buffer, length)?;
            read = loop {
                match wasi.io.stdin.read(target) {
                    Err(err) if err.kind() == ErrorKind::Interrupted => continue,
                    result => break result.map_err(|_| EIO)?,
                }
            };
            break;
        }
    }
    store_u32(memory, size, read)
}

/// Writes every buffer to stdout or stderr, and flushes it, so that what the
/// program writes shows at once.
fn fd_write(
    mut caller: Caller<'_, Wasi>,
    fd: i32,
    iovecs: i32,
    count: i32,
    size: i32,
) -> Result<(), i32> {
    let (memory, wasi) = memory(&mut caller)?;
    let out = match fd {
        1 => &mut wasi.io.stdout,
        2 => &mut wasi.io.stderr,
        _ => return Err(EBADF),
    };
    let mut written = 0;
    for i in 0..count {
        let iovec = iovecs.wrapping_add(8 * i);
        let buffer = load_u32(memory, iovec)? as i32;
        let length = load_u32(memory, iovec.wrapping_add(4))? as usize;
        out.write_all(bytes(memory, buffer, length)?)
            .map_err(|_| EIO)?;
        written += length;
    }
    out.flush().map_err(|_| EIO)?;
    store_u32(memory, size, written)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io::Write;
    use std::rc::Rc;

    use super::{Io, run_with};
    use crate::error::SgleamError;

    #[derive(Clone, Default)]
    struct Output(Rc<RefCell<Vec<u8>>>);

    impl Write for Output {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Output {
        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.borrow()).into_owned()
        }
    }

    /// Runs the module of `wat` with `args` and `stdin`, and returns the result
    /// with what it wrote to stdout.
    fn run(wat: &str, args: &[&str], stdin: &str) -> (Result<i32, SgleamError>, String) {
        let program = wat::parse_str(wat).expect("a valid module");
        let stdout = Output::default();
        let io = Io {
            stdin: Box::new(std::io::Cursor::new(stdin.as_bytes().to_vec())),
            stdout: Box::new(stdout.clone()),
            stderr: Box::new(std::io::sink()),
        };
        let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
        let result = run_with(&program, &args, io);
        (result, stdout.text())
    }

    const FD_WRITE: &str = r#"(import "wasi_snapshot_preview1" "fd_write"
        (func $fd_write (param i32 i32 i32 i32) (result i32)))"#;

    /// Calls `fd_write` on fd 1 with the `size` bytes at `address`. The iovec
    /// goes at 0, and the size written at 8.
    const PRINT: &str = r#"(func $print (param $address i32) (param $size i32)
        (i32.store (i32.const 0) (local.get $address))
        (i32.store (i32.const 4) (local.get $size))
        (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 8))))"#;

    #[test]
    fn a_program_that_returns_exits_with_0() {
        let (result, out) = run(
            &format!(
                r#"(module {FD_WRITE}
                    (memory (export "memory") 1)
                    (data (i32.const 16) "hello\n")
                    {PRINT}
                    (func (export "_start") (call $print (i32.const 16) (i32.const 6))))"#
            ),
            &[],
            "",
        );
        assert_eq!(result.ok(), Some(0));
        assert_eq!(out, "hello\n");
    }

    #[test]
    fn proc_exit_gives_its_code() {
        let (result, _) = run(
            r#"(module
                (import "wasi_snapshot_preview1" "proc_exit" (func $exit (param i32)))
                (func (export "_start") (call $exit (i32.const 3)) (unreachable)))"#,
            &[],
            "",
        );
        assert_eq!(result.ok(), Some(3));
    }

    #[test]
    fn a_trap_says_what_happened() {
        let (result, out) = run(
            r#"(module (func (export "_start") (unreachable)))"#,
            &[],
            "",
        );
        assert!(matches!(result, Err(SgleamError::UserProgramFailed)));
        assert!(out.starts_with("WebAssembly trap: "), "{out}");
    }

    #[test]
    fn an_import_that_sgleam_does_not_provide_names_itself() {
        let (result, _) = run(
            r#"(module (import "foo" "bar" (func)) (func (export "_start")))"#,
            &[],
            "",
        );
        match result {
            Err(SgleamError::UnknownImport { module, name }) => {
                assert_eq!((module.as_str(), name.as_str()), ("foo", "bar"));
            }
            other => panic!("{other:?}"),
        }
    }

    /// More than the 88_000 calls of the JavaScript target.
    #[test]
    fn four_hundred_thousand_nested_calls_fit_in_the_stack() {
        let (result, _) = run(
            r#"(module
                (func $down (param $n i32) (result i32)
                  (if (result i32) (i32.eqz (local.get $n))
                    (then (i32.const 0))
                    (else (i32.add (i32.const 1)
                      (call $down (i32.sub (local.get $n) (i32.const 1)))))))
                (func (export "_start") (drop (call $down (i32.const 400000)))))"#,
            &[],
            "",
        );
        assert_eq!(result.ok(), Some(0));
    }

    #[test]
    fn a_recursion_without_end_overflows_the_stack() {
        let (result, out) = run(
            r#"(module
                (func $forever (param $n i32) (result i32)
                  (i32.add (i32.const 1) (call $forever (local.get $n))))
                (func (export "_start") (drop (call $forever (i32.const 0)))))"#,
            &[],
            "",
        );
        assert!(matches!(result, Err(SgleamError::UserProgramFailed)));
        assert_eq!(out, "Stack overflow\n");
    }

    /// The calls of `$f` and `$g` are at the offset 5 of their bodies, after
    /// the declarations of the locals, `i32.const 1` and `local.get 0`, and
    /// `gleam.lines` gives them the lines 6 and 10 of `src/m.gleam`.
    #[test]
    fn a_stack_overflow_shows_the_calls_that_repeat() {
        let (result, out) = run(
            r#"(module
                (func $f (param $n i32) (result i32)
                  (i32.add (i32.const 1) (call $g (local.get $n))))
                (func $g (param $n i32) (result i32)
                  (i32.add (i32.const 1) (call $f (local.get $n))))
                (func (export "_start") (drop (call $f (i32.const 0))))
                (@custom "gleam.lines"
                  "\03\01f\01g\0bsrc/m.gleam"
                  "\02\00\00\02\01\01\02"
                  "\02\00\05\06\01\05\0a"))"#,
            &[],
            "",
        );
        assert!(matches!(result, Err(SgleamError::UserProgramFailed)));
        assert_eq!(
            out,
            "Stack overflow\n  at f (m.gleam:6)\n  at g (m.gleam:10)\n  ...\n"
        );
    }

    /// A module that calls `env.wasm_try` with `$f` twice, and exits with
    /// the sum of the results, or with the code that `$f` exits with.
    fn wasm_try_module(f: &str) -> String {
        format!(
            r#"(module
                (import "env" "wasm_try" (func $try (param funcref anyref) (result i32)))
                (import "wasi_snapshot_preview1" "proc_exit" (func $exit (param i32)))
                (func $forever (param $n i32) (result i32)
                  (i32.add (i32.const 1) (call $forever (local.get $n))))
                (func $f (param anyref) {f})
                (elem declare func $f)
                (memory (export "memory") 1)
                (func (export "_start")
                  (call $exit (i32.add
                    (call $try (ref.func $f) (ref.null any))
                    (call $try (ref.func $f) (ref.null any))))))"#
        )
    }

    #[test]
    fn wasm_try_returns_0_when_the_call_returns() {
        let (result, _) = run(&wasm_try_module(""), &[], "");
        assert_eq!(result.ok(), Some(0));
    }

    /// The program goes on after each stack overflow.
    #[test]
    fn wasm_try_returns_1_after_a_stack_overflow() {
        let (result, _) = run(
            &wasm_try_module("(drop (call $forever (i32.const 0)))"),
            &[],
            "",
        );
        assert_eq!(result.ok(), Some(2));
    }

    #[test]
    fn an_exit_goes_through_wasm_try() {
        let (result, _) = run(&wasm_try_module("(call $exit (i32.const 7))"), &[], "");
        assert_eq!(result.ok(), Some(7));
    }

    #[test]
    fn another_trap_goes_through_wasm_try() {
        let (result, out) = run(&wasm_try_module("(unreachable)"), &[], "");
        assert!(matches!(result, Err(SgleamError::UserProgramFailed)));
        assert!(out.starts_with("WebAssembly trap: "), "{out}");
    }

    #[test]
    fn args_get_gives_the_name_of_the_program_and_the_args() {
        let (result, out) = run(
            &format!(
                r#"(module {FD_WRITE}
                    (import "wasi_snapshot_preview1" "args_sizes_get"
                      (func $sizes (param i32 i32) (result i32)))
                    (import "wasi_snapshot_preview1" "args_get"
                      (func $args (param i32 i32) (result i32)))
                    (memory (export "memory") 1)
                    {PRINT}
                    (func (export "_start")
                      (drop (call $sizes (i32.const 16) (i32.const 20)))
                      (drop (call $args (i32.const 32) (i32.const 64)))
                      (call $print (i32.const 64) (i32.load (i32.const 20)))
                      (call $print (i32.const 16) (i32.const 1))))"#
            ),
            &["a", "bc"],
            "",
        );
        assert_eq!(result.ok(), Some(0));
        assert_eq!(out, "program.wasm\0a\0bc\0\u{3}");
    }

    #[test]
    fn fd_read_reads_stdin() {
        let (result, out) = run(
            &format!(
                r#"(module {FD_WRITE}
                    (import "wasi_snapshot_preview1" "fd_read"
                      (func $fd_read (param i32 i32 i32 i32) (result i32)))
                    (memory (export "memory") 1)
                    {PRINT}
                    (func (export "_start")
                      (i32.store (i32.const 16) (i32.const 64))
                      (i32.store (i32.const 20) (i32.const 100))
                      (drop (call $fd_read (i32.const 0) (i32.const 16) (i32.const 1) (i32.const 24)))
                      (call $print (i32.const 64) (i32.load (i32.const 24)))))"#
            ),
            &[],
            "a line\n",
        );
        assert_eq!(result.ok(), Some(0));
        assert_eq!(out, "a line\n");
    }
}
