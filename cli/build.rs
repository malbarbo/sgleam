use std::{env, fmt::Write, fs, path::Path};

/// One test per input file, so they run in parallel: a single test walking the
/// directory is as slow as the sum of its files.
fn main() {
    println!("cargo:rerun-if-changed=tests/inputs");
    println!("cargo:rerun-if-changed=tests/images");

    let mut src = String::new();
    tests(&mut src, "tests/inputs", "run_file", "run", |_| true);
    tests(&mut src, "tests/inputs", "run_tests", "test", |path| {
        file_name(path).starts_with("check")
    });
    tests(&mut src, "tests/images", "run_images", "run", |_| true);
    // Most of the library of sgleam calls a JavaScript host, which wasmtime
    // is not. Only env, random, and the colors and styles do without one.
    tests(&mut src, "tests/inputs", "run_wasm", "wasm", without_host);
    tests(&mut src, "tests/inputs", "test_wasm", "wasm-test", |path| {
        file_name(path).starts_with("check") && without_host(path)
    });

    let out = env::var("OUT_DIR").expect("OUT_DIR");
    fs::write(Path::new(&out).join("file_tests.rs"), src).expect("write the generated tests");
}

fn tests(src: &mut String, dir: &str, snapshot: &str, command: &str, keep: fn(&Path) -> bool) {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {dir}: {e}"))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| file_name(path).ends_with(".gleam") && keep(path))
        .map(|path| file_name(&path).to_string())
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
        // An image is a whole snapshot of its own: the svg, and nothing around it.
        let check = if snapshot == "run_images" {
            "check_image"
        } else {
            "check"
        };
        let _ = writeln!(
            src,
            "#[test]\nfn {ident}() {{ {check}(\"{snapshot}\", \"{command}\", \"{dir}/{name}\"); }}"
        );
    }
}

/// Returns `true` if the file at `path` imports only the modules of sgleam
/// that do without a host, `false` otherwise.
fn without_host(path: &Path) -> bool {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
        .lines()
        .filter_map(|line| line.strip_prefix("import sgleam/"))
        .all(|module| {
            matches!(
                module.split(['.', ' ']).next(),
                Some("env" | "random" | "color" | "fill" | "stroke" | "style")
            )
        })
}

fn file_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|name| name.to_str())
        .expect("a utf8 file name")
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
