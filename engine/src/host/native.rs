//! The operating system answers. Ctrl-C stops a program, the file system holds
//! an image.

use std::cell::RefCell;
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use sinteract::display::{Backend, Display, Printer, Terminal, TerminalOptions, Window};
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

/// The display of the frames of a run.
enum Frames {
    /// No run showed a frame yet, or the last run ended.
    Closed,
    Window(Window),
    /// The terminal hides what the program prints, so the last lines show
    /// under the frame, and stay on screen after it closes.
    Terminal(Terminal, Vec<String>),
    /// Neither a window nor the terminal opened, so the run does not try again
    /// at each frame.
    Unavailable,
}

impl Frames {
    fn display(&mut self) -> Option<&mut dyn Display> {
        match self {
            Frames::Window(window) => Some(window),
            Frames::Terminal(terminal, _) => Some(terminal),
            Frames::Closed | Frames::Unavailable => None,
        }
    }
}

/// The rows under a frame in the terminal, for the last lines the program
/// printed. An `echo` takes two of them.
const FOOTER_ROWS: u16 = 5;

thread_local! {
    static FRAMES: RefCell<Frames> = const { RefCell::new(Frames::Closed) };
}

pub fn show_scene(scene: Scene) -> bool {
    FRAMES.with_borrow_mut(|frames| {
        if let Frames::Closed = frames {
            *frames = open_frames(scene.width(), scene.height());
        }
        frames
            .display()
            .is_some_and(|display| display.present(scene).is_ok())
    })
}

fn open_frames(width: f32, height: f32) -> Frames {
    let mode = display_mode();
    let options = TerminalOptions {
        // Raw mode turns Ctrl-C into a key, so the reader of the terminal
        // stops the program in place of the signal.
        on_interrupt: Some(Box::new(interrupt)),
        keep_last_frame: true,
        footer_rows: FOOTER_ROWS,
        backend: backend(mode),
    };
    match mode {
        DisplayMode::None => return Frames::Unavailable,
        DisplayMode::Auto | DisplayMode::Window => {
            if let Ok(window) = Window::open("sgleam", width, height) {
                return Frames::Window(window);
            }
            if mode == DisplayMode::Window {
                return Frames::Unavailable;
            }
        }
        DisplayMode::Kitty | DisplayMode::Sixel | DisplayMode::Blocks => {}
    }
    match Terminal::open_with(options) {
        Ok(terminal) => Frames::Terminal(terminal, vec![]),
        Err(_) => Frames::Unavailable,
    }
}

pub fn close_frames() {
    let frames = FRAMES.replace(Frames::Closed);
    if let Frames::Terminal(terminal, lines) = frames {
        // Dropping the terminal prints the last frame, and the lines of the
        // footer go under it.
        drop(terminal);
        for line in lines {
            println!("{line}");
        }
    }
}

pub fn print_line(line: &str) {
    FRAMES.with_borrow_mut(|frames| match frames {
        Frames::Terminal(terminal, lines) => {
            lines.extend(line.split('\n').map(String::from));
            let excess = lines.len().saturating_sub(FOOTER_ROWS.into());
            lines.drain(..excess);
            let _ = terminal.set_footer(lines);
        }
        Frames::Closed | Frames::Window(_) | Frames::Unavailable => println!("{line}"),
    })
}

pub fn wait_event(ms: u64) -> Vec<String> {
    let end = Instant::now() + Duration::from_millis(ms);
    FRAMES.with_borrow_mut(|frames| {
        let Some(display) = frames.display() else {
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
