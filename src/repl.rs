use std::path::PathBuf;

use rustyline::{
    error::ReadlineError,
    history::FileHistory,
    validate::{ValidationContext, ValidationResult, Validator},
    Completer, Editor, Helper, Highlighter, Hinter, Result, Validator,
};

const PROMPT: &str = "> ";
const QUIT: &str = ":quit";
const HISTORY_FILE: &str = ".sgleam_history";

// TODO: add auto ident
// TODO: add completation
pub struct ReplReader {
    editor: Option<Editor<InputValidator, FileHistory>>,
}

impl ReplReader {
    pub fn new() -> Result<ReplReader> {
        let mut editor = Editor::new()?;

        editor.set_helper(Some(InputValidator {
            validator: BracketsStringValidador {},
        }));

        if let Some(history) = &history_path() {
            let _ = editor.load_history(history);
        }

        Ok(ReplReader {
            editor: Some(editor),
        })
    }
}

pub fn welcome_message() -> String {
    format!(
        "Welcome to {}.\nType ctrl-d ou \"{QUIT}\" to exit.\n",
        crate::version()
    )
}

// FIXME: this is not needed, Editor has an iter method...
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

#[derive(Completer, Helper, Highlighter, Hinter, Validator)]
struct InputValidator {
    #[rustyline(Validator)]
    validator: BracketsStringValidador,
}

struct BracketsStringValidador {}

impl Validator for BracketsStringValidador {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<ValidationResult> {
        Ok(validade_brackets_and_string(ctx.input()))
    }
}

fn validade_brackets_and_string(string: &str) -> ValidationResult {
    let mut stack = Vec::new();
    let mut chars = string.chars();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                stack.push('"');
                while let Some(c) = chars.next() {
                    if c == '"' {
                        stack.pop();
                        break;
                    }
                    if c == '\\' && matches!(chars.clone().next(), Some('\\' | '\"')) {
                        chars.next();
                        continue;
                    }
                }
            }

            '(' | '[' | '{' => stack.push(c),

            ')' | ']' | '}' => {
                // FIXME: can we stop the prompt?
                if !bracket_match(stack.pop().unwrap_or(' '), c) {
                    return ValidationResult::Invalid(None);
                }
            }
            _ => {}
        }
    }

    if stack.is_empty() {
        ValidationResult::Valid(None)
    } else {
        ValidationResult::Incomplete
    }
}

fn bracket_match(a: char, b: char) -> bool {
    matches!([a, b], ['(', ')'] | ['[', ']'] | ['{', '}'])
}

#[cfg(test)]
mod tests {
    use rustyline::validate::ValidationResult;

    use crate::repl::validade_brackets_and_string;

    #[test]
    fn test_brackets_and_string_ok() {
        assert!(matches!(
            validade_brackets_and_string("4 + (3 * { [4] - 2 })"),
            ValidationResult::Valid(None)
        ));
        assert!(matches!(
            validade_brackets_and_string("\"ca\\\"sa\""),
            ValidationResult::Valid(None)
        ));
        assert!(matches!(
            validade_brackets_and_string("\"ca\"sa\""),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validade_brackets_and_string("4 + 3 * { 4 - 2 })"),
            ValidationResult::Invalid(None)
        ));
        assert!(matches!(
            validade_brackets_and_string("4 + (3 * { 4 - 2 )"),
            ValidationResult::Invalid(None)
        ));
    }
}
