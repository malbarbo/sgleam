#![allow(
    clippy::missing_safety_doc,
    clippy::large_enum_variant,
    clippy::result_large_err
)]

pub mod engine;
pub mod error;
pub mod format;
pub mod gleam;
#[cfg(not(target_arch = "wasm32"))]
pub mod logger;
pub mod panic;
pub mod parser;
pub mod quickjs;
pub mod repl;
pub mod run;

use rust_embed::Embed;

pub const GLEAM_VERSION: &str = gleam_core::version::COMPILER_VERSION;

pub const GLEAM_STDLIB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gleam-stdlib.tar"));
pub const GLEAM_STDLIB_BIGINT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/gleam-stdlib-bigint.tar"));
pub const GLEAM_STDLIB_VERSION: &str = "0.68.0";
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
    "sgleam/font",
    "sgleam/image",
    "sgleam/math",
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

pub const QUICKJS_VERSION: &str = "0.11.0";

pub fn version() -> String {
    format!(
        "sgleam {SGLEAM_VERSION} (using gleam {GLEAM_VERSION}, stdlib {GLEAM_STDLIB_VERSION} and quickjs {QUICKJS_VERSION})"
    )
}

/// Version string without the "sgleam" prefix, for use with clap's `--version`
/// (which prepends the binary name automatically).
pub fn version_for_clap() -> String {
    format!(
        "{SGLEAM_VERSION} (using gleam {GLEAM_VERSION}, stdlib {GLEAM_STDLIB_VERSION} and quickjs {QUICKJS_VERSION})"
    )
}
