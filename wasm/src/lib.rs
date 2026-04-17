#![allow(clippy::missing_safety_doc)]

use engine::{
    engine::Engine as _,
    error::{self, show_error},
    gleam::{Project, get_module},
    quickjs::QuickJsEngine,
    repl::{Repl, ReplOutput},
    substitution::{SubstitutionModule, SubstitutionStep},
};
use gleam_core::build::Module;
use std::sync::atomic::{AtomicBool, Ordering};

static INIT: AtomicBool = AtomicBool::new(false);

fn init() {
    if !INIT.swap(true, Ordering::Relaxed) {
        engine::panic::add_handler();
    }
}

// --- Memory ---

#[unsafe(no_mangle)]
pub extern "C" fn string_allocate(size: usize) -> *mut u8 {
    let mut buffer = Vec::with_capacity(size);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn string_deallocate(ptr: *mut u8, size: usize) {
    assert!(!ptr.is_null());
    unsafe {
        let _ = Vec::from_raw_parts(ptr, 0, size);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cstr_deallocate(ptr: *mut std::ffi::c_char) {
    assert!(!ptr.is_null());
    unsafe {
        let _ = std::ffi::CString::from_raw(ptr);
    }
}

fn new_string(ptr: *mut u8, len: usize) -> String {
    assert!(!ptr.is_null());
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).into()
}

fn to_cstr(s: String) -> *mut std::ffi::c_char {
    match std::ffi::CString::new(s) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// --- Config ---

fn parse_config_bigint(config: &str) -> bool {
    config.split_whitespace().any(|entry| {
        entry
            .split_once('=')
            .is_some_and(|(k, v)| k == "bigint" && v == "true")
    })
}

const REPL_OK: u32 = 0;
const REPL_ERROR: u32 = 1;
const REPL_QUIT: u32 = 2;

#[cfg(target_arch = "wasm32")]
mod stepper_ui {
    use super::SubstitutionStep;

    mod ffi {
        #[link(wasm_import_module = "env")]
        unsafe extern "C" {
            #[link_name = "get_key_event"]
            pub fn ffi_get_key_event(key: *mut u8, len: usize, modifiers: *mut bool) -> usize;
            #[link_name = "sleep"]
            pub fn ffi_sleep(ms: u64);
        }
    }

    const EVENT_KEYPRESS: usize = 0;
    const EVENT_KEYDOWN: usize = 1;
    const EVENT_KEYUP: usize = 2;
    const EVENT_NONE: usize = 3;

    enum StepperKey {
        Next,
        Prev,
        Quit,
        None,
    }

    pub fn display_stepper(steps: &[SubstitutionStep]) {
        if steps.is_empty() {
            println!("No steps to display.");
            return;
        }

        let mut current = 0usize;
        loop {
            render_stepper_ansi(steps, current);

            match read_key() {
                StepperKey::Next => {
                    if current < steps.len() {
                        current += 1;
                    }
                }
                StepperKey::Prev => {
                    current = current.saturating_sub(1);
                }
                StepperKey::Quit => {
                    // Limpa a tela e volta ao normal ao sair
                    print!("\x1b[2J\x1b[H\x1b[0m");
                    return;
                }
                StepperKey::None => sleep_ms(16),
            }
        }
    }

    fn sleep_ms(ms: u64) {
        unsafe { ffi::ffi_sleep(ms) }
    }

    fn read_key() -> StepperKey {
        let mut key = [0u8; 32];
        let mut modifiers = [false; 5];
        let event =
            unsafe { ffi::ffi_get_key_event(key.as_mut_ptr(), key.len(), modifiers.as_mut_ptr()) };

        if event == EVENT_NONE || event == EVENT_KEYUP {
            return StepperKey::None;
        }
        if event != EVENT_KEYPRESS && event != EVENT_KEYDOWN {
            return StepperKey::None;
        }

        let key = String::from_utf8_lossy(&key)
            .trim_matches(char::from(0))
            .to_string();
        match key.as_str() {
            "ArrowRight" | "ArrowDown" | "l" | "j" => StepperKey::Next,
            "ArrowLeft" | "ArrowUp" | "h" | "k" => StepperKey::Prev,
            "q" | "Escape" => StepperKey::Quit,
            _ => StepperKey::None,
        }
    }

    fn render_stepper_ansi(steps: &[SubstitutionStep], current: usize) {
        const FRAME_COLS: usize = 80;
        const BASE_CONTENT_ROWS: usize = 18;

        let total_steps = steps.len();
        let left_step = if current > 0 {
            steps.get(current - 1)
        } else {
            None
        };
        let right_step = if current < total_steps {
            steps.get(current)
        } else {
            None
        };

        let title_str = if current < total_steps {
            format!(" Stepper - Step {}/{} ", current + 1, total_steps)
        } else {
            " Stepper - Finished ".to_string()
        };
        let help_str = " q: quit, arrows/hjkl: navigate ";

        let available_width = FRAME_COLS.saturating_sub(7);
        let col_left_w = available_width / 2;
        let col_right_w = available_width - col_left_w;
        let left_fill = col_left_w + 1;
        let right_fill = col_right_w + 1;

        let transition_note = right_step
            .and_then(|s| s.note.as_deref())
            .unwrap_or("")
            .trim();
        let left_title = if left_step.is_some() {
            if transition_note.is_empty() {
                format!("Step {}", current)
            } else {
                format!("Step {} ({})", current, transition_note)
            }
        } else {
            String::new()
        };

        let right_title = if right_step.is_some() {
            format!("Step {}", current + 1)
        } else {
            String::new()
        };

        let left_lines: Vec<String> = left_step
            .map(|s| s.formatted.lines().map(ToString::to_string).collect())
            .unwrap_or_default();
        let right_lines: Vec<String> = right_step
            .map(|s| s.formatted.lines().map(ToString::to_string).collect())
            .unwrap_or_default();
        let context_lines: Vec<String> = right_step
            .and_then(|s| s.context.as_deref())
            .map(|c| c.lines().map(ToString::to_string).collect())
            .unwrap_or_default();

        let mut context_rows = context_lines.len();
        let mut content_rows = BASE_CONTENT_ROWS;
        if context_rows > 0 {
            if content_rows > context_rows + 4 {
                content_rows -= context_rows + 1;
            } else {
                context_rows = 0;
            }
        }

        let title_trunc = truncate_with_ellipsis(&title_str, left_fill);
        let help_trunc = truncate_with_ellipsis(help_str, right_fill);
        let left_pad = left_fill.saturating_sub(title_trunc.chars().count());
        let right_pad = right_fill.saturating_sub(help_trunc.chars().count());

        // Escapes ANSI comuns
        let clear = "\x1b[2J\x1b[H";
        let cyan = "\x1b[36m";
        let yellow_bold = "\x1b[33;1m";
        let grey = "\x1b[90m";
        let reset = "\x1b[0m";
        let bold = "\x1b[1m";

        let mut out = String::with_capacity(4096);
        out.push_str(clear);

        // Topo
        out.push_str(&format!("{cyan}╭─{reset}"));
        out.push_str(&format!("{yellow_bold}{title_trunc}{reset}"));
        out.push_str(&format!(
            "{cyan}{}┬{}{reset}",
            "─".repeat(left_pad),
            "─".repeat(right_pad)
        ));
        out.push_str(&format!("{grey}{help_trunc}{reset}"));
        out.push_str(&format!("{cyan}─╮\r\n{reset}"));

        // Header
        let left_title_cell = format!(
            "{:<width$}",
            truncate_with_ellipsis(&left_title, col_left_w),
            width = col_left_w
        );
        let right_title_cell = format!(
            "{:<width$}",
            truncate_with_ellipsis(&right_title, col_right_w),
            width = col_right_w
        );
        out.push_str(&format!("{cyan}│ {reset}{bold}{left_title_cell}{reset}{cyan} │ {reset}{bold}{right_title_cell}{reset}{cyan} │\r\n{reset}"));

        // Divisor Header
        out.push_str(&format!(
            "{cyan}├─{}─┼─{}─┤\r\n{reset}",
            "─".repeat(col_left_w),
            "─".repeat(col_right_w)
        ));

        // Conteúdo Principal
        for i in 0..content_rows {
            out.push_str(&format!("{cyan}│ {reset}"));
            if let Some(line) = left_lines.get(i) {
                out.push_str(&highlight_gleam_padded(line, col_left_w));
            } else {
                out.push_str(&" ".repeat(col_left_w));
            }
            out.push_str(&format!("{cyan} │ {reset}"));
            if let Some(line) = right_lines.get(i) {
                out.push_str(&highlight_gleam_padded(line, col_right_w));
            } else {
                out.push_str(&" ".repeat(col_right_w));
            }
            out.push_str(&format!("{cyan} │\r\n{reset}"));
        }

        // Contexto Inferior
        if context_rows > 0 {
            out.push_str(&format!(
                "{cyan}├─{}─┴─{}─┤\r\n{reset}",
                "─".repeat(col_left_w),
                "─".repeat(col_right_w)
            ));
            let full_width = FRAME_COLS.saturating_sub(4);
            for i in 0..context_rows {
                out.push_str(&format!("{cyan}│ {reset}"));
                if let Some(line) = context_lines.get(i) {
                    out.push_str(&highlight_gleam_padded(line, full_width));
                } else {
                    out.push_str(&" ".repeat(full_width));
                }
                out.push_str(&format!("{cyan} │\r\n{reset}"));
            }
            out.push_str(&format!(
                "{cyan}╰─{}─╯\r\n{reset}",
                "─".repeat(FRAME_COLS.saturating_sub(4))
            ));
        } else {
            out.push_str(&format!(
                "{cyan}╰─{}─┴─{}─╯\r\n{reset}",
                "─".repeat(col_left_w),
                "─".repeat(col_right_w)
            ));
        }

        print!("{out}");
    }

    fn truncate_with_ellipsis(text: &str, max_chars: usize) -> String {
        if text.chars().count() <= max_chars {
            return text.to_string();
        }
        if max_chars <= 3 {
            return ".".repeat(max_chars);
        }
        let mut out = text
            .chars()
            .take(max_chars.saturating_sub(3))
            .collect::<String>();
        out.push_str("...");
        out
    }

    fn highlight_gleam_padded(input: &str, width: usize) -> String {
        let chunks = highlight_gleam_chunks(input);
        let mut out = String::new();
        let mut visible = 0usize;

        for chunk in chunks {
            if visible >= width {
                break;
            }
            let chunk_len = chunk.text.chars().count();
            let remaining = width.saturating_sub(visible);

            let color = match chunk.class {
                "sg-code-comment" => "\x1b[32m",  // verde
                "sg-code-string" => "\x1b[33m",   // amarelo
                "sg-code-number" => "\x1b[35m",   // magenta
                "sg-code-keyword" => "\x1b[35m",  // magenta
                "sg-code-function" => "\x1b[36m", // cyan
                "sg-code-type" => "\x1b[34m",     // azul
                _ => "\x1b[0m",
            };

            let text_to_print = if chunk_len <= remaining {
                visible += chunk_len;
                chunk.text
            } else {
                let kept: String = chunk.text.chars().take(remaining).collect();
                visible += kept.chars().count();
                kept
            };

            out.push_str(&format!("{color}{text_to_print}\x1b[0m"));
        }

        if visible < width {
            out.push_str(&" ".repeat(width - visible));
        }

        out
    }

    struct StyledChunk {
        class: &'static str,
        text: String,
    }

    const KEYWORDS: &[&str] = &[
        "as", "assert", "case", "const", "else", "external", "fn", "if", "import", "let", "opaque",
        "panic", "pub", "todo", "type", "use",
    ];

    fn push_chunk(chunks: &mut Vec<StyledChunk>, class: &'static str, text: String) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = chunks.last_mut()
            && last.class == class
        {
            last.text.push_str(&text);
            return;
        }
        chunks.push(StyledChunk { class, text });
    }

    fn highlight_gleam_chunks(input: &str) -> Vec<StyledChunk> {
        let chars: Vec<char> = input.chars().collect();
        let len = chars.len();
        let mut i = 0;
        let mut chunks = Vec::new();

        while i < len {
            let c = chars[i];
            if c == '/' && i + 1 < len && chars[i + 1] == '/' {
                let start = i;
                while i < len && chars[i] != '\n' {
                    i += 1;
                }
                push_chunk(
                    &mut chunks,
                    "sg-code-comment",
                    chars[start..i].iter().collect(),
                );
                continue;
            }
            if c == '"' {
                let start = i;
                i += 1;
                while i < len {
                    let sc = chars[i];
                    i += 1;
                    if sc == '\\' && i < len {
                        i += 1;
                    } else if sc == '"' {
                        break;
                    }
                }
                push_chunk(
                    &mut chunks,
                    "sg-code-string",
                    chars[start..i].iter().collect(),
                );
                continue;
            }
            if c.is_ascii_digit() {
                let start = i;
                while i < len
                    && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '.')
                {
                    i += 1;
                }
                push_chunk(
                    &mut chunks,
                    "sg-code-number",
                    chars[start..i].iter().collect(),
                );
                continue;
            }
            if c.is_ascii_alphabetic() || c == '_' {
                let start = i;
                while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let class = if KEYWORDS.contains(&word.as_str()) {
                    "sg-code-keyword"
                } else if word == "True" || word == "False" || word == "Nil" {
                    "sg-code-number"
                } else if c.is_ascii_uppercase() {
                    "sg-code-type"
                } else if i < len && chars[i] == '(' {
                    "sg-code-function"
                } else {
                    "sg-code-base"
                };
                push_chunk(&mut chunks, class, word);
                continue;
            }
            if matches!(
                c,
                '+' | '-' | '*' | '/' | '%' | '<' | '>' | '=' | '!' | '|' | '&' | '.'
            ) {
                let start = i;
                i += 1;
                while i < len && matches!(chars[i], '>' | '=' | '.' | '|' | '&') {
                    i += 1;
                }
                push_chunk(
                    &mut chunks,
                    "sg-code-function",
                    chars[start..i].iter().collect(),
                );
                continue;
            }
            push_chunk(&mut chunks, "sg-code-base", c.to_string());
            i += 1;
        }
        chunks
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod stepper_ui {
    use super::SubstitutionStep;

    pub fn display_stepper(steps: &[SubstitutionStep]) {
        for (index, step) in steps.iter().enumerate() {
            if index > 0 {
                println!();
            }
            println!("{}", step.formatted);
        }
    }
}

// --- REPL ---

fn default_repl() -> Repl<QuickJsEngine> {
    Repl::new(Project::default(), None).expect("A repl")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn repl_new(
    code_ptr: *mut u8,
    code_len: usize,
    config_ptr: *mut u8,
    config_len: usize,
) -> *mut Repl<QuickJsEngine> {
    init();

    let source = new_string(code_ptr, code_len);
    let config = new_string(config_ptr, config_len);

    if parse_config_bigint(&config) {
        gleam_core::javascript::set_bigint_enabled(true);
    }

    if source.trim().is_empty() {
        return Box::leak(Box::new(default_repl()));
    }

    let mut project = Project::default();
    project.write_source("user.gleam", &source);
    let modules = match project.compile(true) {
        Err(err) => {
            show_error(&error::SgleamError::Gleam(err));
            return std::ptr::null_mut();
        }
        Ok(modules) => modules,
    };
    let module = get_module(&modules, "user");
    if module.map(has_examples).unwrap_or(false) {
        let _ = QuickJsEngine::new(project.fs.clone()).run_tests(&["user"]);
    }
    let substitution_module = {
        let mut result = SubstitutionModule::default();
        for module in modules
            .iter()
            .filter(|m| !m.name.starts_with("gleam/") && !m.name.starts_with("sgleam/"))
        {
            result.merge(SubstitutionModule::from_module(module));
        }
        Some(result)
    };
    let mut repl = Repl::new(project, module).expect("A repl");
    repl.set_substitution_module(substitution_module);
    Box::leak(Box::new(repl))
}

fn has_examples(module: &Module) -> bool {
    module.ast.definitions.functions.iter().any(|f| {
        f.publicity.is_public()
            && f.name
                .as_ref()
                .map(|(_, name)| name.ends_with("_examples"))
                .unwrap_or(false)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn repl_run(repl: *mut Repl<QuickJsEngine>, ptr: *mut u8, len: usize) -> u32 {
    assert!(!repl.is_null());

    let mut repl = unsafe { Box::from_raw(repl) };
    let ret = match repl.run(&new_string(ptr, len)) {
        Ok(ReplOutput::StdOut) => REPL_OK,
        Ok(ReplOutput::Error) => REPL_ERROR,
        Ok(ReplOutput::Quit) => REPL_QUIT,
        Err(err) => {
            show_error(&err);
            REPL_ERROR
        }
    };
    if ret == REPL_OK
        && let Some(steps) = repl.take_stepper_steps()
    {
        use std::io::IsTerminal;
        if std::io::stdout().is_terminal() {
            stepper_ui::display_stepper(&steps);
        } else {
            for (index, step) in steps.iter().enumerate() {
                println!("{}", step.formatted);
                if index + 1 < steps.len() {
                    println!();
                }
            }
        }
    }

    Box::leak(repl);

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn repl_destroy(repl: *mut Repl<QuickJsEngine>) {
    unsafe {
        let _ = Box::from_raw(repl);
    };
}

// --- Completion ---

fn is_break_char(c: char) -> bool {
    !c.is_alphanumeric() && c != '_' && c != ':' && c != '.'
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn repl_complete(
    repl: *mut Repl<QuickJsEngine>,
    text_ptr: *mut u8,
    text_len: usize,
    cursor_pos: usize,
) -> *mut std::ffi::c_char {
    assert!(!repl.is_null());
    let state = unsafe { &*repl };
    let text = new_string(text_ptr, text_len);

    // Extract the word being completed at cursor_pos
    let before = &text[..cursor_pos.min(text.len())];
    let start = before
        .rfind(|c: char| is_break_char(c))
        .map(|i| i + 1)
        .unwrap_or(0);
    let prefix = &before[start..];

    if prefix.is_empty() {
        return std::ptr::null_mut();
    }

    let all = state.completions();
    let candidates: Vec<&str> = all
        .iter()
        .filter(|name| name.starts_with(prefix))
        .map(|s| s.as_str())
        .collect();

    if candidates.is_empty() {
        return std::ptr::null_mut();
    }

    let mut result = format!("c {start}");
    for c in &candidates {
        result.push(' ');
        result.push_str(c);
    }

    to_cstr(result)
}

// --- Format ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format(ptr: *mut u8, len: usize) -> *mut std::ffi::c_char {
    init();

    match engine::format::format_source(&new_string(ptr, len)) {
        Ok(out) => to_cstr(out),
        Err(err) => {
            show_error(&error::SgleamError::Gleam(err));
            std::ptr::null_mut()
        }
    }
}

// --- Version ---

#[unsafe(no_mangle)]
pub extern "C" fn version() -> *mut std::ffi::c_char {
    to_cstr(engine::version())
}
