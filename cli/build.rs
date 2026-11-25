use std::{env, fmt::Write, fs, path::Path};

/// One test per input file, so they run in parallel: a single test walking the
/// directory is as slow as the sum of its files.
fn main() {
    println!("cargo:rerun-if-changed=tests/inputs");
    println!("cargo:rerun-if-changed=tests/images");

    let mut src = String::new();
    let all: fn(&str) -> bool = |_| true;
    let checks: fn(&str) -> bool = |name| name.starts_with("check");
    // Each row is the directory, the snapshot, the command, the function of
    // `integration.rs` that runs the test, and the files to keep. An image is
    // a whole snapshot of its own: the svg, and nothing around it.
    #[rustfmt::skip]
    let rows = [
        ("tests/inputs", "run_file", "run", "check", all),
        ("tests/inputs", "run_tests", "test", "check_test", checks),
        ("tests/images", "run_images", "run", "check_image", all),
    ];
    for (dir, snapshot, command, check, keep) in rows {
        tests(&mut src, dir, snapshot, command, check, keep);
    }

    let out = env::var("OUT_DIR").expect("OUT_DIR");
    fs::write(Path::new(&out).join("file_tests.rs"), src).expect("write the generated tests");

    static_libs(Path::new(&out));
}

/// The libraries that winit and softbuffer open, in the order of the link.
const STATIC_LIBS: &[&str] = &[
    "X11-xcb",
    "X11",
    "Xcursor",
    "Xi",
    "Xext",
    "Xrender",
    "Xfixes",
    "xkbcommon-x11",
    "xkbcommon",
    "wayland-client",
    "ffi",
    "xcb-xkb",
    "xcb",
    "Xau",
    "Xdmcp",
];

/// Links the libraries of `SGLEAM_STATIC_LIBS` into a static musl binary,
/// with the dlopen of static/dlshim.c that finds their symbols.
fn static_libs(out: &Path) {
    println!("cargo:rerun-if-env-changed=SGLEAM_STATIC_LIBS");
    println!("cargo:rerun-if-changed=../static/dlshim.c");
    let Ok(dir) = env::var("SGLEAM_STATIC_LIBS") else {
        return;
    };
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("musl") {
        return;
    }
    let mut names = std::collections::BTreeSet::new();
    for lib in STATIC_LIBS {
        let path = Path::new(&dir).join(format!("lib{lib}.a"));
        println!("cargo:rerun-if-changed={}", path.display());
        let nm = std::process::Command::new("nm")
            .args(["-g", "--defined-only", "-P"])
            .arg(&path)
            .output()
            .expect("run nm");
        assert!(nm.status.success(), "nm {}", path.display());
        for line in String::from_utf8_lossy(&nm.stdout).lines() {
            let mut fields = line.split_whitespace();
            let (Some(name), Some(kind)) = (fields.next(), fields.next()) else {
                continue;
            };
            if name.ends_with(':') || !"TDBRVWC".contains(kind) {
                continue;
            }
            names.insert(name.to_string());
        }
    }
    let externs: String = names
        .iter()
        .map(|n| format!("extern char {n};\n"))
        .collect();
    let symbols: String = names
        .iter()
        .map(|n| format!("    {{\"{n}\", &{n}}},\n"))
        .collect();
    let libraries: String = STATIC_LIBS
        .iter()
        .map(|l| format!("    \"lib{l}\",\n"))
        .collect();
    let table = format!(
        "{externs}static const struct dlshim_symbol dlshim_symbols[] = {{\n{symbols}}};\n\
         static const char *const dlshim_libraries[] = {{\n{libraries}}};\n"
    );
    fs::write(out.join("dltable.inc"), table).expect("write dltable.inc");

    cc::Build::new()
        .file("../static/dlshim.c")
        .include(out)
        .warnings(false)
        // The weak dlopen of libc.a must not win over this one.
        .link_lib_modifier("+whole-archive")
        .compile("dlshim");
    println!("cargo:rustc-link-search=native={dir}");
    println!("cargo:rustc-link-arg-bins=-Wl,--start-group");
    for lib in STATIC_LIBS {
        println!("cargo:rustc-link-arg-bins=-l:lib{lib}.a");
    }
    // The link arguments come after the libc of the target, so the group
    // names it again for what the libraries call. The libgcc of the
    // directory, from the compiler that built the libraries, has the
    // builtins they call, such as __popcountdi2.
    println!("cargo:rustc-link-arg-bins=-lc");
    let libgcc = Path::new(&dir).join("libgcc.a");
    assert!(libgcc.exists(), "no {}", libgcc.display());
    println!("cargo:rerun-if-changed={}", libgcc.display());
    println!("cargo:rustc-link-arg-bins={}", libgcc.display());
    println!("cargo:rustc-link-arg-bins=-Wl,--end-group");
}

fn tests(
    src: &mut String,
    dir: &str,
    snapshot: &str,
    command: &str,
    check: &str,
    keep: fn(&str) -> bool,
) {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {dir}: {e}"))
        .map(|entry| entry.expect("a directory entry").file_name())
        .map(|name| name.to_str().expect("a utf8 file name").to_string())
        .filter(|name| name.ends_with(".gleam") && keep(name))
        .collect();
    names.sort();

    for name in names {
        let ident = format!("{snapshot}_{}", sanitize(&name));
        // `.invalid.gleam` is a file name, and the test is named after it.
        if ident.contains("__") {
            let _ = writeln!(src, "#[allow(non_snake_case)]");
        }
        // The deep recursion needs a stack the other platforms do not give it.
        if name.contains("stackoverflow") {
            let _ = writeln!(src, r#"#[cfg(target_os = "linux")]"#);
        }
        // The image snapshots are long and many; CI runs them with --ignored.
        if snapshot == "run_images" {
            let _ = writeln!(src, "#[ignore]");
        }
        let _ = writeln!(
            src,
            "#[test]\nfn {ident}() {{ {check}(\"{snapshot}\", \"{command}\", \"{dir}/{name}\"); }}"
        );
    }
}

/// A file name is not an identifier: `.invalid.gleam` and `Invalid.gleam` are
/// both valid inputs, and have to end up with different names.
fn sanitize(name: &str) -> String {
    let stem = name.strip_suffix(".gleam").unwrap_or(name);
    stem.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}
