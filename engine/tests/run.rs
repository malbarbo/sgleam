//! A path given to `run_main` is the name of the module it will be compiled as,
//! so it is relative to the directory the program runs from. A path that names
//! no module is an error and not silence: a silent `Ok` here once left a whole
//! suite of file tests passing on empty output.

use camino::Utf8PathBuf;

use engine::{error::SgleamError, run::run_main};

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
