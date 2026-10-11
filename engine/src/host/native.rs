//! The operating system answers. Ctrl-C stops a program, the file system holds
//! an image.

use std::cell::RefCell;
use std::sync::{
    OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use sinteract::display::{Display, Printer, TerminalOptions, open_native};
use sinteract::event::{Event, InputEvent, Interrupt, KeyEvent, KeyKind};
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
    let end = Instant::now() + Duration::from_millis(ms);
    // Short slices, so an interruption ends a long sleep at once.
    loop {
        let now = Instant::now();
        if now >= end || STOP.load(Ordering::Relaxed) {
            return;
        }
        std::thread::sleep((end - now).min(Duration::from_millis(10)));
    }
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

/// The display of the frames of a run.
enum Frames {
    /// No run showed a frame yet, or the last run ended.
    Closed,
    Open(Box<dyn Display>),
    /// Neither a window nor the terminal opened, so the run does not try again
    /// at each frame.
    Unavailable,
}

thread_local! {
    static FRAMES: RefCell<Frames> = const { RefCell::new(Frames::Closed) };
}

pub fn show_scene(scene: Scene) -> bool {
    FRAMES.with_borrow_mut(|frames| {
        if let Frames::Closed = frames {
            *frames = open_frames(scene.width(), scene.height());
        }
        match frames {
            Frames::Open(display) => display.present(scene).is_ok(),
            Frames::Closed | Frames::Unavailable => false,
        }
    })
}

fn open_frames(width: f32, height: f32) -> Frames {
    let options = TerminalOptions {
        // Raw mode turns Ctrl-C into a key, so the reader of the terminal
        // stops the program in place of the signal.
        on_interrupt: Some(Box::new(interrupt)),
        keep_last_frame: true,
    };
    match open_native("sgleam", width, height, options) {
        Ok(display) => Frames::Open(display),
        Err(_) => Frames::Unavailable,
    }
}

pub fn close_frames() {
    // Dropping the display closes it.
    FRAMES.with_borrow_mut(|frames| *frames = Frames::Closed);
}

pub fn wait_event(ms: u64) -> Vec<String> {
    let end = Instant::now() + Duration::from_millis(ms);
    FRAMES.with_borrow_mut(|frames| {
        let Frames::Open(display) = frames else {
            sleep(ms);
            return vec![];
        };
        // Short slices, so an interruption ends a long wait at once.
        loop {
            let deadline = (Instant::now() + Duration::from_millis(10)).min(end);
            match display.wait_event(Some(deadline)) {
                Ok(Event::Input(InputEvent::Key(key))) => return key_event(key),
                // A world has no use for ticks, the mouse or the size of the
                // display.
                Ok(_) | Err(Interrupt::Timeout | Interrupt::Wake) => {}
                Err(Interrupt::Close | Interrupt::Read(_)) => return vec!["close".into()],
            }
            if Instant::now() >= end || STOP.load(Ordering::Relaxed) {
                return vec![];
            }
        }
    })
}

/// The kind first, then the name of the key, then one name per modifier that
/// is on.
fn key_event(key: KeyEvent) -> Vec<String> {
    let kind = match key.kind {
        KeyKind::Press => "keypress",
        KeyKind::Down => "keydown",
        KeyKind::Up => "keyup",
    };
    let mut event = vec![kind.to_string(), key.key];
    let modifiers = key.modifiers;
    for (on, name) in [
        (modifiers.alt, "alt"),
        (modifiers.ctrl, "ctrl"),
        (modifiers.shift, "shift"),
        (modifiers.meta, "meta"),
    ] {
        if on {
            event.push(name.into());
        }
    }
    event
}
