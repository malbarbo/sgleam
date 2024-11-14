use clap::{
    arg,
    builder::{styling, Styles},
    command, Parser,
};
use gleam_core::Error;
use sgleam::{
    format,
    gleam::show_gleam_error,
    logger, panic,
    run::{run_check_or_test, run_interative, run_main},
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
                show_gleam_error(err);
            }
        })
        .expect("Create the run thread")
        .join();
}

fn run() -> Result<(), Error> {
    let cli = Cli::parse();

    // TODO: include quickjs version
    if cli.version {
        println!("{}", sgleam::version());
        return Ok(());
    }

    if cli.format {
        return format::run(cli.paths.is_empty(), false, cli.paths);
    }

    if cli.test || cli.check {
        return run_check_or_test(&cli.paths, cli.test);
    }

    if cli.interative || cli.paths.is_empty() {
        if cli.paths.len() > 1 {
            eprintln!("Specify at most one file to enter interative mode.");
            exit(1);
        }
        return run_interative(cli.paths.first(), cli.quiet);
    }

    if cli.paths.len() != 1 {
        eprintln!("Specify only one file to run.");
        exit(1);
    }

    run_main(&cli.paths[0])
}
