#![allow(
    clippy::missing_safety_doc,
    clippy::large_enum_variant,
    clippy::result_large_err
)]

pub mod engine;
pub mod error;
pub mod format;
pub mod gleam;
pub mod host;
#[cfg(not(target_arch = "wasm32"))]
pub mod logger;
pub mod panic;
pub mod parser;
pub mod quickjs;
pub mod repl;
pub mod run;
pub mod scope;
pub mod shell;
pub mod source;

use rust_embed::Embed;

pub const GLEAM_VERSION: &str = gleam_core::version::COMPILER_VERSION;

pub const GLEAM_STDLIB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gleam-stdlib.tar"));
pub const GLEAM_STDLIB_VERSION: &str = "1.0.5";

#[derive(Embed)]
#[folder = "../lib/sgleam/"]
#[prefix = "sgleam/"]
pub struct SgleamLib;

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
    format!("sgleam {}", version_short())
}

/// Version string without the "sgleam" prefix, for use with `--version`
/// (the CLI framework prepends the binary name automatically).
pub fn version_short() -> String {
    let quickjs = quickjs::version();
    format!(
        "{SGLEAM_VERSION} (using gleam {GLEAM_VERSION}, stdlib {GLEAM_STDLIB_VERSION} and quickjs {quickjs})"
    )
}
