#![allow(clippy::result_large_err)]

#[cfg(target_arch = "wasm32")]
compile_error!("The cli crate does not support wasm32. Use `cargo build -p sgleam-wasm --target wasm32-wasip1` instead.");

mod config;
mod repl_reader;

use camino::Utf8PathBuf;
use clap::{
    builder::{styling, Styles},
    CommandFactory, FromArgMatches, Parser,
};
use engine::{
    error::{show_error, SgleamError},
    format,
    gleam::{find_imports, get_module, Project},
    quickjs::QuickJsEngine,
    repl::{welcome_message, Repl, ReplOutput},
    run::{copy_files_and_build, run_check, run_main, run_test},
};
use gleam_core::{
    error::{FileIoAction, FileKind},
    javascript::set_bigint_enabled,
};

const STYLES: Styles = Styles::styled()
    .header(styling::AnsiColor::Yellow.on_default())
    .usage(styling::AnsiColor::Yellow.on_default())
    .literal(styling::AnsiColor::Green.on_default());

/// The student version of gleam.
#[derive(Parser)]
#[command(
    name = "sgleam",
    about,
    styles = STYLES,
    args_conflicts_with_subcommands = true,
    disable_help_flag = true,
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Use Number instead of BigInt for integers.
    #[arg(short, global = true)]
    number: bool,

    /// File to run (shorthand for `sgleam run FILE`).
    file: Option<String>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Start interactive REPL (default).
    Repl {
        /// File to load before the REPL starts.
        file: Option<String>,
        /// Suppress welcome message.
        #[arg(short)]
        quiet: bool,
    },
    /// Execute a program.
    Run {
        /// Gleam file to run.
        file: String,
    },
    /// Run tests.
    Test {
        /// Gleam file to test.
        file: String,
    },
    /// Format source code (reads stdin if no files given).
    Format {
        /// Files to format.
        files: Vec<String>,
    },
    /// Check source code (compile only).
    Check {
        /// Gleam file to check.
        file: String,
    },
}

fn main() {
    engine::panic::add_handler();
    engine::logger::initialise_logger();
    // Error is handled by the panic hook.
    let result = std::thread::Builder::new()
        .stack_size(engine::STACK_SIZE)
        .name("run".into())
        .spawn(|| {
            if let Err(err) = run() {
                show_error(&err);
                return false;
            }
            true
        })
        .expect("Create the run thread")
        .join();
    if !matches!(result, Ok(true)) {
        std::process::exit(1);
    }
}

fn run() -> Result<(), SgleamError> {
    let version: Box<str> = engine::version_for_clap().into();
    let version: &'static str = Box::leak(version);
    let cli = Cli::command().version(version).get_matches();
    let cli = Cli::from_arg_matches(&cli).expect("valid args");

    set_bigint_enabled(!cli.number);

    let command = match (cli.command, cli.file) {
        (Some(cmd), _) => cmd,
        (None, Some(file)) => Command::Run { file },
        (None, None) => Command::Repl {
            file: None,
            quiet: false,
        },
    };

    match command {
        Command::Repl { file, quiet } => {
            let paths = file
                .map(|f| make_relative_to_current_dir(f.into()))
                .transpose()?;
            let paths = paths.as_slice();
            run_interactive(paths, quiet)
        }
        Command::Run { file } => {
            let file = make_relative_to_current_dir(file.into())?;
            let files = find_imports(vec![file])?;
            run_main(&files)
        }
        Command::Test { file } => {
            let file = make_relative_to_current_dir(file.into())?;
            let user_files = vec![file];
            let files = find_imports(user_files.clone())?;
            run_test(&user_files, &files)
        }
        Command::Format { files } => {
            let paths = files
                .into_iter()
                .map(|f| make_relative_to_current_dir(f.into()))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(format::run(false, paths)?)
        }
        Command::Check { file } => {
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
        .map(|p| Utf8PathBuf::from(p.as_str().replace('\\', "/")))
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

const COMPLETION_EXTRAS: &[&str] = &[
    // REPL commands
    ":quit", ":type ", ":debug", ":help", ":theme ", // Keywords and builtins
    "let", "fn", "type", "import", "case", "pub", "const", "assert", "use", "if", "else", "True",
    "False", "Nil", "Ok", "Error", "panic", "todo",
];

fn run_interactive(paths: &[Utf8PathBuf], quiet: bool) -> Result<(), SgleamError> {
    let cfg = config::load();
    repl_reader::set_theme(cfg.theme == "light");

    if !quiet {
        print!("{}", welcome_message());
    }

    let mut project = Project::default();
    let modules = copy_files_and_build(&mut project, paths)?;
    let module = paths.first().and_then(|input| {
        let name = input.with_extension("");
        let name = name.as_str().replace('\\', "/");
        get_module(&modules, &name)
    });

    let mut repl = Repl::<QuickJsEngine>::new(project, module)?;
    let completions = repl_reader::Completions::default();
    update_completions(&repl, &completions);
    let reader = repl_reader::ReplReader::new(completions.clone())
        .map_err(|e| SgleamError::Other(e.into()))?;
    for input in reader {
        let trimmed = input.trim();
        if trimmed == ":help" {
            println!("Commands:");
            println!("  :help          Show this help");
            println!("  :quit          Exit the REPL");
            println!("  :type <expr>   Show the type of an expression");
            println!("  :theme         Show the current theme");
            println!("  :theme light   Switch to One Light theme");
            println!("  :theme dark    Switch to One Dark theme");
            println!("  :debug         Toggle debug mode");
            continue;
        }
        if trimmed == ":theme" {
            let name = if repl_reader::is_light_theme() {
                "light"
            } else {
                "dark"
            };
            println!("{name}");
            continue;
        }
        if let Some(name) = trimmed.strip_prefix(":theme ") {
            let name = name.trim();
            match name {
                "light" | "dark" => {
                    repl_reader::set_theme(name == "light");
                    config::save(name);
                }
                _ => println!("Unknown theme: {name}. Use 'light' or 'dark'."),
            }
            continue;
        }
        match repl.run(&input) {
            Err(err) => show_error(&err),
            Ok(ReplOutput::Quit) => break,
            _ => {}
        }
        update_completions(&repl, &completions);
    }

    Ok(())
}

fn update_completions(repl: &Repl<QuickJsEngine>, completions: &repl_reader::Completions) {
    let mut names = repl.completions();
    names.extend(COMPLETION_EXTRAS.iter().map(|s| s.to_string()));
    names.sort();
    names.dedup();
    *completions.borrow_mut() = names;
}
