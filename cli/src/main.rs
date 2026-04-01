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
    gleam::{Project, find_imports, get_module},
    quickjs::QuickJsEngine,
    repl::{Repl, ReplOutput, welcome_message},
    run::{copy_files_and_build, run_check, run_main, run_test},
};
use gleam_core::{
    error::{FileIoAction, FileKind},
    javascript::set_bigint_enabled,
};

/// Use Number instead of BigInt for integers
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
        .version(engine::version_short().leak() as &str)
        .descr("The student version of gleam")
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
            let paths = file
                .map(|f| make_relative_to_current_dir(f.into()))
                .transpose()?;
            let paths = paths.as_slice();
            run_interactive(paths, quiet)
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
            let paths = files
                .into_iter()
                .map(|f| make_relative_to_current_dir(f.into()))
                .collect::<Result<Vec<_>, _>>()?;
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
