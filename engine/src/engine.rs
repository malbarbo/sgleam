use gleam_core::io::memory::InMemoryFileSystem;

use crate::error::SgleamError;

/// A module written by the repl.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplFile {
    /// The path of the module as the compiled JavaScript names it, such as
    /// `src/repl1.gleam`. It is the path the runtime looks `lines` up by.
    pub path: String,
    /// Where each line of the module came from: `lines[n]` is the input line
    /// that line `n` of the module was copied from, or 0 for a line the repl
    /// wrote itself. Lines are counted from 1, so `lines[0]` is unused.
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
            MainFunction::Smain | MainFunction::SmainStdin | MainFunction::SmainStdinLines => {
                "smain"
            }
        }
    }
}

pub trait Engine: Clone {
    fn new(fs: InMemoryFileSystem) -> Result<Self, SgleamError>;

    fn run_main(
        &self,
        module: &str,
        main: MainFunction,
        show_output: bool,
    ) -> Result<(), SgleamError>;

    fn has_var(&self, key: &str) -> bool;

    fn run_tests(&self, modules: &[&str]) -> Result<(), SgleamError>;

    fn interrupt(&self);
}
