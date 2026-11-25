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

    static_libs();
}

/// The libraries that winit and softbuffer link under `cfg(no_dlopen)`.
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
/// which has no dlopen. The forks of dlib, x11-dl and x11rb refer to their
/// functions under `cfg(no_dlopen)`. Only the binary links them, so `cargo
/// test` does not link under the cfg.
fn static_libs() {
    if env::var_os("CARGO_CFG_NO_DLOPEN").is_none() {
        return;
    }
    assert_eq!(
        env::var("CARGO_CFG_TARGET_ENV").as_deref(),
        Ok("musl"),
        "cfg(no_dlopen) links the musl libraries of SGLEAM_STATIC_LIBS"
    );
    println!("cargo:rerun-if-env-changed=SGLEAM_STATIC_LIBS");
    let dir = env::var("SGLEAM_STATIC_LIBS").expect("cfg(no_dlopen) needs SGLEAM_STATIC_LIBS");
    // The build script runs in cli/ and the linker in the workspace, so a
    // relative path would name two directories.
    assert!(
        Path::new(&dir).is_absolute(),
        "SGLEAM_STATIC_LIBS is not absolute: {dir}"
    );
    println!("cargo:rustc-link-search=native={dir}");
    println!("cargo:rustc-link-arg-bins=-Wl,--start-group");
    for lib in STATIC_LIBS {
        let path = Path::new(&dir).join(format!("lib{lib}.a"));
        println!("cargo:rerun-if-changed={}", path.display());
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
