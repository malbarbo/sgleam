//! The operating system answers. Ctrl-C stops a program, and a window or the
//! terminal shows a world.

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::sync::{
    OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use sinteract::display::{Backend, Native, Printer, StageOptions, TerminalOptions};
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
    STOP.load(Ordering::Relaxed)
}

pub fn clear_interrupt() {
    STOP.store(false, Ordering::Relaxed);
}

pub fn sleep(ms: u64) {
    let end = Instant::now() + Duration::from_millis(ms);
    // Short slices, so an interruption ends a long sleep at once.
    loop {
        let now = Instant::now();
        if now >= end || check_interrupt() {
            return;
        }
        std::thread::sleep((end - now).min(Duration::from_millis(10)));
    }
}

/// Where the frames of a world and the images of the repl go.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisplayMode {
    /// A window when one opens, else the terminal with the backend it shows.
    #[default]
    Auto,
    /// Only a window. The images of the repl go as in `Auto`.
    Window,
    /// Only the terminal, through the graphics of Kitty, for the frames and
    /// the images of the repl alike.
    Kitty,
    /// Only the terminal, through Sixel, for the frames and the images of the
    /// repl alike.
    Sixel,
    /// Only the terminal, through blocks in true color, for the frames and the
    /// images of the repl alike.
    Blocks,
    /// No display at all, so each image prints as the word `Image`.
    None,
}

static DISPLAY_MODE: OnceLock<DisplayMode> = OnceLock::new();

pub fn set_display(mode: DisplayMode) {
    let first = DISPLAY_MODE.set(mode);
    debug_assert!(first.is_ok(), "the display is chosen once");
}

static PLAYERS: OnceLock<NonZeroU32> = OnceLock::new();

/// Makes a multiplayer world on a display of this process play with
/// `players`, each in a window, and not with the fewest it takes.
pub fn set_players(players: NonZeroU32) {
    let first = PLAYERS.set(players);
    debug_assert!(first.is_ok(), "the players are chosen once");
}

fn display_mode() -> DisplayMode {
    DISPLAY_MODE.get().copied().unwrap_or_default()
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
        mode => backend(mode)
            .map(Printer::with_backend)
            .or_else(|| Printer::new().ok()),
    }
}

thread_local! {
    // The first use makes the printer, so a run that prints no image never
    // asks the terminal anything.
    static PRINTER: RefCell<Option<Printer>> = RefCell::new(new_printer());
}

pub fn prints_images() -> bool {
    PRINTER.with_borrow(Option::is_some)
}

pub fn print_scene(scene: &Scene) -> bool {
    PRINTER.with_borrow_mut(|printer| printer.as_mut().is_some_and(|p| p.print(scene).is_ok()))
}

/// The rows under a frame in the terminal, for the last lines the program
/// printed. An `echo` takes two of them.
const FOOTER_ROWS: u16 = 5;

pub(super) fn stage_options() -> StageOptions {
    let mode = display_mode();
    StageOptions {
        display: match mode {
            DisplayMode::Auto => Native::Auto,
            DisplayMode::Window => Native::Window,
            DisplayMode::Kitty | DisplayMode::Sixel | DisplayMode::Blocks => Native::Terminal,
            DisplayMode::None => Native::None,
        },
        players: PLAYERS.get().copied(),
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
