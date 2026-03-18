import gleam/io
import sgleam/fill
import sgleam/image
import sgleam/stroke
import sgleam/style

pub fn main() {
  // rectangle
  tag("rectangle")
  io.println(image.to_svg(image.rectangle(100, 50, fill.blue)))

  // square
  tag("square")
  io.println(image.to_svg(image.square(80, fill.red)))

  // circle
  tag("circle")
  io.println(image.to_svg(image.circle(40, fill.green)))

  // ellipse
  tag("ellipse")
  io.println(image.to_svg(image.ellipse(60, 30, fill.yellow)))

  // triangle
  tag("triangle")
  io.println(image.to_svg(image.triangle(80, fill.orange)))

  // right_triangle
  tag("right_triangle")
  io.println(image.to_svg(image.right_triangle(60, 40, fill.purple)))

  // star
  tag("star")
  io.println(image.to_svg(image.star(50, fill.gold)))

  // radial_star
  tag("radial_star")
  io.println(image.to_svg(image.radial_star(5, 20, 50, fill.orange)))

  // text
  tag("text")
  io.println(image.to_svg(image.text("Olá!", 24, fill.black)))

  // style_join
  tag("style_join")
  io.println(
    image.to_svg(image.circle(
      40,
      style.join([fill.red, stroke.black, stroke.width(2)]),
    )),
  )

  // beside
  tag("beside")
  io.println(
    image.to_svg(image.beside(
      image.square(40, fill.red),
      image.square(40, fill.blue),
    )),
  )

  // above
  tag("above")
  io.println(
    image.to_svg(image.above(
      image.square(40, fill.red),
      image.square(40, fill.blue),
    )),
  )

  // overlay
  tag("overlay")
  io.println(
    image.to_svg(image.overlay(
      image.circle(20, fill.red),
      image.square(60, fill.blue),
    )),
  )

  // scene
  tag("scene")
  io.println(
    image.to_svg(
      image.empty_scene(200, 200)
      |> image.place_image(100, 100, image.circle(20, fill.red)),
    ),
  )

  // hexagon
  tag("hexagon")
  io.println(image.to_svg(image.regular_polygon(40, 6, fill.blue)))

  // rhombus
  tag("rhombus")
  io.println(image.to_svg(image.rhombus(60, 45, fill.red)))
}

fn tag(name: String) {
  io.println("<!--IMG:" <> name <> "-->")
}
