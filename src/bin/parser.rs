use clap::Parser;
use gleam_core::{
    error::{FileIoAction, FileKind},
    Error,
};
use sgleam::{error::show_error, parser::*};
use std::fs;

#[derive(Parser)]
struct Cli {
    /// Input file.
    path: String,
}

fn main() {
    if let Err(err) = run() {
        show_error(&err.into());
    }
}

fn run() -> Result<(), gleam_core::Error> {
    let cli = Cli::parse();

    let src = fs::read_to_string(&cli.path).map_err(|err| Error::FileIo {
        action: FileIoAction::Read,
        kind: FileKind::File,
        path: cli.path.clone().into(),
        err: Some(err.to_string()),
    })?;

    for item in parse_repl(&src).map_err(|error| Error::Parse {
        path: cli.path.into(),
        src: src.into(),
        error,
    })? {
        println!("{item:#?}");
    }

    Ok(())
}
