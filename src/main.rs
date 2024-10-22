use camino::Utf8PathBuf;
use clap::{
    arg,
    builder::{styling, Styles},
    command, Parser,
};
use gleam_core::{io::FileSystemWriter, type_, Error};
use sgleam::{
    format,
    gleam::{
        compile, get_main_function, get_module, show_gleam_error, to_error_nonutf8_path, Project,
    },
    javascript::{create_js_context, run_js},
    logger, panic,
    repl::ReplReader,
    STACK_SIZE,
};
use std::{path::PathBuf, process::exit, thread};
use vec1::vec1;

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
    panic::add_handler();
    logger::initialise_logger();
    thread::Builder::new()
        .stack_size(STACK_SIZE)
        .name("run".into())
        .spawn(|| {
            if let Err(err) = run() {
                show_gleam_error(err);
            }
        })
        .expect("Create the run thread")
        .join()
        .expect("Join the run thread");
}

fn run() -> Result<(), Error> {
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

    let main_module = input.file_stem().unwrap_or("");
    if input.extension() != Some("gleam") || main_module.is_empty() {
        eprintln!("{input}: is not a valid gleam file.");
        exit(1);
    }

    if main_module == "sgleam" {
        return Err(Error::Type {
            path: input,
            src: "".into(),
            errors: vec1![type_::Error::ReservedModuleName {
                name: "sgleam".into(),
            }],
        });
    }

    if cli.format {
        format::run(false, false, vec![input.into()])?;
        return Ok(());
    }

    let mut project = Project::default();
    project.copy_to_source(&input)?;

    let modules = compile(&mut project, cli.interative)?;

    if cli.interative {
        repl(&mut project, Some(main_module));
    } else if let Some(module) = get_module(modules, main_module) {
        let _mainf = get_main_function(&module)?;
        let context = &create_js_context(project.fs.clone(), Project::out().into());
        let source = main_js_script(main_module, cli.test);
        run_js(context, source);
    } else {
        // The gleam compile ignored the file because of the file name.
    }

    Ok(())
}

fn repl(project: &mut Project, module: Option<&str>) {
    let editor = ReplReader::new().expect("Create the reader for repl");
    let context = create_js_context(project.fs.clone(), Project::out().into());
    for (n, code) in editor.filter(|s| !s.is_empty()).enumerate() {
        #[cfg(debug_assertions)]
        let start = std::time::Instant::now();

        let file = format!("repl{n}.gleam");
        write_repl_source(project, &file, &code, module);
        match compile(project, true) {
            Err(err) => show_gleam_error(err),
            Ok(_) => {
                run_js(&context, main_js_script(&format!("repl{n}"), false));
            }
        }
        project
            .fs
            .delete_file(&Project::source().join(file))
            .expect("Delete repl file");

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

fn main_js_script(module: &str, test: bool) -> String {
    if !test {
        format!(
            "
            import {{ try_main }} from \"./sgleam_ffi.mjs\";
            import {{ main }} from \"./{module}.mjs\";
            try_main(main);
            "
        )
    } else {
        format!(
            "
            import {{ run_tests }} from \"./sgleam_ffi.mjs\";
            import * as {module} from \"./{module}.mjs\";
            run_tests({module});
            "
        )
    }
}
