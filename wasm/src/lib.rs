#![allow(clippy::missing_safety_doc)]

use engine::{
    engine::Engine as _,
    error::{self, show_error},
    gleam::{Project, get_module},
    quickjs::QuickJsEngine,
    repl::Repl,
    shell::Shell,
};
use gleam_core::build::Module;
use std::sync::atomic::{AtomicBool, Ordering};

static INIT: AtomicBool = AtomicBool::new(false);

fn init() {
    if !INIT.swap(true, Ordering::Relaxed) {
        engine::panic::add_handler();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn string_allocate(size: usize) -> *mut u8 {
    init();
    if size == 0 {
        return std::ptr::NonNull::dangling().as_ptr();
    }
    let layout = string_layout(size);
    let ptr = unsafe { std::alloc::alloc(layout) };
    if ptr.is_null() {
        std::alloc::handle_alloc_error(layout);
    }
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn string_deallocate(ptr: *mut u8, size: usize) {
    init();
    assert!(!ptr.is_null());
    if size == 0 {
        return;
    }
    assert!(ptr != std::ptr::NonNull::dangling().as_ptr());
    unsafe { std::alloc::dealloc(ptr, string_layout(size)) };
}

fn string_layout(size: usize) -> std::alloc::Layout {
    std::alloc::Layout::from_size_align(size, 1).expect("more bytes than a layout holds")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cstr_deallocate(ptr: *mut std::ffi::c_char) {
    init();
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

fn parse_config_bigint(config: &str) -> bool {
    config.split_whitespace().any(|entry| {
        entry
            .split_once('=')
            .is_some_and(|(k, v)| k == "bigint" && v == "true")
    })
}

fn default_repl() -> Result<Repl<QuickJsEngine>, error::SgleamError> {
    Repl::new(Project::default(), None)
}

fn leak_shell(repl: Result<Repl<QuickJsEngine>, error::SgleamError>) -> *mut Shell<QuickJsEngine> {
    match repl {
        Ok(repl) => Box::leak(Box::new(Shell::new(repl))),
        Err(err) => {
            show_error(&err);
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn repl_new(
    code_ptr: *mut u8,
    code_len: usize,
    config_ptr: *mut u8,
    config_len: usize,
) -> *mut Shell<QuickJsEngine> {
    init();

    let source = new_string(code_ptr, code_len);
    let config = new_string(config_ptr, config_len);

    gleam_core::javascript::set_bigint_enabled(parse_config_bigint(&config));

    if source.trim().is_empty() {
        return leak_shell(default_repl());
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
    if module.map(has_examples).unwrap_or(false)
        && let Ok(engine) = QuickJsEngine::new(project.fs.clone())
    {
        let _ = engine.run_tests(&["user"]);
    }
    leak_shell(Repl::new(project, module))
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
pub unsafe extern "C" fn repl_run(
    repl: *mut Shell<QuickJsEngine>,
    ptr: *mut u8,
    len: usize,
) -> u32 {
    init();
    assert!(!repl.is_null());
    let repl = unsafe { &mut *repl };
    repl.run(&new_string(ptr, len)) as u32
}

/// Whether the line at the prompt is finished: -1 to run it, otherwise the
/// indentation the next line starts with. See SimpleCode's ENGINE.md.
///
/// The repl is not asked: what a Gleam input is waiting on is in its text, and
/// the parser reads it without a session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn repl_ready(
    _repl: *mut Shell<QuickJsEngine>,
    ptr: *mut u8,
    len: usize,
) -> i32 {
    init();
    engine::shell::ready_state(&new_string(ptr, len))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn repl_destroy(repl: *mut Shell<QuickJsEngine>) {
    init();
    // What `repl_new` answers with when it has no shell to give is the null
    // the host hands back here, having taken it for one.
    if repl.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(repl);
    };
}

// --- Completion ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn repl_complete(
    repl: *mut Shell<QuickJsEngine>,
    text_ptr: *mut u8,
    text_len: usize,
    cursor_pos: usize,
) -> *mut std::ffi::c_char {
    init();
    assert!(!repl.is_null());
    let state = unsafe { &*repl };
    let text = new_string(text_ptr, text_len);

    let (start, prefix) = engine::shell::word_at(&text, cursor_pos);

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

    // A space apart, and so without the one a candidate that opens something
    // carries at its end: two spaces in a row would read as an empty candidate.
    let mut result = format!("c {start}");
    for c in &candidates {
        result.push(' ');
        result.push_str(c.trim_end());
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
    init();
    to_cstr(engine::version())
}
