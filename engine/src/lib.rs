#![allow(
    clippy::missing_safety_doc,
    clippy::large_enum_variant,
    clippy::result_large_err
)]

pub mod engine;
pub mod error;
#[cfg(all(not(target_arch = "wasm32"), feature = "resvg"))]
pub mod fonts;
pub mod format;
pub mod gleam;
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
#[cfg(all(not(target_arch = "wasm32"), feature = "resvg"))]
pub mod text_metrics;

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

/// Asked of the engine that is linked in, and not written down beside it: a
/// number kept by hand goes on being reported long after the build it named
/// was replaced.
pub fn quickjs_version() -> &'static str {
    // SAFETY: quickjs hands back a pointer to a string constant of its own,
    // which is there before any runtime is and outlives everything reading it.
    let version = unsafe { std::ffi::CStr::from_ptr(rquickjs::qjs::JS_GetVersion()) };
    version.to_str().unwrap_or("unknown")
}

pub fn version() -> String {
    format!("sgleam {}", version_short())
}

/// Version string without the "sgleam" prefix, for use with `--version`
/// (the CLI framework prepends the binary name automatically).
pub fn version_short() -> String {
    let quickjs = quickjs_version();
    format!(
        "{SGLEAM_VERSION} (using gleam {GLEAM_VERSION}, stdlib {GLEAM_STDLIB_VERSION} and quickjs {quickjs})"
    )
}
