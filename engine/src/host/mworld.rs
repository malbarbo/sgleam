//! The stage and frames of a multiplayer world.
//!
//! A `MultiFrames` opens its stage before the Gleam loop starts, because the
//! loop needs the fixed players from the session start. Each show keeps the
//! latest scene for one player, and a stage tick sends the pending scenes.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::time::Instant;

use sinteract::display::{PresentError, Stage, StageEvent, StageOptions};
use sinteract::event::{InputEvent, Interrupt, KeyEvent, KeyKind};
use sinteract::scene::Scene;
use sinteract::session::{Player, PlayerRange, Target};

/// What [`MultiFrames::wait_event`] returns.
pub(super) enum MultiFrameEvent {
    Input { player: u32, event: InputEvent },
    Close,
    Timeout,
}

pub(super) struct MultiFrames {
    title: String,
    options: Box<dyn FnMut() -> StageOptions>,
    state: State,
    footer: Vec<String>,
}

enum State {
    Closed,
    Open(Open),
    /// A one-player world keeps running when no native display opens, just as
    /// the original `Frames` adapter does for tests and `-d none`.
    Unavailable,
}

struct Open {
    stage: Stage,
    players: BTreeMap<u32, Player>,
    pending: BTreeMap<Player, Scene>,
    inputs: VecDeque<(Player, InputEvent)>,
    after_down: HashMap<Player, bool>,
    closed: bool,
}

impl MultiFrames {
    pub(super) fn new(
        title: impl Into<String>,
        options: impl FnMut() -> StageOptions + 'static,
    ) -> Self {
        MultiFrames {
            title: title.into(),
            options: Box::new(options),
            state: State::Closed,
            footer: Vec::new(),
        }
    }

    pub(super) fn is_open(&self) -> bool {
        matches!(self.state, State::Open(_))
    }

    /// Opens one stage and returns alternating player numbers and nicknames.
    pub(super) fn open(
        &mut self,
        width: f32,
        height: f32,
        min_players: u32,
        max_players: u32,
    ) -> Result<Vec<String>, String> {
        if !matches!(self.state, State::Closed) {
            return Err("a world is already running".into());
        }
        let range = PlayerRange::new(min_players, max_players)
            .ok_or_else(|| "the number of players must be between 1 and 1024".to_string())?;
        let options = (self.options)();
        match Stage::open(&self.title, width, height, range, options) {
            Ok((stage, players)) => {
                let result = players
                    .iter()
                    .flat_map(|(player, nickname)| {
                        [player.number().get().to_string(), nickname.clone()]
                    })
                    .collect();
                let players = players
                    .into_iter()
                    .map(|(player, _)| (player.number().get(), player))
                    .collect();
                self.state = State::Open(Open {
                    stage,
                    players,
                    pending: BTreeMap::new(),
                    inputs: VecDeque::new(),
                    after_down: HashMap::new(),
                    closed: false,
                });
                Ok(result)
            }
            // A program that accepts one player can still calculate and print
            // images when the user explicitly disabled the native display.
            Err(error) if min_players == 1 => {
                tracing::debug!(%error, "world display unavailable");
                self.state = State::Unavailable;
                Ok(vec!["1".into(), String::new()])
            }
            Err(error) => Err(error.to_string()),
        }
    }

    /// Keeps `scene` for the next display tick of `player`.
    pub(super) fn show(&mut self, player: u32, scene: Scene) -> bool {
        let State::Open(open) = &mut self.state else {
            return false;
        };
        let Some(&player) = open.players.get(&player) else {
            return false;
        };
        open.pending.insert(player, scene);
        true
    }

    pub(super) fn wait_event(
        &mut self,
        deadline: Instant,
        mut on_error: impl FnMut(&dyn std::error::Error),
    ) -> MultiFrameEvent {
        let State::Open(open) = &mut self.state else {
            return MultiFrameEvent::Timeout;
        };
        loop {
            if let Some((player, event)) = open.inputs.pop_front() {
                return MultiFrameEvent::Input {
                    player: player.number().get(),
                    event,
                };
            }
            if open.closed {
                return MultiFrameEvent::Close;
            }
            match open.stage.wait(Some(deadline)) {
                Ok(StageEvent::Tick) => open.present(&mut on_error),
                Ok(StageEvent::Input { player, event }) => open.take_input(player, event),
                Ok(StageEvent::Error(error)) => on_error(&error),
                Err(Interrupt::Timeout) => return MultiFrameEvent::Timeout,
                Err(Interrupt::Wake) => {}
                Err(Interrupt::Read(error)) => {
                    on_error(&error);
                    open.closed = true;
                }
                Err(Interrupt::Close) => open.closed = true,
            }
        }
    }

    /// Shows a line below a native frame when that display reserves a footer.
    pub(super) fn print_line(&mut self, line: &str) -> bool {
        let State::Open(open) = &mut self.state else {
            return false;
        };
        let rows = usize::from(open.stage.footer_rows());
        if rows == 0 {
            return false;
        }
        self.footer.extend(line.split('\n').map(String::from));
        let excess = self.footer.len().saturating_sub(rows);
        self.footer.drain(..excess);
        open.stage.set_footer(&self.footer).is_ok()
    }

    /// Sends every last pending frame and closes the stage.
    pub(super) fn close(&mut self) -> Vec<String> {
        if let State::Open(open) = &mut self.state {
            open.present(&mut |_| {});
            open.stage.close();
        }
        self.state = State::Closed;
        std::mem::take(&mut self.footer)
    }
}

impl Open {
    fn present(&mut self, on_error: &mut impl FnMut(&dyn std::error::Error)) {
        for (player, scene) in std::mem::take(&mut self.pending) {
            if let Err(error) = self.stage.present(Target::Player(player), scene) {
                on_error(&error);
                if let PresentError::Closed | PresentError::Io(_) = error {
                    self.closed = true;
                    break;
                }
            }
        }
    }

    fn take_input(&mut self, player: Player, event: InputEvent) {
        if let InputEvent::Key(key) = event {
            if key.kind == KeyKind::Press && !self.after_down.get(&player).copied().unwrap_or(false)
            {
                self.inputs.push_back((
                    player,
                    InputEvent::Key(KeyEvent {
                        kind: KeyKind::Down,
                        ..key.clone()
                    }),
                ));
            }
            self.after_down.insert(player, key.kind == KeyKind::Down);
            self.inputs.push_back((player, InputEvent::Key(key)));
        } else {
            self.inputs.push_back((player, event));
        }
    }
}
