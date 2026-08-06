use engine::repl::{QUIT, TYPE, welcome_message};
use indoc::formatdoc;
use insta::assert_snapshot;

/// Strip the random 8-hex suffix from internal REPL names so snapshot tests
/// are deterministic.
fn strip_repl_suffix(s: &str) -> String {
    let mut result = s.to_string();
    for prefix in ["repl_main_", "repl_print_", "repl_save_", "repl_load_"] {
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

// These tests launch the sgleam binary as a subprocess. Tests that only need
// Repl::run() can go in tests (which uses the capture feature).

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
    // Import with same short name shadows the old alias
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/io.{{println}}
            import sgleam/io
            io.input("") <> "ok""#
        }),
        r#""ok""#
    );
    // Explicit as avoids conflict
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import sgleam/io as sio
            sio.input("") <> "ok""#
        }),
        r#""ok""#
    );
    // Multiple import aliases used together
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
    // Alias that conflicts with another module's short name
    assert_eq!(
        repl_exec(&formatdoc! {r#"
            import gleam/int as io
            io.to_string(1)"#
        }),
        r#""1""#
    );
    // Multiple aliases for the same module
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
fn repl_import_discard_alias() {
    // `as _` brings in `input` without taking the `io` name from gleam/io.
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
    // No name collision with internal repl_main
    assert_eq!(repl_exec("let repl_main = 10"), "10");
    assert_eq!(
        repl_exec("let #(repl_main, b) = #(1, 2)\nrepl_main\nb"),
        "#(1, 2)\n1\n2"
    );
}

#[test]
fn repl_let_annotation() {
    // The annotation narrows what the inference alone would produce.
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
fn repl_rollback_failed_fn() {
    // A function is pre-registered before being compiled, so a failing one must
    // not survive into the next input.
    let (out, err) = run_sgleam_cmd(&["repl", "-q"], Some("fn g(a) { a + \"x\" }\nlet y = 2"));
    assert_eq!(err.matches("Type mismatch").count(), 1, "got: {err}");
    assert_eq!(out.trim(), "2");
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
    // Type with name that is substring of another type (e.g. In vs Int)
    // should NOT be blocked by variables of the longer type
    assert_eq!(
        repl_exec(&formatdoc! {"
            let x = 42
            type In {{ Inner }}
            Inner"
        }),
        "42\nInner"
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
    // `b` keeps the value it was defined with, the way a fn body would.
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
    // Only the redefined name changes: `a` becoming a variable leaves `b`
    // alone, and a new const can still read it.
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
    // A redefinition takes only its own name, like `let` and `fn`.
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
    // A `let` is out of reach of a const, as it would be in a source file.
    // Rejected before compiling, so the message is the repl's own.
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
fn repl_fn_redefine_recursive() {
    // A recursive call in the new definition reaches the new definition, not the
    // stored value of the old one.
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
    // A stored value reaches the generated module as a module constant, and the
    // type checker inlines a constant used in a guard.
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

// A stored value is a module constant, so the definition reaches the generated
// module whole: the head is still there and the lines still line up.
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
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("let x = "));
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
fn repl_error_import_module() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("import gleam/nope"));
    assert_snapshot!(strip_repl_suffix(&err));
}

// Each assertion kind reports the values it evaluated, and nothing else.
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
fn repl_let_result_does_not_warn() {
    // Saving the value must not warn about the unused `Result` it creates.
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("let r = Ok(1)"));
    assert_eq!(err, "");
}

// The pattern fails in both generated bindings; only the relocated one shows.
#[test]
fn repl_error_let_pattern() {
    let (_, err) = run_sgleam_cmd(&["repl", "-q"], Some("let assert Ok(v) = 1"));
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
    assert_eq!(
        repl_exec(&format!("import gleam/int\n{TYPE} int.add")),
        "fn(Int, Int) -> Int"
    );
    assert_eq!(
        repl_exec(&format!("import gleam/list\n{TYPE} list.filter_map")),
        "fn(List(b), fn(b) -> Result(c, d)) -> List(c)"
    );
    // :type does not evaluate
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
        "Error at tests/inputs/user.gleam (boom:27)\n  boom\nError: here\n"
    );
}

#[test]
fn repl_user_module_named_like_a_generated_one_keeps_the_location() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("repl1_0.gleam"),
        "pub fn boom() {\n  panic as \"boom\"\n}\n",
    )
    .unwrap();

    let out = assert_cmd::cargo::cargo_bin_cmd!()
        .current_dir(dir.path())
        .args(["repl", "-q", "repl1_0.gleam"])
        .write_stdin("boom()\n")
        .output()
        .expect("run sgleam")
        .stdout;

    assert_eq!(
        String::from_utf8_lossy(&out),
        "Error at repl1_0.gleam (boom:2)\n  boom\n"
    );
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
                .args(["check", path.to_str().unwrap()])
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
        .args(["repl", "-q", "tests/tests/images/world1.gleam"])
        .write_stdin("main()\n")
        .assert()
        .success();
}

#[test]
fn world_run_in_repl_number() {
    let root = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    assert_cmd::cargo::cargo_bin_cmd!()
        .current_dir(&root)
        .args(["repl", "-n", "-q", "tests/tests/images/world1.gleam"])
        .write_stdin("main()\n")
        .assert()
        .success();
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
            let s = strip_repl_suffix(s);
            normalize_warning_locations(&s)
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

#[cfg(feature = "wasm-backend")]
fn normalize_warning_locations(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' && s.len() > 0 {
            // no-op, handled below
        }
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

#[allow(dead_code)]
fn run_sgleam_cmd_native_only(args: &[&str], input: Option<&str>) -> (String, String) {
    run_native(args, input)
}
