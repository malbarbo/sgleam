#![allow(clippy::missing_safety_doc)]

use camino::Utf8Path;
use engine::{
    coverage,
    error::{self, show_error},
    format,
    gleam::{Project, get_module},
    host::HostOptions,
    input, panic,
    quickjs::QuickJsEngine,
    repl::Repl,
    run::{run_modules_coverage, run_modules_tests},
    shell::{self, Shell},
};
use gleam_core::build::Module;
use std::{
    alloc,
    ffi::CString,
    ptr,
    sync::atomic::{AtomicBool, Ordering},
};

/// The module that holds the code of the page.
const USER: &str = "user";
const USER_FILE: &str = "user.gleam";

static INIT: AtomicBool = AtomicBool::new(false);

fn init() {
    if !INIT.swap(true, Ordering::Relaxed) {
        panic::add_handler();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn string_allocate(size: usize) -> *mut u8 {
    init();
    if size == 0 {
        return ptr::NonNull::dangling().as_ptr();
    }
    let layout = string_layout(size);
    let ptr = unsafe { alloc::alloc(layout) };
    if ptr.is_null() {
        alloc::handle_alloc_error(layout);
    }
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn string_deallocate(ptr: *mut u8, size: usize) {
    init();
    assert!(!ptr.is_null());
    if size == 0 {
        return;
    }
    assert_ne!(ptr, ptr::NonNull::dangling().as_ptr());
    unsafe { alloc::dealloc(ptr, string_layout(size)) };
}

fn string_layout(size: usize) -> std::alloc::Layout {
    alloc::Layout::from_size_align(size, 1).expect("more bytes than a layout holds")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cstr_deallocate(ptr: *mut std::ffi::c_char) {
    init();
    assert!(!ptr.is_null());
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

fn new_string(ptr: *mut u8, len: usize) -> String {
    assert!(!ptr.is_null());
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).into()
}

fn to_cstr(s: String) -> *mut std::ffi::c_char {
    match CString::new(s) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Applies the config the page passed, where `bigint=true` turns the bigints
/// on. Returns `true` if the config has `coverage=true`, `false` otherwise.
fn configure(ptr: *mut u8, len: usize) -> bool {
    let config = new_string(ptr, len);
    let has = |key| config.split_whitespace().any(|entry| entry == key);
    gleam_core::javascript::set_bigint_enabled(has("bigint=true"));
    has("coverage=true")
}

/// What `repl_new` gives the page.
pub struct Session {
    shell: Shell<QuickJsEngine>,
    /// The answer of `repl_coverage`.
    coverage: Option<String>,
}

fn default_repl() -> Result<Repl<QuickJsEngine>, error::SgleamError> {
    let project = Project::default();
    let engine = QuickJsEngine::new(project.fs.clone(), HostOptions)?;
    Ok(Repl::new(project, None, engine))
}

fn leak_session(
    repl: Result<Repl<QuickJsEngine>, error::SgleamError>,
    coverage: Option<String>,
) -> *mut Session {
    match repl {
        Ok(repl) => Box::leak(Box::new(Session {
            shell: Shell::new(repl),
            coverage,
        })),
        Err(err) => {
            show_error(&err);
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn repl_new(
    code_ptr: *mut u8,
    code_len: usize,
    config_ptr: *mut u8,
    config_len: usize,
) -> *mut Session {
    init();

    let source = new_string(code_ptr, code_len);
    let coverage = configure(config_ptr, config_len);

    if source.trim().is_empty() {
        return leak_session(default_repl(), None);
    }

    let mut project = Project::default();
    project.write_source(USER_FILE, &source);
    let modules = match project.compile(true) {
        Err(err) => {
            show_error(&error::SgleamError::Gleam(err));
            return ptr::null_mut();
        }
        Ok(modules) => modules,
    };
    let module = get_module(&modules, USER);
    let not_run = match module {
        Some(module) if has_examples(module) => run_examples(&mut project, module, coverage),
        _ => None,
    };
    let repl = QuickJsEngine::new(project.fs.clone(), HostOptions)
        .map(|engine| Repl::new(project, module, engine));
    leak_session(repl, not_run)
}

/// Runs the examples of `module`. With `coverage`, it runs them with the
/// marks and returns the regions that did not run, as `repl_coverage` gives
/// them. Returns `None` if it measured nothing.
fn run_examples(project: &mut Project, module: &Module, coverage: bool) -> Option<String> {
    if !coverage {
        let _ = run_modules_tests(project, &[(Utf8Path::new(USER_FILE), module)], HostOptions);
        return None;
    }
    let (coverage, _) =
        run_modules_coverage(project, &[(Utf8Path::new(USER_FILE), module)], HostOptions);
    let files = coverage.inspect_err(show_error).ok()?;
    if files.is_empty() {
        return None;
    }
    let mut out = String::new();
    for (_, regions) in coverage::not_run(&files) {
        for region in regions {
            out.push_str(&format!(
                "{} {} {}\n",
                region.shown.start,
                region.shown.end,
                region.kind.name()
            ));
        }
    }
    Some(out)
}

fn has_examples(module: &Module) -> bool {
    module.ast.definitions.functions.iter().any(|f| {
        f.publicity.is_public()
            && f.name
                .as_ref()
                .map(|(_, name)| name.ends_with("_examples"))
                .unwrap_or(false)
    })
}

/// Runs the `main` or the `smain` of `code`, as `sgleam run` does. Returns 0
/// if the program ran to the end, 1 otherwise. Only the `wasm/sgleam.ts`
/// wrapper calls it. The page never does.
#[cfg(feature = "wasm-backend")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(
    code_ptr: *mut u8,
    code_len: usize,
    config_ptr: *mut u8,
    config_len: usize,
) -> u32 {
    init();

    let source = new_string(code_ptr, code_len);
    configure(config_ptr, config_len);

    let mut project = Project::default();
    project.write_source(USER_FILE, &source);
    let result = project
        .compile(false)
        .map_err(error::SgleamError::Gleam)
        .and_then(|modules| {
            let module = get_module(&modules, USER).expect("the module of the page");
            engine::run::run_module(&project, module, USER_FILE.into(), HostOptions, vec![])
        });
    match result {
        Ok(()) => 0,
        Err(err) => {
            show_error(&err);
            1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn repl_run(repl: *mut Session, ptr: *mut u8, len: usize) -> u32 {
    init();
    assert!(!repl.is_null());
    let repl = unsafe { &mut *repl };
    repl.shell.run(&new_string(ptr, len)) as u32
}

/// `repl_ready` (see SimpleCode's ENGINE.md). `cursor` is a byte offset into
/// the text, as `repl_complete` takes one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn repl_ready(
    _repl: *mut Session,
    ptr: *mut u8,
    len: usize,
    cursor: usize,
) -> i32 {
    init();
    shell::ready_state(&new_string(ptr, len), cursor)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn repl_destroy(repl: *mut Session) {
    init();
    // `repl_new` answers with null when it has no session to give, and the host
    // hands that null back here, taking it for a session.
    if repl.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(repl);
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn repl_complete(
    repl: *mut Session,
    text_ptr: *mut u8,
    text_len: usize,
    cursor_pos: usize,
) -> *mut std::ffi::c_char {
    init();
    assert!(!repl.is_null());
    let state = unsafe { &*repl };
    let text = new_string(text_ptr, text_len);
    let names = state.shell.completions();
    let (start, candidates) = input::complete(&names, &text, cursor_pos);

    // Nothing to offer, which is also what an empty word gives back.
    if candidates.is_empty() {
        return ptr::null_mut();
    }

    // The offset, then one candidate per line, so that a candidate can carry
    // the space that follows it -- `import ` leaves the caret past it.
    let mut result = format!("c {start}");
    for c in &candidates {
        result.push('\n');
        result.push_str(c);
    }

    to_cstr(result)
}

/// `repl_coverage` (see SimpleCode's ENGINE.md). The code that the examples
/// of `repl_new` did not run, one `<start> <end> <kind>` per line, or null if
/// `repl_new` measured nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn repl_coverage(repl: *mut Session) -> *mut std::ffi::c_char {
    init();
    assert!(!repl.is_null());
    let repl = unsafe { &*repl };
    repl.coverage.clone().map_or(ptr::null_mut(), to_cstr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format(ptr: *mut u8, len: usize) -> *mut std::ffi::c_char {
    init();

    match format::format_source(&new_string(ptr, len)) {
        Ok(out) => to_cstr(out),
        Err(err) => {
            show_error(&error::SgleamError::Gleam(err));
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn version() -> *mut std::ffi::c_char {
    init();
    to_cstr(engine::version())
}

/// Face `i` of the fonts that the frames of a world draw with, for the canvas
/// of the page. It gives the pointer and the length of the bytes, the pointer
/// and the length of the family name, the weight, and 1 for italic. Past the
/// last face it gives null. See ENGINE.md in simplecode.
// The records hold 32-bit pointers, which only wasm32 has.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn font_face(i: u32) -> *const [u32; 6] {
    sinteract::text::face_record(i).map_or(ptr::null(), |face| face as *const [u32; 6])
}
