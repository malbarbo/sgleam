use camino::Utf8Path;
use clap::{
    arg,
    builder::{styling, Styles},
    command, Parser,
};
use gleam_core::io::{FileSystemReader, FileSystemWriter};
use sgleam::{
    gleam::{build, compile, show_gleam_error, Project},
    javascript::{create_js_context, run_js},
    repl::ReplReader,
};
use std::{path::PathBuf, process::exit, time::Instant};

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
    let cli = Cli::parse();

    // TODO: include quickjs version
    if cli.version {
        println!("{}", sgleam::version());
        return;
    }

    let path = if let Some(path) = cli.path {
        path
    } else {
        repl(&mut Project::new(), None);
        return;
    };

    let input = Utf8Path::from_path(&path).unwrap();
    if !input.is_file() {
        eprintln!("{input}: does not exist or is not a file.");
        exit(1);
    }

    if input.extension() != Some("gleam") {
        eprintln!("{input}: is not a gleam file.");
        exit(1);
    }

    if cli.format {
        if let Err(err) = sgleam::format::run(false, false, vec![input.as_str().into()]) {
            show_gleam_error(err);
            exit(1)
        }
        return;
    }

    let mut project = Project::new();

    match build(&mut project, input, cli.test) {
        Err(err) => show_gleam_error(err),
        Ok(_) => {
            if cli.interative {
                repl(&mut project, Some(input.file_stem().unwrap()));
            } else {
                let source = project.fs.read(&Project::main()).unwrap();
                run_js(
                    &create_js_context(
                        project.fs.clone(),
                        Project::out().as_std_path().to_path_buf(),
                    ),
                    source,
                )
            }
        }
    }
}

fn repl(project: &mut Project, user_module: Option<&str>) {
    let editor = ReplReader::new().unwrap();
    let context = create_js_context(
        project.fs.clone(),
        Project::out().as_std_path().to_path_buf(),
    );
    for (n, code) in editor.filter(|s| !s.is_empty()).enumerate() {
        let start = Instant::now();
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
        let duration = start.elapsed();
        println!("Time elapsed: {:?}", duration);
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
