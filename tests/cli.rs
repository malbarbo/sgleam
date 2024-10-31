use assert_cmd::prelude::*;
use indoc::formatdoc;
use sgleam::repl::welcome_message;

use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn repl_smoke_test() {
    let lit = "13\nTrue\n\"casa\"\nOk(Nil)\n781239812731283189237890123781923";
    assert_eq!(repl_exec(lit), lit);
}

#[test]
fn repl_user_module_import() {
    let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/inputs/user.gleam");
    assert_eq!(
        repl_exec_args(&["-q", "-i", input], "one\ntwo()\nlet _: Three = Num3"),
        "1\n2\nNum3\n"
    );
}

#[test]
fn format_stdin() {
    assert_eq!(
        repl_exec_args(
            &["-f"],
            &formatdoc! {r#"
        import gleam / io.{{ debug , }}
        fn main() {{
            debug("Hello world!" )
        }}
    "#},
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
    assert_eq!(repl_exec_args(&[], ""), welcome_message())
}

fn repl_exec(s: &str) -> String {
    repl_exec_args(&["-q"], s)
        .strip_suffix('\n')
        .unwrap()
        .into()
}

fn repl_exec_args(args: &[&str], s: &str) -> String {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .args(args)
        .spawn()
        .expect("Spawn child process");

    let mut stdin = child.stdin.take().expect("Open stdin");

    let s = format!("{s}\n");
    std::thread::spawn(move || {
        stdin.write_all(s.as_bytes()).expect("Write to stdin");
    });

    let output = child.wait_with_output().expect("Read stdout");
    String::from_utf8_lossy(&output.stdout).to_string()
}
