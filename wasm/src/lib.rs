#![allow(clippy::missing_safety_doc)]

use engine::{
    engine::Engine as _,
    error::{self, show_error},
    gleam::{Project, get_module},
    quickjs::QuickJsEngine,
    repl::{Repl, ReplOutput},
};
use gleam_core::build::Module;

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
    Box::leak(Box::new(Repl::new(project, module).expect("A repl")))
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
        Ok(output) => output as u32,
        Err(err) => {
            show_error(&err);
            ReplOutput::Error as u32
        }
    };

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
