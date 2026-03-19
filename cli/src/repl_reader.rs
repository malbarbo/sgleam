use std::path::PathBuf;

use rustyline::{
    error::ReadlineError,
    highlight::Highlighter,
    history::FileHistory,
    validate::{ValidationContext, ValidationResult, Validator},
    Completer, Editor, Helper, Hinter, Result, Validator,
};

const PROMPT: &str = "> ";
const HISTORY_FILE: &str = ".sgleam_history";

// TODO: add auto ident
// TODO: add completation
pub struct ReplReader {
    // We use Option to implement Iterator which ends after the first None.
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

impl Iterator for ReplReader {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut editor = self.editor.take()?;

        match editor.readline(PROMPT) {
            Ok(input) => {
                if !input.trim().is_empty() {
                    let _ = editor.add_history_entry(&input);
                }
                self.editor = Some(editor);
                Some(input)
            }
            Err(ReadlineError::Interrupted) => {
                self.editor = Some(editor);
                Some("".into())
            }
            Err(err) => {
                if !matches!(err, ReadlineError::Eof) {
                    eprintln!("Error: {err}");
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
    dirs::home_dir().map(|p: PathBuf| p.join(HISTORY_FILE))
}

#[derive(Completer, Helper, Hinter, Validator)]
struct InputValidator {
    #[rustyline(Validator)]
    validator: BracketsStringValidador,
}

// ANSI color codes matching the web editor's One Light theme.
const RESET: &str = "\x1b[0m";
const GRAY: &str = "\x1b[90m"; // comment
const GREEN: &str = "\x1b[32m"; // string
const YELLOW: &str = "\x1b[33m"; // number, boolean
const MAGENTA: &str = "\x1b[35m"; // keyword
const BLUE: &str = "\x1b[34m"; // function
const CYAN: &str = "\x1b[36m"; // operator, type

const KEYWORDS: &[&str] = &[
    "as", "assert", "case", "const", "else", "external", "fn", "if", "import", "let", "opaque",
    "panic", "pub", "todo", "type", "use",
];

impl Highlighter for InputValidator {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> std::borrow::Cow<'l, str> {
        std::borrow::Cow::Owned(highlight_gleam(line))
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        true
    }
}

fn highlight_gleam(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];

        // Comments
        if c == '/' && i + 1 < len && chars[i + 1] == '/' {
            out.push_str(GRAY);
            while i < len && chars[i] != '\n' {
                out.push(chars[i]);
                i += 1;
            }
            out.push_str(RESET);
            continue;
        }

        // Strings
        if c == '"' {
            out.push_str(GREEN);
            out.push(c);
            i += 1;
            while i < len {
                let sc = chars[i];
                out.push(sc);
                i += 1;
                if sc == '\\' && i < len {
                    out.push(chars[i]);
                    i += 1;
                } else if sc == '"' {
                    break;
                }
            }
            out.push_str(RESET);
            continue;
        }

        // Numbers
        if c.is_ascii_digit() {
            out.push_str(YELLOW);
            while i < len
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '.')
            {
                out.push(chars[i]);
                i += 1;
            }
            out.push_str(RESET);
            continue;
        }

        // Identifiers and keywords
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();

            if KEYWORDS.contains(&word.as_str()) {
                out.push_str(MAGENTA);
                out.push_str(&word);
                out.push_str(RESET);
            } else if word == "True" || word == "False" || word == "Nil" {
                out.push_str(YELLOW);
                out.push_str(&word);
                out.push_str(RESET);
            } else if c.is_ascii_uppercase() {
                // Type name
                out.push_str(CYAN);
                out.push_str(&word);
                out.push_str(RESET);
            } else if i < len && chars[i] == '(' {
                // Function call
                out.push_str(BLUE);
                out.push_str(&word);
                out.push_str(RESET);
            } else {
                out.push_str(&word);
            }
            continue;
        }

        // Operators
        if matches!(
            c,
            '+' | '-' | '*' | '/' | '%' | '<' | '>' | '=' | '!' | '|' | '&' | '.'
        ) {
            out.push_str(CYAN);
            out.push(c);
            i += 1;
            // Consume multi-char operators
            while i < len && matches!(chars[i], '>' | '=' | '.' | '|' | '&') {
                out.push(chars[i]);
                i += 1;
            }
            out.push_str(RESET);
            continue;
        }

        out.push(c);
        i += 1;
    }

    out
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

    use crate::repl_reader::validade_brackets_and_string;

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
