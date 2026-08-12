//! What the binary prints for every file under `tests/inputs` and
//! `tests/images`, one test per file. The list is written by `build.rs`, which
//! reads the directories at build time: a single test walking them would run
//! them in series, and the files are what there is most of.

use insta::{Settings, assert_snapshot};

/// The binary is run from the directory of the file it is given, and given only
/// its name — so a path in a diagnostic is the one a user of that file sees,
/// and says nothing of where these tests keep it.
fn output_of(command: &str, file: &str) -> (String, String) {
    let (dir, name) = file.rsplit_once('/').expect("a file in a directory");
    let out = assert_cmd::cargo::cargo_bin_cmd!()
        .current_dir(format!("{}/{dir}", env!("CARGO_MANIFEST_DIR")))
        .args([command, name])
        .output()
        .expect("run sgleam");
    let text = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .replace('\\', "/")
            .replace("\r\n", "\n")
    };
    (text(&out.stdout), text(&out.stderr))
}

/// The snapshot is named after what the file is run for and the file it was run
/// on, as `glob!` named it: `run_file@hello_world.gleam`.
fn bind(file: &str) -> impl Drop {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_suffix(file.rsplit('/').next().expect("a file name"));
    settings.set_omit_expression(true);
    settings.bind_to_scope()
}

fn check(snapshot: &str, command: &str, file: &str) {
    let _guard = bind(file);
    let (out, err) = output_of(command, file);
    assert_snapshot!(snapshot, format!("STDOUT\n{out}STDERR\n{err}"));
}

/// An image is the whole of what its file prints, and nothing around it.
fn check_image(snapshot: &str, command: &str, file: &str) {
    let _guard = bind(file);
    let (out, _) = output_of(command, file);
    assert_snapshot!(snapshot, out);
}

include!(concat!(env!("OUT_DIR"), "/file_tests.rs"));
