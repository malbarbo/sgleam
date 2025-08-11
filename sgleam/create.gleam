import gleam/int
import sgleam/image.{type Image}
import sgleam/system

pub fn animation(to_image: fn(Int) -> Image, delay: Int) -> Nil {
  animate_loop(to_image, int.max(0, delay), 0)
}

fn animate_loop(to_image: fn(Int) -> Image, delay: Int, frame: Int) {
  system.show_svg(frame |> to_image |> image.to_svg)
  system.sleep(delay)
  animate_loop(to_image, delay, frame + 1)
}
