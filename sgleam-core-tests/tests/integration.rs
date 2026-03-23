use camino::Utf8PathBuf;
use indoc::formatdoc;
use insta::{assert_snapshot, glob};
use sgleam_core::{
    engine::Engine,
    error::show_error,
    gleam::{get_module, Project},
    quickjs::{capture_output, QuickJsEngine},
    repl::Repl,
    run::{get_main, run_main, run_test},
};

const INPUTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../cli/tests/inputs");
const IMAGES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../cli/tests/images");

fn run_file_captured(path: &str) -> (String, String) {
    let path = Utf8PathBuf::from(path);
    capture_output(|| {
        if let Err(err) = run_main(&[path.clone()]) {
            show_error(&err);
        }
    })
}

fn run_tests_captured(path: &str) -> (String, String) {
    let path = Utf8PathBuf::from(path);
    capture_output(|| {
        if let Err(err) = run_test(&[path.clone()], &[path.clone()]) {
            show_error(&err);
        }
    })
}

#[test]
fn run_file() {
    glob!(INPUTS_DIR, "*.gleam", |path| {
        let path = path.as_os_str().to_str().expect("a valid path");
        if path.contains("stackoverflow") && !cfg!(target_os = "linux") {
            return;
        }
        let (out, err) = run_file_captured(path);
        assert_snapshot!(formatdoc! {"
            STDOUT
            {out}
            STDERR
            {err}"
        });
    });
}

#[test]
fn run_tests() {
    glob!(INPUTS_DIR, "check*.gleam", |path| {
        let path = path.as_os_str().to_str().expect("a valid path");
        if path.contains("stackoverflow") && !cfg!(target_os = "linux") {
            return;
        }
        let (out, err) = run_tests_captured(path);
        assert_snapshot!(formatdoc! {"
            STDOUT
            {out}
            STDERR
            {err}"
        });
    });
}

#[cfg(target_os = "linux")]
#[test]
fn run_images() {
    glob!(IMAGES_DIR, "*.gleam", |path| {
        let path = path.as_os_str().to_str().expect("a valid path");
        let (out, _) = run_image_captured(path);
        assert_snapshot!(format!("{out}"));
    });
}

fn run_image_captured(path: &str) -> (String, String) {
    let path = camino::Utf8Path::new(path);
    let name = path.file_name().expect("a valid filename");
    let content = std::fs::read_to_string(path).expect("read file");
    capture_output(|| {
        let mut project = Project::default();
        project.write_source(name, &content);
        let modules = match project.compile(false) {
            Ok(m) => m,
            Err(err) => {
                show_error(&err.into());
                return;
            }
        };
        let stem = path.file_stem().unwrap_or("");
        if let Some(module) = get_module(&modules, stem) {
            match get_main(module) {
                Ok(main) => {
                    let engine = QuickJsEngine::new(project.fs.clone());
                    if let Err(err) = engine.run_main(&module.name, main, false) {
                        show_error(&err);
                    }
                }
                Err(err) => show_error(&err),
            }
        }
    })
}


// --- Completion tests ---

fn new_repl() -> Repl<QuickJsEngine> {
    Repl::new(Project::default(), None).expect("create repl")
}

fn completions_matching(repl: &Repl<QuickJsEngine>, prefix: &str) -> Vec<String> {
    repl.completions()
        .into_iter()
        .filter(|c| c.starts_with(prefix))
        .collect()
}

#[test]
fn completion_default_module_aliases() {
    let repl = new_repl();
    let c = repl.completions();
    // Default module aliases are available
    assert!(c.contains(&"int".to_string()));
    assert!(c.contains(&"list".to_string()));
    assert!(c.contains(&"io".to_string()));
    assert!(c.contains(&"float".to_string()));
}

#[test]
fn completion_qualified_names() {
    let repl = new_repl();
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
    capture_output(|| {
        repl.run("let my_var = 42").unwrap();
    });
    let c = completions_matching(&repl, "my_");
    assert_eq!(c, vec!["my_var"]);
}

#[test]
fn completion_after_fn() {
    let mut repl = new_repl();
    capture_output(|| {
        repl.run("fn my_func(x) { x + 1 }").unwrap();
    });
    let c = completions_matching(&repl, "my_");
    assert_eq!(c, vec!["my_func"]);
}

#[test]
fn completion_after_import_alias() {
    let mut repl = new_repl();
    capture_output(|| {
        repl.run("import gleam/int as i").unwrap();
    });
    // "i" alias should have qualified completions
    let c = completions_matching(&repl, "i.");
    assert!(c.contains(&"i.to_string".to_string()));
    assert!(c.contains(&"i.add".to_string()));
    // "int" alias should also still work (default alias remains)
    let c = completions_matching(&repl, "int.");
    assert!(c.contains(&"int.to_string".to_string()));
}

#[test]
fn completion_after_import_new_module() {
    let mut repl = new_repl();
    // sgleam/io is NOT in GLEAM_MODULES_NAMES (to avoid conflicting with gleam/io)
    assert!(
        completions_matching(&repl, "io.input").is_empty()
            || !repl.completions().iter().any(|c| c == "io.input")
    );
    capture_output(|| {
        repl.run("import sgleam/io").unwrap();
    });
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
    capture_output(|| {
        repl.run("import gleam/int.{to_string}").unwrap();
    });
    let c = completions_matching(&repl, "to_string");
    assert_eq!(c, vec!["to_string"]);
}

#[test]
fn completion_fn_shadows_alias() {
    let mut repl = new_repl();
    assert!(completions_matching(&repl, "io.").len() > 0);
    capture_output(|| {
        repl.run("fn io() { 1 }").unwrap();
    });
    // "io" is now a function, not a module alias — no io.* completions
    let c = completions_matching(&repl, "io.");
    assert!(
        c.is_empty(),
        "expected no io.* completions after fn io(), got: {c:?}"
    );
    // But "io" itself should still be a completion (as a function)
    assert!(repl.completions().contains(&"io".to_string()));
}

// TODO: user module public names loaded via file are not tracked in self.names,
// so they don't appear in completions. They are imported via user_import string
// and accessible at runtime but invisible to the completion system.
