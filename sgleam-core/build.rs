use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use flate2::{write::GzEncoder, Compression};

pub fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    let outdir = PathBuf::from(env::var("OUT_DIR").unwrap());

    create_tar(&outdir, "gleam-stdlib.tar", "sgleam-stdlib-0.69.0");
    create_tar(
        &outdir,
        "gleam-stdlib-bigint.tar",
        "sgleam-stdlib-bigint-0.69.0",
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

    let stdlib_src = stdlib.join("src");
    let tar_file = fs::File::create(&tar).expect("create tar file");
    let encoder = GzEncoder::new(tar_file, Compression::best());
    let mut archive = tar::Builder::new(encoder);
    archive
        .append_dir_all(".", &stdlib_src)
        .expect("append stdlib to tar");
    archive
        .into_inner()
        .expect("finish tar")
        .finish()
        .expect("finish gzip");
}
