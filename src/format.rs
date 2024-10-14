// This file is from gleam project
// compiler-cli/src/format.rs

use gleam_core::{
    error::{Error, FileIoAction, FileKind, Result, StandardIoAction, Unformatted},
    io::{Content, OutputFile},
};
use std::{
    fs::File,
    io::{Read, Write},
    str::FromStr,
};

use camino::{Utf8Path, Utf8PathBuf};

pub fn run(stdin: bool, check: bool, files: Vec<String>) -> Result<()> {
    if stdin {
        process_stdin(check)
    } else {
        process_files(check, files)
    }
}

fn process_stdin(check: bool) -> Result<()> {
    let src = read_stdin()?.into();
    let mut out = String::new();
    gleam_core::format::pretty(&mut out, &src, Utf8Path::new("<stdin>"))?;

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

fn process_files(check: bool, files: Vec<String>) -> Result<()> {
    if check {
        check_files(files)
    } else {
        format_files(files)
    }
}

fn check_files(files: Vec<String>) -> Result<()> {
    let problem_files = unformatted_files(files)?;

    if problem_files.is_empty() {
        Ok(())
    } else {
        Err(Error::Format { problem_files })
    }
}

fn format_files(files: Vec<String>) -> Result<()> {
    for file in unformatted_files(files)? {
        write_output(&OutputFile {
            path: file.destination,
            content: Content::Text(file.output),
        })?;
    }
    Ok(())
}

fn unformatted_files(files: Vec<String>) -> Result<Vec<Unformatted>> {
    let mut problem_files = Vec::with_capacity(files.len());

    for file_path in files {
        let path = Utf8PathBuf::from_str(&file_path).map_err(|e| Error::FileIo {
            action: FileIoAction::Open,
            kind: FileKind::File,
            path: Utf8PathBuf::from(file_path),
            err: Some(e.to_string()),
        })?;

        if !path.is_dir() {
            format_file(&mut problem_files, path)?;
        }
    }

    Ok(problem_files)
}

fn format_file(problem_files: &mut Vec<Unformatted>, path: Utf8PathBuf) -> Result<()> {
    let src = read(&path)?.into();
    let mut output = String::new();
    gleam_core::format::pretty(&mut output, &src, &path)?;

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

fn write_output(file: &OutputFile) -> Result<(), Error> {
    let OutputFile { path, content } = file;
    match content {
        Content::Binary(buffer) => write_bytes(path, buffer),
        Content::Text(buffer) => write(path, buffer),
    }
}

fn write(path: &Utf8Path, text: &str) -> Result<(), Error> {
    write_bytes(path, text.as_bytes())
}

fn write_bytes(path: &Utf8Path, bytes: &[u8]) -> Result<(), Error> {
    let dir_path = path.parent().ok_or_else(|| Error::FileIo {
        action: FileIoAction::FindParent,
        kind: FileKind::Directory,
        path: path.to_path_buf(),
        err: None,
    })?;

    std::fs::create_dir_all(dir_path).map_err(|e| Error::FileIo {
        action: FileIoAction::Create,
        kind: FileKind::Directory,
        path: dir_path.to_path_buf(),
        err: Some(e.to_string()),
    })?;

    let mut f = File::create(path).map_err(|e| Error::FileIo {
        action: FileIoAction::Create,
        kind: FileKind::File,
        path: path.to_path_buf(),
        err: Some(e.to_string()),
    })?;

    f.write_all(bytes).map_err(|e| Error::FileIo {
        action: FileIoAction::WriteTo,
        kind: FileKind::File,
        path: path.to_path_buf(),
        err: Some(e.to_string()),
    })?;
    Ok(())
}
