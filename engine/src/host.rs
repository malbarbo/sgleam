//! What a program asks of the world around it. The page answers in `wasm` and
//! the operating system in `native`.
//!
//! Each target provides every function re-exported below, under the name and
//! with the meaning written here. A `cfg` on a re-export says that only one
//! target has the function, and whatever asks for it says the same `cfg`.
//! The frames of a world are the same on both, over `Frames` of sinteract.

#[cfg(not(target_arch = "wasm32"))]
use std::cell::OnceCell;
use std::cell::RefCell;
use std::num::NonZeroU32;
use std::time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
use sinteract::display::Printer;
use sinteract::display::{FrameEvent, Frames, OpenError, ShowError, StageError};
use sinteract::event::{InputEvent, KeyEvent, KeyKind, PadButton, PadEvent};
use sinteract::scene::Scene;
use sinteract::session::PlayerRange;

/// Makes the world ready to answer. An engine calls it as it is built. The
/// first call does the work and a later one gives back its result.
pub use target::init;

/// Returns `true` if an interruption is waiting, `false` otherwise. The
/// request stays until the next input starts, so every check of the run sees
/// it.
pub use target::check_interrupt;

/// Drops a waiting interruption, so the input about to run does not stop at
/// its first check.
pub use target::clear_interrupt;

/// `system.sleep` holds the program for `ms` milliseconds. An interruption may
/// end it early.
pub use target::sleep;

/// `system.draw_svg` puts a drawing on the page. Only the browser draws, so a
/// native run has no such function.
#[cfg(target_arch = "wasm32")]
pub use target::draw_svg;

/// The choice of the display for the frames of a world and the images of the
/// repl. The browser always draws on the page, so it has no such choice.
#[cfg(not(target_arch = "wasm32"))]
pub use target::DisplayMode;

/// Fails when the terminal does not show the display that the command line
/// names.
#[cfg(not(target_arch = "wasm32"))]
pub use target::check_display;

/// What the command line chooses for the displays of an engine.
pub use target::HostOptions;

#[cfg(target_arch = "wasm32")]
#[path = "host/wasm.rs"]
mod target;
#[cfg(not(target_arch = "wasm32"))]
#[path = "host/native.rs"]
mod target;

/// The displays of one engine. The frames show the worlds, and on a native
/// target the printer shows the images of the repl in the terminal.
pub(crate) struct Host {
    #[cfg(not(target_arch = "wasm32"))]
    options: HostOptions,
    frames: RefCell<Frames>,
    players: Option<NonZeroU32>,
    // Empty until the first print, so a run that prints no image never asks
    // the terminal anything. `None` inside means there is no terminal to
    // print to.
    #[cfg(not(target_arch = "wasm32"))]
    printer: OnceCell<Option<RefCell<Printer>>>,
}

impl Host {
    pub(crate) fn new(options: HostOptions) -> Host {
        Host {
            #[cfg(not(target_arch = "wasm32"))]
            options,
            frames: RefCell::new(Frames::new("sgleam", move || {
                target::stage_options(options)
            })),
            players: target::players(options),
            #[cfg(not(target_arch = "wasm32"))]
            printer: OnceCell::new(),
        }
    }

    /// Shows a frame of a world, in a window or the terminal, or in a session
    /// of sinteract on the page. The first frame of a world opens the display
    /// with its own size, and the frame goes out at the next tick of the
    /// display. Returns `true` if the display is open, `false` if `-d` asked
    /// for none, and the error of the display if it failed to open. An
    /// animation has no players, so `--players` is an error.
    pub fn show_scene(&self, scene: Scene) -> Result<bool, String> {
        if let Some(players) = self.players {
            return Err(format!(
                "an animation takes no players, and --players asks for {players}"
            ));
        }
        match self.frames.borrow_mut().show(scene) {
            Ok(()) => Ok(true),
            Err(
                ShowError::Open(StageError::Open(OpenError::Disabled)) | ShowError::Unavailable,
            ) => Ok(false),
            Err(ShowError::Open(error)) => Err(error.to_string()),
        }
    }

    /// Chooses the players of a multiplayer world and returns alternating
    /// player numbers and nicknames from its fixed start. The first frame
    /// of the world opens its display, at the size of the frame.
    pub fn open_mworld(&self, min_players: u32, max_players: u32) -> Result<Vec<String>, String> {
        let mut frames = self.frames.borrow_mut();
        if frames.is_open() {
            return Err("a world is already running".into());
        }
        let range = PlayerRange::new(min_players, max_players)
            .ok_or_else(|| "the number of players must be between 1 and 1024".to_string())?;
        match frames.open(range, self.players) {
            Ok(players) => Ok(players
                .into_iter()
                .flat_map(|(player, nickname)| [player.number().get().to_string(), nickname])
                .collect()),
            // `-d none` asks for no display, so the world runs without one,
            // with the players of `--players`, or the fewest it takes.
            Err(StageError::Open(OpenError::Disabled)) => {
                Ok((1..=self.players.unwrap_or(range.min()).get())
                    .flat_map(|player| [player.to_string(), String::new()])
                    .collect())
            }
            Err(error) => Err(error.to_string()),
        }
    }

    /// Keeps the latest scene for one player until the next stage tick. The
    /// first scene of a world opens its display. Returns `true` if the
    /// display is open, `false` if `-d` asked for none, and the error of the
    /// display if it failed to open.
    pub fn show_player_scene(&self, player: u32, scene: Scene) -> Result<bool, String> {
        let mut frames = self.frames.borrow_mut();
        let Some(&player) = frames.players().iter().find(|p| p.number().get() == player) else {
            return Ok(false);
        };
        match frames.show_to(player, scene) {
            Ok(()) => Ok(true),
            Err(
                ShowError::Open(StageError::Open(OpenError::Disabled)) | ShowError::Unavailable,
            ) => Ok(false),
            Err(ShowError::Open(error)) => Err(error.to_string()),
        }
    }

    /// Closes the display of the frames, so the world that comes next opens
    /// its own. A window goes away, and the terminal keeps the last frame on
    /// screen.
    pub fn close_frames(&self) {
        // The terminal shows the lines of `print_line` under the frame, and
        // they print under it after the close.
        let lines = self.frames.borrow_mut().close();
        for line in lines {
            println!("{line}");
        }
    }

    /// `console.log` prints a line, and so does `echo`. While the terminal
    /// shows the frames of a world, the line shows under the frame and prints
    /// when the frames close.
    pub fn print_line(&self, line: &str) {
        if !self.frames.borrow_mut().print_line(line) {
            println!("{line}");
        }
    }

    /// `system.wait_event` waits at most `ms` milliseconds for an input of a
    /// player, a tick of the display, or the close of the display. For an
    /// input it gives the kind of the event, the number of the player, and
    /// the name of the key or of the pad input, then for a key the modifiers
    /// that are on. A scene shown after a tick goes out at the next one. It
    /// gives nothing when the time runs out.
    pub fn wait_event(&self, ms: u64) -> Vec<String> {
        let end = Instant::now() + Duration::from_millis(ms);
        loop {
            // Short slices, so an interruption ends a long wait at once.
            let deadline = (Instant::now() + Duration::from_millis(10)).min(end);
            let event = self
                .frames
                .borrow_mut()
                .wait_event(deadline, |error| tracing::warn!("{error}"));
            let event = match event {
                FrameEvent::Key(player, key) => {
                    player_event(player.number().get(), InputEvent::Key(key))
                }
                FrameEvent::Pad(player, pad) => {
                    player_event(player.number().get(), InputEvent::Pad(pad))
                }
                FrameEvent::Tick => return vec!["tick".into()],
                FrameEvent::Close => return vec!["close".into()],
                FrameEvent::Timeout => None,
            };
            if let Some(event) = event {
                return event;
            }
            if Instant::now() >= end || target::check_interrupt() {
                return vec![];
            }
        }
    }

    /// Prints a scene in the terminal at the cursor. Returns `true` if the
    /// scene went out, `false` otherwise. With `-d none` nothing goes out. A
    /// backend that `-d` names prints to stdout whatever it is, and otherwise
    /// the scene goes out only if stdout is a terminal that shows images. The
    /// browser draws an svg instead, so it has no such function.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn print_scene(&self, scene: &Scene) -> bool {
        self.printer()
            .is_some_and(|printer| printer.borrow_mut().print(scene).is_ok())
    }

    /// Returns `true` if [`Host::print_scene`] has a terminal to print to,
    /// `false` otherwise. A failed check asks it before it prints the label
    /// of an image.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn prints_images(&self) -> bool {
        self.printer().is_some()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn printer(&self) -> Option<&RefCell<Printer>> {
        self.printer
            .get_or_init(|| target::new_printer(self.options).map(RefCell::new))
            .as_ref()
    }
}

fn player_event(player: u32, event: InputEvent) -> Option<Vec<String>> {
    match event {
        InputEvent::Key(key) => {
            let mut event = key_event(key);
            event.insert(1, player.to_string());
            Some(event)
        }
        InputEvent::Pad(PadEvent::Down(button)) => pad_event("paddown", player, button),
        InputEvent::Pad(PadEvent::Up(button)) => pad_event("padup", player, button),
        InputEvent::Pad(PadEvent::Connected | PadEvent::Disconnected)
        | InputEvent::Mouse(_)
        | InputEvent::Resize { .. } => None,
    }
}

fn pad_event(kind: &str, player: u32, button: PadButton) -> Option<Vec<String>> {
    let input = match button {
        PadButton::DpadUp => "Up",
        PadButton::DpadDown => "Down",
        PadButton::DpadLeft => "Left",
        PadButton::DpadRight => "Right",
        PadButton::A => "ActionA",
        PadButton::B => "ActionB",
        PadButton::X
        | PadButton::Y
        | PadButton::LeftShoulder
        | PadButton::RightShoulder
        | PadButton::Select
        | PadButton::Start => return None,
    };
    Some(vec![kind.into(), player.to_string(), input.into()])
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

/// `system.now_ms` gives milliseconds since the epoch, and `world` times its
/// ticks by it.
pub fn now_ms() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use sinteract::event::{InputEvent, KeyEvent, KeyKind, Modifiers, PadButton, PadEvent};

    use super::player_event;

    #[test]
    fn a_key_event_carries_its_player_and_modifiers() {
        let event = player_event(
            3,
            InputEvent::Key(KeyEvent {
                kind: KeyKind::Down,
                key: "a".into(),
                modifiers: Modifiers {
                    ctrl: true,
                    shift: true,
                    ..Modifiers::default()
                },
            }),
        );
        assert_eq!(
            event,
            Some(vec![
                "keydown".into(),
                "3".into(),
                "a".into(),
                "ctrl".into(),
                "shift".into(),
            ])
        );
    }

    #[test]
    fn the_teaching_pad_keeps_only_its_six_inputs() {
        assert_eq!(
            player_event(2, InputEvent::Pad(PadEvent::Down(PadButton::DpadUp))),
            Some(vec!["paddown".into(), "2".into(), "Up".into()])
        );
        assert_eq!(
            player_event(2, InputEvent::Pad(PadEvent::Up(PadButton::B))),
            Some(vec!["padup".into(), "2".into(), "ActionB".into()])
        );
        assert_eq!(
            player_event(2, InputEvent::Pad(PadEvent::Down(PadButton::Start))),
            None
        );
    }
}
