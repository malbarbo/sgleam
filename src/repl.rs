use std::path::PathBuf;

use rustyline::{error::ReadlineError, DefaultEditor, Result};

const PROMPT: &str = "> ";
const QUIT: &str = "quit";
const HISTORY_FILE: &str = ".sgleam_history";

pub struct ReplReader {
    editor: Option<DefaultEditor>,
}

impl ReplReader {
    pub fn new() -> Result<ReplReader> {
        let mut editor = DefaultEditor::new()?;

        if let Some(history) = &history_path() {
            let _ = editor.load_history(history);
        }

        // TODO: compile a initial project so the first expression do not take long
        println!("Welcome to {}.", crate::version());
        println!("Type ctrl-d ou \"{QUIT}\" to exit.");

        Ok(ReplReader {
            editor: Some(editor),
        })
    }
}

impl Iterator for ReplReader {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut editor = match self.editor.take() {
            None => return None,
            Some(editor) => editor,
        };

        match editor.readline(PROMPT) {
            Ok(input) => {
                if input.trim() == QUIT {
                    None
                } else {
                    let _ = editor.add_history_entry(&input);
                    self.editor = Some(editor);
                    Some(input)
                }
            }
            Err(ReadlineError::Interrupted) => {
                self.editor = Some(editor);
                Some("".into())
            }
            Err(err) => {
                if !matches!(err, ReadlineError::Eof) {
                    // TODO: improve error message
                    println!("Error: {:?}", err);
                }
                if let Some(history) = &history_path() {
                    let _ = editor.save_history(history);
                }
                None
            }
        }
    }
}

fn history_path() -> Option<PathBuf> {
    dirs::home_dir().map(|p| p.join(HISTORY_FILE))
}
