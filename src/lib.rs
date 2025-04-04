use error::show_error;
use gleam::{compile, get_module, Project};
use gleam_core::javascript::set_bigint_enabled;
use quickjs::QuickJsEngine;
use repl::{Repl, ReplOutput};

pub mod engine;
pub mod error;
pub mod format;
pub mod gleam;
pub mod logger;
pub mod panic;
pub mod parser;
pub mod quickjs;
pub mod repl;
#[cfg(not(target_arch = "wasm32"))]
pub mod repl_reader;
pub mod run;

pub const GLEAM_VERSION: &str = gleam_core::version::COMPILER_VERSION;

pub const GLEAM_STDLIB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gleam-stdlib.tar"));
pub const GLEAM_STDLIB_BIGINT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/gleam-stdlib-bigint.tar"));
pub const GLEAM_STDLIB_VERSION: &str = "0.57.0";
pub const GLEAM_MODULES_NAMES: &[&str] = &[
    "gleam/bit_array",
    "gleam/bool",
    "gleam/bytes_tree",
    "gleam/dict",
    "gleam/dynamic",
    "gleam/float",
    "gleam/function",
    "gleam/int",
    "gleam/io",
    "gleam/list",
    "gleam/option",
    "gleam/order",
    "gleam/pair",
    "gleam/result",
    "gleam/set",
    "gleam/string",
    "gleam/string_tree",
    "gleam/uri",
];

pub const SGLEAM_CHECK: &str = include_str!("../check.gleam");
pub const SGLEAM_FFI_MJS: &str = include_str!("../sgleam_ffi.mjs");
pub const SGLEAM_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const STACK_SIZE: usize = 128 * 1024 * 1024;

#[macro_export]
macro_rules! swrite {
    ($s:expr, $($arg:tt)*) => {
        let _ = write!($s, $($arg)*);
    };
}

#[macro_export]
macro_rules! swriteln {
    ($s:expr, $($arg:tt)*) => {
        let _ = writeln!($s, $($arg)*);
    };
}

pub fn version() -> String {
    format!(
        "sgleam {SGLEAM_VERSION} (using gleam {GLEAM_VERSION} and stdlib {GLEAM_STDLIB_VERSION})"
    )
}

#[no_mangle]
pub extern "C" fn string_allocate(size: usize) -> *mut u8 {
    let mut buffer = Vec::with_capacity(size);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

#[no_mangle]
pub extern "C" fn string_deallocate(ptr: *mut u8, size: usize) {
    assert!(!ptr.is_null());
    unsafe {
        let _ = Vec::from_raw_parts(ptr, 0, size);
    }
}

fn new_string(ptr: *mut u8, len: usize) -> String {
    assert!(!ptr.is_null());
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).into()
}


// TODO: add repl_destroy
#[no_mangle]
pub extern "C" fn repl_new(str: *mut u8, len: usize) -> *mut Repl<QuickJsEngine> {
    let mut project = Project::default();
    project.write_source("user.gleam", &new_string(str, len));
    let mut modules = match compile(&mut project, false) {
        Err(err) => {
            show_error(&error::SgleamError::Gleam(err));
            return std::ptr::null_mut();
        }
        Ok(modules) => modules,
    };
    modules
        .retain(|module| !module.name.starts_with("gleam/") && !module.name.starts_with("sgleam/"));
    let module = get_module(&modules, "user");
    Box::leak(Box::new(Repl::new(project, module).expect("An repl")))
}

fn new_repl(repl: *mut Repl<QuickJsEngine>) -> Box<Repl<QuickJsEngine>> {
    unsafe { Box::from_raw(repl) }
}

#[no_mangle]
pub extern "C" fn repl_run(repl: *mut Repl<QuickJsEngine>, str: *mut u8, len: usize) -> bool {
    assert!(!repl.is_null());

    let mut repl = new_repl(repl);
    let ret = match repl.run(&new_string(str, len)) {
        Ok(ReplOutput::Quit) => true,
        Err(err) => {
            show_error(&err);
            false
        }
        _ => false,
    };

    Box::leak(repl);

    ret
}

#[no_mangle]
pub extern "C" fn use_bigint(flag: bool) {
    set_bigint_enabled(flag);
}
