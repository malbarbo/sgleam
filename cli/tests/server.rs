//! `sgleam server` against a room of sinteract, with the test as the server.
#![cfg(unix)]

use std::fs::File;
use std::io::{PipeWriter, Read, Write};
use std::num::NonZeroU16;
use std::os::fd::OwnedFd;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use command_fds::{CommandFdExt, FdMapping};
use indoc::indoc;
use sinteract::server::{Lobby, Next, Room};
use sinteract::session::{ENGINE_TO_SERVER_FD, FrameRate, PlayerRange, SERVER_TO_ENGINE_FD};

/// How long the test waits for the engine before it fails.
const TIMEOUT: Duration = Duration::from_secs(20);

/// A run of `sgleam server game.gleam`, with the ends of its pipes that the
/// server keeps.
struct Engine {
    child: Child,
    to_engine: Option<PipeWriter>,
    from_engine: Receiver<Vec<u8>>,
    // The module of the game lives here until the engine ends.
    _dir: tempfile::TempDir,
}

impl Engine {
    fn spawn(source: &str) -> Engine {
        let dir = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(dir.path().join("game.gleam"), source).expect("write the module");
        let (engine_reads, to_engine) = std::io::pipe().expect("a pipe to the engine");
        let (from_engine, engine_writes) = std::io::pipe().expect("a pipe from the engine");
        let mut command = Command::new(env!("CARGO_BIN_EXE_sgleam"));
        command
            .current_dir(dir.path())
            .args(["server", "game.gleam"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .fd_mappings(vec![
                FdMapping {
                    parent_fd: OwnedFd::from(engine_reads),
                    child_fd: SERVER_TO_ENGINE_FD,
                },
                FdMapping {
                    parent_fd: OwnedFd::from(engine_writes),
                    child_fd: ENGINE_TO_SERVER_FD,
                },
            ])
            .expect("fd 3 and fd 4");
        let child = command.spawn().expect("run sgleam");
        // The command holds the end of fd 4 that the engine writes, and the
        // reader sees the end of fd 4 only once no one holds it.
        drop(command);
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut from_engine = from_engine;
            let mut buf = vec![0; 1 << 16];
            while let Ok(n @ 1..) = from_engine.read(&mut buf) {
                let Some(bytes) = buf.get(..n) else { return };
                if send.send(bytes.to_vec()).is_err() {
                    return;
                }
            }
        });
        Engine {
            child,
            to_engine: Some(to_engine),
            from_engine: receive,
            _dir: dir,
        }
    }

    /// The bytes that the engine wrote next, or `None` at the end of fd 4.
    fn read(&self, deadline: Instant) -> Option<Vec<u8>> {
        let left = deadline.saturating_duration_since(Instant::now());
        match self.from_engine.recv_timeout(left) {
            Ok(bytes) => Some(bytes),
            Err(RecvTimeoutError::Disconnected) => None,
            Err(RecvTimeoutError::Timeout) => panic!("the engine said nothing in time"),
        }
    }

    /// The lobby after the hello of the engine.
    fn hello(&self) -> Lobby {
        let deadline = Instant::now() + TIMEOUT;
        let mut lobby = Lobby::new();
        while lobby.player_range().is_none() {
            let bytes = self.read(deadline).expect("a hello before the end");
            lobby = lobby.from_engine(&bytes).expect("a hello");
        }
        lobby
    }

    /// Writes what the room has for the engine, and closes fd 3 once the
    /// room writes nothing more.
    fn write(&mut self, room: &mut Room) {
        let mut buf = Vec::new();
        if room.take_engine_output(&mut buf) {
            let to_engine = self.to_engine.as_mut().expect("fd 3 open");
            to_engine.write_all(&buf).expect("write to the engine");
        } else {
            self.to_engine = None;
        }
    }

    /// Closes the room, takes the last bytes of the engine, and returns what
    /// the engine printed once it exits with success.
    fn close(mut self, mut room: Room) -> String {
        room.close();
        self.write(&mut room);
        let deadline = Instant::now() + TIMEOUT;
        while let Some(bytes) = self.read(deadline) {
            room.from_engine(&bytes).expect("a whole message");
        }
        room.engine_ended().expect("the end after a whole message");
        let out = self.child.wait_with_output().expect("wait for sgleam");
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(out.status.success(), "{}: {stdout}", out.status);
        stdout
    }
}

fn frame_rate() -> FrameRate {
    FrameRate::from_hz(NonZeroU16::new(30).expect("30 is not 0"))
}

#[test]
fn server_plays_a_world_in_a_room() {
    let mut engine = Engine::spawn(indoc! {"
        import sgleam/fill
        import sgleam/image
        import sgleam/world

        pub fn smain() {
          world.create(0, fn(_) { image.square(10, fill.red) })
          |> world.run
        }
    "});
    let lobby = engine.hello();
    assert_eq!(lobby.player_range(), PlayerRange::new(1, 1));
    let mut room = lobby
        .start(&["ana"], frame_rate())
        .map_err(|(_, error)| error)
        .expect("start");
    let (player, _) = room.players().next().expect("a player");
    let conn = room.connect(player).expect("a connection");
    let deadline = Instant::now() + TIMEOUT;
    loop {
        room.tick();
        engine.write(&mut room);
        let bytes = engine.read(deadline).expect("a frame before the end");
        room.from_engine(&bytes).expect("whole messages");
        if let Next::Send(_) = room.next_for(conn) {
            break;
        }
    }
    engine.close(room);
}

#[test]
fn server_says_the_players_of_an_mworld() {
    let mut engine = Engine::spawn(indoc! {"
        import sgleam/fill
        import sgleam/image
        import sgleam/mworld

        pub fn smain() {
          mworld.create(0, min: 2, max: 3, to_image: fn(_, _) {
            image.square(10, fill.red)
          })
          |> mworld.run
        }
    "});
    let lobby = engine.hello();
    assert_eq!(lobby.player_range(), PlayerRange::new(2, 3));
    let mut room = lobby
        .start(&["ana", "bia"], frame_rate())
        .map_err(|(_, error)| error)
        .expect("start");
    engine.write(&mut room);
    engine.close(room);
}

#[test]
fn server_fails_without_the_pipes_of_a_server() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(
        dir.path().join("game.gleam"),
        indoc! {"
            import sgleam/fill
            import sgleam/image
            import sgleam/world

            pub fn smain() {
              world.create(0, fn(_) { image.square(10, fill.red) })
              |> world.run
            }
        "},
    )
    .expect("write the module");
    // A file on fd 3 and fd 4, as a shell gives with 3>file, and never the
    // pipes of a jobserver that the test may have.
    let null = || OwnedFd::from(File::open("/dev/null").expect("/dev/null"));
    let mut command = Command::new(env!("CARGO_BIN_EXE_sgleam"));
    command
        .current_dir(dir.path())
        .args(["server", "game.gleam"])
        .stdin(Stdio::null())
        .fd_mappings(vec![
            FdMapping {
                parent_fd: null(),
                child_fd: SERVER_TO_ENGINE_FD,
            },
            FdMapping {
                parent_fd: null(),
                child_fd: ENGINE_TO_SERVER_FD,
            },
        ])
        .expect("fd 3 and fd 4");
    let out = command.output().expect("run sgleam");
    assert!(!out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "Error: fd 3 is not a pipe or a socket, so no server opened it\n"
    );
}
