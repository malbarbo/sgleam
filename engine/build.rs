// What fails here fails the build, in front of whoever ran it, and not in front
// of a user with a crash report to fill in.
#![allow(clippy::unwrap_used)]

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use flate2::{Compression, write::GzEncoder};

pub fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=../lib/sgleam");

    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo::rustc-env=GIT_HASH={git_hash}");

    let build_date = {
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_secs();
        let (y, m, d) = civil_from_days((secs / 86400) as i64);
        format!("{y:04}-{m:02}-{d:02}")
    };
    println!("cargo::rustc-env=BUILD_DATE={build_date}");
    // Cargo takes a missing file for changed and would run this script at
    // every build, so it watches only the files git names. In a worktree
    // `.git` is a file, HEAD is under the worktree and the refs are not.
    for path in git_paths() {
        if Path::new(&path).exists() {
            println!("cargo::rerun-if-changed={path}");
        }
    }

    let outdir = PathBuf::from(env::var("OUT_DIR").unwrap());

    create_tar(&outdir, "gleam-stdlib.tar", "sgleam-stdlib-1.0.5");
}

/// Returns `true` if the clone at `repo` already has `revision`, `false`
/// otherwise. A clone that has it needs no pull, and so no network.
fn has_revision(repo: &Path, revision: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "--verify", "--quiet"])
        .arg(format!("{revision}^{{commit}}"))
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Convert days since 1970-01-01 to (year, month, day).
/// Algorithm from Howard Hinnant.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// The files that change when HEAD moves: HEAD itself, the refs, and the
/// refs packed by a gc. None outside a git repository.
fn git_paths() -> Vec<String> {
    let Some(output) = Command::new("git")
        .args([
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "HEAD",
            "--git-common-dir",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
    else {
        return Vec::new();
    };
    let output = String::from_utf8_lossy(&output.stdout);
    let mut lines = output.lines();
    let (Some(head), Some(common)) = (lines.next(), lines.next()) else {
        return Vec::new();
    };
    vec![
        head.to_owned(),
        format!("{common}/refs"),
        format!("{common}/packed-refs"),
    ]
}

fn create_tar(outdir: &Path, name: &str, hash: &str) {
    let stdlib = &outdir.join("gleam-stdlib");
    let tar = outdir.join(name);

    if !stdlib.exists() {
        assert!(
            Command::new("git")
                .arg("clone")
                .arg("https://github.com/malbarbo/gleam-stdlib")
                .arg(stdlib)
                .status()
                .unwrap()
                .success()
        );
    } else if !has_revision(stdlib, hash) {
        env::set_current_dir(stdlib).unwrap();
        assert!(
            Command::new("git")
                .arg("checkout")
                .arg("main")
                .status()
                .unwrap()
                .success()
        );
        assert!(
            Command::new("git")
                .arg("pull")
                .arg("--all")
                .status()
                .unwrap()
                .success()
        );
    }

    env::set_current_dir(stdlib).unwrap();
    assert!(
        Command::new("git")
            .arg("checkout")
            .arg(hash)
            .status()
            .unwrap()
            .success()
    );

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
