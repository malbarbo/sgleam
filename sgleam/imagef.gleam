import gleam/bool
import gleam/float
import gleam/int
import gleam/list
import gleam/string
import sgleam/color.{type Color}
import sgleam/math.{cos_deg, hypot, sin_deg}
import sgleam/style.{type Style}
import sgleam/xplace.{type XPlace, Center, Left, Right}
import sgleam/yplace.{type YPlace, Bottom, Middle, Top}

// FIXME: adjuste figure with outline only (https://docs.racket-lang.org/teachpack/2htdpimage-guide.html#%28part._nitty-gritty%29)
// TODO: add constants for dash
// TODO: wedge
// TODO: all curve funtions
// TODO: all text functions
// TODO: triangle/sa...
// TODO: pulled_regular_polygon
// TODO: place_images
// TODO: place_images_align
// TODO: bitmap...
// TODO: freeze
// TODO: create pen?
// TODO: equality test
// TODO: pin holes...

// **************************
// * Point
// **************************

pub type Point {
  Point(x: Float, y: Float)
}

fn point_translate(p: Point, dx: Float, dy: Float) -> Point {
  Point(p.x +. dx, p.y +. dy)
}

fn point_rotate(p: Point, center: Point, angle: Float) -> Point {
  let dx = p.x -. center.x
  let dy = p.y -. center.y

  Point(
    center.x +. dx *. cos_deg(angle) -. dy *. sin_deg(angle),
    center.y +. dx *. sin_deg(angle) +. dy *. cos_deg(angle),
  )
}

fn point_flip_x(p: Point) -> Point {
  Point(0.0 -. p.x, p.y)
}

fn point_flip_y(p: Point) -> Point {
  Point(p.x, 0.0 -. p.y)
}

// **************************
// * Align
// **************************

fn x_place_dx(x_place: XPlace, wa: Float, wb: Float) -> #(Float, Float) {
  case x_place {
    Left -> #(0.0, 0.0)
    Center -> {
      let wm = float.max(wa, wb)
      #(mid(wm, wa), mid(wm, wb))
    }
    Right -> {
      let wm = float.max(wa, wb)
      #(wm -. wa, wm -. wb)
    }
  }
}

fn y_place_dy(y_place: YPlace, ha: Float, hb: Float) -> #(Float, Float) {
  case y_place {
    Top -> #(0.0, 0.0)
    Middle -> {
      let hm = float.max(ha, hb)
      #(mid(hm, ha), mid(hm, hb))
    }
    Bottom -> {
      let hm = float.max(ha, hb)
      #(hm -. ha, hm -. hb)
    }
  }
}

fn mid(a: Float, b: Float) -> Float {
  { a -. b } /. 2.0
}

// **************************
// * Image
// **************************

pub opaque type Image {
  Rectangle(
    style: Style,
    center: Point,
    width: Float,
    height: Float,
    angle: Float,
  )
  Ellipse(
    style: Style,
    center: Point,
    width: Float,
    height: Float,
    angle: Float,
  )
  Polygon(style: Style, points: List(Point))
  Combination(Image, Image)
  Crop(center: Point, width: Float, height: Float, angle: Float, image: Image)
}

pub const empty = Rectangle(style.none, Point(0.0, 0.0), 0.0, 0.0, 0.0)

pub fn width(img: Image) -> Float {
  let #(min, max) = box(img)
  max.x -. min.x
}

pub fn height(img: Image) -> Float {
  let #(min, max) = box(img)
  max.y -. min.y
}

pub fn dimension(img: Image) -> #(Float, Float) {
  let #(min, max) = box(img)
  #(max.x -. min.x, max.y -. min.y)
}

pub fn center(img: Image) -> Point {
  let #(min, max) = box(img)
  Point(mid(max.x, min.x), mid(max.y, min.y))
}

fn translate(img: Image, dx: Float, dy: Float) -> Image {
  use <- bool.guard(dx == 0.0 && dy == 0.0, img)
  case img {
    Rectangle(center:, ..) ->
      Rectangle(..img, center: point_translate(center, dx, dy))
    Ellipse(center:, ..) ->
      Ellipse(..img, center: point_translate(center, dx, dy))
    Polygon(points:, ..) ->
      Polygon(..img, points: list.map(points, point_translate(_, dx, dy)))
    Combination(a, b) -> Combination(translate(a, dx, dy), translate(b, dx, dy))
    Crop(center:, image:, ..) ->
      Crop(
        ..img,
        center: point_translate(center, dx, dy),
        image: translate(image, dx, dy),
      )
  }
}

fn fix_position(img: Image) -> Image {
  let #(min, _) = box(img)
  case min == Point(0.0, 0.0) {
    True -> img
    False -> translate(img, 0.0 -. min.x, 0.0 -. min.y)
  }
}

fn box(img: Image) -> #(Point, Point) {
  case img {
    Rectangle(center:, width:, height:, angle:, ..) -> {
      let hw = width /. 2.0
      let hh = height /. 2.0
      let abs = float.absolute_value
      let dx = hw *. abs(cos_deg(angle)) +. hh *. abs(sin_deg(angle))
      let dy = hw *. abs(sin_deg(angle)) +. hh *. abs(cos_deg(angle))
      #(
        point_translate(center, 0.0 -. dx, 0.0 -. dy),
        point_translate(center, dx, dy),
      )
    }
    Ellipse(center:, width:, height:, angle:, ..) -> {
      let dx = hypot(width *. cos_deg(angle), height *. sin_deg(angle))
      let dy = hypot(width *. sin_deg(angle), height *. cos_deg(angle))
      #(
        point_translate(center, 0.0 -. dx, 0.0 -. dy),
        point_translate(center, dx, dy),
      )
    }
    Combination(a, b) -> {
      let #(amin, amax) = box(a)
      let #(bmin, bmax) = box(b)
      #(
        Point(float.min(amin.x, bmin.x), float.min(amin.y, bmin.y)),
        Point(float.max(amax.x, bmax.x), float.max(amax.y, bmax.y)),
      )
    }
    Polygon(points:, ..) -> {
      let min_x = list.fold(points, 0.0, fn(min, p) { float.min(min, p.x) })
      let min_y = list.fold(points, 0.0, fn(min, p) { float.min(min, p.y) })
      let max_x = list.fold(points, 0.0, fn(max, p) { float.max(max, p.x) })
      let max_y = list.fold(points, 0.0, fn(max, p) { float.max(max, p.y) })
      #(Point(min_x, min_y), Point(max_x, max_y))
    }
    Crop(center:, width:, height:, angle:, ..) -> {
      let hw = width /. 2.0
      let hh = height /. 2.0
      let abs = float.absolute_value
      let dx = hw *. abs(cos_deg(angle)) +. hh *. abs(sin_deg(angle))
      let dy = hw *. abs(sin_deg(angle)) +. hh *. abs(cos_deg(angle))
      #(
        point_translate(center, 0.0 -. dx, 0.0 -. dy),
        point_translate(center, dx, dy),
      )
    }
  }
}

// **************************
// * Basic images
// **************************

pub fn rectangle(width: Float, height: Float, style: Style) -> Image {
  let width = positive(width)
  let height = positive(height)
  Rectangle(style, Point(width /. 2.0, height /. 2.0), width, height, 0.0)
}

pub fn square(side: Float, style: Style) -> Image {
  rectangle(side, side, style)
}

pub fn ellipse(width: Float, height: Float, style: Style) -> Image {
  let hw = positive(width) /. 2.0
  let hh = positive(height) /. 2.0
  Ellipse(style, Point(hw, hh), hw, hh, 0.0)
}

pub fn circle(radius: Float, style: Style) -> Image {
  ellipse(2.0 *. radius, 2.0 *. radius, style)
}

pub fn line(x: Float, y: Float, style: Style) -> Image {
  Polygon(style, [Point(0.0, 0.0), Point(x, y)])
  |> fix_position
}

pub fn add_line(
  img: Image,
  x1: Float,
  y1: Float,
  x2: Float,
  y2: Float,
  style: Style,
) -> Image {
  Combination(img, Polygon(style, [Point(x1, y1), Point(x2, y2)]))
  |> fix_position
}

// **************************
// * Polygons
// **************************

pub fn triangle(side: Float, style: Style) -> Image {
  let side = positive(side)
  // side *. sqrt(3.0) /. 2.0
  let height = side *. 0.8660254037844386
  Polygon(style, [
    Point(side /. 2.0, 0.0),
    Point(side, height),
    Point(0.0, height),
  ])
}

pub fn right_triangle(side1: Float, side2: Float, style: Style) -> Image {
  let side1 = positive(side1)
  let side2 = positive(side2)
  Polygon(style, [Point(0.0, 0.0), Point(0.0, side2), Point(side1, side2)])
}

pub fn isosceles_triangle(
  side_length: Float,
  angle: Float,
  style: Style,
) -> Image {
  let side_length = positive(side_length)
  let hangle = angle /. 2.0
  Polygon(style, [
    Point(side_length *. sin_deg(hangle), side_length *. cos_deg(hangle)),
    Point(0.0, 0.0),
    Point(0.0 -. side_length *. sin_deg(hangle), side_length *. cos_deg(hangle)),
  ])
  |> fix_position
}

pub fn rhombus(side_length: Float, angle: Float, style: Style) -> Image {
  let side_length = positive(side_length)
  let height = 2.0 *. side_length *. cos_deg(angle /. 2.0)
  let width = 2.0 *. side_length *. sin_deg(angle /. 2.0)
  Polygon(style, [
    Point(0.0, height /. 2.0),
    Point(width /. 2.0, 0.0),
    Point(width, height /. 2.0),
    Point(width /. 2.0, height),
  ])
}

pub fn regular_polygon(
  side_length: Float,
  side_count: Int,
  style: Style,
) -> Image {
  star_polygon(side_length, side_count, 1, style)
}

pub fn polygon(points: List(Point), style: Style) -> Image {
  Polygon(style, points) |> fix_position
}

pub fn add_polygon(img: Image, points: List(Point), style: Style) -> Image {
  Combination(img, Polygon(style, points)) |> fix_position
}

pub fn star_polygon(
  side_length: Float,
  side_count: Int,
  step_count: Int,
  style: Style,
) -> Image {
  let side_count = int.max(1, side_count)
  let side_countf = int.to_float(side_count)
  let step_count = int.max(1, step_count)
  let radius = positive(side_length) /. { 2.0 *. sin_deg(180.0 /. side_countf) }
  let alpha = case int.is_even(side_count) {
    True -> -180.0 /. side_countf
    False -> -90.0
  }

  list.range(0, side_count - 1)
  |> list.map(fn(i) {
    let theta =
      alpha +. 360.0 *. int.to_float(i * step_count % side_count) /. side_countf
    Point(radius *. cos_deg(theta), radius *. sin_deg(theta))
  })
  |> Polygon(style, _)
  |> fix_position
}

pub fn star(side_length: Float, style: Style) -> Image {
  star_polygon(side_length, 5, 2, style)
}

pub fn radial_start(
  point_count: Int,
  inner_radius: Float,
  outer_radius: Float,
  style: Style,
) -> Image {
  let point_count = int.max(2, point_count)
  let inner_radius = positive(inner_radius)
  let outer_radius = positive(outer_radius)
  let alpha = case int.is_even(point_count) {
    True -> -180.0 /. int.to_float(point_count)
    False -> -90.0
  }

  list.range(0, 2 * point_count - 1)
  |> list.flat_map(fn(i) {
    let theta1 =
      alpha +. 360.0 *. int.to_float(i * 2) /. int.to_float(2 * point_count)
    let theta2 =
      alpha +. 360.0 *. int.to_float(i * 2 + 1) /. int.to_float(2 * point_count)
    [
      Point(outer_radius *. cos_deg(theta1), outer_radius *. sin_deg(theta1)),
      Point(inner_radius *. cos_deg(theta2), inner_radius *. sin_deg(theta2)),
    ]
  })
  |> Polygon(style, _)
  |> fix_position
}

fn positive(n: Float) -> Float {
  float.max(0.0, n)
}

// **************************
// * Transformations
// **************************

pub fn rotate(img: Image, angle: Float) -> Image {
  // the api for the user is counter clockwise, but the implementation is clockwise
  rotate_around(img, center(img), 0.0 -. angle)
  |> fix_position
}

fn rotate_around(img: Image, center: Point, angle: Float) -> Image {
  case img {
    Rectangle(..) ->
      Rectangle(
        ..img,
        center: point_rotate(img.center, center, angle),
        angle: img.angle +. angle,
      )
    Ellipse(..) ->
      Ellipse(
        ..img,
        center: point_rotate(img.center, center, angle),
        angle: img.angle +. angle,
      )
    Polygon(..) ->
      Polygon(
        ..img,
        points: list.map(img.points, point_rotate(_, center, angle)),
      )
    Combination(a, b) ->
      Combination(
        rotate_around(a, center, angle),
        rotate_around(b, center, angle),
      )
    Crop(..) ->
      Crop(
        ..img,
        center: point_rotate(img.center, center, angle),
        angle: img.angle +. angle,
        image: rotate_around(img.image, center, angle),
      )
  }
}

pub fn scale(img: Image, factor: Float) -> Image {
  scale_xy(img, factor, factor)
}

pub fn scale_xy(img: Image, x_factor: Float, y_factor: Float) -> Image {
  let x_factor = positive(x_factor)
  let y_factor = positive(y_factor)
  case img {
    Rectangle(width:, height:, ..) ->
      Rectangle(..img, width: width *. x_factor, height: height *. y_factor)
    Ellipse(width:, height:, ..) ->
      Ellipse(..img, width: width *. x_factor, height: height *. y_factor)
    Polygon(points:, ..) ->
      Polygon(
        ..img,
        points: list.map(points, fn(p) {
          Point(p.x *. x_factor, p.y *. y_factor)
        }),
      )
    Combination(a, b) ->
      Combination(
        scale_xy(a, x_factor, y_factor),
        scale_xy(b, x_factor, y_factor),
      )
    Crop(width:, height:, image:, ..) ->
      Crop(
        ..img,
        width: width *. x_factor,
        height: height *. y_factor,
        image: scale_xy(image, x_factor, y_factor),
      )
  }
  |> fix_position
}

pub fn flip_horizontal(img: Image) -> Image {
  flip(img, point_flip_x)
}

pub fn flip_vertical(img: Image) -> Image {
  flip(img, point_flip_y)
}

fn flip(img: Image, point_flip: fn(Point) -> Point) -> Image {
  case img {
    Rectangle(center:, angle:, ..) ->
      Rectangle(..img, center: point_flip(center), angle: 0.0 -. angle)
    Ellipse(center:, angle:, ..) -> {
      Ellipse(..img, center: point_flip(center), angle: 0.0 -. angle)
    }
    Polygon(points:, ..) -> Polygon(..img, points: list.map(points, point_flip))
    Combination(a, b) -> Combination(flip(a, point_flip), flip(b, point_flip))
    Crop(center:, angle:, image:, ..) ->
      Crop(
        ..img,
        center: point_flip(center),
        angle: 0.0 -. angle,
        image: flip(image, point_flip),
      )
  }
  |> fix_position
}

pub fn frame(img: Image) -> Image {
  color_frame(img, color.black)
}

pub fn color_frame(img: Image, color: Color) -> Image {
  overlay(img, rectangle(width(img), height(img), style.stroke(color)))
}

pub fn crop(
  img: Image,
  x: Float,
  y: Float,
  width: Float,
  height: Float,
) -> Image {
  let width = positive(width)
  let height = positive(height)
  Crop(
    Point(width /. 2.0, height /. 2.0),
    width,
    height,
    0.0,
    translate(img, 0.0 -. x, 0.0 -. y),
  )
}

pub fn crop_align(
  img: Image,
  x_place: XPlace,
  y_place: YPlace,
  crop_width: Float,
  crop_height: Float,
) -> Image {
  let crop_width = positive(crop_width)
  let crop_height = positive(crop_height)
  let #(_, dx) = x_place_dx(x_place, width(img), crop_width)
  let #(_, dy) = y_place_dy(y_place, height(img), crop_height)
  crop(img, dx, dy, crop_width, crop_height)
}

// **************************
// * Overlaying
// **************************

pub fn combine(images: List(Image), op: fn(Image, Image) -> Image) -> Image {
  list.fold(images, empty, op)
}

pub fn above(a: Image, b: Image) -> Image {
  above_align(Center, a, b)
}

pub fn above_align(x_place: XPlace, a: Image, b: Image) -> Image {
  let #(dxa, dxb) = x_place_dx(x_place, width(a), width(b))
  Combination(translate(a, dxa, 0.0), translate(b, dxb, height(a)))
}

pub fn beside(a: Image, b: Image) -> Image {
  beside_align(Middle, a, b)
}

pub fn beside_align(y_place: YPlace, a: Image, b: Image) -> Image {
  let #(dya, dyb) = y_place_dy(y_place, height(a), height(b))
  Combination(translate(a, 0.0, dya), translate(b, width(a), dyb))
}

pub fn overlay(top: Image, bottom: Image) -> Image {
  overlay_align(Center, Middle, top, bottom)
}

pub fn overlay_align(
  x_place: XPlace,
  y_place: YPlace,
  top: Image,
  bottom: Image,
) -> Image {
  let #(dxa, dxb) = x_place_dx(x_place, width(top), width(bottom))
  let #(dya, dyb) = y_place_dy(y_place, height(top), height(bottom))
  Combination(translate(bottom, dxb, dyb), translate(top, dxa, dya))
  |> fix_position
}

pub fn overlay_offset(top: Image, x: Float, y: Float, bottom: Image) -> Image {
  overlay(top, translate(bottom, x, y))
}

pub fn overlay_align_offset(
  x_place: XPlace,
  y_place: YPlace,
  top: Image,
  x: Float,
  y: Float,
  bottom: Image,
) -> Image {
  overlay_align(x_place, y_place, top, translate(bottom, x, y))
}

pub fn overlay_xy(top: Image, x: Float, y: Float, bottom: Image) -> Image {
  Combination(translate(bottom, x, y), top)
  |> fix_position
}

pub fn underlay(bottom: Image, top: Image) -> Image {
  overlay(top, bottom)
}

pub fn underlay_align(
  x_place: XPlace,
  y_place: YPlace,
  bottom: Image,
  top: Image,
) -> Image {
  overlay_align(x_place, y_place, top, bottom)
}

pub fn underlay_offset(bottom: Image, x: Float, y: Float, top: Image) -> Image {
  overlay(translate(top, x, y), bottom)
}

pub fn underlay_align_offset(
  x_place: XPlace,
  y_place: YPlace,
  bottom: Image,
  x: Float,
  y: Float,
  top: Image,
) -> Image {
  underlay_align(x_place, y_place, bottom, translate(top, x, y))
}

pub fn underlay_xy(bottom: Image, x: Float, y: Float, top: Image) -> Image {
  Combination(bottom, translate(top, x, y))
  |> fix_position
}

// **************************
// * Placing
// **************************

pub fn empty_scene(width: Float, height: Float) -> Image {
  empty_scene_color(width, height, color.black)
}

pub fn empty_scene_color(width: Float, height: Float, color: Color) -> Image {
  rectangle(width, height, style.stroke(color))
}

pub fn place_image(scene: Image, x: Float, y: Float, img: Image) -> Image {
  place_image_align(scene, x, y, Center, Middle, img)
}

pub fn place_image_align(
  scene: Image,
  x: Float,
  y: Float,
  x_place: XPlace,
  y_place: YPlace,
  img: Image,
) -> Image {
  let dx = case x_place {
    Center -> width(img) /. -2.0
    Left -> 0.0
    Right -> 0.0 -. width(img)
  }
  let dy = case y_place {
    Bottom -> 0.0 -. height(img)
    Middle -> height(img) /. -2.0
    Top -> 0.0
  }
  Combination(scene, translate(img, x +. dx, y +. dy))
  |> crop(0.0, 0.0, width(scene), height(scene))
  |> fix_position
}

pub fn place_line(
  scene: Image,
  x1: Float,
  y1: Float,
  x2: Float,
  y2: Float,
  style: Style,
) -> Image {
  Combination(scene, Polygon(style, [Point(x1, y1), Point(x2, y2)]))
  |> crop(0.0, 0.0, width(scene), height(scene))
  |> fix_position
}

pub fn place_polygon(scene: Image, points: List(Point), style: Style) -> Image {
  Combination(scene, Polygon(style, points))
  |> crop(0.0, 0.0, width(scene), height(scene))
  |> fix_position
}

pub fn put_image(scene: Image, x: Float, y: Float, img: Image) -> Image {
  place_image(scene, x, height(scene) -. y, img)
}

// **************************
// * SVG
// **************************

pub fn to_svg(img: Image) -> String {
  "<svg "
  <> attrib("width", width(img))
  <> attrib("height", height(img))
  <> "xmlns=\"http://www.w3.org/2000/svg\">\n"
  <> to_svg_(img, 1)
  <> "</svg>"
}

fn to_svg_(img: Image, level: Int) -> String {
  case img {
    Rectangle(style:, center:, width:, height:, angle:) -> {
      ident(level)
      <> "<rect "
      <> attrib("x", center.x -. width /. 2.0)
      <> attrib("y", center.y -. height /. 2.0)
      <> attrib("width", width)
      <> attrib("height", height)
      <> attribs("transform", rotate_str(angle, center))
      <> style.to_svg(style)
      <> "/>\n"
    }
    Ellipse(style:, center:, width:, height:, angle:) -> {
      ident(level)
      <> "<ellipse "
      <> attrib("cx", center.x)
      <> attrib("cy", center.y)
      <> attrib("rx", width)
      <> attrib("ry", height)
      <> attribs("transform", rotate_str(angle, center))
      <> style.to_svg(style)
      <> "/>\n"
    }
    Polygon(style:, points: [p1, p2]) -> {
      ident(level)
      <> "<line "
      <> attrib("x1", p1.x)
      <> attrib("y1", p1.y)
      <> attrib("x2", p2.x)
      <> attrib("y2", p2.y)
      <> style.to_svg(style)
      <> "/>\n"
    }
    Polygon(style:, points:) -> {
      let points =
        points
        |> list.map(fn(p) {
          float.to_string(p.x) <> "," <> float.to_string(p.y)
        })
        |> string.join(" ")
      ident(level)
      <> "<polygon "
      <> attribs("points", points)
      <> style.to_svg(style)
      <> "/>\n"
    }
    Combination(a, b) ->
      ident(level)
      <> "<g>\n"
      <> to_svg_(a, level + 1)
      <> to_svg_(b, level + 1)
      <> ident(level)
      <> "</g>\n"
    Crop(center:, width:, height:, angle:, image:) -> {
      let clipid = "clip" <> int.to_string(next_clip_id())
      let rect =
        "<rect "
        <> attrib("x", center.x -. width /. 2.0)
        <> attrib("y", center.y -. height /. 2.0)
        <> attrib("width", width)
        <> attrib("height", height)
        <> attribs("transform", rotate_str(angle, center))
        <> "/>"
      ident(level)
      <> "<defs>"
      <> "<clipPath "
      <> attribs("id", clipid)
      <> ">"
      <> rect
      <> "</clipPath>"
      <> "</defs>\n"
      <> ident(level)
      <> "<g "
      <> attribs("clip-path", "url(#" <> clipid <> ")")
      <> ">\n"
      <> to_svg_(image, level + 1)
      <> ident(level)
      <> "</g>\n"
    }
  }
}

fn rotate_str(angle: Float, center: Point) -> String {
  "rotate("
  <> float.to_string(angle)
  <> " "
  <> float.to_string(center.x)
  <> " "
  <> float.to_string(center.y)
  <> ")"
}

fn ident(level: Int) -> String {
  string.repeat(" ", 2 * level)
}

fn attrib(name: String, value: Float) -> String {
  name <> "=\"" <> float.to_string(value) <> "\" "
}

fn attribs(name: String, value: String) -> String {
  name <> "=\"" <> value <> "\" "
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "next_clip_id")
fn next_clip_id() -> Int
