use gleam_core::io::memory::InMemoryFileSystem;

use crate::error::SgleamError;

/// A module written by the repl.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplFile {
    /// The path of the module as the compiled JavaScript names it, such as
    /// `src/repl1.gleam`. The runtime uses this path to find `lines`.
    pub path: String,
    /// Where each line of the module came from. `lines[n]` is the input line
    /// behind line `n` of the module, or 0 for a line the repl wrote itself.
    /// The count starts at 1, so `lines[0]` is unused.
    ///
    /// It is how the runtime turns a line of this module back into a line of
    /// the input. For the input `fn f(x) {\n  echo x\n}` written as
    ///
    /// ```text
    /// 1  import gleam/io    the repl
    /// 2  pub fn f(x) {      input line 1
    /// 3  let a = a()        the repl
    /// 4    echo x           input line 2
    /// 5  }                  input line 3
    /// ```
    ///
    /// `lines` is `[0, 0, 1, 0, 2, 3]`.
    pub lines: Vec<u32>,
}

/// The entry point sgleam adds to gleam's `main`. `run` looks it up by this
/// name and the launcher imports it by this name.
pub const SMAIN: &str = "smain";

/// The JavaScript the generated code calls into: the `@external` of a
/// generated module names it, and the launcher imports from it.
pub const SGLEAM_FFI: &str = "./sgleam/sgleam_ffi.mjs";

#[derive(Debug, Clone, PartialEq)]
pub enum MainFunction {
    Main,
    /// The repl's entry point, and the new files it wrote.
    ReplMain {
        name: String,
        files: Vec<ReplFile>,
    },
    Smain,
    SmainStdin,
    SmainStdinLines,
}

impl MainFunction {
    pub fn name(&self) -> &str {
        match self {
            MainFunction::Main => "main",
            MainFunction::ReplMain { name, .. } => name,
            MainFunction::Smain | MainFunction::SmainStdin | MainFunction::SmainStdinLines => SMAIN,
        }
    }

    /// The name the launcher knows this entry point by. It says what the
    /// program is handed, and the launcher hands it.
    pub fn kind(&self) -> &'static str {
        match self {
            MainFunction::Main => "Main",
            MainFunction::ReplMain { .. } => "ReplMain",
            MainFunction::Smain => "Smain",
            MainFunction::SmainStdin => "SmainStdin",
            MainFunction::SmainStdinLines => "SmainStdinLines",
        }
    }

    /// Returns `true` if the run prints what the entry point gives back. Only
    /// an `smain` is written to be read that way; the repl prints its own.
    pub fn show_output(&self) -> bool {
        !matches!(self, MainFunction::Main | MainFunction::ReplMain { .. })
    }
}

pub trait Engine: Clone {
    fn new(fs: InMemoryFileSystem) -> Result<Self, SgleamError>;

    fn run_main(&self, module: &str, main: MainFunction) -> Result<(), SgleamError>;

    fn has_var(&self, key: &str) -> bool;

    fn run_tests(&self, modules: &[&str]) -> Result<(), SgleamError>;
}
