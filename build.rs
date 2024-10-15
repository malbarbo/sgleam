use std::{env, path::PathBuf, process::Command};

pub fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    let outdir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let stdlib = &outdir.join("gleam-stdlib");
    let tar = outdir.join("gleam-stdlib.tar");
    if tar.is_file() {
        return;
    }
    if !stdlib.exists() {
        assert!(Command::new("git")
            .arg("clone")
            .arg("-b")
            .arg("bigint")
            .arg("https://github.com/malbarbo/gleam-stdlib")
            .arg(stdlib)
            .status()
            .unwrap()
            .success());
    }

    let stdlib_src = &stdlib.join("src");
    assert!(Command::new("tar")
        .arg("cf")
        .arg(tar)
        .arg("-C")
        .arg(stdlib_src)
        .arg(".")
        .status()
        .unwrap()
        .success());
}
