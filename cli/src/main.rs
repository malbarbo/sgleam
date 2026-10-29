#![allow(clippy::result_large_err)]

#[cfg(target_arch = "wasm32")]
compile_error!(
    "The cli crate does not support wasm32. Use `cargo build -p wasm --target wasm32-wasip1` instead."
);

mod config;
mod repl_reader;

/// The name of the program in its help and in its directories for data and
/// config.
const NAME: &str = env!("CARGO_PKG_NAME");

use bpaf::{Bpaf, Parser};
use camino::Utf8PathBuf;
use engine::{
    error::{SgleamError, show_error},
    format,
    gleam::{Project, copy_files_and_build, find_imports},
    host::{DisplayMode, HostOptions, check_display},
    quickjs::QuickJsEngine,
    repl::Repl,
    run::{run_check, run_main, run_test},
    shell::{Shell, Status, welcome_message},
};
use gleam_core::{
    error::{FileIoAction, FileKind},
    javascript::set_bigint_enabled,
};
use repl_reader::Theme;
use std::{cell::Cell, num::NonZeroU32, rc::Rc};

fn number_arg() -> impl bpaf::Parser<bool> {
    bpaf::short('n')
        .help("Use Number instead of BigInt for integers")
        .switch()
}

fn display_arg() -> impl bpaf::Parser<DisplayMode> {
    bpaf::short('d')
        .long("display")
        .help("Show images in window, kitty, sixel, blocks or none")
        .argument::<String>("MODE")
        .parse(|mode| match mode.as_str() {
            "window" => Ok(DisplayMode::Window),
            "kitty" => Ok(DisplayMode::Kitty),
            "sixel" => Ok(DisplayMode::Sixel),
            "blocks" => Ok(DisplayMode::Blocks),
            "none" => Ok(DisplayMode::None),
            _ => Err("expected window, kitty, sixel, blocks or none"),
        })
        .fallback(DisplayMode::Auto)
}

fn players_arg() -> impl bpaf::Parser<Option<NonZeroU32>> {
    bpaf::long("players")
        .help("Play a multiplayer world with N players")
        .argument::<NonZeroU32>("N")
        .optional()
}

#[derive(Debug, Clone, Bpaf)]
enum Command {
    /// Start interactive REPL (default).
    #[bpaf(command)]
    Repl {
        #[bpaf(external(number_arg))]
        number: bool,
        #[bpaf(external(display_arg))]
        display: DisplayMode,
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
        #[bpaf(external(display_arg))]
        display: DisplayMode,
        #[bpaf(external(players_arg))]
        players: Option<NonZeroU32>,
        /// Gleam file to run.
        #[bpaf(positional("FILE"))]
        file: String,
    },
    /// Run a program as the engine of a game server, on fd 3 and fd 4.
    #[bpaf(command, hide)]
    Server {
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
        #[bpaf(external(display_arg))]
        display: DisplayMode,
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
    Help {
        /// Command to show the help of.
        #[bpaf(positional("COMMAND"), optional)]
        command: Option<String>,
    },
}

fn cli() -> bpaf::OptionParser<Option<Command>> {
    let number = number_arg();
    let display = display_arg();
    let players = players_arg();
    let file = bpaf::positional::<String>("FILE");
    let file_as_run = bpaf::construct!(Command::Run {
        number,
        display,
        players,
        file
    });
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

    // The program runs on the main thread, where macOS lets a window open. A
    // panic unwinds only in a debug build, and the hook already reported it.
    let failed =
        std::panic::catch_unwind(|| run().inspect_err(show_error).is_err()).unwrap_or(true);

    if failed {
        std::process::exit(1)
    }
}

/// Print what `parser` answers to `args`, which ask it for its help.
fn show_help<T>(parser: bpaf::OptionParser<T>, args: &[&str]) {
    if let Err(err) = parser.run_inner(bpaf::Args::from(args).set_name(NAME)) {
        err.print_message(80);
    }
}

fn run() -> Result<(), SgleamError> {
    // The name of the executable ends in .exe on Windows.
    let args = bpaf::Args::current_args().set_name(NAME);
    let command = match cli().run_inner(args) {
        Ok(command) => command,
        Err(err) => {
            err.print_message(100);
            std::process::exit(err.exit_code())
        }
    };
    let command = command.unwrap_or(Command::Repl {
        number: false,
        display: DisplayMode::Auto,
        file: None,
        quiet: false,
    });

    let number = matches!(
        &command,
        Command::Repl { number: true, .. }
            | Command::Run { number: true, .. }
            | Command::Server { number: true, .. }
            | Command::Test { number: true, .. }
            | Command::Check { number: true, .. }
    );

    set_bigint_enabled(!number);

    let options = match &command {
        Command::Run {
            display, players, ..
        } => HostOptions {
            display: *display,
            players: *players,
        },
        Command::Server { .. } => HostOptions {
            display: DisplayMode::Session,
            players: None,
        },
        Command::Repl { display, .. } | Command::Test { display, .. } => HostOptions {
            display: *display,
            players: None,
        },
        _ => HostOptions::default(),
    };
    check_display(options.display)?;

    match command {
        Command::Help { command: None } => {
            show_help(cli(), &["--help"]);
            Ok(())
        }
        // The commands alone, without the shorthand, so that a name that is
        // not a command lists the commands and does not run as a file.
        Command::Help {
            command: Some(name),
        } => {
            show_help(crate::command().to_options(), &[&name, "--help"]);
            Ok(())
        }
        Command::Repl { file, quiet, .. } => {
            let paths = match file {
                Some(file) => find_imports(vec![user_module(file)?])?,
                None => vec![],
            };
            run_interactive(&paths, quiet, options)
        }
        Command::Run { file, .. } | Command::Server { file, .. } => {
            let file = user_module(file)?;
            let files = find_imports(vec![file])?;
            run_main(&files, options)
        }
        Command::Test { file, .. } => {
            let file = user_module(file)?;
            let user_files = vec![file];
            let files = find_imports(user_files.clone())?;
            run_test(&user_files, &files, options)
        }
        Command::Format { check, files } => {
            let paths = files.into_iter().map(Utf8PathBuf::from).collect();
            Ok(format::run(check, paths)?)
        }
        Command::Check { file, .. } => {
            let file = user_module(file)?;
            let files = find_imports(vec![file])?;
            run_check(&files)
        }
    }
}

/// The file a command names, as a path the module name can come from.
fn user_module(file: String) -> Result<Utf8PathBuf, SgleamError> {
    let path = make_relative_to_current_dir(file.into())?;
    if path.is_dir() {
        return Err(SgleamError::PathIsADirectory { path });
    }
    Ok(path)
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

fn run_interactive(
    paths: &[Utf8PathBuf],
    quiet: bool,
    options: HostOptions,
) -> Result<(), SgleamError> {
    let theme = Rc::new(Cell::new(config::load().theme));

    if !quiet {
        print!("{}", welcome_message());
    }

    let mut project = Project::default();
    let built = copy_files_and_build(&mut project, paths)?;
    let module = built.module(0);

    let engine = QuickJsEngine::new(project.fs.clone(), options)?;
    let mut shell = Shell::new(Repl::new(project, module, engine));
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
