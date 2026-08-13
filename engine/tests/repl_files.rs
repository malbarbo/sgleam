//! What the repl hands the runtime to place a line of a module it wrote. A map
//! is as long as its module and is kept for the rest of the session, so only a
//! module a place can be reached in goes over. These drive the repl in the
//! test's own process, over an engine that runs nothing and keeps what it was
//! told.

use std::cell::RefCell;

use engine::{
    engine::{Engine, MainFunction, ReplFile},
    error::SgleamError,
    gleam::Project,
    repl::{Repl, ReplOutput},
};
use gleam_core::io::memory::InMemoryFileSystem;

thread_local! {
    static HANDED_OVER: RefCell<Vec<ReplFile>> = const { RefCell::new(Vec::new()) };
}

/// The engine has no way of being handed anything at construction, and libtest
/// gives each test a thread of its own, which is what keeps these apart.
#[derive(Clone)]
struct Recorder;

impl Engine for Recorder {
    fn new(_fs: InMemoryFileSystem) -> Recorder {
        HANDED_OVER.with_borrow_mut(Vec::clear);
        Recorder
    }

    fn run_main(
        &self,
        _module: &str,
        main: MainFunction,
        _show_output: bool,
    ) -> Result<(), SgleamError> {
        if let MainFunction::ReplMain { files, .. } = main {
            HANDED_OVER.with_borrow_mut(|handed| handed.extend(files));
        }
        Ok(())
    }

    // Nothing ran, so nothing raised before the value was remembered.
    fn has_var(&self, _index: usize) -> bool {
        true
    }

    fn truncate_vars(&self, _count: usize) {}

    fn run_tests(&self, _modules: &[&str]) -> Result<(), SgleamError> {
        Ok(())
    }

    fn interrupt(&self) {}
}

fn run(repl: &mut Repl<Recorder>, input: &str) {
    assert!(
        matches!(repl.run(input), ReplOutput::StdOut),
        "{input:?} did not run"
    );
}

fn handed_over_paths() -> Vec<String> {
    HANDED_OVER.with_borrow(|handed| handed.iter().map(|file| file.path.clone()).collect())
}

/// A module compiled to check an import defines nothing, so no place in it is
/// ever reached. Everything else goes over: one that ran nothing still raises
/// later, from a function it defined.
#[test]
fn the_runtime_is_told_of_the_modules_it_can_reach() {
    let mut repl: Repl<Recorder> = Repl::new(Project::default(), None);
    run(&mut repl, "fn f() { 1 }");
    run(&mut repl, "import gleam/int");
    run(&mut repl, "import gleam/float");
    run(&mut repl, "let x = f()");
    run(&mut repl, "x");

    // `repl2_1` and `repl3_1` are what checked the two imports.
    assert_eq!(
        handed_over_paths(),
        [
            "repl1.gleam",
            "repl4.gleam",
            "repl4_1.gleam",
            "repl5_1.gleam"
        ]
    );
}
