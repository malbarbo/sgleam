use std::cell::RefCell;
use std::io::IsTerminal;
use std::path::PathBuf;

use engine::shell::word_at;
use rustyline::{
    Cmd, ConditionalEventHandler, Context, Editor, Event, EventContext, EventHandler, Helper,
    Hinter, KeyCode, KeyEvent, Modifiers, Movement, Prompt, RepeatCount, Result, Validator,
    completion::Completer,
    error::ReadlineError,
    highlight::{CmdKind, Highlighter},
    history::FileHistory,
    validate::{ValidationContext, ValidationResult, Validator},
};

const HISTORY_DIR: &str = "sgleam";
const HISTORY_FILE: &str = "history";

pub struct ReplReader {
    // `next` takes the editor to read and gives it back, except on the read
    // that ends the session, which is how the iteration stops.
    editor: Option<Editor<InputHelper, FileHistory>>,
}

impl ReplReader {
    pub fn new(completions: Vec<String>, theme: Theme) -> Result<ReplReader> {
        let config = rustyline::Config::builder()
            .completion_type(rustyline::CompletionType::List)
            .build();
        let mut editor = Editor::with_config(config)?;

        let color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();

        editor.set_helper(Some(InputHelper {
            validator: CompleteInputValidator::default(),
            completions,
            color,
            theme,
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
        for close in [']', ')', '}'] {
            editor.bind_sequence(
                KeyEvent(KeyCode::Char(close), Modifiers::NONE),
                EventHandler::Conditional(Box::new(AutoDedent(close))),
            );
        }

        if let Some(history) = &history_path() {
            let _ = editor.load_history(history);
        }

        Ok(ReplReader {
            editor: Some(editor),
        })
    }

    /// The names Tab offers, which grow with every input the repl takes.
    pub fn set_completions(&mut self, completions: Vec<String>) {
        if let Some(helper) = self.editor.as_mut().and_then(Editor::helper_mut) {
            helper.completions = completions;
        }
    }

    pub fn set_theme(&mut self, theme: Theme) {
        if let Some(helper) = self.editor.as_mut().and_then(Editor::helper_mut) {
            helper.theme = theme;
        }
    }
}

struct ReplPrompt {
    /// The prompt in the theme's colors, or none when the output takes no color.
    styled: Option<String>,
}

impl Prompt for ReplPrompt {
    fn raw(&self) -> &str {
        "> "
    }

    fn styled(&self) -> &str {
        self.styled.as_deref().unwrap_or(self.raw())
    }

    fn continuation_raw(&self) -> &str {
        "  "
    }
}

impl Iterator for ReplReader {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut editor = self.editor.take()?;
        let styled = editor
            .helper()
            .filter(|h| h.color)
            .map(|h| format!("{}>{RESET} ", h.theme.palette().prompt));
        let prompt = ReplPrompt { styled };

        match editor.readline(&prompt) {
            Ok(input) => {
                if !input.trim().is_empty() {
                    let _ = editor.add_history_entry(&input);
                }
                self.editor = Some(editor);
                Some(input)
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C throws the input away, so nothing is left pending.
                take_pending(&editor);
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
                // An input the file ended in the middle of still runs: the
                // user wants to read what is wrong with it.
                let pending = take_pending(&editor);
                (!pending.trim().is_empty()).then_some(pending)
            }
        }
    }
}

fn take_pending(editor: &Editor<InputHelper, FileHistory>) -> String {
    editor
        .helper()
        .map_or_else(String::new, |helper| helper.validator.pending.take())
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
    validator: CompleteInputValidator,
    completions: Vec<String>,
    color: bool,
    theme: Theme,
}

impl Completer for InputHelper {
    type Candidate = String;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Result<(usize, Vec<String>)> {
        let (start, prefix) = word_at(line, pos);
        if prefix.is_empty() {
            return Ok((start, vec![]));
        }
        let candidates = self
            .completions
            .iter()
            .filter(|name| name.starts_with(prefix))
            .cloned()
            .collect();
        Ok((start, candidates))
    }
}

const RESET: &str = "\x1b[0m";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn parse(name: &str) -> Option<Theme> {
        match name {
            "dark" => Some(Theme::Dark),
            "light" => Some(Theme::Light),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }

    fn palette(self) -> &'static Palette {
        match self {
            Theme::Dark => &ONE_DARK,
            Theme::Light => &ONE_LIGHT,
        }
    }
}

struct Palette {
    comment: &'static str,
    string: &'static str,
    number: &'static str,
    keyword: &'static str,
    function: &'static str,
    type_: &'static str,
    command: &'static str,
    prompt: &'static str,
}

// Zed One Dark
const ONE_DARK: Palette = Palette {
    comment: "\x1b[38;2;93;99;111m",
    string: "\x1b[38;2;161;193;129m",
    number: "\x1b[38;2;191;149;106m",
    keyword: "\x1b[38;2;180;119;207m",
    function: "\x1b[38;2;115;173;233m",
    type_: "\x1b[38;2;223;193;132m",
    command: "\x1b[38;2;130;137;151m",
    prompt: "\x1b[38;2;115;173;233m",
};

// Zed One Light
const ONE_LIGHT: Palette = Palette {
    comment: "\x1b[38;2;162;163;167m",
    string: "\x1b[38;2;100;159;87m",
    number: "\x1b[38;2;173;110;37m",
    keyword: "\x1b[38;2;164;73;171m",
    function: "\x1b[38;2;91;121;227m",
    type_: "\x1b[38;2;193;132;1m",
    command: "\x1b[38;2;105;108;119m",
    prompt: "\x1b[38;2;91;121;227m",
};

const KEYWORDS: &[&str] = &[
    "as", "assert", "case", "const", "echo", "else", "external", "fn", "if", "import", "let",
    "opaque", "panic", "pub", "todo", "type", "use",
];

impl Highlighter for InputHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> std::borrow::Cow<'l, str> {
        if self.color {
            std::borrow::Cow::Owned(highlight_gleam(line, self.theme.palette()))
        } else {
            std::borrow::Cow::Borrowed(line)
        }
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: CmdKind) -> bool {
        self.color
    }
}

fn highlight_gleam(input: &str, t: &Palette) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    // A command is the first word of the input and nowhere else, which is what
    // the shell itself reads. Anywhere else the `:` is the one of a type
    // annotation, and the word after it is Gleam.
    let mut start = 0;
    while start < len && chars[start].is_whitespace() {
        start += 1;
    }
    if chars.get(start) == Some(&':') && chars.get(start + 1).is_some_and(|c| c.is_alphabetic()) {
        out.extend(&chars[..start]);
        out.push_str(t.command);
        out.push(':');
        i = start + 1;
        while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
            out.push(chars[i]);
            i += 1;
        }
        out.push_str(RESET);
    }

    while i < len {
        let c = chars[i];

        if c == '/' && i + 1 < len && chars[i + 1] == '/' {
            out.push_str(t.comment);
            while i < len && chars[i] != '\n' {
                out.push(chars[i]);
                i += 1;
            }
            out.push_str(RESET);
            continue;
        }

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

        // A letter outside ASCII is in no name the language allows, but it is
        // in the word the user typed: were it not a word char, what follows it
        // would start a word of its own and be read as a keyword.
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
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
            } else if c.is_uppercase() {
                out.push_str(t.type_);
                out.push_str(&word);
                out.push_str(RESET);
            } else if i < len && chars[i] == '(' {
                out.push_str(t.function);
                out.push_str(&word);
                out.push_str(RESET);
            } else {
                out.push_str(&word);
            }
            continue;
        }

        if matches!(
            c,
            '+' | '-' | '*' | '/' | '%' | '<' | '>' | '=' | '!' | '|' | '&' | '.'
        ) {
            out.push_str(t.function);
            out.push(c);
            i += 1;
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

#[derive(Default)]
struct CompleteInputValidator {
    /// The unfinished input. The editor throws it away when the file ends
    /// before the rest of it, and says nothing about it, so the reader keeps a
    /// copy here.
    pending: RefCell<String>,
}

impl Validator for CompleteInputValidator {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<ValidationResult> {
        let result = validate(ctx.input());
        *self.pending.borrow_mut() = match result {
            ValidationResult::Incomplete => ctx.input().into(),
            _ => String::new(),
        };
        Ok(result)
    }
}

/// Whether the line the user just ended is the whole input, which only the
/// parser can say. The prompt in the browser asks the same function through the
/// `repl_ready` export.
fn validate(input: &str) -> ValidationResult {
    if engine::shell::ready_state(input) < 0 {
        ValidationResult::Valid(None)
    } else {
        ValidationResult::Incomplete
    }
}

/// The indentation of the caret's own line, one level deeper when the text in
/// front of the caret leaves a bracket open. The new line starts with this
/// whenever the engine answered about a place the caret is not in.
fn local_indent(input: &str, pos: usize) -> String {
    let before = &input[..pos];
    let line = before.rsplit_once('\n').map_or(before, |(_, line)| line);
    let current: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    if line.trim_end().ends_with(['(', '[', '{']) {
        format!("{current}  ")
    } else {
        current
    }
}

/// Enter, in the three shapes the prompt in the browser gives it: a line opened
/// inside an input of several lines, a line accepted, a line grown by one.
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
        let pos = ctx.pos();
        let at_end = pos == input.len();

        if !at_end && input.contains('\n') {
            return Some(Cmd::Insert(1, format!("\n{}", local_indent(input, pos))));
        }

        let ready = engine::shell::ready_state(input);
        if ready < 0 {
            return None; // rustyline's own Enter, which accepts the line
        }

        let indent = if at_end {
            " ".repeat(ready as usize)
        } else {
            local_indent(input, pos)
        };
        Some(Cmd::Insert(1, format!("\n{indent}")))
    }
}

/// Tab indents when only whitespace comes before the caret, and completes
/// otherwise.
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
        let before = &line[..pos];
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        if before[line_start..].chars().all(|c| c.is_whitespace()) {
            Some(Cmd::Insert(1, "  ".into()))
        } else {
            Some(Cmd::Complete)
        }
    }
}

/// Backspace takes back a whole level of indentation on a continuation line
/// with only spaces before the caret.
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
        if line_start > 0 && current_line.len() > 1 && current_line.bytes().all(|b| b == b' ') {
            let spaces = current_line.len();
            let remove = if spaces.is_multiple_of(2) { 2 } else { 1 };
            Some(Cmd::Kill(Movement::BackwardChar(remove)))
        } else {
            None
        }
    }
}

/// A closing bracket typed on a continuation line with only spaces before it
/// takes a level of indentation back first. Every kind of bracket costs a level
/// going in, so every kind gives one back.
struct AutoDedent(char);

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
            Some(Cmd::Replace(Movement::BackwardChar(2), Some(self.0.into())))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use rustyline::validate::ValidationResult;

    use crate::repl_reader::validate;

    fn incomplete(input: &str) -> bool {
        matches!(validate(input), ValidationResult::Incomplete)
    }

    #[test]
    fn an_input_that_ends_before_what_it_started() {
        assert!(incomplete("use a <-"));
        assert!(incomplete("todo as"));

        assert!(incomplete("case 1 {"));
        assert!(incomplete("pub fn f() {\n  1"));
        assert!(incomplete("io.println("));
        assert!(incomplete("[1, 2"));
        assert!(incomplete("import gleam/io.{"));
        assert!(incomplete("\"ca\"sa\""));
        assert!(incomplete("let x = \"abc"));
        assert!(incomplete("let x ="));
        assert!(incomplete("1 +"));
        assert!(incomplete("case 1 { 1 ->"));
    }

    #[test]
    fn a_bracket_the_input_only_mentions() {
        assert!(!incomplete("1 + 1 // {"));
        assert!(!incomplete("// {"));
        assert!(!incomplete("1 + 1 // \""));
        assert!(!incomplete("\"{\""));
    }

    #[test]
    fn an_input_that_is_whole_and_does_not_compile() {
        assert!(!incomplete("let x = )"));
        assert!(!incomplete("1 + + 1"));
        assert!(!incomplete("4 + 3 * { 4 - 2 })"));
        assert!(!incomplete("4 + (3 * { 4 - 2 )"));
        assert!(!incomplete("4 + (3 * { [4] - 2 })"));
    }

    #[test]
    fn a_command_is_asked_about_what_it_carries() {
        assert!(incomplete(":type case 1 {"));
        assert!(incomplete(":time [1,"));
        assert!(!incomplete(":quit"));
        assert!(!incomplete(":debug"));
        assert!(!incomplete(":type 1 + 1"));
    }

    #[test]
    fn a_blank_line_ends_an_input_with_nothing_open() {
        // The reader has nothing else to offer: `let x =` has no bracket to
        // type, and what the compiler says about it is what the user is after.
        assert!(!incomplete("let x =\n"));
        // With a bracket open it is a blank line inside the input, which is
        // how a function with two statements in it is written.
        assert!(incomplete("case 1 {\n  "));
        assert!(incomplete("pub fn f() {\n  let x = 1\n"));
    }

    #[test]
    fn an_indent_read_off_the_caret_s_own_line() {
        use crate::repl_reader::local_indent;
        assert_eq!(local_indent("case x {", 8), "  ");
        assert_eq!(local_indent("  f(", 4), "    ");
        assert_eq!(local_indent("  f(x)", 6), "  ");
        assert_eq!(local_indent("a\n  b", 5), "  ");
        assert_eq!(local_indent("case x {\n  1 -> 2", 17), "  ");
        assert_eq!(local_indent("f(1)", 0), "");
        assert_eq!(local_indent("  [\n    1,\n  ]", 13), "  ");
    }

    #[test]
    fn a_whole_input_is_whole() {
        assert!(!incomplete("4 + 3 * { [4] - 2 }"));
        assert!(!incomplete("\"ca\\\"sa\""));
        assert!(!incomplete(""));
        assert!(!incomplete("pub fn f() {\n  1\n}"));
    }

    #[test]
    fn a_word_a_letter_outside_ascii_is_in_is_still_one_word() {
        use crate::repl_reader::{ONE_DARK, highlight_gleam};
        assert!(highlight_gleam("as", &ONE_DARK).contains(ONE_DARK.keyword));
        assert_eq!(highlight_gleam("\u{e7}as", &ONE_DARK), "\u{e7}as");
        assert_eq!(highlight_gleam("\u{e7}Int", &ONE_DARK), "\u{e7}Int");
        assert_eq!(highlight_gleam("as\u{e7}", &ONE_DARK), "as\u{e7}");
    }

    #[test]
    fn a_command_is_not_gleam_and_is_not_colored_as_gleam() {
        use crate::repl_reader::{ONE_DARK, RESET, highlight_gleam};
        // The `type` of `:type` is not the keyword that declares a type.
        assert_eq!(
            highlight_gleam(":type foo", &ONE_DARK),
            format!("{}:type{RESET} foo", ONE_DARK.command)
        );
        // Only the word the input starts with: the other `:` is Gleam's own.
        assert!(!highlight_gleam("let x: Int = 1", &ONE_DARK).contains(ONE_DARK.command));
        assert!(!highlight_gleam("f(a: 1)", &ONE_DARK).contains(ONE_DARK.command));
    }
}
