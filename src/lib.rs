pub mod error;
pub mod format;
pub mod gleam;
pub mod javascript;
pub mod logger;
pub mod panic;
pub mod parser;
pub mod repl;
#[cfg(not(target_arch = "wasm32"))]
pub mod repl_reader;

#[cfg(target_arch = "wasm32")]
pub mod repl_reader_wasm;

#[cfg(target_arch = "wasm32")]
pub use repl_reader_wasm as repl_reader;

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
