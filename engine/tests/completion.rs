//! What the repl offers to complete, which is the scope it holds, read from
//! the outside. These drive the repl in the test's own process; what it prints
//! goes to the test log, which libtest only shows for a test that failed.

use engine::{
    gleam::{Project, get_module},
    quickjs::QuickJsEngine,
    repl::{Repl, ReplOutput},
};

fn new_repl() -> Repl<QuickJsEngine> {
    Repl::new(Project::default(), None).expect("start the repl")
}

fn new_repl_with_source(source: &str) -> Repl<QuickJsEngine> {
    let mut project = Project::default();
    project.write_source("user.gleam", source);
    let modules = project.compile(true).expect("compile user module");
    let module = get_module(&modules, "user");
    Repl::new(project, module).expect("start the repl")
}

/// An input of a setup has to work, or what it was setting up is not what the
/// test goes on to read.
fn run(repl: &mut Repl<QuickJsEngine>, input: &str) {
    assert!(
        matches!(repl.run(input), ReplOutput::StdOut),
        "{input:?} did not run"
    );
}

fn completions_matching(repl: &Repl<QuickJsEngine>, prefix: &str) -> Vec<String> {
    repl.completions()
        .into_iter()
        .filter(|c| c.starts_with(prefix))
        .collect()
}

#[test]
fn completion_no_module_before_import() {
    let repl = new_repl();
    let c = repl.completions();
    assert!(!c.contains(&"int".to_string()));
    assert!(completions_matching(&repl, "int.").is_empty());
}

#[test]
fn completion_qualified_names() {
    let mut repl = new_repl();
    run(&mut repl, "import gleam/int");
    run(&mut repl, "import gleam/option");
    let c = completions_matching(&repl, "int.");
    assert!(c.contains(&"int.to_string".to_string()));
    assert!(c.contains(&"int.add".to_string()));
    // Types too
    let c = completions_matching(&repl, "option.");
    assert!(c.contains(&"option.Some".to_string()));
    assert!(c.contains(&"option.None".to_string()));
}

#[test]
fn completion_after_let() {
    let mut repl = new_repl();
    run(&mut repl, "let my_var = 42");
    let c = completions_matching(&repl, "my_");
    assert_eq!(c, vec!["my_var"]);
}

#[test]
fn completion_after_fn() {
    let mut repl = new_repl();
    run(&mut repl, "fn my_func(x) { x + 1 }");
    let c = completions_matching(&repl, "my_");
    assert_eq!(c, vec!["my_func"]);
}

#[test]
fn completion_after_import_alias() {
    let mut repl = new_repl();
    run(&mut repl, "import gleam/int as i");
    // "i" alias should have qualified completions
    let c = completions_matching(&repl, "i.");
    assert!(c.contains(&"i.to_string".to_string()));
    assert!(c.contains(&"i.add".to_string()));
    // The alias replaces the short name, which is not bound
    assert!(completions_matching(&repl, "int.").is_empty());
}

#[test]
fn completion_after_import_new_module() {
    let mut repl = new_repl();
    assert!(completions_matching(&repl, "io.input").is_empty());
    run(&mut repl, "import sgleam/io");
    // After importing, io now points to sgleam/io, and io.input should be available
    let c = completions_matching(&repl, "io.input");
    assert!(
        c.contains(&"io.input".to_string()),
        "expected io.input after importing sgleam/io, got: {c:?}"
    );
}

#[test]
fn completion_after_import_unqualified() {
    let mut repl = new_repl();
    run(&mut repl, "import gleam/int.{to_string}");
    let c = completions_matching(&repl, "to_string");
    assert_eq!(c, vec!["to_string"]);
}

#[test]
fn completion_fn_with_module_name() {
    let mut repl = new_repl();
    run(&mut repl, "import gleam/io");
    run(&mut repl, "fn io() { 1 }");
    // The module keeps its own namespace, so both stay available.
    let c = completions_matching(&repl, "io.");
    assert!(
        c.contains(&"io.println".to_string()),
        "expected io.* completions after fn io(), got: {c:?}"
    );
    assert!(repl.completions().contains(&"io".to_string()));
}

#[test]
fn completion_user_module_names() {
    let repl = new_repl_with_source("pub const one = 1\n\npub type Three {\n  Num3\n}\n");
    let c = repl.completions();
    assert!(c.contains(&"one".to_string()));
    assert!(c.contains(&"Three".to_string()));
    assert!(c.contains(&"Num3".to_string()));
    assert!(c.contains(&"user.one".to_string()));
}
