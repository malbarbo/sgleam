use camino::Utf8PathBuf;
use engine::{
    engine::Engine,
    error::show_error,
    gleam::{Project, get_module},
    output::capture_output,
    quickjs::QuickJsEngine,
    repl::Repl,
    run::{get_main, run_main, run_test},
};
use indoc::formatdoc;
use insta::{Settings, assert_snapshot, glob};

const INPUTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../cli/tests/inputs");
const IMAGES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/images");

fn insta_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.add_filter(
        r"(?m)`[^`]*[/\\]cli[/\\]tests[/\\]inputs[/\\]",
        "`<INPUTS>/",
    );
    settings
}

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
    let _guard = insta_settings().bind_to_scope();
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
    let _guard = insta_settings().bind_to_scope();
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

#[test]
#[ignore]
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
    Repl::new(Project::default(), None)
}

fn new_repl_with_source(source: &str) -> Repl<QuickJsEngine> {
    let mut project = Project::default();
    project.write_source("user.gleam", source);
    let modules = project.compile(true).expect("compile user module");
    let module = get_module(&modules, "user");
    Repl::new(project, module)
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
    capture_output(|| {
        repl.run("import gleam/int");
        repl.run("import gleam/option");
    });
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
        repl.run("let my_var = 42");
    });
    let c = completions_matching(&repl, "my_");
    assert_eq!(c, vec!["my_var"]);
}

#[test]
fn completion_after_fn() {
    let mut repl = new_repl();
    capture_output(|| {
        repl.run("fn my_func(x) { x + 1 }");
    });
    let c = completions_matching(&repl, "my_");
    assert_eq!(c, vec!["my_func"]);
}

#[test]
fn completion_after_import_alias() {
    let mut repl = new_repl();
    capture_output(|| {
        repl.run("import gleam/int as i");
    });
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
    capture_output(|| {
        repl.run("import sgleam/io");
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
        repl.run("import gleam/int.{to_string}");
    });
    let c = completions_matching(&repl, "to_string");
    assert_eq!(c, vec!["to_string"]);
}

#[test]
fn completion_fn_with_module_name() {
    let mut repl = new_repl();
    capture_output(|| {
        repl.run("import gleam/io");
        repl.run("fn io() { 1 }");
    });
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
