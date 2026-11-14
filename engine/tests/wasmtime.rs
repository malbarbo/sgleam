//! The interrupt is one flag for the whole process, so this test has a
//! process of its own.

#![cfg(unix)]

use std::time::Duration;

use engine::error::SgleamError;

#[test]
fn an_interrupt_stops_a_program_that_loops() {
    let program =
        wat::parse_str(r#"(module (func (export "_start") (loop $forever (br $forever))))"#)
            .expect("a valid module");
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(500));
        // SAFETY: `kill` only sends a signal, and the engine handles SIGINT.
        unsafe { libc::kill(libc::getpid(), libc::SIGINT) };
    });
    let result = engine::wasmtime::run(&program, &[]);
    assert!(
        matches!(result, Err(SgleamError::Interrupted)),
        "{result:?}"
    );
}
