import gleam/list
import gleam/string
import sgleam/fill
import sgleam/image
import sgleam/stroke
import sgleam/style

pub fn smain() {
  [
    #("rectangle", image.rectangle(100, 50, fill.blue)),
    #("square", image.square(80, fill.red)),
    #("circle", image.circle(40, fill.green)),
    #("ellipse", image.ellipse(60, 30, fill.yellow)),
    #("triangle", image.triangle(80, fill.orange)),
    #("right_triangle", image.right_triangle(60, 40, fill.purple)),
    #("star", image.star(50, fill.gold)),
    #("radial_star", image.radial_star(5, 20, 50, fill.orange)),
    #("text", image.text("Hello!", 24, fill.black)),
    #(
      "style_join",
      image.circle(40, style.join([fill.red, stroke.black, stroke.width(2)])),
    ),
    #(
      "beside",
      image.beside(image.square(40, fill.red), image.square(40, fill.blue)),
    ),
    #(
      "above",
      image.above(image.square(40, fill.red), image.square(40, fill.blue)),
    ),
    #(
      "overlay",
      image.overlay(image.circle(20, fill.red), image.square(60, fill.blue)),
    ),
    #(
      "scene",
      image.empty_scene(200, 200)
        |> image.place_image(100, 100, image.circle(20, fill.red)),
    ),
    #("hexagon", image.regular_polygon(40, 6, fill.blue)),
    #("rhombus", image.rhombus(60, 45, fill.red)),
  ]
  |> list.map(tagged)
  |> string.join("\n")
}

fn tagged(named: #(String, image.Image)) -> String {
  "<!--IMG:" <> named.0 <> "-->\n" <> image.to_svg(named.1)
}
