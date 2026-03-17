use camino::Utf8PathBuf;
use indoc::formatdoc;
use insta::{assert_snapshot, glob};
use sgleam_core::{
    error::show_error,
    quickjs::capture_output,
    run::{run_main, run_test},
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
        let (out, _) = run_file_captured(path);
        assert_snapshot!(format!("{out}"));
    });
}
