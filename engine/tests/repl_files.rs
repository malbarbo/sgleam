//! What the repl hands the runtime to place a line of a module it wrote. A map
//! is as long as its module and stays for the rest of the session, so only a
//! module with places to reach goes over. These drive the repl in the test's
//! own process, over an engine that runs nothing and remembers what the repl
//! told it.

use std::{cell::RefCell, rc::Rc};

use engine::{
    engine::{Engine, MainFunction, ReplFile},
    error::SgleamError,
    gleam::Project,
    repl::Repl,
};
use gleam_core::io::memory::InMemoryFileSystem;

/// `run_main` takes `&self`, and the repl clones the engine to snapshot itself,
/// so an `Rc` holds what the repl told it — which is also what keeps a rollback
/// from taking it back, as a rollback does not take back a real run either.
#[derive(Clone)]
struct Recorder {
    handed: Rc<RefCell<Vec<ReplFile>>>,
}

impl Engine for Recorder {
    fn new(_fs: InMemoryFileSystem) -> Result<Recorder, SgleamError> {
        Ok(Recorder {
            handed: Rc::default(),
        })
    }

    fn run_main(
        &self,
        _module: &str,
        main: MainFunction,
        _show_output: bool,
    ) -> Result<(), SgleamError> {
        if let MainFunction::ReplMain { files, .. } = main {
            self.handed.borrow_mut().extend(files);
        }
        Ok(())
    }

    // Nothing ran, so nothing raised before the run could remember the value.
    fn has_var(&self, _key: &str) -> bool {
        true
    }

    fn run_tests(&self, _modules: &[&str]) -> Result<(), SgleamError> {
        Ok(())
    }

    fn interrupt(&self) {}
}

fn run(repl: &mut Repl<Recorder>, input: &str) {
    assert!(repl.run(input).is_ok(), "{input:?} did not run");
}

fn handed_over_paths(repl: &Repl<Recorder>) -> Vec<String> {
    repl.engine()
        .handed
        .borrow()
        .iter()
        .map(|file| file.path.clone())
        .collect()
}

#[test]
fn the_runtime_is_told_of_the_modules_it_can_reach() {
    let mut repl: Repl<Recorder> = Repl::new(Project::default(), None).expect("start the repl");
    run(&mut repl, "fn f() { 1 }");
    run(&mut repl, "import gleam/int");
    run(&mut repl, "import gleam/float");
    run(&mut repl, "let x = f()");
    run(&mut repl, "x");

    // `repl2_1` and `repl3_1` are what checked the two imports.
    assert_eq!(
        handed_over_paths(&repl),
        [
            "src/repl1.gleam",
            "src/repl4.gleam",
            "src/repl4_1.gleam",
            "src/repl5_1.gleam"
        ]
    );
}
