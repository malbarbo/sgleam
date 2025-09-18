use crate::{
    engine::Engine as _,
    error::{self, show_error},
    gleam::{compile, get_module, Project},
    quickjs::QuickJsEngine,
    repl::{Repl, ReplOutput},
};
use camino::Utf8Path;
use gleam_core::{build::Module, javascript::set_bigint_enabled};

#[no_mangle]
pub extern "C" fn string_allocate(size: usize) -> *mut u8 {
    let mut buffer = Vec::with_capacity(size);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn string_deallocate(ptr: *mut u8, size: usize) {
    assert!(!ptr.is_null());
    unsafe {
        let _ = Vec::from_raw_parts(ptr, 0, size);
    }
}

fn new_string(ptr: *mut u8, len: usize) -> String {
    assert!(!ptr.is_null());
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).into()
}

#[no_mangle]
pub unsafe extern "C" fn repl_new(str: *mut u8, len: usize) -> *mut Repl<QuickJsEngine> {
    let source = new_string(str, len);
    let mut project = Project::default();
    project.write_source("user.gleam", &source);
    let modules = match compile(&mut project, false) {
        Err(err) => {
            show_error(&error::SgleamError::Gleam(err));
            return Box::leak(Box::new(
                Repl::new(Project::default(), None).expect("An repl"),
            ));
        }
        Ok(modules) => modules,
    };
    let module = get_module(&modules, "user");
    if !source.is_empty() && module.map(has_examples).unwrap_or(false) {
        QuickJsEngine::new(project.fs.clone()).run_tests(&["user"]);
    }
    Box::leak(Box::new(Repl::new(project, module).expect("An repl")))
}

fn has_examples(module: &Module) -> bool {
    module.ast.definitions.iter().any(|d| match d {
        gleam_core::ast::Definition::Function(f) => {
            f.publicity.is_public()
                && f.name
                    .as_ref()
                    .map(|(_, name)| name.ends_with("_examples"))
                    .unwrap_or(false)
        }
        _ => false,
    })
}

#[no_mangle]
pub unsafe extern "C" fn repl_destroy(repl: *mut Repl<QuickJsEngine>) {
    unsafe {
        let _ = Box::from_raw(repl);
    };
}

#[no_mangle]
pub unsafe extern "C" fn repl_run(
    repl: *mut Repl<QuickJsEngine>,
    str: *mut u8,
    len: usize,
) -> bool {
    assert!(!repl.is_null());

    let mut repl = unsafe { Box::from_raw(repl) };
    let ret = match repl.run(&new_string(str, len)) {
        Ok(ReplOutput::Quit) => true,
        Err(err) => {
            show_error(&err);
            false
        }
        _ => false,
    };

    Box::leak(repl);

    ret
}

#[no_mangle]
pub unsafe extern "C" fn format(str: *mut u8, len: usize) -> *mut i8 {
    let mut out = String::new();
    if let Err(err) = gleam_core::format::pretty(
        &mut out,
        &new_string(str, len).into(),
        Utf8Path::new("user.gleam"),
    ) {
        show_error(&error::SgleamError::Gleam(err));
        return std::ptr::null_mut();
    }
    match std::ffi::CString::new(out) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cstr_deallocate(ptr: *mut i8) {
    assert!(!ptr.is_null());
    unsafe {
        let _ = std::ffi::CString::from_raw(ptr);
    }
}

#[no_mangle]
pub extern "C" fn use_bigint(flag: bool) {
    set_bigint_enabled(flag);
}
