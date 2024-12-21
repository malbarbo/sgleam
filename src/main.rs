use camino::Utf8PathBuf;
use clap::{
    arg,
    builder::{styling, Styles},
    command, Parser,
};
use gleam_core::error::{FileIoAction, FileKind};
use sgleam::{
    error::{show_error, SgleamError},
    format,
    gleam::find_imports,
    logger, panic,
    run::{run_check, run_interative, run_main, run_test},
    STACK_SIZE,
};
use std::{process::exit, thread};

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
    /// Don't print welcome message on entering interactive mode.
    #[arg(short)]
    quiet: bool,
    /// Print version.
    #[arg(short, long)]
    version: bool,
    /// Input files.
    paths: Vec<String>,
}

fn main() {
    panic::add_handler();
    logger::initialise_logger();
    // Error is handled by the panic hook.
    let _ = thread::Builder::new()
        .stack_size(STACK_SIZE)
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

    // TODO: include quickjs version
    if cli.version {
        println!("{}", sgleam::version());
        return Ok(());
    }

    let paths = cli
        .paths
        .into_iter()
        .map(|path| make_relative_to_current_dir(path.into()))
        .collect::<Result<Vec<_>, _>>()?;

    if cli.format {
        return Ok(format::run(false, paths)?);
    }

    if cli.check {
        return Ok(run_check(&paths)?);
    }

    if cli.test {
        return Ok(run_test(&paths)?);
    }

    match &paths[..] {
        [] => run_interative(&paths, cli.quiet),
        [path] => {
            let paths = find_imports(path.clone())?;
            if cli.interative {
                run_interative(&paths, cli.quiet)
            } else {
                run_main(&paths)
            }
        }
        _ => {
            eprintln!("Specify at most one.");
            exit(1);
        }
    }
}

fn make_relative_to_current_dir(path: Utf8PathBuf) -> Result<Utf8PathBuf, SgleamError> {
    let current_dir = get_current_dir()?;
    path.canonicalize_utf8()
        .map_err(|e| gleam_core::Error::FileIo {
            kind: FileKind::File,
            action: FileIoAction::Canonicalise,
            path: path.clone(),
            err: Some(e.to_string()),
        })?
        .strip_prefix(&current_dir)
        .map(Utf8PathBuf::from)
        .map_err(|_| SgleamError::PathNotInCurrentDir { current_dir, path })
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
