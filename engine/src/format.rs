// This file is from gleam project
// compiler-cli/src/format.rs

use gleam_core::error::{Error, FileIoAction, FileKind, Result, StandardIoAction, Unformatted};
use std::io::Read;

use camino::{Utf8Path, Utf8PathBuf};

pub fn format_source(source: &str) -> Result<String> {
    let mut out = String::new();
    gleam_format::pretty(&mut out, &source.into(), Utf8Path::new("user.gleam"))?;
    Ok(out)
}

pub fn run(check: bool, files: Vec<Utf8PathBuf>) -> Result<()> {
    if files.is_empty() {
        process_stdin(check)
    } else {
        process_files(check, files)
    }
}

fn process_stdin(check: bool) -> Result<()> {
    let src = read_stdin()?.into();
    let mut out = String::new();
    gleam_format::pretty(&mut out, &src, Utf8Path::new("<stdin>"))?;

    if !check {
        print!("{out}");
        return Ok(());
    }

    if src != out {
        return Err(Error::Format {
            problem_files: vec![Unformatted {
                source: Utf8PathBuf::from("<standard input>"),
                destination: Utf8PathBuf::from("<standard output>"),
                input: src,
                output: out,
            }],
        });
    }

    Ok(())
}

fn process_files(check: bool, files: Vec<Utf8PathBuf>) -> Result<()> {
    if check {
        check_files(files)
    } else {
        format_files(files)
    }
}

fn check_files(files: Vec<Utf8PathBuf>) -> Result<()> {
    let problem_files = unformatted_files(files)?;

    if problem_files.is_empty() {
        Ok(())
    } else {
        Err(Error::Format { problem_files })
    }
}

fn format_files(files: Vec<Utf8PathBuf>) -> Result<()> {
    for file in unformatted_files(files)? {
        write(&file.destination, &file.output)?;
    }
    Ok(())
}

fn unformatted_files(files: Vec<Utf8PathBuf>) -> Result<Vec<Unformatted>> {
    let mut problem_files = Vec::with_capacity(files.len());

    for path in files {
        if path.is_dir() {
            for path in gleam_files(&path)? {
                format_file(&mut problem_files, path)?;
            }
        } else {
            format_file(&mut problem_files, path)?;
        }
    }

    Ok(problem_files)
}

/// Every `.gleam` file under `dir`, in an order that does not depend on the
/// file system. A directory named on the command line stands for the files in
/// it, so that a check over one is a check over all of them.
fn gleam_files(dir: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
    let mut files = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for path in read_dir(&dir)? {
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension() == Some("gleam") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// What `dir` holds, without the hidden entries: a directory like `.git` holds
/// nothing anyone asked to format, and a name that is not utf-8 is not one a
/// gleam module can have.
fn read_dir(dir: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
    let read_error = |err: std::io::Error| Error::FileIo {
        action: FileIoAction::Read,
        kind: FileKind::Directory,
        path: dir.to_path_buf(),
        err: Some(err.to_string()),
    };

    let mut paths = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(read_error)? {
        let path = entry.map_err(read_error)?.path();
        if let Ok(path) = Utf8PathBuf::from_path_buf(path)
            && !path.file_name().is_some_and(|name| name.starts_with('.'))
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn format_file(problem_files: &mut Vec<Unformatted>, path: Utf8PathBuf) -> Result<()> {
    let src = read(&path)?.into();
    let mut output = String::new();
    gleam_format::pretty(&mut output, &src, &path)?;

    if src != output {
        problem_files.push(Unformatted {
            source: path.clone(),
            destination: path,
            input: src,
            output,
        });
    }
    Ok(())
}

fn read_stdin() -> Result<String> {
    let mut src = String::new();
    let _ = std::io::stdin()
        .read_to_string(&mut src)
        .map_err(|e| Error::StandardIo {
            action: StandardIoAction::Read,
            err: Some(e.kind()),
        })?;
    Ok(src)
}

fn read(path: impl AsRef<Utf8Path> + std::fmt::Debug) -> Result<String, Error> {
    std::fs::read_to_string(path.as_ref()).map_err(|err| Error::FileIo {
        action: FileIoAction::Read,
        kind: FileKind::File,
        path: Utf8PathBuf::from(path.as_ref()),
        err: Some(err.to_string()),
    })
}

fn write(path: &Utf8Path, text: &str) -> Result<(), Error> {
    std::fs::write(path, text).map_err(|err| Error::FileIo {
        action: FileIoAction::WriteTo,
        kind: FileKind::File,
        path: path.to_path_buf(),
        err: Some(err.to_string()),
    })
}
