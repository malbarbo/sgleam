use gleam_core::io::memory::InMemoryFileSystem;

use crate::error::SgleamError;

/// A module the repl wrote, named for the runtime that runs it.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplFile {
    pub path: String,
    /// The line of the input each line of the module was copied from, indexed
    /// by line, and 0 for a line the repl wrote. It is what says where in the
    /// input a place in this file is — the file itself the user never saw.
    pub lines: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MainFunction {
    Main,
    /// The repl's entry point, and the files it wrote that the runtime has not
    /// been told about yet — a place in one of them is reported as the input it
    /// came from, as the user never saw the file.
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
            _ => "smain",
        }
    }
}

pub trait Engine: Clone {
    fn new(fs: InMemoryFileSystem) -> Self;

    fn run_main(
        &self,
        module: &str,
        main: MainFunction,
        show_output: bool,
    ) -> Result<(), SgleamError>;

    /// Whether the run remembered a value under `key`.
    fn has_var(&self, key: &str) -> bool;

    fn run_tests(&self, modules: &[&str]) -> Result<(), SgleamError>;

    fn interrupt(&self);
}
