use gleam_core::io::memory::InMemoryFileSystem;

#[derive(Debug, Clone, PartialEq)]
pub enum MainFunction {
    Main,
    ReplMain(String),
    Smain,
    SmainStdin,
    SmainStdinLines,
}

impl MainFunction {
    pub fn name(&self) -> &str {
        match self {
            MainFunction::Main => "main",
            MainFunction::ReplMain(name) => name,
            _ => "smain",
        }
    }
}

pub trait Engine: Clone {
    fn new(fs: InMemoryFileSystem) -> Self;

    fn run_main(&self, module: &str, main: MainFunction, show_output: bool);

    fn has_var(&self, index: usize) -> bool;

    fn run_tests(&self, modules: &[&str]);

    fn interrupt(&self);
}
