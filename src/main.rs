use camino::Utf8PathBuf;
use clap::{
    arg,
    builder::{styling, Styles},
    command, Parser,
};
use gleam_core::{
    build::Target,
    io::{FileSystemReader, FileSystemWriter},
    Error,
};
use sgleam::{
    gleam::{build, compile, show_gleam_error, to_error_nonutf8_path, Project},
    javascript::{create_js_context, run_js},
    repl::ReplReader,
    STACK_SIZE,
};
use std::{path::PathBuf, process::exit, thread};

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
    /// Go to iterative mode.
    #[arg(short, group = "cmd")]
    interative: bool,
    /// Run tests.
    #[arg(short, group = "cmd")]
    test: bool,
    /// Format source code.
    #[arg(short, group = "cmd")]
    format: bool,
    /// Print version.
    #[arg(short, long)]
    version: bool,
    /// The program file.
    // TODO: allow multiple files
    path: Option<PathBuf>,
}

fn main() {
    thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(|| {
            if let Err(err) = main2() {
                show_gleam_error(err);
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

fn main2() -> Result<(), Error> {
    let cli = Cli::parse();

    // TODO: include quickjs version
    if cli.version {
        println!("{}", sgleam::version());
        return Ok(());
    }

    let path = if let Some(path) = cli.path {
        path
    } else {
        repl(&mut Project::default(), None);
        return Ok(());
    };

    let input = Utf8PathBuf::from_path_buf(path).map_err(to_error_nonutf8_path)?;
    if !input.is_file() {
        eprintln!("{input}: does not exist or is not a file.");
        exit(1);
    }

    if input.extension() != Some("gleam") {
        eprintln!("{input}: is not a gleam file.");
        exit(1);
    }

    if cli.format {
        sgleam::format::run(false, false, vec![input.as_str().into()])?;
        return Ok(());
    }

    let mut project = Project::default();

    let module = build(&mut project, &input, cli.test)?;
    if cli.interative {
        repl(&mut project, Some(input.file_stem().unwrap()));
    } else {
        module
            .ast
            .type_info
            .get_main_function(Target::JavaScript)
            .map(|_| ())?;
        let source = project.fs.read(Project::main()).unwrap();
        run_js(
            &create_js_context(project.fs.clone(), Project::out().into()),
            source,
        );
    }
    Ok(())
}

fn repl(project: &mut Project, user_module: Option<&str>) {
    let editor = ReplReader::new().unwrap();
    let context = create_js_context(
        project.fs.clone(),
        Project::out().as_std_path().to_path_buf(),
    );
    for (n, code) in editor.filter(|s| !s.is_empty()).enumerate() {
        #[cfg(debug_assertions)]
        let start = std::time::Instant::now();

        let file = format!("repl{n}.gleam");
        write_repl_source(project, &file, &code, user_module);
        match compile(project, &format!("repl{n}"), true, false) {
            Err(err) => show_gleam_error(err),
            Ok(_) => {
                let source = project.fs.read(Project::main()).unwrap();
                run_js(&context, source);
            }
        }
        project
            .fs
            .delete_file(&Project::source().join(file))
            .unwrap();

        #[cfg(debug_assertions)]
        println!("Time elapsed: {:?}", start.elapsed());
    }
}

fn write_repl_source(project: &mut Project, file: &str, code: &str, user_module: Option<&str>) {
    let user_module = if let Some(module) = user_module {
        format!("import {module}")
    } else {
        "".into()
    };
    project.write_source(
        file,
        &format!(
            "
{user_module}
import gleam/bit_array
import gleam/bool
import gleam/bytes_builder
import gleam/dict
import gleam/dynamic
import gleam/float
import gleam/function
import gleam/int
import gleam/io
import gleam/iterator
import gleam/list
import gleam/option
import gleam/order
import gleam/pair
import gleam/queue
import gleam/regex
import gleam/result
import gleam/set
import gleam/string
import gleam/string_builder
import gleam/uri
pub fn main() {{
    io.debug({{
{code}
    }})
}}
"
        ),
    );
}
