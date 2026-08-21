use engine::repl::{QUIT, TIME, TYPE, welcome_message};
use indoc::{formatdoc, indoc};
use insta::assert_snapshot;

/// Strip the random 8-hex suffix from internal REPL names so snapshot tests
/// are deterministic.
fn strip_repl_suffix(s: &str) -> String {
    let mut result = s.to_string();
    for prefix in [
        "repl_main_",
        "repl_print_",
        "repl_memo_",
        "repl_vals_",
        "repl_value_",
    ] {
        let mut start = 0;
        while let Some(pos) = result[start..].find(prefix) {
            let abs_pos = start + pos;
            let suffix_start = abs_pos + prefix.len();
            if suffix_start + 8 <= result.len()
                && result[suffix_start..suffix_start + 8]
                    .chars()
                    .all(|c| c.is_ascii_hexdigit())
            {
                result.replace_range(suffix_start..suffix_start + 8, "XXXXXXXX");
            }
            start = suffix_start;
        }
    }
    result
}

/// The source of one generated module, out of what `:debug` printed.
fn debug_module<'a>(out: &'a str, name: &str) -> &'a str {
    out.split(&format!("--- {name}.gleam ---"))
        .nth(1)
        .and_then(|rest| rest.split("---").next())
        .unwrap_or_default()
}

// These tests launch the sgleam binary as a subprocess. What a file prints is
// snapshotted in integration.rs; tests that only need Repl's scope go in
// engine/tests/completion.rs.

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
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/int.{{to_string}}
            to_string(1)"#
        }),
        r#""1""#
    );
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
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/int.{{to_string as its}}
            its(42)"#
        }),
        r#""42""#
    );
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/int.{{to_string}}
            fn to_string(_x) {{ "custom" }}
            to_string(1)"#
        }),
        r#""custom""#
    );
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/int.{{to_string as its}}
            fn its(_x) {{ "custom" }}
            its(1)"#
        }),
        r#""custom""#
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            import gleam/option.{{type Option}}
            let x: Option(Int) = option.Some(1)
            x"
        }),
        "Some(1)\nSome(1)"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            import gleam/option.{{type Option}}
            type Option {{ Custom }}
            Custom"
        }),
        "Custom"
    );
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/io.{{println}}
            import sgleam/io
            io.input("") <> "ok""#
        }),
        r#""ok""#
    );
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import sgleam/io as sio
            sio.input("") <> "ok""#
        }),
        r#""ok""#
    );
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/int as i
            import gleam/float as f
            i.to_string(1)
            f.to_string(1.0)"#
        }),
        r#""1"
"1.0""#
    );
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/int as io
            io.to_string(1)"#
        }),
        r#""1""#
    );
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/int as i
            import gleam/int as j
            i.to_string(1)
            j.to_string(2)"#
        }),
        r#""1"
"2""#
    );
}

#[test]
fn repl_import_of_the_input_is_in_scope_for_its_definitions() {
    assert_eq!(
        repl_exec("import gleam/int fn f() { int.to_string(1) } f()"),
        r#""1""#
    );
    assert_eq!(
        repl_exec("import gleam/int.{to_string} fn f() { to_string(2) } f()"),
        r#""2""#
    );
    assert_eq!(
        repl_exec(
            "import gleam/option.{type Option} type Box { Box(Option(Int)) } Box(option.None)"
        ),
        "Box(None)"
    );
    assert_eq!(
        repl_exec(
            "import gleam/int.{max} const m = 3 fn f(x) { case x { n if n > m -> max(n, 0) _ -> 0 } } f(4)"
        ),
        "4"
    );
}

#[test]
fn repl_import_unqualified_survives_alias_shadow() {
    assert_snapshot!(repl_exec(&formatdoc! {r#"
        import gleam/io.{{println}}
        import sgleam/io
        println("hello")"#
    }));
}

#[test]
fn repl_import_re_import_restores_name() {
    assert_snapshot!(repl_exec(&formatdoc! {r#"
        import gleam/io.{{println}}
        import sgleam/io
        import gleam/io
        io.println("hello")"#
    }));
}

#[test]
fn repl_import_shadow_debug() {
    let (out, _) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {r#"
            :debug
            import sgleam/io
            io.input("") <> "ok""#
        }),
    );
    assert_snapshot!(strip_repl_suffix(&out));
}

#[test]
fn repl_import_var_shadows_unqualified() {
    assert_snapshot!(repl_exec(&formatdoc! {r#"
        import gleam/int.{{to_string}}
        to_string(1)
        let to_string = 42
        to_string"#
    }));
}

#[test]
fn repl_import_const_shadows_unqualified() {
    assert_snapshot!(repl_exec(&formatdoc! {r#"
        import gleam/int.{{to_string}}
        to_string(1)
        const to_string = "hi"
        to_string"#
    }));
}

#[test]
fn repl_import_fn_shadows_alias_then_reimport() {
    assert_snapshot!(repl_exec(&formatdoc! {r#"
        import gleam/io
        io.println("before")
        fn io() {{ 1 }}
        io()
        import gleam/io
        io.println("restored")"#}));
}

#[test]
fn repl_import_let_shadows_alias() {
    assert_snapshot!(repl_exec(&formatdoc! {r#"
        import gleam/int
        int.to_string(1)
        let int = 42
        int
        import gleam/int
        int.add(1, 2)"#}));
}

#[test]
fn repl_import_alias_shadows_module() {
    assert_snapshot!(repl_exec(&formatdoc! {r#"
        import gleam/io
        io.println("before")
        import gleam/int as io
        io.to_string(1)"#}));
}

#[test]
fn repl_import_two_aliases_for_one_module() {
    let (out, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {r#"
            import gleam/int
            import gleam/int as i
            int.to_string(1)
            i.to_string(2)"#
        }),
    );
    assert_eq!(err, "");
    assert_eq!(out, "\"1\"\n\"2\"\n");
}

#[test]
fn repl_import_discard_alias() {
    let (out, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {r#"
            import gleam/io
            import sgleam/io.{{input}} as _
            io.println("kept")
            {TYPE}input
            io.input("")"#
        }),
    );
    assert_eq!(out, "kept\nNil\nfn(String) -> String\n");
    assert!(err.contains("does not have a `input` value"), "{err}");
}

#[test]
fn repl_import_alias_then_unqualified() {
    assert_snapshot!(repl_exec(&formatdoc! {r#"
        import gleam/int as i
        i.to_string(1)
        import gleam/int.{{add}}
        add(2, 3)
        i.to_string(10)"#
    }));
}

#[test]
fn repl_import_unqualified_then_alias() {
    assert_snapshot!(repl_exec(&formatdoc! {r#"
        import gleam/int.{{to_string}}
        to_string(1)
        import gleam/int as i
        i.to_string(2)
        to_string(3)"#
    }));
}

#[test]
fn repl_import_unqualified_type_and_value() {
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/uri.{{type Uri, Uri}}
            import gleam/option.{{None}}
            let u: Uri = Uri(None, None, None, None, "/", None, None)
            u.path"#
        }),
        "Uri(scheme: None, userinfo: None, host: None, port: None, path: \"/\", query: None, fragment: None)\n\"/\""
    );
}

#[test]
fn repl_import_type_and_value_from_different_modules() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            import gleam/uri.{{type Uri}}
            import gleam/option.{{Some as Uri}}
            fn f(u: Uri) -> String {{ u.path }}
            Uri(1)"
        }),
        "Some(1)"
    );
}

#[test]
fn repl_import_type_again_keeps_the_value() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            import gleam/option.{{type Option, Some}}
            import gleam/option.{{type Option}}
            let o: Option(Int) = Some(1)
            o"
        }),
        "Some(1)\nSome(1)"
    );
}

#[test]
fn repl_type_and_imported_value_with_same_name() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            type Foo {{ Bar }}
            import gleam/option.{{Some as Foo}}
            let b: Foo = Bar
            Foo(1)"
        }),
        "Bar\nSome(1)"
    );
}

#[test]
fn repl_const_shadows_module_name() {
    assert_eq!(
        repl_exec("import gleam/list\nconst list = 1\nlist\nlist.length([1, 2])"),
        "1\n2"
    );
}

#[test]
fn repl_let() {
    assert_eq!(repl_exec("let x = 10\nx + 1"), "10\n11");
    assert_eq!(repl_exec("let repl_main = 10"), "10");
    assert_eq!(
        repl_exec("let #(repl_main, b) = #(1, 2)\nrepl_main\nb"),
        "#(1, 2)\n1\n2"
    );
}

#[test]
fn repl_let_annotation() {
    assert_eq!(repl_exec("let e: List(Int) = []\n:type e"), "[]\nList(Int)");
    let (out, err) = run_sgleam_cmd(&["repl", "-q"], Some("let w: Float = 1"));
    assert!(err.contains("Expected type:\n\n    Float"), "{err}");
    assert_eq!(out, "");
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
fn repl_let_string_prefix_pattern() {
    assert_eq!(
        repl_exec("let assert \"a\" <> rest = \"abc\" rest"),
        "\"abc\"\n\"bc\""
    );
    assert_eq!(
        repl_exec("let assert \"a\" as p <> rest = \"abc\" p rest"),
        "\"abc\"\n\"a\"\n\"bc\""
    );
    assert_eq!(repl_exec("let assert \"a\" <> _ = \"abc\""), "\"abc\"");
}

#[test]
fn repl_input_stops_at_the_first_error() {
    let (out, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {r#"
            let x = 1 let y = x + "a" let z = 3
            x
            y
            z"#
        }),
    );
    assert_eq!(out, "1\n1\n");
    assert_eq!(err.matches("Type mismatch").count(), 1, "got: {err}");
    assert_eq!(err.matches("is not in scope").count(), 2, "got: {err}");
}

#[test]
fn repl_input_stops_at_a_runtime_error() {
    let (out, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {r#"
            let a = 1 panic as "boom" let b = 2
            a
            b"#
        }),
    );
    assert_eq!(out, "1\nError at <repl>:1\n  boom\n1\n");
    assert!(err.contains("`b` is not in scope"), "got: {err}");
}

/// Told apart by the message quickjs-ng throws. With Bellard's quickjs the
/// message was another one, and this printed a raw RangeError.
///
/// Native only: `update_stack_limit` in quickjs.c leaves the limit at zero
/// under `__wasi__`, so the wasm build never raises this itself. The recursion
/// runs until the host stack is gone and the module traps, printing nothing.
#[test]
fn repl_stack_overflow_shows_the_frames() {
    let (out, err) = run_native(
        &["repl", "-q"],
        Some("pub fn f(x: Int) -> Int { 1 + f(x + 1) }\nf(1)"),
    );
    let frames = "  at f (/build/repl1.mjs:6:21)\n".repeat(8);
    assert_eq!(out, format!("Stack overflow\n{frames}  ...\n"));
    assert_eq!(err, "");
}

/// Ctrl-C was once told apart by message alone, so this panic also said
/// `Interrupted.`.
#[test]
fn repl_panic_saying_interrupted_is_still_a_panic() {
    let (out, err) = run_sgleam_cmd(&["repl", "-q"], Some(r#"panic as "interrupted""#));
    assert_eq!(out, "Error at <repl>:1\n  interrupted\n");
    assert_eq!(err, "");
}

#[test]
fn repl_rollback_failed_fn() {
    let (out, err) = run_sgleam_cmd(&["repl", "-q"], Some("fn g(a) { a + \"x\" }\nlet y = 2"));
    assert_eq!(err.matches("Type mismatch").count(), 1, "got: {err}");
    assert_eq!(out.trim(), "2");
}

#[test]
fn repl_rollback_drops_the_values_it_saved() {
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            let a = "hello" let b = bad_name
            let c = 5
            c + 1"#
        }),
        "\"hello\"\n5\n6"
    );
}

#[test]
fn repl_binding_that_did_not_run_is_not_bound() {
    let (out, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {r#"
            let a = 1 let z = bad_name
            let b = {{ panic as "boom" }}
            b"#
        }),
    );
    assert_eq!(out, "1\nError at <repl>:1\n  boom\n");
    assert!(err.contains("`b` is not in scope"), "got: {err}");
}

#[test]
fn repl_binding_that_raised_frees_its_slot_for_the_next() {
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            let x = 1
            let y = {{ panic as "boom" }}
            let y = 10
            y
            x"#
        }),
        "1\nError at <repl>:1\n  boom\n10\n10\n1"
    );
}

#[test]
fn repl_let_assert() {
    assert_eq!(repl_exec("let assert 2 = 1 + 1"), "2");
    assert_eq!(repl_exec("let assert 2 as var = 1 + 1 var"), "2\n2");
}

#[test]
fn repl_let_assert_message() {
    let message = |input: &str| run_sgleam_cmd(&["repl", "-q"], Some(input)).0;
    assert!(
        message(r#"let assert Ok(v) = Error(1) as "what the user wrote""#)
            .contains("  what the user wrote")
    );
    assert!(message(r#"let assert Ok(_) = Error(1) as "discarded""#).contains("  discarded"));
    assert!(
        message("let m = \"from a let\"\nlet assert Ok(v) = Error(1) as m")
            .contains("  from a let")
    );
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
    assert_eq!(
        repl_exec(&formatdoc! {"
            type Color {{ Red Green Blue }}
            Red"
        }),
        "Red"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            type Val {{ A(Int) }}
            let x = A(42)
            type Val {{ B(String) }}
            x
            {TYPE}x"
        }),
        "A(42)\nA(42)\nrepl1.Val"
    );
    let (_, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {"
            type Val {{ A(Int) }}
            let x = A(42)
            type Val {{ B(String) }}
            fn f(v: Val) {{ v }}
            f(x)"
        }),
    );
    assert!(err.contains("repl1.Val"), "{err}");
}

#[test]
fn repl_type_redefine_keeps_the_old() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            type A(x) {{ MkA(x) }}
            type B {{ MkB(A(Int)) }}
            type A {{ MkA2 }}
            MkB(MkA(1))"
        }),
        "MkB(MkA(1))"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            type A(x) {{ A(x) }}
            type B {{ B(A(Int)) }}
            type A {{ A }} type B {{ B(A) }}
            B(A)"
        }),
        "B(A)"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            type A(x) {{ MkA(x) }}
            type A {{ MkA2 }}
            let v = MkA(1)
            {TYPE}v"
        }),
        "MkA(1)\nrepl1.A(Int)"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            type A(x) {{ MkA(x) }}
            fn f(v: A(Int)) {{ v }}
            type A {{ MkA2 }}
            f(MkA(1))"
        }),
        "MkA(1)"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            type A {{ MkA(B) }} type B {{ MkB(Int) }}
            type B {{ MkB2 }}
            MkA(MkB(1))"
        }),
        "MkA(MkB(1))"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            type A(x) {{ A(x) }}
            let v = A(1)
            type A {{ A }} fn f() {{ A }}
            v
            f()"
        }),
        "A(1)\nA(1)\nA"
    );
}

#[test]
fn repl_fn_redefine_keeps_the_old() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            fn g() {{ 1 }}
            fn h() {{ g() * 10 }}
            fn g() {{ 100 }}
            h()
            g()"
        }),
        "10\n100"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            fn g() {{ 1 }}
            fn g() {{ 2 }}
            import repl1
            repl1.g()"
        }),
        "1"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            fn even(n) {{ case n {{ 0 -> True _ -> odd(n - 1) }} }} fn odd(n) {{ case n {{ 0 -> False _ -> even(n - 1) }} }}
            even(10)"
        }),
        "True"
    );
}

#[test]
fn repl_the_module_of_an_input_needs_no_import() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            fn g() {{ 1 }}
            fn g() {{ 2 }}
            #(repl1.g(), g())"
        }),
        "#(1, 2)"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            type T {{ A }}
            type T {{ A }}
            let y: repl1.T = repl1.A
            y"
        }),
        "A\nA"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            let x = 1
            let x = 2
            #(repl1.x(), x)"
        }),
        "1\n2\n#(1, 2)"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            fn q() {{ 1 }} let x = 1
            let x = 2
            #(repl1_1_vals.x(), x)"
        }),
        "1\n2\n#(1, 2)"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            import gleam/int fn q() {{ 1 }} let x = 1
            let x = 2
            #(repl1_2_vals.x(), x)"
        }),
        "1\n2\n#(1, 2)"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            fn g() {{ 1 }}
            import gleam/int as repl1
            repl1.to_string(9)"
        }),
        "\"9\""
    );
}

// What follows are the transcripts of docs/repl-redefinition.md: what changes
// here has to change there.

#[test]
fn doc_a_value_outlives_the_type_it_holds() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            type T {{ A B }}
            let x = A
            type T {{ C }}
            {TYPE}x"
        }),
        "A\nrepl1.T"
    );
}

#[test]
fn doc_the_old_and_the_new_do_not_mix() {
    let (_, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {"
            type T {{ A B }}
            let x = A
            type T {{ C }}
            fn f(v: T) {{ v }}
            f(x)"
        }),
    );
    assert!(
        err.contains(indoc! {"
            Expected type:

                T

            Found type:

                repl1.T"
        }),
        "{err}"
    );
}

#[test]
fn doc_a_redefinition_does_not_reach_back() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            let x = 1
            fn f(y) {{ x + y }}
            let x = 100
            f(1)"
        }),
        "1\n100\n2"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            fn g(n) {{ n + 1 }}
            fn h(n) {{ g(n) * 10 }}
            fn g(n) {{ n + 100 }}
            h(1)"
        }),
        "20"
    );
}

#[test]
fn doc_mutual_recursion_across_inputs_is_unbound() {
    let (_, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some("fn even(n) { case n { 0 -> True _ -> odd(n - 1) } }"),
    );
    assert!(
        err.contains("The name `odd` is not in scope here."),
        "{err}"
    );
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("type A { MkA(B) }"));
    assert!(err.contains("Unknown type"), "{err}");
}

#[test]
fn doc_a_fn_cannot_read_a_let_of_its_own_input() {
    let (out, err) = run_sgleam_cmd(&["repl", "-q"], Some("let x = 1 fn f() { x }\nx"));
    assert_eq!(out, "");
    assert_eq!(err.matches("is not in scope").count(), 2, "got: {err}");
}

#[test]
fn repl_error_type_beside_type() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("type A { A } type B { B(A(Int)) }"));
    assert_snapshot!(strip_repl_suffix(&err));
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
fn repl_let_shadows_a_const_another_const_uses() {
    let (out, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {"
            const a = 1
            const b = a
            let a = 5
            b
            a"
        }),
    );
    assert_eq!(err, "");
    assert_eq!(out, "5\n1\n5\n");
}

#[test]
fn repl_let_shadows_an_import_a_const_uses() {
    let (out, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {"
            import gleam/int.{{to_string}}
            const f = to_string
            let to_string = 5
            f(1)
            to_string"
        }),
    );
    assert_eq!(err, "");
    assert_eq!(out, "5\n\"1\"\n5\n");
}

#[test]
fn repl_const_redefined_keeps_the_value_of_who_used_it() {
    let (out, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {"
            const a = 1
            const b = a
            const a = 99
            b
            a"
        }),
    );
    assert_eq!(err, "");
    assert_eq!(out, "1\n99\n");
}

#[test]
fn repl_const_can_use_a_repl_type() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            type Color {{ Red Green }}
            const c = Red
            c"
        }),
        "Red"
    );
}

#[test]
fn repl_const_can_use_a_fn() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            fn g() {{ 1 }}
            const h = g
            h()"
        }),
        "1"
    );
}

#[test]
fn repl_const_survives_a_name_taken_over() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            const a = 1
            const b = 2
            let a = 5
            const c = b
            c"
        }),
        "5\n2"
    );
}

#[test]
fn repl_const_redefine_keeps_the_others() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            const a = 1
            const b = 2
            const a = 9
            const d = b
            d
            a"
        }),
        "2\n9"
    );
}

#[test]
fn repl_const_must_be_a_constant_expression() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("const bad = int.to_string(1)"));
    assert!(
        err.contains("Functions can only be called within other functions"),
        "{err}"
    );
    assert!(err.contains("const bad = int.to_string(1)"), "{err}");
}

#[test]
fn repl_const_cannot_reference_a_runtime_value() {
    let out = repl_exec(&formatdoc! {"
        let y = 1
        const c = y
        y"
    });
    assert_eq!(
        out,
        "1\n`y` is a variable, not a constant. A constant can only use \
         literals, other constants and functions.\n1"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            let option = 1
            import gleam/option
            const c = option.None
            c"
        }),
        "1\nNone"
    );
}

#[test]
fn repl_fn() {
    assert_eq!(repl_exec("fn f(a) { a + 1 }\nf(1)"), "2");
}

#[test]
fn repl_fn_redefine() {
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
fn repl_fn_attributes() {
    let (out, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {r#"
            @deprecated("use g") fn f() {{ 1 }}
            f()"#
        }),
    );
    assert_eq!(out.trim(), "1");
    assert!(err.contains("This value has been deprecated"), "{err}");
}

#[test]
fn repl_defines_a_name_the_other_namespace_holds() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            import gleam/option.{{type Option, Some}}
            type Some {{ Wrapped(Int) }} fn f() {{ Some(1) }}
            f()"
        }),
        "Some(1)"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            import gleam/option.{{type Option, Some}}
            type Foo {{ Option }} fn g(x: Option(Int)) {{ x }}
            g(Some(1))"
        }),
        "Some(1)"
    );
}

#[test]
fn repl_fn_external() {
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            @external(javascript, "./sgleam/sgleam_ffi.mjs", "repl_print") fn p(a: Int) -> Int
            p(7)"#
        }),
        "7\n7"
    );
}

#[test]
fn repl_fn_redefine_recursive() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            fn f(n) {{ 100 }}
            fn f(n) {{ case n {{ 0 -> 0 _ -> f(n - 1) + 1 }} }}
            f(3)"
        }),
        "3"
    );
}

#[test]
fn repl_value_in_guard() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            let x = 5
            fn f(n) {{ case n {{ m if m == x -> 1 _ -> 0 }} }}
            f(5)
            f(1)"
        }),
        "5\n1\n0"
    );
}

#[test]
fn repl_stored_value_runs_once() {
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/io
            let x = io.println("side")
            x
            fn f() {{ x }}
            f()"#
        }),
        "side\nNil\nNil\nNil"
    );
}

#[test]
fn repl_a_name_a_body_already_has_is_not_bound_again() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            let x = 1
            fn f(x) {{ x }}
            f(9)
            fn g() {{ let x = 2 x }}
            g()"
        }),
        "1\n9\n2"
    );
}

#[test]
fn repl_warns_inside_a_body_it_wrote_into() {
    let (_, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some("let z = 5\nfn f() {\n  let a = 1\n  z\n}"),
    );
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_const_in_guard_reaches_what_it_names() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            type T {{ A B }}
            const c = A
            let y = A
            case y {{ z if z == c -> \"eq\" _ -> \"ne\" }}"
        }),
        "A\n\"eq\""
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            type T {{ A B }}
            type P {{ P(T) }}
            const c = P(A)
            const d = c
            let y = P(A)
            case y {{ z if z == d -> \"eq\" _ -> \"ne\" }}"
        }),
        "P(A)\n\"eq\""
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            type T {{ A B }}
            type T {{ A B }}
            const c = repl1.A
            let y = repl1.A
            case y {{ z if z == c -> \"eq\" _ -> \"ne\" }}"
        }),
        "A\n\"eq\""
    );
}

#[test]
fn repl_const_update_in_guard_reaches_its_constructor() {
    let out = out_in_dir(
        &[(
            "u.gleam",
            "pub type P {\n  P(a: Int, b: Int)\n}\n\npub const base = P(1, 2)\n",
        )],
        &["repl", "-q", "u.gleam"],
        Some(&formatdoc! {"
            import u.{{type P, P, base}}
            const c = P(..base, b: 3)
            let y = P(1, 3)
            case y {{ z if z == c -> \"eq\" _ -> \"ne\" }}
        "}),
    );

    assert_eq!(out, "P(a: 1, b: 3)\n\"eq\"\n");
}

#[test]
fn repl_const_of_a_user_module_in_guard() {
    let out = out_in_dir(
        &[(
            "v.gleam",
            "pub type P {\n  P(Int)\n}\n\npub const c = P(1)\n",
        )],
        &["repl", "-q", "v.gleam"],
        Some(&formatdoc! {"
            let y = c
            case y {{ z if z == c -> \"eq\" _ -> \"ne\" }}
        "}),
    );

    assert_eq!(out, "P(1)\n\"eq\"\n");
}

#[test]
fn repl_loads_the_modules_the_file_imports() {
    let out = out_in_dir(
        &[
            (
                "util/mat.gleam",
                "pub fn double(x: Int) -> Int {\n  x * 2\n}\n",
            ),
            (
                "main.gleam",
                "import util/mat\n\npub fn main() {\n  mat.double(21)\n}\n",
            ),
        ],
        &["repl", "-q", "main.gleam"],
        Some("main()\nimport util/mat\nmat.double(1)\n"),
    );

    assert_eq!(out, "42\n2\n");
}

#[test]
fn repl_imports_only_what_the_input_writes() {
    let (out, _) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(":debug\nfn untouched() { 1 }\nlet x = 2\nx + 1\nuntouched()"),
    );
    let expr = debug_module(&out, "repl3_1");
    assert!(expr.contains("import repl2.{x}"), "{expr}");
    assert!(!expr.contains("untouched"), "{expr}");
    assert!(debug_module(&out, "repl4_1").contains("untouched"), "{out}");
}

#[test]
fn repl_imports_only_the_types_the_input_writes() {
    let (out, _) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(":debug\ntype T { A }\ntype U { B }\nlet x: T = A"),
    );
    let expr = debug_module(&out, "repl3_1");
    assert!(expr.contains("import repl1.{type T}"), "{expr}");
    assert!(!expr.contains("type U"), "{expr}");
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
    assert_eq!(
        repl_exec(
            "fn is_even(n) { case n { 0 -> True _ -> is_odd(n - 1) } } fn is_odd(n) { case n { 0 -> False _ -> is_even(n - 1) } }\nis_even(4)\nis_odd(3)"
        ),
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
        repl_exec("import gleam/list\nfn keep(_) { True }\nlist.filter([1, 2], keep)"),
        "[1, 2]"
    );
    assert_eq!(
        repl_exec("import gleam/list\nlet keep = fn (_) { True }\nlist.filter([1, 2], keep)"),
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
        repl_exec("import gleam/result\nuse x <- result.try(Ok(10))\nOk(x)"),
        "use statements are not supported outside blocks."
    );
    assert_eq!(
        repl_exec("import gleam/result\n{use x <- result.try(Ok(10))\nOk(x)}"),
        "Ok(10)"
    );
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
fn repl_error_let_annotation() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("let x: String = 2"));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_error_const() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some(r#"const x: Int = "a""#));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_error_type() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("type T { T(Nope) }"));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_error_fn_body() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some(r#"fn f(a) { a + "x" }"#));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_error_over_a_definition_head() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("fn f() { 1 } fn f() { 2 }"));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_error_fn_head_with_stored_value() {
    let (_, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {"
            let z = 5
            fn f(x: Nope) {{ x + z }}"
        }),
    );
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_error_multiline_fn() {
    let (_, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {"
            let z = 5
            fn f(x) {{
              let a = 1
              x + z + a + nope
            }}"
        }),
    );
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_error_syntax() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("let x = )"));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_unknown_command() {
    let (out, _) = run_sgleam_cmd(&["repl", "-q"], Some(":typ x\n:type\n:quit now"));
    assert_snapshot!(strip_repl_suffix(&out));
}

#[test]
fn repl_input_the_file_ends_in_the_middle_of() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("let x = 1\ncase x {"));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_error_assert() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some(r#"assert 1 == "a""#));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_error_type_cmd() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some(":type nope"));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_error_import_item() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("import gleam/int.{nope}"));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_error_import_item_among_others() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("import gleam/int.{to_string, nope}"));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_error_import_module() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("import gleam/nope"));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_assert_failure() {
    let (out, _) = run_sgleam_cmd(
        &["repl", "-q"],
        Some(&formatdoc! {r#"
            fn f(x, y) {{ x > y }}
            let a = False
            let n = 1
            assert n == 2
            assert a || a
            assert True && a
            assert f(1, 2)
            let assert Ok(v) = Error("boom")"#
        }),
    );
    assert_snapshot!(strip_repl_suffix(&out));
}

#[test]
fn repl_warning() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("assert 1 == 1"));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_warns_about_what_the_user_wrote() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("fn f() {\n  let a = 1\n  todo\n}"));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_warns_once_about_a_pattern_it_copies() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("let assert x = 1\nx"));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_does_not_warn_about_its_own_scaffolding() {
    let (_, err) = run_sgleam_cmd(
        &["repl", "-q"],
        Some("import gleam/int\nimport gleam/list.{length}\nlet x = 1\nx"),
    );
    assert_eq!(err, "");
}

#[test]
fn repl_let_result_does_not_warn() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("let r = Ok(1)"));
    assert_eq!(err, "");
}

#[test]
fn repl_error_let_pattern() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("let assert Ok(v) = 1"));
    assert_snapshot!(strip_repl_suffix(&err));
}

#[test]
fn repl_no_collision_with_internal_names() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            let repl_print = 10
            repl_print
            1 + 2"}),
        "10\n10\n3"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            let repl_save = 10
            let x = 1
            x"}),
        "10\n1\n1"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            fn repl_print(x) {{ x + 1 }}
            repl_print(10)"}),
        "11"
    );
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
    assert!(
        out.contains("--- repl1_1.gleam ---"),
        "expected generated code header"
    );
    assert!(
        out.contains("pub fn repl_main_"),
        "expected repl_main in generated code"
    );
    assert!(out.contains("1"), "expected result");
    assert!(
        !out.contains("repl2_1.gleam"),
        "expected no generated code after :debug off"
    );
    assert!(out.contains("2"), "expected result");
}

#[test]
fn repl_type_cmd() {
    assert_eq!(repl_exec(&format!("{TYPE} 10")), "Int");
    assert_eq!(repl_exec(&format!("{TYPE} let a = True")), "Bool");
    let (out, err) = run_sgleam_cmd(&["repl", "-q"], Some(&format!("{TYPE} let x = 10\nx")));
    assert_eq!(out.trim(), "Int");
    assert!(
        err.contains("is not in scope"),
        "expected error for undefined x, got: {err}"
    );
    assert_eq!(
        repl_exec(&format!("import gleam/int\n{TYPE} int.add")),
        "fn(Int, Int) -> Int"
    );
    assert_eq!(
        repl_exec(&format!("import gleam/list\n{TYPE} list.filter_map")),
        "fn(List(b), fn(b) -> Result(c, d)) -> List(c)"
    );
    assert_eq!(
        repl_exec(&format!(
            "import gleam/io\n{TYPE} {{ io.println(\"\") Ok(1) }}"
        )),
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
fn repl_time_cmd() {
    let (out, _) = run_sgleam_cmd(&["repl", "-q"], Some(&format!("{TIME} let x = 10\nx")));
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines[0], "10");
    assert!(lines[1].starts_with("Time: "), "got: {out}");
    assert_eq!(lines[2], "10");
}

#[test]
fn repl_time_cmd_error() {
    let (out, _) = run_sgleam_cmd(&["repl", "-q"], Some(&format!("{TIME} panic as \"boom\"")));
    assert_eq!(out, "Error at <repl>:1\n  boom\n");
}

#[test]
fn repl_time_cmd_def() {
    assert_eq!(
        repl_exec(&format!("{TIME} const a = 1")),
        format!("{TIME}command cannot be used with definitions.")
    );
}

#[test]
fn repl_type_module() {
    assert_eq!(
        repl_exec(&format!(
            "import gleam/list\ntype List {{}}\n{TYPE} list.map"
        )),
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
                import gleam/list
                one
                two()
                let _: Three = Num3
                let _: Pair = Pair(1, 2)
                user.two()
                user()
                list(7)
                list.length([1, 2])
                "
            })
        ),
        "1\n2\nNum3\nPair(1, 2)\n2\n\"self\"\n7\n2\n"
    );
}

/// Where the input ends is the parser's answer, not the reader's guess. A
/// bracket inside a comment opens nothing, and a reader waiting for it to close
/// waits forever, swallowing every line after it.
#[test]
fn repl_reads_to_the_end_of_the_item() {
    assert_eq!(repl_exec("1 + 1 // {\n5\n6"), "2\n5\n6");
    assert_eq!(repl_exec("fn f(x) {\n  x + 1\n}\nf(1)"), "2");
    assert_eq!(
        repl_exec("import gleam/io\nio.println(\"a\nb\")"),
        "a\nb\nNil"
    );
    assert_eq!(repl_exec("[1,\n2]"), "[1, 2]");
    assert_eq!(repl_exec(":type case Ok(1) {\n  _ -> 2\n}"), "Int");
}

#[test]
fn repl_error_in_a_module_that_ran_nothing() {
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            fn f() {{ panic as "boom" }}
            f()"#
        }),
        "Error at <repl>:1\n  boom"
    );
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            fn g() {{
              panic as "old"
            }}
            fn h() {{ g() }}
            h()"#
        }),
        "Error at <repl>:2\n  old"
    );
}

#[test]
fn repl_echo_is_located_in_the_input() {
    assert_eq!(repl_exec("echo 42"), "<repl>:1\n42\n42");
    assert_eq!(repl_exec(r#"echo 42 as "nota""#), "<repl>:1 nota\n42\n42");
    assert_eq!(
        repl_exec("fn g(x) {\n  echo x\n  echo x * 2\n}\ng(5)"),
        "<repl>:2\n5\n<repl>:3\n10\n10"
    );
}

#[test]
fn repl_echo_of_an_earlier_input_is_still_located() {
    assert_eq!(
        repl_exec(&formatdoc! {"
            fn g(x) {{
              echo x
            }}
            import gleam/int
            import gleam/float
            g(5)"
        }),
        "<repl>:2\n5\n5"
    );
    assert_eq!(
        repl_exec(&formatdoc! {"
            let f = fn() {{ echo 7 }}
            import gleam/list
            f()"
        }),
        "//fn() { ... }\n<repl>:1\n7\n7"
    );
}

#[test]
fn user_module_echo_keeps_the_location() {
    let out = out_in_dir(
        &[(
            "eu.gleam",
            "pub fn main() {\n  echo 42\n  echo 7 as \"nota\"\n}\n",
        )],
        &["eu.gleam"],
        None,
    );

    assert_eq!(out, "eu.gleam:2\n42\neu.gleam:3 nota\n7\n");
}

/// `sgleam run x.gleam | head` leaves the program printing to a pipe no one
/// reads. The write fails, and a failed print used to panic, which the hook
/// reports as a bug in the gleam compiler for the student to go and file.
#[test]
#[cfg(unix)]
fn a_reader_that_went_away_is_not_a_crash() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(
        dir.path().join("count.gleam"),
        indoc! {"
            import gleam/int
            import gleam/io

            pub fn count(n: Int) -> Nil {
              case n {
                0 -> Nil
                _ -> {
                  io.println(int.to_string(n))
                  count(n - 1)
                }
              }
            }

            pub fn main() {
              count(200000)
            }
        "},
    )
    .expect("write the module");

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_sgleam"))
        .current_dir(dir.path())
        .args(["run", "count.gleam"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("run sgleam");
    // Closed while the file is still being compiled, so every print of the
    // program lands on it.
    drop(child.stdout.take());
    let out = child.wait_with_output().expect("wait for sgleam");

    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
}

#[test]
fn repl_runtime_error_is_located_in_the_input() {
    assert_eq!(repl_exec(r#"panic as "boom""#), "Error at <repl>:1\n  boom");
    assert_eq!(
        repl_exec("fn f(x) {\n  let y = x + 1\n  panic as \"deep\"\n}\nf(1)"),
        "Error at <repl>:3\n  deep"
    );
    assert_eq!(
        repl_exec("let assert Ok(v) = Error(1)"),
        "Error at <repl>:1\n  Pattern match failed, no pattern matched the value.\n  value: Error(1)"
    );
    assert_eq!(
        repl_exec("fn f() {\n  assert 1 == 2\n}\nf()"),
        "Error at <repl>:2\n  Assertion failed.\n  operator: ==\n  left: 1\n  right: 2"
    );
}

#[test]
fn repl_user_module_error_keeps_the_location() {
    // Native only: the wasm backend loads a file by its base name, so the very
    // path this test is about is what the backends disagree on.
    let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/inputs/user.gleam");
    let (out, _) = run_native(
        &["repl", "-q", input],
        Some(&formatdoc! { "
            boom()
            panic as \"here\"
            "
        }),
    );
    assert_eq!(
        out,
        "Error at tests/inputs/user.gleam (boom:29)\n  boom\nError at <repl>:1\n  here\n"
    );
}

#[test]
fn repl_user_module_named_like_a_generated_one_keeps_the_location() {
    for name in ["repl0.gleam", "repl1.gleam"] {
        let out = out_in_dir(
            &[(name, "pub fn boom() {\n  panic as \"boom\"\n}\n")],
            &["repl", "-q", name],
            Some("fn g() { 7 }\ng()\nboom()\n"),
        );

        assert_eq!(out, format!("7\nError at {name} (boom:2)\n  boom\n"));
    }
}

#[test]
fn repl_let_of_a_type_from_a_module_out_of_scope() {
    let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/inputs/user.gleam");
    let (out, err) = run_sgleam_cmd(
        &["repl", "-q", input],
        Some("let x = maybe()\n:type x\nlet y = x\ny"),
    );
    assert_eq!(err, "");
    assert_eq!(out, "None\noption.Option(Int)\nNone\nNone\n");
}

#[test]
fn repl_let_of_a_shadowed_prelude_type() {
    assert_eq!(
        repl_exec("type List { L }\nlet x = [1]\n:type x\nx\nlet y = L\ny"),
        "[1]\ngleam.List(Int)\n[1]\nL\nL"
    );
}

#[test]
fn repl_let_of_a_type_whose_module_name_the_user_took() {
    assert_eq!(
        repl_exec("import gleam/int as gleam\ntype List { L }\nlet x = [1]\nx\n:type x"),
        "[1]\n[1]\ngleam.List(Int)"
    );
}

#[test]
fn repl_let_of_two_types_whose_modules_share_a_name() {
    let out = out_in_dir(
        &[(
            "option.gleam",
            "import gleam/option as opt\n\npub type Mine {\n  Mine\n}\n\n\
             pub fn mine() {\n  Mine\n}\n\npub fn maybe() -> opt.Option(Int) {\n  opt.None\n}\n",
        )],
        &["repl", "-q", "option.gleam"],
        Some("let t = #(mine(), maybe())\nt\n"),
    );

    assert_eq!(out, "#(Mine, None)\n#(Mine, None)\n");
}

#[test]
fn repl_user_module_shadow() {
    let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/inputs/user.gleam");
    let (out, err) = run_sgleam_cmd(
        &["repl", "-q", input],
        Some(&formatdoc! { "
            type Three {{ Num0 }}
            let _: Three = Num0
            fn two() {{ 22 }}
            two()
            const one = 11
            one
            "
        }),
    );
    assert_eq!(err, "");
    assert_eq!(out, "Num0\n22\n11\n");
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

// `smain(String)` / `smain(List(String))` entry points require `run_main`'s
// dispatch based on the detected smain signature. The WASM wrapper only calls
// `main()`, so these stay native-only.
#[test]
fn smain_list_string() {
    let input = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/inputs/smain_list_string.gleam"
    );
    let (out, err) = run_sgleam_cmd_native_only(
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
    let (out, err) = run_sgleam_cmd_native_only(&["run", input], Some("hello\nworld"));
    assert_snapshot!(formatdoc! {"
        STDOUT
        {out}
        STDERR
        {err}"
    });
}

#[test]
fn smain_type_alias() {
    let input = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/inputs/smain_alias.gleam"
    );
    let (out, _err) = run_sgleam_cmd_native_only(&["run", input], Some("hello"));
    assert_eq!(out.trim(), "hello");
}

// Bit array offsets and sizes are plain JavaScript numbers, so they must be
// generated the same way with and without BigInt integers.
#[test]
fn run_bit_array() {
    let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/inputs/bit_array.gleam");
    let expected = formatdoc! {"
        <<1, 2>>
        <<1, 44>>
        <<1, 44>>
        <<63, 192, 0, 0>>
        <<0, 104, 0, 105>>
        7
        258
        -1
        258
        <<20, 30>>
        <<9, 9>>
        3
        42"
    };
    assert_eq!(
        run_sgleam_cmd_native_only(&["run", input], None).0.trim(),
        expected
    );
    assert_eq!(
        run_sgleam_cmd_native_only(&["run", "-n", input], None)
            .0
            .trim(),
        expected
    );
}

fn run_sgleam_cmd_stdout(args: &[&str], input: Option<&str>) -> String {
    run_sgleam_cmd(args, input).0
}

#[test]
fn error_output_has_ansi_colors() {
    // Use a file within the current directory to trigger a compile error with source location.
    // This exercises write_span() → codespan_reporting, which must emit ANSI codes.
    let file = std::env::current_dir()
        .expect("the current directory")
        .join("tests/inputs/unknown_variable.gleam");
    std::fs::write(&file, "pub fn main() { unknown_variable }\n").expect("write the module");

    let output = assert_cmd::Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .expect("cargo bin")
        .env("FORCE_COLOR", "1")
        .arg("run")
        .arg(&file)
        .output()
        .expect("run sgleam");

    let _ = std::fs::remove_file(&file);

    assert!(
        output.stderr.contains(&0x1b),
        "expected ANSI escape codes in stderr, got: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn examples_compile() {
    let project_root = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .canonicalize()
        .expect("canonicalize project root");
    let examples_dir = project_root.join("examples");
    for entry in std::fs::read_dir(&examples_dir).expect("read examples dir") {
        let path = entry.expect("read entry").path();
        if path.extension().and_then(|e| e.to_str()) == Some("gleam") {
            let output = assert_cmd::cargo::cargo_bin_cmd!()
                .current_dir(&project_root)
                .args(["check", path.to_str().expect("a utf8 path")])
                .output()
                .expect("run sgleam check");
            let err = String::from_utf8_lossy(&output.stderr);
            assert!(
                !err.contains("error:"),
                "example {} failed to compile:\n{err}",
                path.display()
            );
        }
    }
}

/// Every backend agreement test until now is a repl session: the file
/// commands print through the same JS and were never compared. Listing the
/// file is the whole test — `run_sgleam_cmd` runs both backends and asserts
/// they agree.
///
/// The paths are relative to the crate directory, which is the cwd of both:
/// neither `run_native` nor `run_wasm` sets one.
#[test]
fn test_output_agrees_between_backends() {
    // Not `check_todo_panic_stackoverflow`: the wasm build never raises the
    // overflow itself, as `repl_stack_overflow_shows_the_frames` says.
    for file in [
        "tests/inputs/check_eq.gleam",
        "tests/inputs/check_approx.gleam",
        "tests/inputs/check_true_false.gleam",
        "tests/inputs/check_progress.gleam",
    ] {
        run_sgleam_cmd(&["test", file], None);
    }
}

#[test]
fn check_output_agrees_between_backends() {
    // Not a file with examples: the wasm wrapper reaches `check` through
    // `repl_new`, which runs them. Nor `Invalid.gleam`, whose whole diagnostic
    // is about a file name the wasm library is never given, nor `gleam.gleam`.
    for file in [
        "tests/inputs/hello_world.gleam",
        "tests/inputs/import_unknown.gleam",
        "tests/inputs/main_todo.gleam",
    ] {
        run_sgleam_cmd(&["check", file], None);
    }
}

#[test]
fn check_location_is_relative_to_the_test() {
    let out = out_in_dir(
        &[
            (
                "helper.gleam",
                indoc! {"
                    pub fn boom() -> Int {
                      panic as \"boom\"
                    }
                "},
            ),
            (
                "t.gleam",
                indoc! {"
                    import helper
                    import sgleam/check

                    pub fn boom_examples() {
                      check.eq(helper.boom(), 1)
                      check.eq(1 + 1, 3)
                    }
                "},
            ),
        ],
        &["test", "t.gleam"],
        None,
    );

    assert!(out.contains("  t.gleam/boom_examples\n"), "got: {out}");
    assert!(out.contains("Error at helper.gleam (boom:2)"), "got: {out}");
    assert!(out.contains("Failure at line 6"), "got: {out}");
}

/// What every other test runner says in its exit code, and all a CI job has
/// of the run to read.
#[test]
fn failing_tests_exit_with_nonzero() {
    let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/inputs/check_eq.gleam");
    let output = assert_cmd::cargo::cargo_bin_cmd!()
        .args(["test", input])
        .output()
        .expect("run sgleam");
    assert!(
        !output.status.success(),
        "expected non-zero exit code for a failed check"
    );
}

#[test]
fn passing_tests_exit_with_zero() {
    let status = run_in_dir(
        &[(
            "ok.gleam",
            indoc! {"
                import sgleam/check

                pub fn add_examples() {
                  check.eq(1 + 1, 2)
                }
            "},
        )],
        &["test", "ok.gleam"],
        None,
    )
    .status;

    assert!(status.success(), "expected a zero exit code, got: {status}");
}

#[test]
fn runtime_error_exits_with_nonzero() {
    let input = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/inputs/runtime_panic.gleam"
    );
    let output = assert_cmd::cargo::cargo_bin_cmd!()
        .args(["run", input])
        .output()
        .expect("run sgleam");
    assert!(
        !output.status.success(),
        "expected non-zero exit code for runtime error"
    );
}

// world.run() calls sleep via sgleam_ffi.mjs → sgleam.sleep (rquickjs).
// Regression test: the WASM import was renamed from "sleep" to "sgleam_sleep"
// to avoid collision with the POSIX sleep symbol in wasm32-wasip1.
#[test]
fn world_run_in_repl_bigint() {
    let root = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    assert_cmd::cargo::cargo_bin_cmd!()
        .current_dir(&root)
        .args(["repl", "-q", "cli/tests/images/world1.gleam"])
        .write_stdin("main()\n")
        .assert()
        .success();
}

#[test]
fn world_run_in_repl_number() {
    let root = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    assert_cmd::cargo::cargo_bin_cmd!()
        .current_dir(&root)
        .args(["repl", "-n", "-q", "cli/tests/images/world1.gleam"])
        .write_stdin("main()\n")
        .assert()
        .success();
}

/// A project of its own: the files written into a fresh directory, and the
/// binary run there. A test that needs a module of the user's needs somewhere
/// to put it, which is most of what writing one takes.
fn run_in_dir(files: &[(&str, &str)], args: &[&str], input: Option<&str>) -> std::process::Output {
    let dir = tempfile::tempdir().expect("a temporary directory");
    for (name, source) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create the directory");
        }
        std::fs::write(path, source).expect("write the module");
    }
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!();
    cmd.current_dir(dir.path()).args(args);
    if let Some(input) = input {
        cmd.write_stdin(input.to_string());
    }
    cmd.output().expect("run sgleam")
}

/// What `run_in_dir` printed.
fn out_in_dir(files: &[(&str, &str)], args: &[&str], input: Option<&str>) -> String {
    String::from_utf8_lossy(&run_in_dir(files, args, input).stdout).into_owned()
}

fn run_native(args: &[&str], input: Option<&str>) -> (String, String) {
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

#[cfg(feature = "wasm-backend")]
fn run_wasm(args: &[&str], input: Option<&str>) -> (String, String) {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("canonicalize project root");
    let sgleam_ts = project_root.join("wasm/sgleam.ts");
    let mut cmd = std::process::Command::new("deno");
    cmd.args([
        "run",
        "--quiet",
        "--allow-read",
        sgleam_ts.to_str().expect("utf8"),
    ]);
    cmd.args(args);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().expect("spawn deno");
    if let Some(input) = input {
        use std::io::Write;
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin
            .write_all(format!("{input}\n").as_bytes())
            .expect("write stdin");
    }
    let output = child.wait_with_output().expect("wait deno");
    (
        String::from_utf8_lossy(&output.stdout)
            .replace('\\', "/")
            .replace("\r\n", "\n"),
        String::from_utf8_lossy(&output.stderr)
            .replace('\\', "/")
            .replace("\r\n", "\n"),
    )
}

fn run_sgleam_cmd(args: &[&str], input: Option<&str>) -> (String, String) {
    let native = run_native(args, input);
    #[cfg(feature = "wasm-backend")]
    {
        let wasm = run_wasm(args, input);
        // Random per-run suffixes (`repl_main_XXXXXXXX`, etc.) and warning
        // line numbers depending on HashMap iteration order would cause
        // spurious diffs between backends.
        let normalize = |s: &str| {
            let s = normalize_user_module(&strip_repl_suffix(s), args);
            normalize_durations(&normalize_warning_locations(&s))
        };
        assert_eq!(
            normalize(&native.0),
            normalize(&wasm.0),
            "stdout: native vs wasm differ for args {args:?}"
        );
        assert_eq!(
            normalize(&native.1),
            normalize(&wasm.1),
            "stderr: native vs wasm differ for args {args:?}"
        );
    }
    native
}

/// The wasm library is given source and not a path, so everything it compiles
/// is `user.gleam`. The name of the file is the one thing the two backends
/// cannot be made to agree on without changing what the library is asked for.
#[cfg(feature = "wasm-backend")]
fn normalize_user_module(s: &str, args: &[&str]) -> String {
    let mut s = s.to_string();
    for arg in args {
        if arg.ends_with(".gleam") {
            s = s.replace(arg, "user.gleam");
        }
    }
    s
}

#[cfg(feature = "wasm-backend")]
fn normalize_warning_locations(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        result.push(c);
        // Detect pattern like ".gleam:NN:MM" and replace numbers
        if result.ends_with(".gleam:") {
            // consume and replace digits
            while matches!(chars.peek(), Some(ch) if ch.is_ascii_digit()) {
                chars.next();
            }
            result.push('N');
            if chars.peek() == Some(&':') {
                chars.next();
                result.push(':');
                while matches!(chars.peek(), Some(ch) if ch.is_ascii_digit()) {
                    chars.next();
                }
                result.push('N');
            }
        }
    }
    // Also normalize "┌─ /src/... :NN:MM" already handled by .gleam prefix.
    // Normalize leading line-number gutter in error/warning diagnostics:
    //   "42 │ ..." → "NN │ ..."  (these differ because of non-deterministic
    //   HashMap ordering that shifts code up/down a few lines)
    let mut lines: Vec<String> = Vec::new();
    for line in result.lines() {
        let trimmed = line.trim_start();
        let indent_len = line.len() - trimmed.len();
        if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
            let rest_trimmed = rest.trim_start_matches(|c: char| c.is_ascii_digit());
            if rest_trimmed.starts_with(" │") || rest_trimmed.starts_with(" \u{2502}") {
                let mut out = String::new();
                out.push_str(&line[..indent_len]);
                out.push_str("NN");
                out.push_str(rest_trimmed);
                lines.push(out);
                continue;
            }
        }
        lines.push(line.to_string());
    }
    let mut joined = lines.join("\n");
    if result.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// `:time` reports a measurement, which no two runs agree on.
#[cfg(feature = "wasm-backend")]
fn normalize_durations(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for line in s.split_inclusive('\n') {
        if line.starts_with("Time: ") {
            result.push_str("Time: N");
            if line.ends_with('\n') {
                result.push('\n');
            }
        } else {
            result.push_str(line);
        }
    }
    result
}

#[allow(dead_code)]
fn run_sgleam_cmd_native_only(args: &[&str], input: Option<&str>) -> (String, String) {
    run_native(args, input)
}
