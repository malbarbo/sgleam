#![allow(clippy::result_large_err)]

#[cfg(target_arch = "wasm32")]
compile_error!(
    "The cli crate does not support wasm32. Use `cargo build -p wasm --target wasm32-wasip1` instead."
);

mod config;
mod repl_reader;

use bpaf::{Bpaf, Parser};
use camino::Utf8PathBuf;
use engine::{
    error::{SgleamError, show_error},
    format,
    gleam::{Project, find_imports},
    quickjs::QuickJsEngine,
    repl::Repl,
    run::{copy_files_and_build, run_check, run_main, run_test},
    shell::{Shell, Status, welcome_message},
};
use gleam_core::{
    error::{FileIoAction, FileKind},
    javascript::set_bigint_enabled,
};
use repl_reader::Theme;
use std::{cell::Cell, rc::Rc};

fn number_arg() -> impl bpaf::Parser<bool> {
    bpaf::short('n')
        .help("Use Number instead of BigInt for integers")
        .switch()
}

#[derive(Debug, Clone, Bpaf)]
enum Command {
    /// Start interactive REPL (default).
    #[bpaf(command)]
    Repl {
        #[bpaf(external(number_arg))]
        number: bool,
        /// Suppress welcome message.
        #[bpaf(short)]
        quiet: bool,
        /// File to load before the REPL starts.
        #[bpaf(positional("FILE"))]
        file: Option<String>,
    },
    /// Execute a program.
    #[bpaf(command)]
    Run {
        #[bpaf(external(number_arg))]
        number: bool,
        /// Gleam file to run.
        #[bpaf(positional("FILE"))]
        file: String,
    },
    /// Run tests.
    #[bpaf(command)]
    Test {
        #[bpaf(external(number_arg))]
        number: bool,
        /// Gleam file to test.
        #[bpaf(positional("FILE"))]
        file: String,
    },
    /// Format source code (reads stdin if no files given).
    #[bpaf(command)]
    Format {
        /// Check if files are formatted without modifying them.
        #[bpaf(long)]
        check: bool,
        /// Files to format.
        #[bpaf(positional("FILE"), many)]
        files: Vec<String>,
    },
    /// Check source code (compile only).
    #[bpaf(command)]
    Check {
        #[bpaf(external(number_arg))]
        number: bool,
        /// Gleam file to check.
        #[bpaf(positional("FILE"))]
        file: String,
    },
    /// Show help information.
    #[bpaf(command)]
    Help,
}

fn cli() -> bpaf::OptionParser<Option<Command>> {
    let number = number_arg();
    let file = bpaf::positional::<String>("FILE");
    let file_as_run = bpaf::construct!(Command::Run { number, file });
    let cmd = bpaf::construct!([command(), file_as_run]).optional();
    bpaf::construct!(cmd)
        .to_options()
        .version(engine::version_short().as_str())
        .descr("The student version of gleam")
}

/// std ignores SIGPIPE before main, which turns a closed pipe into a panic.
#[cfg(unix)]
fn die_on_a_closed_pipe() {
    // SAFETY: SIG_DFL is no handler, so nothing of ours runs at signal time.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn die_on_a_closed_pipe() {}

fn main() {
    die_on_a_closed_pipe();
    engine::panic::add_handler();
    engine::logger::initialise_logger();

    let run_thread = std::thread::Builder::new()
        .stack_size(engine::STACK_SIZE)
        .name("run".into())
        .spawn(|| run().inspect_err(show_error).is_err());

    let failed = match run_thread {
        Err(err) => {
            eprintln!("Could not start the run thread: {err}");
            true
        }
        Ok(thread) => thread
            .join()
            // an Err is a panic, which the hook already reported; a release
            // build aborts instead of unwinding, so only a debug build has one
            .unwrap_or(true),
    };

    if failed {
        std::process::exit(1)
    }
}

fn run() -> Result<(), SgleamError> {
    let command = cli().run().unwrap_or(Command::Repl {
        number: false,
        file: None,
        quiet: false,
    });

    let number = matches!(
        &command,
        Command::Repl { number: true, .. }
            | Command::Run { number: true, .. }
            | Command::Test { number: true, .. }
            | Command::Check { number: true, .. }
    );

    set_bigint_enabled(!number);

    match command {
        Command::Help => {
            if let Err(err) = cli().run_inner(bpaf::Args::from(&["--help"])) {
                err.print_message(80);
            }
            Ok(())
        }
        Command::Repl { file, quiet, .. } => {
            let paths = match file {
                Some(file) => find_imports(vec![make_relative_to_current_dir(file.into())?])?,
                None => vec![],
            };
            run_interactive(&paths, quiet)
        }
        Command::Run { file, .. } => {
            let file = make_relative_to_current_dir(file.into())?;
            let files = find_imports(vec![file])?;
            run_main(&files)
        }
        Command::Test { file, .. } => {
            let file = make_relative_to_current_dir(file.into())?;
            let user_files = vec![file];
            let files = find_imports(user_files.clone())?;
            run_test(&user_files, &files)
        }
        Command::Format { check, files } => {
            let paths = files.into_iter().map(Utf8PathBuf::from).collect();
            Ok(format::run(check, paths)?)
        }
        Command::Check { file, .. } => {
            let file = make_relative_to_current_dir(file.into())?;
            let files = find_imports(vec![file])?;
            run_check(&files)
        }
    }
}

fn make_relative_to_current_dir(path: Utf8PathBuf) -> Result<Utf8PathBuf, SgleamError> {
    let current_dir = canonicalise(get_current_dir()?)?;
    canonicalise(path.clone())?
        .strip_prefix(&current_dir)
        // Stripping the current directory from itself leaves nothing, which
        // names no file.
        .map(|p| match p.as_str().replace('\\', "/") {
            relative if relative.is_empty() => Utf8PathBuf::from("."),
            relative => Utf8PathBuf::from(relative),
        })
        .map_err(|_| SgleamError::PathNotInCurrentDir { current_dir, path })
}

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

fn run_interactive(paths: &[Utf8PathBuf], quiet: bool) -> Result<(), SgleamError> {
    let theme = Rc::new(Cell::new(config::load().theme));

    if !quiet {
        print!("{}", welcome_message());
    }

    let mut project = Project::default();
    let built = copy_files_and_build(&mut project, paths)?;
    let module = built.module(0);

    let mut shell = Shell::new(Repl::<QuickJsEngine>::new(project, module)?);
    shell.add(
        ":theme",
        Some("[light|dark]"),
        "Show or switch the theme",
        theme_action(theme.clone()),
    );

    let mut reader = repl_reader::ReplReader::new(shell.completions(), theme.get())
        .map_err(|e| SgleamError::Other(e.into()))?;
    while let Some(input) = reader.next() {
        if shell.run(&input) == Status::Quit {
            break;
        }
        reader.set_completions(shell.completions());
        reader.set_theme(theme.get());
    }

    Ok(())
}

fn theme_action(theme: Rc<Cell<Theme>>) -> impl FnMut(&str) -> Result<(), String> {
    move |name| {
        if name.is_empty() {
            println!("{}", theme.get().name());
            return Ok(());
        }
        let Some(chosen) = Theme::parse(name) else {
            return Err(format!("Unknown theme: {name}. Use 'light' or 'dark'."));
        };
        theme.set(chosen);
        config::save(chosen);
        Ok(())
    }
}
