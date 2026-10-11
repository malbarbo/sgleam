//! The operating system answers. Ctrl-C stops a program, the file system holds
//! an image.

use std::cell::RefCell;
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use sinteract::display::{
    Backend, FrameEvent, Frames, Native, Printer, StageOptions, TerminalOptions,
};
use sinteract::event::{KeyEvent, KeyKind};
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

/// Where the frames of a world and the images of the repl go.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisplayMode {
    /// A window when one opens, else the terminal with the backend it shows.
    #[default]
    Auto,
    /// Only a window. The images of the repl go as in `Auto`.
    Window,
    // Only the terminal, through the backend of the same name. The images of
    // the repl go through it as well.
    Kitty,
    Sixel,
    Blocks,
    /// No display at all, so each image prints as the word `Image`.
    None,
}

static DISPLAY_MODE: Mutex<DisplayMode> = Mutex::new(DisplayMode::Auto);

pub fn set_display(mode: DisplayMode) {
    *DISPLAY_MODE.lock().unwrap_or_else(|e| e.into_inner()) = mode;
}

fn display_mode() -> DisplayMode {
    *DISPLAY_MODE.lock().unwrap_or_else(|e| e.into_inner())
}

/// The backend of the terminal that `mode` asks for, or `None` to ask the
/// terminal.
fn backend(mode: DisplayMode) -> Option<Backend> {
    match mode {
        DisplayMode::Kitty => Some(Backend::Kitty),
        DisplayMode::Sixel => Some(Backend::Sixel),
        DisplayMode::Blocks => Some(Backend::HalfBlocks),
        DisplayMode::Auto | DisplayMode::Window | DisplayMode::None => None,
    }
}

fn new_printer() -> Option<Printer> {
    match display_mode() {
        DisplayMode::None => None,
        mode => match backend(mode) {
            Some(backend) => Some(Printer::with_backend(backend)),
            None => Printer::new().ok(),
        },
    }
}

pub fn print_scene(scene: &Scene) -> bool {
    thread_local! {
        // The first print probes the terminal, so a run that prints no image
        // never asks it anything.
        static PRINTER: RefCell<Option<Printer>> = RefCell::new(new_printer());
    }
    PRINTER.with_borrow_mut(|printer| printer.as_mut().is_some_and(|p| p.print(scene).is_ok()))
}

/// The rows under a frame in the terminal, for the last lines the program
/// printed. An `echo` takes two of them.
const FOOTER_ROWS: u16 = 5;

fn stage_options() -> StageOptions {
    let mode = display_mode();
    StageOptions {
        display: match mode {
            DisplayMode::Auto | DisplayMode::None => Native::Auto,
            DisplayMode::Window => Native::Window,
            DisplayMode::Kitty | DisplayMode::Sixel | DisplayMode::Blocks => Native::Terminal,
        },
        terminal: TerminalOptions {
            // Raw mode turns Ctrl-C into a key, so the reader of the terminal
            // stops the program in place of the signal.
            on_interrupt: Some(Box::new(interrupt)),
            keep_last_frame: true,
            footer_rows: FOOTER_ROWS,
            backend: backend(mode),
        },
    }
}

thread_local! {
    static FRAMES: RefCell<Frames> = RefCell::new(Frames::new("sgleam", stage_options));
}

pub fn show_scene(scene: Scene) -> bool {
    if display_mode() == DisplayMode::None {
        return false;
    }
    FRAMES.with_borrow_mut(|frames| frames.show(scene).is_ok())
}

pub fn close_frames() {
    // The frame stays on screen, and the lines of the footer go under it.
    for line in FRAMES.with_borrow_mut(Frames::close) {
        println!("{line}");
    }
}

pub fn print_line(line: &str) {
    if !FRAMES.with_borrow_mut(|frames| frames.print_line(line)) {
        println!("{line}");
    }
}

pub fn wait_event(ms: u64) -> Vec<String> {
    let end = Instant::now() + Duration::from_millis(ms);
    FRAMES.with_borrow_mut(|frames| {
        if !frames.is_open() {
            sleep(ms);
            return vec![];
        }
        // Short slices, so an interruption ends a long wait at once.
        loop {
            let deadline = (Instant::now() + Duration::from_millis(10)).min(end);
            match frames.wait_event(deadline, |err| tracing::warn!("{err}")) {
                FrameEvent::Key(key) => return key_event(key),
                FrameEvent::Close => return vec!["close".into()],
                FrameEvent::Timeout => {}
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
