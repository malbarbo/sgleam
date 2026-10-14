//! What a program asks of the world around it. The page answers in `wasm` and
//! the operating system in `native`.
//!
//! Each target provides every function re-exported below, under the name and
//! with the meaning written here. A `cfg` on a re-export says that only one
//! target has the function, and whatever asks for it says the same `cfg`.
//! The frames of a world are the same on both, over `Frames` of sinteract.

use std::cell::RefCell;
use std::time::{Duration, Instant};

use sinteract::display::{FrameEvent, Frames};
use sinteract::event::{InputEvent, KeyEvent, KeyKind, PadButton, PadEvent};
use sinteract::scene::Scene;

use self::mworld::{MultiFrameEvent, MultiFrames};

mod mworld;

/// Makes the world ready to answer. An engine calls it as it is built. The
/// first call does the work and a later one gives back its result.
pub use target::init;

/// Returns `true` if an interruption is waiting, `false` otherwise. Reading the
/// request clears it, so the run after an interrupted one does not stop at its
/// first check.
pub use target::check_interrupt;

/// `system.sleep` holds the program for `ms` milliseconds. An interruption may
/// end it early.
pub use target::sleep;

/// `system.draw_svg` puts a drawing on the page. Only the browser draws, so a
/// native run has no such function.
#[cfg(target_arch = "wasm32")]
pub use target::draw_svg;

/// Prints a scene in the terminal at the cursor. Returns `true` if the scene
/// went out, `false` if stdout is not a terminal that shows images or the
/// print failed. The browser draws an svg instead, so it has no such
/// function.
#[cfg(not(target_arch = "wasm32"))]
pub use target::print_scene;

/// The choice of the display for the frames of a world and the images of the
/// repl. `set_display` makes it before a run, and `Auto` stands until then.
/// The browser always draws on the page, so it has no such choice.
#[cfg(not(target_arch = "wasm32"))]
pub use target::{DisplayMode, set_display};

#[cfg(target_arch = "wasm32")]
#[path = "host/wasm.rs"]
mod target;
#[cfg(not(target_arch = "wasm32"))]
#[path = "host/native.rs"]
mod target;

thread_local! {
    static FRAMES: RefCell<Frames> = RefCell::new(Frames::new("sgleam", target::stage_options));
    static MULTI_FRAMES: RefCell<MultiFrames> = RefCell::new(MultiFrames::new("sgleam", target::stage_options));
}

/// Shows a frame of a world, in a window or the terminal, or in a session of
/// sinteract on the page. The first frame of a world opens the display with
/// its own size, and the frame goes out at the next tick of the display.
/// Returns `true` if the display is open, `false` if it did not open.
pub fn show_scene(scene: Scene) -> bool {
    FRAMES.with_borrow_mut(|frames| frames.show(scene).is_ok())
}

/// Opens the stage of a multiplayer world and returns alternating player
/// numbers and nicknames from its fixed start.
pub fn open_mworld(
    width: f32,
    height: f32,
    min_players: u32,
    max_players: u32,
) -> Result<Vec<String>, String> {
    MULTI_FRAMES.with_borrow_mut(|frames| frames.open(width, height, min_players, max_players))
}

/// Keeps the latest scene for one player until the next stage tick.
pub fn show_player_scene(player: u32, scene: Scene) -> bool {
    MULTI_FRAMES.with_borrow_mut(|frames| frames.show(player, scene))
}

/// Closes the display of the frames, so the world that comes next opens its
/// own. A window goes away, and the terminal keeps the last frame on screen.
pub fn close_frames() {
    // The terminal shows the lines of `print_line` under the frame, and they
    // print under it after the close.
    for line in FRAMES.with_borrow_mut(Frames::close) {
        println!("{line}");
    }
    for line in MULTI_FRAMES.with_borrow_mut(MultiFrames::close) {
        println!("{line}");
    }
}

/// `console.log` prints a line, and so does `echo`. While the terminal shows
/// the frames of a world, the line shows under the frame and prints when the
/// frames close.
pub fn print_line(line: &str) {
    if !MULTI_FRAMES.with_borrow_mut(|frames| frames.print_line(line))
        && !FRAMES.with_borrow_mut(|frames| frames.print_line(line))
    {
        println!("{line}");
    }
}

/// `system.wait_event` waits at most `ms` milliseconds for a key or for the
/// display to close. It gives the kind of the event, then for a key its name
/// and the modifiers that are on, or nothing when the time runs out. With no
/// display open, it sleeps.
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
            if Instant::now() >= end || target::stopping() {
                return vec![];
            }
        }
    })
}

/// Waits for an input of a fixed player, for the session to close, or for
/// `ms` milliseconds. Display ticks send the latest frame of every player.
pub fn wait_mworld_event(ms: u64) -> Vec<String> {
    if !MULTI_FRAMES.with_borrow(MultiFrames::is_open) {
        sleep(ms);
        return vec![];
    }
    let end = Instant::now() + Duration::from_millis(ms);
    loop {
        let deadline = (Instant::now() + Duration::from_millis(10)).min(end);
        let event = MULTI_FRAMES.with_borrow_mut(|frames| {
            frames.wait_event(deadline, |error| tracing::warn!("{error}"))
        });
        match event {
            MultiFrameEvent::Input { player, event } => {
                if let Some(event) = player_event(player, event) {
                    return event;
                }
            }
            MultiFrameEvent::Close => return vec!["close".into()],
            MultiFrameEvent::Timeout => {}
        }
        if Instant::now() >= end || target::stopping() {
            return vec![];
        }
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
