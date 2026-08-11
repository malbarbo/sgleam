use gleam_core::io::memory::InMemoryFileSystem;

use crate::error::SgleamError;

#[derive(Debug, Clone, PartialEq)]
pub enum MainFunction {
    Main,
    /// The repl's entry point, and the files it wrote that the runtime has not
    /// been told about yet — an error raised in one of them is reported with
    /// no location, as the user never saw the file.
    ReplMain {
        name: String,
        files: Vec<String>,
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

    fn has_var(&self, index: usize) -> bool;

    /// Drops the values saved past `count`.
    fn truncate_vars(&self, count: usize);

    fn run_tests(&self, modules: &[&str]) -> Result<(), SgleamError>;

    fn interrupt(&self);
}
