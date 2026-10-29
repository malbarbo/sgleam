//! The operating system answers. Ctrl-C stops a program, and a window or the
//! terminal shows a world.

use std::num::NonZeroU32;
use std::sync::{
    OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use sinteract::display::{Printer, StageMode, TerminalGraphics, TerminalOptions};

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
    /// A session with the server that ran this process, on fd 3 and fd 4.
    /// The server keeps stdout as a log, so each image prints as the word
    /// `Image`.
    Session,
}

/// What the command line chooses for the displays of an engine.
#[derive(Clone, Copy, Debug, Default)]
pub struct HostOptions {
    pub display: DisplayMode,
    /// How many players a multiplayer world has, each in a window, or the
    /// fewest it takes when `None`.
    pub players: Option<NonZeroU32>,
}

pub(super) fn players(options: HostOptions) -> Option<NonZeroU32> {
    options.players
}

/// The graphics of the terminal that `mode` asks for, or `None` to ask the
/// terminal.
fn graphics(mode: DisplayMode) -> Option<TerminalGraphics> {
    match mode {
        DisplayMode::Kitty => Some(TerminalGraphics::Kitty),
        DisplayMode::Sixel => Some(TerminalGraphics::Sixel),
        DisplayMode::Blocks => Some(TerminalGraphics::HalfBlocks),
        DisplayMode::Auto | DisplayMode::Window | DisplayMode::None | DisplayMode::Session => None,
    }
}

/// Fails when `mode` asks for the graphics of a terminal that does not show
/// them. A mode of blocks goes on, because a terminal with true color often
/// does not say so.
pub fn check_display(mode: DisplayMode) -> Result<(), SgleamError> {
    match mode {
        DisplayMode::Kitty if !TerminalGraphics::Kitty.is_supported() => {
            Err(SgleamError::DisplayNotSupported { mode: "kitty" })
        }
        DisplayMode::Sixel if !TerminalGraphics::Sixel.is_supported() => {
            Err(SgleamError::DisplayNotSupported { mode: "sixel" })
        }
        _ => Ok(()),
    }
}

pub(super) fn new_printer(options: HostOptions) -> Option<Printer> {
    match options.display {
        DisplayMode::None | DisplayMode::Session => None,
        mode => graphics(mode)
            .map(Printer::with_graphics)
            .or_else(|| Printer::new().ok()),
    }
}

/// The rows under a frame in the terminal, for the last lines the program
/// printed. An `echo` takes two of them.
const FOOTER_ROWS: u16 = 5;

pub(super) fn stage_mode(options: HostOptions) -> StageMode {
    let terminal = TerminalOptions {
        // Raw mode turns Ctrl-C into a key, so the reader of the terminal
        // stops the program in place of the signal.
        on_interrupt: Some(interrupt),
        keep_last_frame: true,
        footer_rows: FOOTER_ROWS,
    };
    let players = options.players;
    match options.display {
        DisplayMode::Auto => StageMode::auto(players, terminal),
        DisplayMode::Window => StageMode::Window { players },
        mode @ (DisplayMode::Kitty | DisplayMode::Sixel | DisplayMode::Blocks) => {
            StageMode::Terminal {
                options: terminal,
                graphics: graphics(mode),
            }
        }
        DisplayMode::None => StageMode::Headless { players },
        DisplayMode::Session => StageMode::Session,
    }
}
