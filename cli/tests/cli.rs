use indoc::formatdoc;
use insta::assert_snapshot;
use sgleam_core::repl::{welcome_message, QUIT, TYPE};

/// Strip the random 8-hex suffix from internal REPL names so snapshot tests
/// are deterministic.
fn strip_repl_suffix(s: &str) -> String {
    let mut result = s.to_string();
    for prefix in ["repl_main_", "repl_print_", "repl_save_", "repl_load_"] {
        while let Some(pos) = result.find(prefix) {
            let suffix_start = pos + prefix.len();
            if suffix_start + 8 <= result.len()
                && result[suffix_start..suffix_start + 8]
                    .chars()
                    .all(|c| c.is_ascii_hexdigit())
            {
                result.replace_range(suffix_start..suffix_start + 8, "XXXXXXXX");
            } else {
                break;
            }
        }
    }
    result
}

// These tests launch the sgleam binary as a subprocess. Tests that only need
// Repl::run() can go in sgleam-core-tests (which uses the capture feature).

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
    // Basic import with unqualified value
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/int.{{to_string}}
            to_string(1)"#
        }),
        r#""1""#
    );
    // Merge imports from same module
    assert_eq!(
        repl_exec(&formatdoc! {"
            import gleam/int.{{to_string}}
            import gleam/int.{{add}}
            import gleam/float.{{to_string}}
            add(1, 2)
            to_string(1.0)"
        }),
        r#"3
"1.0""#
    );
    // Import with rename
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/int.{{to_string as its}}
            its(42)"#
        }),
        r#""42""#
    );
    // Function replaces imported name
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/int.{{to_string}}
            fn to_string(_x) {{ "custom" }}
            to_string(1)"#
        }),
        r#""custom""#
    );
    // Function replaces renamed imported item
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/int.{{to_string as its}}
            fn its(_x) {{ "custom" }}
            its(1)"#
        }),
        r#""custom""#
    );
    // Import type
    assert_eq!(
        repl_exec(&formatdoc! {"
            import gleam/option.{{type Option}}
            let x: Option(Int) = option.Some(1)
            x"
        }),
        "Some(1)\nSome(1)"
    );
    // Type definition replaces imported type
    assert_eq!(
        repl_exec(&formatdoc! {"
            import gleam/option.{{type Option}}
            type Option {{ Custom }}
            Custom"
        }),
        "Custom"
    );
    // Import with same short name renames the old one with as _
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/io.{{println}}
            import sgleam/io
            io.input("") <> "ok""#
        }),
        r#""ok""#
    );
    // Unqualified names from the renamed import still work
    let (out, _) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {r#"
            import gleam/io.{{println}}
            import sgleam/io
            println("hello")"#
        }),
    );
    assert!(
        out.contains("hello"),
        "expected println to work after auto-rename, got: {out}"
    );
    // Explicit as avoids conflict
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import sgleam/io as sio
            sio.input("") <> "ok""#
        }),
        r#""ok""#
    );
    // Re-importing the renamed module restores its short name
    let (out, _) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {r#"
            import gleam/io.{{println}}
            import sgleam/io
            import gleam/io
            io.println("hello")"#
        }),
    );
    assert!(
        out.contains("hello"),
        "expected io.println to work after re-import, got: {out}"
    );
    // Verify auto-rename generates "as _"
    let (out, _) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {r#"
            :debug
            import sgleam/io
            io.input("") <> "ok""#
        }),
    );
    assert!(
        out.contains("import gleam/io as _"),
        "expected 'import gleam/io as _' in debug output, got: {out}"
    );
}

#[test]
fn repl_let() {
    assert_eq!(repl_exec("let x = 10\nx + 1"), "10\n11");
    // No name collision with internal repl_main
    assert_eq!(repl_exec("let repl_main = 10"), "10");
    assert_eq!(
        repl_exec("let #(repl_main, b) = #(1, 2)\nrepl_main\nb"),
        "#(1, 2)\n1\n2"
    );
}
#[test]
fn repl_let_discard() {
    assert_eq!(repl_exec("let _ = True"), "True");
}

#[test]
fn repl_let_pattern() {
    assert_eq!(repl_exec("let #(a, b) = #(True, 1)"), "#(True, 1)");
    assert_eq!(repl_exec("let #(a, b) = #(True, 1) a"), "#(True, 1)\nTrue");
    assert_eq!(repl_exec("let #(a, b) = #(True, 1) b"), "#(True, 1)\n1");
}

#[test]
fn repl_let_nested_pattern() {
    assert_eq!(
        repl_exec("let assert #([f, ..r], a) = #([True], 1)"),
        "#([True], 1)"
    );
    assert_eq!(
        repl_exec("let assert #([f, ..r], a) = #([True], 1) f"),
        "#([True], 1)\nTrue"
    );
    assert_eq!(
        repl_exec("let assert #([f, ..r], a) = #([True], 1) r"),
        "#([True], 1)\n[]"
    );
    assert_eq!(
        repl_exec("let assert #([f, ..r], a) = #([True], 1) a"),
        "#([True], 1)\n1"
    );
}

#[test]
fn repl_rollback() {
    // When the second item in the same input fails, the first is rolled back
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("let x = 1 let y = x + \"a\"\nx"));
    assert!(
        err.contains("Type mismatch"),
        "expected type error for y, got: {err}"
    );
    assert!(
        err.contains("is not in scope"),
        "x should be rolled back, got: {err}"
    );
}

#[test]
fn repl_let_assert() {
    assert_eq!(repl_exec("let assert 2 = 1 + 1"), "2");
    assert_eq!(repl_exec("let assert 2 as var = 1 + 1 var"), "2\n2");
}

#[test]
fn repl_fn_replace_let() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            let a = 1
            fn a() {{ 2 }}
            a()
            let a = 3
            a"
        }),
        "1\n2\n3\n3"
    );
}

#[test]
fn repl_const_redefine() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            const x = 1
            const x = 2
            x"
        }),
        "2"
    );
}

#[test]
fn repl_type_redefine() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            pub type X {{ A }}
            pub type X {{ B(Int) }}
            B(1)"
        }),
        "B(1)"
    );
    // Types without pub are automatically made pub in the REPL
    assert_eq!(
        repl_exec(&formatdoc! {"
            type Color {{ Red Green Blue }}
            Red"
        }),
        "Red"
    );
    // Cannot redefine type while variables of that type exist
    assert_eq!(
        repl_exec(&formatdoc! {"
            type Val {{ A(Int) }}
            let x = A(42)
            type Val {{ B(String) }}
            x"
        }),
        "A(42)\nCannot redefine type `Val` while variables of that type exist.\nA(42)"
    );
}

#[test]
fn repl_const_replace_let() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            let x = 1
            const x = 2
            x"
        }),
        "1\n2"
    );
}

#[test]
fn repl_let_replace_const() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            const x = 1
            let x = 2
            x"
        }),
        "2\n2"
    );
}

#[test]
fn repl_fn() {
    assert_eq!(repl_exec("fn f(a) { a + 1 }\nf(1)"), "2");
}

#[test]
fn repl_fn_redefine() {
    // When f is redefined, g still calls the version of f that existed when g
    // was defined (functions are stored as runtime values, not recompiled from
    // source).
    assert_eq!(
        repl_exec(&formatdoc! {"
            fn f() {{ 1 }}
            fn g() {{ f() }}
            fn f() {{ 2 }}
            g()
            f()"
        }),
        "1\n2"
    );
}

#[test]
fn repl_fn_calling_fn() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            fn double(n) {{ n * 2 }}
            fn quadruple(n) {{ double(double(n)) }}
            quadruple(3)"
        }),
        "12"
    );
    // Mutual recursion (both functions on the same line = same run() call)
    assert_eq!(
        repl_exec("fn is_even(n) { case n { 0 -> True _ -> is_odd(n - 1) } } fn is_odd(n) { case n { 0 -> False _ -> is_even(n - 1) } }\nis_even(4)\nis_odd(3)"),
        "True\nTrue"
    );
}

#[test]
fn repl_fn_main() {
    assert_eq!(repl_exec("fn main() { 10 }\nmain()"), "10");
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
fn repl_fn_capture() {
    assert_eq!(
        repl_exec(&formatdoc! { r#"
            let a = 1
            let b = 2
            fn fun(a) {{
                a + b
            }}
            fun(10)
            "#
        }),
        "1\n2\n12"
    );
}

#[test]
fn repl_use() {
    assert_eq!(
        repl_exec("use x <- result.try(Ok(10))\nOk(x)"),
        "use statements are not supported outside blocks."
    );
    assert_eq!(repl_exec("{use x <- result.try(Ok(10))\nOk(x)}"), "Ok(10)");
}

#[test]
fn repl_quit() {
    assert_eq!(repl_exec(&format!("{QUIT}\n10")), "");
}

#[test]
fn repl_error_expr() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("a"));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_error_let() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some(r#"let x = 1 + "a""#));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_no_collision_with_internal_names() {
    // User variable named repl_print doesn't break expressions
    assert_eq!(
        repl_exec(&formatdoc! {"
            let repl_print = 10
            repl_print
            1 + 2"}),
        "10\n10\n3"
    );
    // User variable named repl_save doesn't break let bindings
    assert_eq!(
        repl_exec(&formatdoc! {"
            let repl_save = 10
            let x = 1
            x"}),
        "10\n1\n1"
    );
    // User function named repl_print works
    assert_eq!(
        repl_exec(&formatdoc! {"
            fn repl_print(x) {{ x + 1 }}
            repl_print(10)"}),
        "11"
    );
    // User variable named repl_main works
    assert_eq!(repl_exec("let repl_main = 42\nrepl_main"), "42\n42");
}

#[test]
fn repl_error_multiline_expr() {
    let (_, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {"
            1 + 2
            a"}),
    );
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_debug() {
    let (out, _) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(":debug\nlet x = 1\n:debug\nlet y = 2"),
    );
    // Debug on: output contains the generated code and the result
    assert!(
        out.contains("--- repl2_1.gleam ---"),
        "expected generated code header"
    );
    assert!(
        out.contains("pub fn repl_main_"),
        "expected repl_main in generated code"
    );
    assert!(out.contains("1"), "expected result");
    // Debug off: output contains only the result
    assert!(
        !out.contains("repl4_1.gleam"),
        "expected no generated code after :debug off"
    );
    assert!(out.contains("2"), "expected result");
}

#[test]
fn repl_type_cmd() {
    assert_eq!(repl_exec(&format!("{TYPE} 10")), "Int");
    assert_eq!(repl_exec(&format!("{TYPE} let a = True")), "Bool");
    // :type does not create variables
    let (out, err) = run_sgleam_cmd(&["repl", "-q"], Some(&format!("{TYPE} let x = 10\nx")));
    assert_eq!(out.trim(), "Int");
    assert!(
        err.contains("is not in scope"),
        "expected error for undefined x, got: {err}"
    );
    assert_eq!(repl_exec(&format!("{TYPE} int.add")), "fn(Int, Int) -> Int");
    assert_eq!(
        repl_exec(&format!("{TYPE} list.filter_map")),
        "fn(List(b), fn(b) -> Result(c, d)) -> List(c)"
    );
    // :type does not evaluate
    assert_eq!(
        repl_exec(&format!("{TYPE} {{ io.println(\"\") Ok(1) }}")),
        "Result(Int, b)", // without the io.println side effect
    );
}

#[test]
fn repl_type_cmd_multi() {
    assert_eq!(
        repl_exec(&format!("{TYPE} 1 False")),
        format!("{TYPE}command expects exactly one expression.")
    );
}

#[test]
fn repl_type_cmd_def() {
    assert_eq!(
        repl_exec(&format!("{TYPE} const a = 1")),
        format!("{TYPE}command cannot be used with definitions.")
    );
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
            &["repl", "-q", input],
            Some(&formatdoc! { "
                one
                two()
                let _: Three = Num3
                "
            })
        ),
        "1\n2\nNum3\n"
    );
}

#[test]
fn format_stdin() {
    assert_eq!(
        run_sgleam_cmd_stdout(
            &["format"],
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
    run_sgleam_cmd_stdout(&["repl", "-q"], Some(s))
        .strip_suffix('\n')
        .unwrap_or("")
        .into()
}

#[test]
fn smain_list_string() {
    let input = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/inputs/smain_list_string.gleam"
    );
    let (out, err) = run_sgleam_cmd(
        &["run", input],
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
    let (out, err) = run_sgleam_cmd(&["run", input], Some("hello\nworld"));
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

#[test]
fn error_output_has_ansi_colors() {
    // Use a file within the current directory to trigger a compile error with source location.
    // This exercises write_span() → codespan_reporting, which must emit ANSI codes.
    let file = std::env::current_dir()
        .unwrap()
        .join("tests/inputs/unknown_variable.gleam");
    std::fs::write(&file, "pub fn main() { unknown_variable }\n").unwrap();

    let output = assert_cmd::Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .expect("cargo bin")
        .env("FORCE_COLOR", "1")
        .arg("run")
        .arg(&file)
        .output()
        .unwrap();

    let _ = std::fs::remove_file(&file);

    assert!(
        output.stderr.contains(&0x1b),
        "expected ANSI escape codes in stderr, got: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_sgleam_cmd(args: &[&str], input: Option<&str>) -> (String, String) {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!();
    cmd.args(args);
    if let Some(input) = input {
        cmd.write_stdin(format!("{input}\n"));
    }
    let output = cmd.output().expect("run sgleam");
    (
        String::from_utf8_lossy(&output.stdout)
            .replace('\\', "/")
            .replace("\r\n", "\n"),
        String::from_utf8_lossy(&output.stderr)
            .replace('\\', "/")
            .replace("\r\n", "\n"),
    )
}
