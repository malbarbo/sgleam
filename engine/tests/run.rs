//! A path given to `run_main` names the module the compiler makes of it, so it
//! is relative to the current directory. A path that names no module is an
//! error and not silence. A silent `Ok` here once left a whole suite of file
//! tests passing on empty output.

use camino::Utf8PathBuf;

use engine::{error::SgleamError, gleam::Project, run::run_main};

#[test]
fn an_absolute_path_names_no_module() {
    let path = Utf8PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../cli/tests/inputs/hello_world.gleam"
    ));
    assert!(path.exists(), "{path} is the file this test runs on");

    assert!(matches!(
        run_main(&[path]),
        Err(SgleamError::NoModuleToRun { .. })
    ));
}

#[test]
#[should_panic(expected = "is not under the source root")]
fn an_absolute_name_is_written_nowhere() {
    Project::default().write_source("/elsewhere/a.gleam", "");
}
