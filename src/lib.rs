#![allow(
    clippy::missing_safety_doc,
    clippy::large_enum_variant,
    clippy::result_large_err
)]

pub mod engine;
pub mod error;
pub mod format;
pub mod gleam;
pub mod logger;
pub mod panic;
pub mod parser;
pub mod quickjs;
pub mod repl;
pub mod run;

#[cfg(target_arch = "wasm32")]
pub mod repl_reader_wasm;
#[cfg(target_arch = "wasm32")]
pub use repl_reader_wasm as repl_reader;
use rust_embed::Embed;

#[cfg(not(target_arch = "wasm32"))]
pub mod repl_reader;

#[cfg(target_arch = "wasm32")]
pub mod exports;

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
    "sgleam/check",
    "sgleam/color",
    "sgleam/fill",
    "sgleam/image",
    "sgleam/stroke",
    "sgleam/style",
    "sgleam/system",
    "sgleam/world",
    "sgleam/xplace",
    "sgleam/yplace",
];

#[derive(Embed)]
#[folder = "sgleam/"]
#[prefix = "sgleam/"]
pub struct Sgleam;

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
