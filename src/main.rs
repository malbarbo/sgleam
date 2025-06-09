#![allow(clippy::result_large_err)]

use camino::Utf8PathBuf;
use clap::{
    arg,
    builder::{styling, Styles},
    command, Parser,
};
use gleam_core::{
    error::{FileIoAction, FileKind},
    javascript::set_bigint_enabled,
};
use sgleam::{
    error::{show_error, SgleamError},
    format,
    gleam::find_imports,
    run::{run_check, run_interative, run_main, run_test},
};
use std::process::exit;

/// The student version of gleam.
#[derive(Parser)]
#[command(
    about,
    styles = Styles::styled()
        .header(styling::AnsiColor::Yellow.on_default())
        .usage(styling::AnsiColor::Yellow.on_default())
        .literal(styling::AnsiColor::Green.on_default())
)]
struct Cli {
    /// Enter iterative mode.
    #[arg(short, group = "cmd")]
    interative: bool,
    /// Run tests.
    #[arg(short, group = "cmd")]
    test: bool,
    /// Format source code.
    #[arg(short, group = "cmd")]
    format: bool,
    /// Check source code.
    #[arg(short, group = "cmd")]
    check: bool,
    /// Use Number instead of BigInt for integers
    #[arg(short)]
    number: bool,
    /// Don't print welcome message on entering interactive mode.
    #[arg(short)]
    quiet: bool,
    /// Print version.
    #[arg(short, long)]
    version: bool,
    /// Input files.
    paths: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
fn main() {
    if let Err(err) = run() {
        show_error(&err);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    sgleam::panic::add_handler();
    sgleam::logger::initialise_logger();
    // Error is handled by the panic hook.
    let _ = std::thread::Builder::new()
        .stack_size(sgleam::STACK_SIZE)
        .name("run".into())
        .spawn(|| {
            if let Err(err) = run() {
                show_error(&err);
            }
        })
        .expect("Create the run thread")
        .join();
}

fn run() -> Result<(), SgleamError> {
    let cli = Cli::parse();

    set_bigint_enabled(!cli.number);

    // TODO: include quickjs version
    if cli.version {
        println!("{}", sgleam::version());
        return Ok(());
    }

    let user_files = cli
        .paths
        .into_iter()
        .map(|path| make_relative_to_current_dir(path.into()))
        .collect::<Result<Vec<_>, _>>()?;

    if cli.format {
        return Ok(format::run(false, user_files)?);
    }

    if user_files.is_empty() {
        return run_interative(&user_files, cli.quiet);
    }

    if !cli.check && !cli.test && user_files.len() != 1 {
        eprintln!("Specify at most one.");
        exit(1);
    }

    let files = find_imports(user_files.clone())?;

    if cli.check {
        run_check(&files)
    } else if cli.test {
        run_test(&user_files, &files)
    } else if cli.interative {
        run_interative(&files, cli.quiet)
    } else {
        run_main(&files)
    }
}

fn make_relative_to_current_dir(path: Utf8PathBuf) -> Result<Utf8PathBuf, SgleamError> {
    let current_dir = canonicalise(get_current_dir()?)?;
    canonicalise(path.clone())?
        .strip_prefix(&current_dir)
        .map(Utf8PathBuf::from)
        .map_err(|_| SgleamError::PathNotInCurrentDir { current_dir, path })
}

// Make the compiler remove some imported functions in the wasm build.
#[cfg_attr(target_arch = "wasm32", inline(always))]
fn canonicalise(path: Utf8PathBuf) -> Result<Utf8PathBuf, gleam_core::Error> {
    path.canonicalize_utf8()
        .map_err(|e| gleam_core::Error::FileIo {
            kind: FileKind::File,
            action: FileIoAction::Canonicalise,
            path,
            err: Some(e.to_string()),
        })
}

fn get_current_dir() -> Result<Utf8PathBuf, gleam_core::Error> {
    let curr_dir = std::env::current_dir().map_err(|e| gleam_core::Error::FileIo {
        kind: FileKind::Directory,
        action: FileIoAction::Open,
        path: ".".into(),
        err: Some(e.to_string()),
    })?;
    Utf8PathBuf::from_path_buf(curr_dir.clone())
        .map_err(|_| gleam_core::Error::NonUtf8Path { path: curr_dir })
}
