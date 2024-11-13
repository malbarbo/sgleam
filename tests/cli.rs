use assert_cmd::prelude::*;
use indoc::formatdoc;
use insta::{assert_snapshot, glob};
use sgleam::repl::welcome_message;

use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

// FIXME: do not launch sgleam process if not necessary, use the module functions.

#[test]
fn repl_smoke_test() {
    let lit = formatdoc! { r#"
        -2
        13
        4.12
        7.0
        True
        "casa"
        Ok(Nil)"#
    };
    assert_eq!(repl_exec(&lit), lit);
}

#[test]
fn repl_bigint() {
    let lit = "781239812731283189237890123781923";
    assert_eq!(repl_exec(lit), lit);
}

#[test]
fn repl_float_to_string() {
    let lit = "[-1.23, -4.0, 1.234, 3.0, 3.0e21, 1.2e-30, -3.0e56, -1.3e-41]";
    assert_eq!(repl_exec(lit), lit);
}

#[test]
fn repl_constructor_types() {
    let lit = formatdoc! { "
        let a = Ok(10)
        a"
    };
    assert_eq!(repl_exec(&lit), "Ok(10)\nOk(10)");
}

#[test]
fn repl_import() {
    assert_eq!(repl_exec("import something"), "imports are not supported.");
}

#[test]
fn repl_user_module_import() {
    let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/inputs/user.gleam");
    assert_eq!(
        run_sgleam_cmd_stdout(
            &["-q", "-i", input],
            Some(
                "one
                  two()
                  let _: Three = Num3"
            )
        ),
        "1\n2\nNum3\n"
    );
}

#[test]
fn format_stdin() {
    assert_eq!(
        run_sgleam_cmd_stdout(
            &["-f"],
            Some(&formatdoc! {r#"
            import gleam / io.{{ debug , }}
            fn main() {{
               debug("Hello world!" )
            }}
            "#}),
        ),
        formatdoc! {r#"
        import gleam/io.{{debug}}

        fn main() {{
          debug("Hello world!")
        }}
        "#}
    )
}

#[test]
fn repl_welcome_message() {
    assert_eq!(run_sgleam_cmd_stdout(&[], None), welcome_message())
}

fn repl_exec(s: &str) -> String {
    run_sgleam_cmd_stdout(&["-q"], Some(s))
        .strip_suffix('\n')
        .unwrap()
        .into()
}

#[test]
fn run_tests() {
    glob!("inputs/check*.gleam", |path| {
        let (out, err) = run_sgleam_cmd(
            &["-t", path.as_os_str().to_str().expect("a valid path")],
            None,
        );
        assert_snapshot!(formatdoc! {"
            STDOUT
            {out}
            STDERR
            {err}"
        });
    });
}

#[test]
fn run_file() {
    glob!("inputs/*.gleam", |path| {
        let (out, err) = run_sgleam_cmd(&[path.as_os_str().to_str().expect("a valid path")], None);
        assert_snapshot!(formatdoc! {"
            STDOUT
            {out}
            STDERR
            {err}"
        });
    });
}

fn run_sgleam_cmd_stdout(args: &[&str], input: Option<&str>) -> String {
    run_sgleam_cmd(args, input).0
}

// FIXME: this seams too complicated
fn run_sgleam_cmd(args: &[&str], input: Option<&str>) -> (String, String) {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(args)
        .spawn()
        .expect("Spawn child process");

    if let Some(input) = input.map(|input| format!("{input}\n")) {
        let mut stdin = child.stdin.take().expect("Open stdin");
        std::thread::spawn(move || {
            stdin.write_all(input.as_bytes()).expect("Write to stdin");
        });
    }

    let mut stdout = child.stdout.take().unwrap();
    let out = Arc::new(Mutex::new(String::new()));
    let out_ = out.clone();
    let tout = std::thread::spawn(move || {
        let out = out_.clone();
        let mut out = out.lock().unwrap();
        stdout.read_to_string(&mut *out)
    });

    let mut stderr = child.stderr.take().unwrap();
    let err = Arc::new(Mutex::new(String::new()));
    let err_ = err.clone();
    let terr = std::thread::spawn(move || {
        let err = err_.clone();
        let mut err = err.lock().unwrap();
        stderr.read_to_string(&mut *err)
    });

    child.wait().expect("Read stdout");
    let _ = tout.join().expect("Join out thread");
    let _ = terr.join().expect("Join err thread");

    (
        Arc::try_unwrap(out).unwrap().into_inner().unwrap(),
        Arc::try_unwrap(err).unwrap().into_inner().unwrap(),
    )
}
