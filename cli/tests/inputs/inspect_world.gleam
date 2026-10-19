import gleam/string
import sgleam/fill
import sgleam/image
import sgleam/world

pub fn smain() {
  let w = world.create(0, fn(_) { image.circle(10, fill.red) })
  let a = world.animation(fn(_) { image.circle(10, fill.red) })
  echo #(w, a)
  string.inspect([w])
}
