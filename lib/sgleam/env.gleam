import sgleam/random.{type Generator}

/// What the host knows when the program starts.
pub opaque type Env {
  Env(args: List(String), random: Generator, time: Int)
}

/// The arguments after the file on the command line.
pub fn args(env: Env) -> List(String) {
  env.args
}

/// A generator with a seed that changes from one run to the next.
pub fn random(env: Env) -> Generator {
  env.random
}

/// The milliseconds since 1970-01-01 00:00 UTC when the program started.
pub fn time(env: Env) -> Int {
  env.time
}

/// An env for tests, with no arguments, `random.new(0)`, and time 0.
pub fn fake() -> Env {
  Env(args: [], random: random.new(0), time: 0)
}

pub fn with_args(env: Env, args: List(String)) -> Env {
  Env(..env, args:)
}

pub fn with_random(env: Env, random: Generator) -> Env {
  Env(..env, random:)
}

pub fn with_time(env: Env, time: Int) -> Env {
  Env(..env, time:)
}

/// The env of this run. The run calls it once for each `smain` and each
/// world that takes an env.
@internal
pub fn from_host() -> Env {
  Env(args: host_args(), random: random.new(host_seed()), time: host_time())
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "host_args")
fn host_args() -> List(String)

@external(javascript, "../sgleam/sgleam_ffi.mjs", "host_seed")
fn host_seed() -> Int

@external(javascript, "../sgleam/sgleam_ffi.mjs", "now_ms")
fn host_time() -> Int
