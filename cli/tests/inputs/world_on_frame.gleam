import sgleam/image
import sgleam/stroke
import sgleam/world

pub type State {
  State(frames: Int, ms: Int, deltas: Int)
}

pub fn draw(state: State) -> image.Image {
  image.circle(10 + state.frames, stroke.red)
}

pub fn frame(state: State, ms: Int) -> State {
  State(..state, frames: state.frames + 1, ms:)
}

pub fn frame_delta(state: State, dt: Int) -> State {
  let deltas = state.deltas + dt
  echo #(state.frames, deltas == state.ms && dt >= 0) as "frame"
  State(..state, deltas:)
}

pub fn smain() {
  world.create(State(0, 0, 0), draw)
  |> world.on_frame(frame)
  |> world.on_frame_delta(frame_delta)
  |> world.stop_when(fn(state) { state.frames == 3 })
}
