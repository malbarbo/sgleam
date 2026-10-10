//! The operating system answers. Ctrl-C stops a program, the file system holds
//! an image.

use std::cell::RefCell;
use std::sync::{
    OnceLock,
    atomic::{AtomicBool, Ordering},
};

use sinteract::display::Printer;
use sinteract::scene::Scene;

use crate::error::SgleamError;

/// One flag serves the whole process, so an interruption reaches every engine
/// at once.
static STOP: AtomicBool = AtomicBool::new(false);

fn interrupt() {
    STOP.store(true, Ordering::Relaxed);
}

/// Puts the Ctrl-C handler in place. A failure stays as well, so a second
/// engine does not run quietly with no way to stop it.
pub fn init() -> Result<(), SgleamError> {
    static CTRLC: OnceLock<Result<(), String>> = OnceLock::new();
    // What ctrlc says already names its subject, so the message goes on as it
    // came.
    CTRLC
        .get_or_init(|| ctrlc::set_handler(interrupt).map_err(|err| err.to_string()))
        .clone()
        .map_err(|err| SgleamError::Other(err.into()))
}

pub fn check_interrupt() -> bool {
    STOP.swap(false, Ordering::Relaxed)
}

pub fn sleep(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

pub fn load_bitmap(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|err| err.to_string())
}

pub fn print_scene(scene: &Scene) -> bool {
    thread_local! {
        // The first print probes the terminal, so a run that prints no image
        // never asks it anything.
        static PRINTER: RefCell<Option<Printer>> = RefCell::new(Printer::new().ok());
    }
    PRINTER.with_borrow_mut(|printer| printer.as_mut().is_some_and(|p| p.print(scene).is_ok()))
}
