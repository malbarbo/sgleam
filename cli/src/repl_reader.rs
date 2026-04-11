use std::cell::RefCell;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use rustyline::{
    Cmd, ConditionalEventHandler, Context, Editor, Event, EventContext, EventHandler, Helper,
    Hinter, KeyCode, KeyEvent, Modifiers, Movement, Prompt, RepeatCount, Result, Validator,
    completion::{self, Completer},
    error::ReadlineError,
    highlight::{CmdKind, Highlighter},
    history::FileHistory,
    validate::{ValidationContext, ValidationResult, Validator},
};

const HISTORY_DIR: &str = "sgleam";
const HISTORY_FILE: &str = "history";

pub type Completions = Rc<RefCell<Vec<String>>>;

pub struct ReplReader {
    // We use Option to implement Iterator which ends after the first None.
    editor: Option<Editor<InputHelper, FileHistory>>,
}

impl ReplReader {
    pub fn new(completions: Completions) -> Result<ReplReader> {
        let config = rustyline::Config::builder()
            .completion_type(rustyline::CompletionType::List)
            .build();
        let mut editor = Editor::with_config(config)?;

        let color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();

        editor.set_helper(Some(InputHelper {
            validator: BracketsStringValidator {},
            completions,
            color,
        }));

        editor.bind_sequence(
            KeyEvent(KeyCode::Enter, Modifiers::NONE),
            EventHandler::Conditional(Box::new(AutoIndentHandler)),
        );
        editor.bind_sequence(
            KeyEvent(KeyCode::Tab, Modifiers::NONE),
            EventHandler::Conditional(Box::new(TabHandler)),
        );
        editor.bind_sequence(
            KeyEvent(KeyCode::Backspace, Modifiers::NONE),
            EventHandler::Conditional(Box::new(SmartBackspace)),
        );
        editor.bind_sequence(
            KeyEvent(KeyCode::Char('}'), Modifiers::NONE),
            EventHandler::Conditional(Box::new(AutoDedent)),
        );

        if let Some(history) = &history_path() {
            let _ = editor.load_history(history);
        }

        Ok(ReplReader {
            editor: Some(editor),
        })
    }
}

struct ReplPrompt {
    color: bool,
}

impl Prompt for ReplPrompt {
    fn raw(&self) -> &str {
        "> "
    }

    fn styled(&self) -> &str {
        if self.color {
            // Leak a formatted string so we can return &str.
            // This is called once per prompt display, and theme changes are rare.
            let s = format!("{}>{RESET} ", theme().prompt);
            Box::leak(s.into_boxed_str())
        } else {
            "> "
        }
    }

    fn continuation_raw(&self) -> &str {
        "  "
    }
}

impl Iterator for ReplReader {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut editor = self.editor.take()?;
        let color = editor.helper().is_some_and(|h| h.color);
        let prompt = ReplPrompt { color };

        match editor.readline(&prompt) {
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
    dirs::data_dir().map(|mut p| {
        p.push(HISTORY_DIR);
        let _ = std::fs::create_dir_all(&p);
        p.push(HISTORY_FILE);
        p
    })
}

#[derive(Helper, Hinter, Validator)]
struct InputHelper {
    #[rustyline(Validator)]
    validator: BracketsStringValidator,
    completions: Completions,
    color: bool,
}

fn is_break_char(c: char) -> bool {
    !c.is_alphanumeric() && c != '_' && c != ':' && c != '.'
}

impl Completer for InputHelper {
    type Candidate = String;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Result<(usize, Vec<String>)> {
        let (start, prefix) = completion::extract_word(line, pos, None, is_break_char);
        if prefix.is_empty() {
            return Ok((start, vec![]));
        }
        let candidates = self
            .completions
            .borrow()
            .iter()
            .filter(|name| name.starts_with(prefix))
            .cloned()
            .collect();
        Ok((start, candidates))
    }
}

const RESET: &str = "\x1b[0m";

struct Theme {
    comment: &'static str,
    string: &'static str,
    number: &'static str,
    keyword: &'static str,
    function: &'static str,
    type_: &'static str,
    prompt: &'static str,
}

// Zed One Dark
const ONE_DARK: Theme = Theme {
    comment: "\x1b[38;2;93;99;111m",
    string: "\x1b[38;2;161;193;129m",
    number: "\x1b[38;2;191;149;106m",
    keyword: "\x1b[38;2;180;119;207m",
    function: "\x1b[38;2;115;173;233m",
    type_: "\x1b[38;2;223;193;132m",
    prompt: "\x1b[38;2;115;173;233m",
};

// Zed One Light
const ONE_LIGHT: Theme = Theme {
    comment: "\x1b[38;2;162;163;167m",
    string: "\x1b[38;2;100;159;87m",
    number: "\x1b[38;2;173;110;37m",
    keyword: "\x1b[38;2;164;73;171m",
    function: "\x1b[38;2;91;121;227m",
    type_: "\x1b[38;2;193;132;1m",
    prompt: "\x1b[38;2;91;121;227m",
};

static USE_LIGHT_THEME: AtomicBool = AtomicBool::new(false);

pub fn set_theme(light: bool) {
    USE_LIGHT_THEME.store(light, Ordering::Relaxed);
}

pub fn is_light_theme() -> bool {
    USE_LIGHT_THEME.load(Ordering::Relaxed)
}

fn theme() -> &'static Theme {
    if is_light_theme() {
        &ONE_LIGHT
    } else {
        &ONE_DARK
    }
}

const KEYWORDS: &[&str] = &[
    "as", "assert", "case", "const", "else", "external", "fn", "if", "import", "let", "opaque",
    "panic", "pub", "todo", "type", "use",
];

impl Highlighter for InputHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> std::borrow::Cow<'l, str> {
        if self.color {
            std::borrow::Cow::Owned(highlight_gleam(line))
        } else {
            std::borrow::Cow::Borrowed(line)
        }
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: CmdKind) -> bool {
        self.color
    }
}

pub fn highlight_gleam(input: &str) -> String {
    let t = theme();
    let mut out = String::with_capacity(input.len() * 2);
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];

        // Comments
        if c == '/' && i + 1 < len && chars[i + 1] == '/' {
            out.push_str(t.comment);
            while i < len && chars[i] != '\n' {
                out.push(chars[i]);
                i += 1;
            }
            out.push_str(RESET);
            continue;
        }

        // Strings
        if c == '"' {
            out.push_str(t.string);
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
            out.push_str(t.number);
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
                out.push_str(t.keyword);
                out.push_str(&word);
                out.push_str(RESET);
            } else if word == "True" || word == "False" || word == "Nil" {
                out.push_str(t.number);
                out.push_str(&word);
                out.push_str(RESET);
            } else if c.is_ascii_uppercase() {
                // Type name
                out.push_str(t.type_);
                out.push_str(&word);
                out.push_str(RESET);
            } else if i < len && chars[i] == '(' {
                // Function call
                out.push_str(t.function);
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
            out.push_str(t.function);
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

struct BracketsStringValidator {}

impl Validator for BracketsStringValidator {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<ValidationResult> {
        Ok(validate_brackets_and_string(ctx.input()))
    }
}

fn validate_brackets_and_string(string: &str) -> ValidationResult {
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
                if !bracket_match(stack.pop().unwrap_or(' '), c) {
                    // Mismatched bracket: submit as-is and let the compiler report the error.
                    return ValidationResult::Valid(None);
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

fn nesting_depth(input: &str) -> usize {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if in_string {
            if c == '\\' {
                chars.next();
            } else if c == '"' {
                in_string = false;
            }
        } else {
            match c {
                '"' => in_string = true,
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
        }
    }
    depth.max(0) as usize
}

struct AutoIndentHandler;

impl ConditionalEventHandler for AutoIndentHandler {
    fn handle(
        &self,
        _evt: &Event,
        _n: RepeatCount,
        _positive: bool,
        ctx: &EventContext,
    ) -> Option<Cmd> {
        let input = ctx.line();
        let at_end = ctx.pos() == input.len();
        if matches!(
            validate_brackets_and_string(input),
            ValidationResult::Incomplete
        ) {
            let depth = nesting_depth(input);
            let indent = "  ".repeat(depth);
            Some(Cmd::Insert(1, format!("\n{indent}")))
        } else if !at_end {
            Some(Cmd::Newline)
        } else {
            None // default behavior (accept line)
        }
    }
}

/// Tab handler: insert 2 spaces for indentation, or trigger completion.
struct TabHandler;

impl ConditionalEventHandler for TabHandler {
    fn handle(
        &self,
        _evt: &Event,
        _n: RepeatCount,
        _positive: bool,
        ctx: &EventContext,
    ) -> Option<Cmd> {
        let line = ctx.line();
        let pos = ctx.pos();
        // If at start of line or only whitespace before cursor, insert indentation
        let before = &line[..pos];
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        if before[line_start..].chars().all(|c| c.is_whitespace()) {
            Some(Cmd::Insert(1, "  ".into()))
        } else {
            // Trigger completion
            Some(Cmd::Complete)
        }
    }
}

/// Smart backspace: on continuation lines with only spaces, snap to 2-space
/// indent boundaries.
struct SmartBackspace;

impl ConditionalEventHandler for SmartBackspace {
    fn handle(
        &self,
        _evt: &Event,
        _n: RepeatCount,
        _positive: bool,
        ctx: &EventContext,
    ) -> Option<Cmd> {
        let line = ctx.line();
        let pos = ctx.pos();
        let line_start = line[..pos].rfind('\n').map_or(0, |i| i + 1);
        let current_line = &line[line_start..pos];
        // Only on continuation lines where cursor is in leading whitespace
        if line_start > 0 && current_line.len() > 1 && current_line.bytes().all(|b| b == b' ') {
            let spaces = current_line.len();
            let remove = if spaces.is_multiple_of(2) { 2 } else { 1 };
            Some(Cmd::Kill(Movement::BackwardChar(remove)))
        } else {
            None
        }
    }
}

/// When `}` is typed on a continuation line with only whitespace, removes one
/// indent level (2 spaces) before inserting `}`.
struct AutoDedent;

impl ConditionalEventHandler for AutoDedent {
    fn handle(
        &self,
        _evt: &Event,
        _n: RepeatCount,
        _positive: bool,
        ctx: &EventContext,
    ) -> Option<Cmd> {
        let line = ctx.line();
        let pos = ctx.pos();
        let line_start = line[..pos].rfind('\n').map_or(0, |i| i + 1);
        let current_line = &line[line_start..pos];
        if line_start > 0 && current_line.len() >= 2 && current_line.bytes().all(|b| b == b' ') {
            Some(Cmd::Replace(Movement::BackwardChar(2), Some("}".into())))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use rustyline::validate::ValidationResult;

    use crate::repl_reader::validate_brackets_and_string;

    #[test]
    fn test_brackets_and_string_ok() {
        assert!(matches!(
            validate_brackets_and_string("4 + (3 * { [4] - 2 })"),
            ValidationResult::Valid(None)
        ));
        assert!(matches!(
            validate_brackets_and_string("\"ca\\\"sa\""),
            ValidationResult::Valid(None)
        ));
        assert!(matches!(
            validate_brackets_and_string("\"ca\"sa\""),
            ValidationResult::Incomplete
        ));
        assert!(matches!(
            validate_brackets_and_string("4 + 3 * { 4 - 2 })"),
            ValidationResult::Valid(None)
        ));
        assert!(matches!(
            validate_brackets_and_string("4 + (3 * { 4 - 2 )"),
            ValidationResult::Valid(None)
        ));
    }
}
