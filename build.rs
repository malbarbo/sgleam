use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

pub fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    let outdir = PathBuf::from(env::var("OUT_DIR").unwrap());

    create_tar(&outdir, "gleam-stdlib.tar", "sgleam-stdlib-0.56.0");
    create_tar(
        &outdir,
        "gleam-stdlib-bigint.tar",
        "sgleam-bigint-stdlib-0.56.0",
    );
}

fn create_tar(outdir: &Path, name: &str, hash: &str) {
    let stdlib = &outdir.join("gleam-stdlib");
    let tar = outdir.join(name);

    if !stdlib.exists() {
        assert!(Command::new("git")
            .arg("clone")
            .arg("https://github.com/malbarbo/gleam-stdlib")
            .arg(stdlib)
            .status()
            .unwrap()
            .success());
    } else {
        env::set_current_dir(stdlib).unwrap();
        assert!(Command::new("git")
            .arg("checkout")
            .arg("main")
            .status()
            .unwrap()
            .success());
        assert!(Command::new("git")
            .arg("pull")
            .arg("--all")
            .status()
            .unwrap()
            .success());
    }

    env::set_current_dir(stdlib).unwrap();
    assert!(Command::new("git")
        .arg("checkout")
        .arg(hash)
        .status()
        .unwrap()
        .success());

    // FIXME: use tar crate
    let stdlib_src = &stdlib.join("src");
    assert!(Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .arg("cf")
        .arg(tar)
        .arg("-C")
        .arg(stdlib_src)
        .arg(".")
        .status()
        .unwrap()
        .success());
}
