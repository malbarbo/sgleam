use std::{env, path::PathBuf, process::Command};

pub fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    let outdir = PathBuf::from(env::var("OUT_DIR").unwrap());

    create_tar(
        &outdir,
        "gleam-stdlib.tar",
        "9f1c71d7512fd93609135d56690a30fdfcd8c655",
    );
    create_tar(
        &outdir,
        "gleam-stdlib-bigint.tar",
        "cdfc63d3da6ef0ae5f28e19afab938b49b062969",
    );
}

fn create_tar(outdir: &PathBuf, name: &str, hash: &str) {
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
