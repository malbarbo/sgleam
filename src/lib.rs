pub mod format;
pub mod gleam;
pub mod javascript;
pub mod logger;
pub mod panic;
pub mod repl;

pub const GLEAM_STDLIB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gleam-stdlib.tar"));
pub const GLEAM_VERSION: &str = gleam_core::version::COMPILER_VERSION;
pub const GLEAM_STDLIB_VERSION: &str = "0.40.0";

pub const SGLEAM_CHECK: &str = include_str!("../check.gleam");
pub const SGLEAM_FFI_MJS: &str = include_str!("../sgleam_ffi.mjs");
pub const SGLEAM_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const STACK_SIZE: usize = 128 * 1024 * 1024;

pub fn version() -> String {
    format!(
        "sgleam {SGLEAM_VERSION} (using gleam {GLEAM_VERSION} and stdlib {GLEAM_STDLIB_VERSION})"
    )
}
