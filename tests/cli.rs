use assert_cmd::prelude::*;
use indoc::formatdoc;
use insta::{assert_snapshot, glob};
use sgleam::repl::{welcome_message, QUIT, TYPE};

use std::{
    io::Write,
    process::{Command, Stdio},
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
fn repl_let() {
    assert_eq!(repl_exec("let x = 10\nx + 1"), "10\n11");
}
#[test]
fn repl_let_discard() {
    assert_eq!(repl_exec("let _ = True"), "True");
}

#[test]
fn repl_let_int_pattern() {
    assert_eq!(
        repl_exec("let 10 = 10"),
        "patterns are not supported in let statements."
    );
}

#[test]
fn repl_fn() {
    assert_eq!(repl_exec("fn f(a) { a + 1 }\nf(1)"), "2");
}

#[test]
fn repl_generic_fn() {
    assert_eq!(
        repl_exec("fn keep(_) { True }\nlist.filter([1, 2], keep)"),
        "[1, 2]"
    );
    assert_eq!(
        repl_exec("let keep = fn (_) { True }\nlist.filter([1, 2], keep)"),
        "//fn(a) { ... }\n[1, 2]"
    );
}

#[test]
fn repl_anonymous_fn() {
    assert_eq!(repl_exec("fn () { 1 }"), "//fn() { ... }");
}

#[test]
fn repl_quit() {
    assert_eq!(repl_exec(&format!("{QUIT}\n10")), "");
}

#[test]
fn repl_type() {
    assert_eq!(repl_exec(&format!("{TYPE} 10")), "Int");
    assert_eq!(repl_exec(&format!("{TYPE} int.add")), "fn(Int, Int) -> Int");
    assert_eq!(
        repl_exec(&format!("{TYPE} list.filter_map")),
        "fn(List(b), fn(b) -> Result(c, d)) -> List(c)"
    );
    // :type does not evaluate
    assert_eq!(
        repl_exec(&format!("{TYPE} io.debug(Ok(1))")),
        "Result(Int, b)", // without the io.debug side effect
    );
    // TODO: check that :type let x = 10 does not create x
}

#[test]
fn repl_type_module() {
    assert_eq!(
        repl_exec(&format!("type List {{}}\n{TYPE} list.map")),
        "fn(gleam.List(b), fn(b) -> c) -> gleam.List(c)"
    );
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
        .unwrap_or("")
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
        let (out, err) = run_sgleam_cmd(&[&path.to_string_lossy().to_string()], None);
        assert_snapshot!(formatdoc! {"
            STDOUT
            {out}
            STDERR
            {err}"
        });
    });
}

#[test]
fn smain_list_string() {
    let input = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/inputs/smain_list_string.gleam"
    );
    let (out, err) = run_sgleam_cmd(
        &[input],
        Some(&formatdoc! {
            "
            An example
            with
            three lines"
        }),
    );
    assert_snapshot!(formatdoc! {"
        STDOUT
        {out}
        STDERR
        {err}"
    });
}

#[test]
fn smain_string() {
    let input = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/inputs/smain_string.gleam"
    );
    let (out, err) = run_sgleam_cmd(&[input], Some("hello\nworld"));
    assert_snapshot!(formatdoc! {"
        STDOUT
        {out}
        STDERR
        {err}"
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

    let result = child.wait_with_output().unwrap();

    // assert!(result.status.success());

    (
        String::from_utf8_lossy(&result.stdout).into_owned(),
        String::from_utf8_lossy(&result.stderr).into_owned(),
    )
}
