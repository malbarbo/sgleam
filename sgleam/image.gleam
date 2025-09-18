import gleam/bool
import gleam/float
import gleam/int
import gleam/list
import gleam/string
import sgleam/color.{type Color}
import sgleam/font.{type Font, Font}
import sgleam/math.{cos_deg, hypot, sin_deg}
import sgleam/style.{type Style}
import sgleam/system
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

pub type Pointf {
  Pointf(x: Float, y: Float)
}

fn point_translate(p: Pointf, dx: Float, dy: Float) -> Pointf {
  Pointf(p.x +. dx, p.y +. dy)
}

fn point_rotate(p: Pointf, center: Pointf, angle: Float) -> Pointf {
  let dx = p.x -. center.x
  let dy = p.y -. center.y

  Pointf(
    center.x +. dx *. cos_deg(angle) -. dy *. sin_deg(angle),
    center.y +. dx *. sin_deg(angle) +. dy *. cos_deg(angle),
  )
}

fn point_flip_x(p: Pointf) -> Pointf {
  Pointf(0.0 -. p.x, p.y)
}

fn point_flip_y(p: Pointf) -> Pointf {
  Pointf(p.x, 0.0 -. p.y)
}

pub type Point {
  Point(x: Int, y: Int)
}

fn point_to_pointf(p: Point) -> Pointf {
  Pointf(int.to_float(p.x), int.to_float(p.y))
}

fn pointf_to_point(p: Pointf) -> Point {
  Point(float.round(p.x), float.round(p.y))
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
  Rectangle(style: Style, box: Box)
  Ellipse(style: Style, box: Box)
  Polygon(style: Style, points: List(Pointf))
  Combination(Image, Image)
  Crop(box: Box, image: Image)
  Text(
    style: Style,
    box: Box,
    text: String,
    flip_vertical: Bool,
    flip_horizontal: Bool,
    font: Font,
  )
}

type Box {
  Box(center: Pointf, width: Float, height: Float, angle: Float)
}

fn box_translate(box: Box, dx: Float, dy: Float) -> Box {
  Box(..box, center: point_translate(box.center, dx, dy))
}

fn box_box(box: Box) -> #(Pointf, Pointf) {
  let hw = box.width /. 2.0
  let hh = box.height /. 2.0
  let abs = float.absolute_value
  let dx = hw *. abs(cos_deg(box.angle)) +. hh *. abs(sin_deg(box.angle))
  let dy = hw *. abs(sin_deg(box.angle)) +. hh *. abs(cos_deg(box.angle))
  #(
    point_translate(box.center, 0.0 -. dx, 0.0 -. dy),
    point_translate(box.center, dx, dy),
  )
}

fn box_rotate(box: Box, center: Pointf, angle: Float) -> Box {
  Box(
    ..box,
    center: point_rotate(box.center, center, angle),
    angle: box.angle +. angle,
  )
}

fn box_scale(box: Box, x_factor: Float, y_factor: Float) -> Box {
  Box(..box, width: box.width *. x_factor, height: box.height *. y_factor)
}

fn box_flip(box: Box, point_flip: fn(Pointf) -> Pointf) -> Box {
  Box(..box, center: point_flip(box.center), angle: 0.0 -. box.angle)
}

pub const empty = Rectangle(style.none, Box(Pointf(0.0, 0.0), 0.0, 0.0, 0.0))

pub fn widthf(img: Image) -> Float {
  let #(min, max) = box(img)
  max.x -. min.x
}

pub fn width(img: Image) -> Int {
  img |> widthf |> float.round
}

pub fn heightf(img: Image) -> Float {
  let #(min, max) = box(img)
  max.y -. min.y
}

pub fn height(img: Image) -> Int {
  img |> heightf |> float.round
}

pub fn dimensionf(img: Image) -> #(Float, Float) {
  let #(min, max) = box(img)
  #(max.x -. min.x, max.y -. min.y)
}

pub fn dimension(img: Image) -> #(Int, Int) {
  let #(width, height) = dimensionf(img)
  #(float.round(width), float.round(height))
}

pub fn centerf(img: Image) -> Pointf {
  let #(min, max) = box(img)
  Pointf(mid(max.x, min.x), mid(max.y, min.y))
}

pub fn center(img: Image) -> Point {
  img |> centerf |> pointf_to_point
}

fn translate(img: Image, dx: Float, dy: Float) -> Image {
  use <- bool.guard(dx == 0.0 && dy == 0.0, img)
  case img {
    Rectangle(box:, ..) -> Rectangle(..img, box: box_translate(box, dx, dy))
    Ellipse(box:, ..) -> Ellipse(..img, box: box_translate(box, dx, dy))
    Polygon(points:, ..) ->
      Polygon(..img, points: list.map(points, point_translate(_, dx, dy)))
    Combination(a, b) -> Combination(translate(a, dx, dy), translate(b, dx, dy))
    Crop(box:, image:) ->
      Crop(box: box_translate(box, dx, dy), image: translate(image, dx, dy))
    Text(box:, ..) -> Text(..img, box: box_translate(box, dx, dy))
  }
}

fn fix_position(img: Image) -> Image {
  let #(min, _) = box(img)
  case min == Pointf(0.0, 0.0) {
    True -> img
    False -> translate(img, 0.0 -. min.x, 0.0 -. min.y)
  }
}

fn box(img: Image) -> #(Pointf, Pointf) {
  case img {
    Rectangle(box:, ..) -> box_box(box)
    Ellipse(box: Box(center:, width:, height:, angle:), ..) -> {
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
        Pointf(float.min(amin.x, bmin.x), float.min(amin.y, bmin.y)),
        Pointf(float.max(amax.x, bmax.x), float.max(amax.y, bmax.y)),
      )
    }
    Polygon(points:, ..) -> {
      let min_x = list.fold(points, 0.0, fn(min, p) { float.min(min, p.x) })
      let min_y = list.fold(points, 0.0, fn(min, p) { float.min(min, p.y) })
      let max_x = list.fold(points, 0.0, fn(max, p) { float.max(max, p.x) })
      let max_y = list.fold(points, 0.0, fn(max, p) { float.max(max, p.y) })
      #(Pointf(min_x, min_y), Pointf(max_x, max_y))
    }
    Crop(box:, ..) -> box_box(box)
    Text(box:, ..) -> box_box(box)
  }
}

// **************************
// * Basic images
// **************************

pub fn rectanglef(width: Float, height: Float, style: Style) -> Image {
  let width = positive(width)
  let height = positive(height)
  Rectangle(style, Box(Pointf(width /. 2.0, height /. 2.0), width, height, 0.0))
}

pub fn rectangle(width: Int, height: Int, style: Style) -> Image {
  rectanglef(int.to_float(width), int.to_float(height), style)
}

pub fn squaref(side: Float, style: Style) -> Image {
  rectanglef(side, side, style)
}

pub fn square(side: Int, style: Style) -> Image {
  squaref(int.to_float(side), style)
}

pub fn ellipsef(width: Float, height: Float, style: Style) -> Image {
  let hw = positive(width) /. 2.0
  let hh = positive(height) /. 2.0
  Ellipse(style, Box(Pointf(hw, hh), hw, hh, 0.0))
}

pub fn ellipse(width: Int, height: Int, style: Style) -> Image {
  ellipsef(int.to_float(width), int.to_float(height), style)
}

pub fn circlef(radius: Float, style: Style) -> Image {
  ellipsef(2.0 *. radius, 2.0 *. radius, style)
}

pub fn circle(radius: Int, style: Style) -> Image {
  circlef(int.to_float(radius), style)
}

pub fn linef(x: Float, y: Float, style: Style) -> Image {
  Polygon(style, [Pointf(0.0, 0.0), Pointf(x, y)])
  |> fix_position
}

pub fn line(x: Int, y: Int, style: Style) -> Image {
  linef(int.to_float(x), int.to_float(y), style)
}

pub fn add_linef(
  img: Image,
  x1: Float,
  y1: Float,
  x2: Float,
  y2: Float,
  style: Style,
) -> Image {
  Combination(img, Polygon(style, [Pointf(x1, y1), Pointf(x2, y2)]))
  |> fix_position
}

pub fn add_line(
  img: Image,
  x1: Int,
  y1: Int,
  x2: Int,
  y2: Int,
  style: Style,
) -> Image {
  add_linef(
    img,
    int.to_float(x1),
    int.to_float(y1),
    int.to_float(x2),
    int.to_float(y2),
    style,
  )
}

// **************************
// * Polygons
// **************************

pub fn trianglef(side: Float, style: Style) -> Image {
  let side = positive(side)
  // side *. sqrt(3.0) /. 2.0
  let height = side *. 0.8660254037844386
  Polygon(style, [
    Pointf(side /. 2.0, 0.0),
    Pointf(side, height),
    Pointf(0.0, height),
  ])
}

pub fn triangle(side: Int, style: Style) -> Image {
  trianglef(int.to_float(side), style)
}

pub fn right_trianglef(side1: Float, side2: Float, style: Style) -> Image {
  let side1 = positive(side1)
  let side2 = positive(side2)
  Polygon(style, [Pointf(0.0, 0.0), Pointf(0.0, side2), Pointf(side1, side2)])
}

pub fn right_triangle(side1: Int, side2: Int, style: Style) -> Image {
  right_trianglef(int.to_float(side1), int.to_float(side2), style)
}

pub fn isosceles_trianglef(
  side_length: Float,
  angle: Float,
  style: Style,
) -> Image {
  let side_length = positive(side_length)
  let hangle = angle /. 2.0
  Polygon(style, [
    Pointf(side_length *. sin_deg(hangle), side_length *. cos_deg(hangle)),
    Pointf(0.0, 0.0),
    Pointf(
      0.0 -. side_length *. sin_deg(hangle),
      side_length *. cos_deg(hangle),
    ),
  ])
  |> fix_position
}

pub fn isosceles_triangle(side_length: Int, angle: Int, style: Style) -> Image {
  isosceles_trianglef(int.to_float(side_length), int.to_float(angle), style)
}

pub fn rhombusf(side_length: Float, angle: Float, style: Style) -> Image {
  let side_length = positive(side_length)
  let height = 2.0 *. side_length *. cos_deg(angle /. 2.0)
  let width = 2.0 *. side_length *. sin_deg(angle /. 2.0)
  Polygon(style, [
    Pointf(0.0, height /. 2.0),
    Pointf(width /. 2.0, 0.0),
    Pointf(width, height /. 2.0),
    Pointf(width /. 2.0, height),
  ])
}

pub fn rhombus(side_length: Int, angle: Int, style: Style) -> Image {
  rhombusf(int.to_float(side_length), int.to_float(angle), style)
}

pub fn regular_polygonf(
  side_length: Float,
  side_count: Int,
  style: Style,
) -> Image {
  star_polygonf(side_length, side_count, 1, style)
}

pub fn regular_polygon(side_length: Int, side_count: Int, style: Style) -> Image {
  regular_polygonf(int.to_float(side_length), side_count, style)
}

pub fn polygonf(points: List(Pointf), style: Style) -> Image {
  Polygon(style, points) |> fix_position
}

pub fn polygon(points: List(Point), style: Style) -> Image {
  polygonf(list.map(points, point_to_pointf), style)
}

pub fn add_polygonf(img: Image, points: List(Pointf), style: Style) -> Image {
  Combination(img, Polygon(style, points)) |> fix_position
}

pub fn add_polygon(img: Image, points: List(Point), style: Style) -> Image {
  add_polygonf(img, list.map(points, point_to_pointf), style)
}

pub fn star_polygonf(
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
    Pointf(radius *. cos_deg(theta), radius *. sin_deg(theta))
  })
  |> Polygon(style, _)
  |> fix_position
}

pub fn star_polygon(
  side_length: Int,
  side_count: Int,
  step_count: Int,
  style: Style,
) -> Image {
  star_polygonf(int.to_float(side_length), side_count, step_count, style)
}

pub fn starf(side_length: Float, style: Style) -> Image {
  star_polygonf(side_length, 5, 2, style)
}

pub fn star(side_length: Int, style: Style) -> Image {
  starf(int.to_float(side_length), style)
}

pub fn radial_startf(
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
      Pointf(outer_radius *. cos_deg(theta1), outer_radius *. sin_deg(theta1)),
      Pointf(inner_radius *. cos_deg(theta2), inner_radius *. sin_deg(theta2)),
    ]
  })
  |> Polygon(style, _)
  |> fix_position
}

pub fn radial_start(
  point_count: Int,
  inner_radius: Int,
  outer_radius: Int,
  style: Style,
) -> Image {
  radial_startf(
    point_count,
    int.to_float(inner_radius),
    int.to_float(outer_radius),
    style,
  )
}

fn positive(n: Float) -> Float {
  float.max(0.0, n)
}

// **************************
// * Text
// **************************

pub fn text_fontf(text: String, font: Font, style: Style) -> Image {
  let width = system.text_width(text, font.family, font.size)
  let height = system.text_height(text, font.family, font.size)
  Text(
    style,
    Box(Pointf(width /. 2.0, height /. 2.0), width, height, 0.0),
    text,
    False,
    False,
    font,
  )
}

pub fn textf(text: String, size: Float, style: Style) -> Image {
  text_fontf(text, Font(..font.default(), size: size), style)
}

pub fn text_font(text: String, font: Font, style: Style) -> Image {
  text_fontf(text, font, style)
}

pub fn text(text: String, size: Int, style: Style) -> Image {
  textf(text, int.to_float(size), style)
}

// **************************
// * Transformations
// **************************

pub fn rotatef(img: Image, angle: Float) -> Image {
  // the api for the user is counter clockwise, but the implementation is clockwise
  rotate_around(img, centerf(img), 0.0 -. angle)
  |> fix_position
}

pub fn rotate(img: Image, angle: Int) -> Image {
  rotatef(img, int.to_float(angle))
}

fn rotate_around(img: Image, center: Pointf, angle: Float) -> Image {
  case img {
    Rectangle(box:, ..) -> Rectangle(..img, box: box_rotate(box, center, angle))
    Ellipse(box:, ..) -> Ellipse(..img, box: box_rotate(box, center, angle))
    Polygon(points:, ..) ->
      Polygon(..img, points: list.map(points, point_rotate(_, center, angle)))
    Combination(a, b) ->
      Combination(
        rotate_around(a, center, angle),
        rotate_around(b, center, angle),
      )
    Crop(box:, image:) ->
      Crop(
        box: box_rotate(box, center, angle),
        image: rotate_around(image, center, angle),
      )
    Text(box:, ..) -> Text(..img, box: box_rotate(box, center, angle))
  }
}

pub fn scalef(img: Image, factor: Float) -> Image {
  scale_xyf(img, factor, factor)
}

pub fn scale(img: Image, factor: Int) -> Image {
  scalef(img, int.to_float(factor))
}

pub fn scale_xyf(img: Image, x_factor: Float, y_factor: Float) -> Image {
  let x_factor = positive(x_factor)
  let y_factor = positive(y_factor)
  case img {
    Rectangle(box:, ..) ->
      Rectangle(..img, box: box_scale(box, x_factor, y_factor))
    Ellipse(box:, ..) -> Ellipse(..img, box: box_scale(box, x_factor, y_factor))
    Polygon(points:, ..) ->
      Polygon(
        ..img,
        points: list.map(points, fn(p) {
          Pointf(p.x *. x_factor, p.y *. y_factor)
        }),
      )
    Combination(a, b) ->
      Combination(
        scale_xyf(a, x_factor, y_factor),
        scale_xyf(b, x_factor, y_factor),
      )
    Crop(box:, image:) ->
      Crop(
        box: box_scale(box, x_factor, y_factor),
        image: scale_xyf(image, x_factor, y_factor),
      )
    Text(box:, ..) -> Text(..img, box: box_scale(box, x_factor, y_factor))
  }
  |> fix_position
}

pub fn scale_xy(img: Image, x_factor: Int, y_factor: Int) -> Image {
  scale_xyf(img, int.to_float(x_factor), int.to_float(y_factor))
}

pub fn flip_horizontal(img: Image) -> Image {
  flip(img, point_flip_x, True, False) |> fix_position
}

pub fn flip_vertical(img: Image) -> Image {
  flip(img, point_flip_y, False, True) |> fix_position
}

fn flip(
  img: Image,
  point_flip: fn(Pointf) -> Pointf,
  flip_horizontal: Bool,
  flip_vertical: Bool,
) -> Image {
  case img {
    Rectangle(box:, ..) -> Rectangle(..img, box: box_flip(box, point_flip))
    Ellipse(box:, ..) -> Ellipse(..img, box: box_flip(box, point_flip))
    Polygon(points:, ..) -> Polygon(..img, points: list.map(points, point_flip))
    Combination(a, b) ->
      Combination(
        flip(a, point_flip, flip_horizontal, flip_vertical),
        flip(b, point_flip, flip_horizontal, flip_vertical),
      )
    Crop(box:, image:) ->
      Crop(
        box: box_flip(box, point_flip),
        image: flip(image, point_flip, flip_horizontal, flip_vertical),
      )
    Text(box:, ..) ->
      Text(
        ..img,
        box: box_flip(box, point_flip),
        flip_horizontal: case flip_horizontal {
          True -> !img.flip_horizontal
          False -> img.flip_horizontal
        },
        flip_vertical: case flip_vertical {
          True -> !img.flip_vertical
          False -> img.flip_vertical
        },
      )
  }
}

pub fn frame(img: Image) -> Image {
  color_frame(img, color.black)
}

pub fn color_frame(img: Image, color: Color) -> Image {
  overlay(img, rectanglef(widthf(img), heightf(img), style.stroke(color)))
}

pub fn cropf(
  img: Image,
  x: Float,
  y: Float,
  width: Float,
  height: Float,
) -> Image {
  let width = positive(width)
  let height = positive(height)
  Crop(
    Box(Pointf(width /. 2.0, height /. 2.0), width, height, 0.0),
    translate(img, 0.0 -. x, 0.0 -. y),
  )
}

pub fn crop(img: Image, x: Int, y: Int, width: Int, height: Int) -> Image {
  cropf(
    img,
    int.to_float(x),
    int.to_float(y),
    int.to_float(width),
    int.to_float(height),
  )
}

pub fn crop_alignf(
  img: Image,
  x_place: XPlace,
  y_place: YPlace,
  crop_width: Float,
  crop_height: Float,
) -> Image {
  let crop_width = positive(crop_width)
  let crop_height = positive(crop_height)
  let #(_, dx) = x_place_dx(x_place, widthf(img), crop_width)
  let #(_, dy) = y_place_dy(y_place, heightf(img), crop_height)
  cropf(img, dx, dy, crop_width, crop_height)
}

pub fn crop_align(
  img: Image,
  x_place: XPlace,
  y_place: YPlace,
  crop_width: Int,
  crop_height: Int,
) -> Image {
  crop_alignf(
    img,
    x_place,
    y_place,
    int.to_float(crop_width),
    int.to_float(crop_height),
  )
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
  let #(dxa, dxb) = x_place_dx(x_place, widthf(a), widthf(b))
  Combination(translate(a, dxa, 0.0), translate(b, dxb, heightf(a)))
}

pub fn beside(a: Image, b: Image) -> Image {
  beside_align(Middle, a, b)
}

pub fn beside_align(y_place: YPlace, a: Image, b: Image) -> Image {
  let #(dya, dyb) = y_place_dy(y_place, heightf(a), heightf(b))
  Combination(translate(a, 0.0, dya), translate(b, widthf(a), dyb))
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
  let #(dxa, dxb) = x_place_dx(x_place, widthf(top), widthf(bottom))
  let #(dya, dyb) = y_place_dy(y_place, heightf(top), heightf(bottom))
  Combination(translate(bottom, dxb, dyb), translate(top, dxa, dya))
  |> fix_position
}

pub fn overlay_offsetf(top: Image, x: Float, y: Float, bottom: Image) -> Image {
  overlay(top, translate(bottom, x, y))
}

pub fn overlay_offset(top: Image, x: Int, y: Int, bottom: Image) -> Image {
  overlay_offsetf(top, int.to_float(x), int.to_float(y), bottom)
}

pub fn overlay_align_offsetf(
  x_place: XPlace,
  y_place: YPlace,
  top: Image,
  x: Float,
  y: Float,
  bottom: Image,
) -> Image {
  overlay_align(x_place, y_place, top, translate(bottom, x, y))
}

pub fn overlay_align_offset(
  x_place: XPlace,
  y_place: YPlace,
  top: Image,
  x: Int,
  y: Int,
  bottom: Image,
) -> Image {
  overlay_align_offsetf(
    x_place,
    y_place,
    top,
    int.to_float(x),
    int.to_float(y),
    bottom,
  )
}

pub fn overlay_xyf(top: Image, x: Float, y: Float, bottom: Image) -> Image {
  Combination(translate(bottom, x, y), top)
  |> fix_position
}

pub fn overlay_xy(top: Image, x: Int, y: Int, bottom: Image) -> Image {
  overlay_xyf(top, int.to_float(x), int.to_float(y), bottom)
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

pub fn underlay_offsetf(bottom: Image, x: Float, y: Float, top: Image) -> Image {
  overlay(translate(top, x, y), bottom)
}

pub fn underlay_offset(bottom: Image, x: Int, y: Int, top: Image) -> Image {
  underlay_offsetf(bottom, int.to_float(x), int.to_float(y), top)
}

pub fn underlay_align_offsetf(
  x_place: XPlace,
  y_place: YPlace,
  bottom: Image,
  x: Float,
  y: Float,
  top: Image,
) -> Image {
  underlay_align(x_place, y_place, bottom, translate(top, x, y))
}

pub fn underlay_align_offset(
  x_place: XPlace,
  y_place: YPlace,
  bottom: Image,
  x: Int,
  y: Int,
  top: Image,
) -> Image {
  underlay_align_offsetf(
    x_place,
    y_place,
    bottom,
    int.to_float(x),
    int.to_float(y),
    top,
  )
}

pub fn underlay_xyf(bottom: Image, x: Float, y: Float, top: Image) -> Image {
  Combination(bottom, translate(top, x, y))
  |> fix_position
}

pub fn underlay_xy(bottom: Image, x: Int, y: Int, top: Image) -> Image {
  underlay_xyf(bottom, int.to_float(x), int.to_float(y), top)
}

// **************************
// * Placing
// **************************

pub fn empty_scenef(width: Float, height: Float) -> Image {
  empty_scene_colorf(width, height, color.black)
}

pub fn empty_scene(width: Int, height: Int) -> Image {
  empty_scenef(int.to_float(width), int.to_float(height))
}

pub fn empty_scene_colorf(width: Float, height: Float, color: Color) -> Image {
  rectanglef(width, height, style.stroke(color))
}

pub fn empty_scene_color(width: Int, height: Int, color: Color) -> Image {
  empty_scene_colorf(int.to_float(width), int.to_float(height), color)
}

pub fn place_imagef(scene: Image, x: Float, y: Float, img: Image) -> Image {
  place_image_alignf(scene, x, y, Center, Middle, img)
}

pub fn place_image(scene: Image, x: Int, y: Int, img: Image) -> Image {
  place_imagef(scene, int.to_float(x), int.to_float(y), img)
}

pub fn place_image_alignf(
  scene: Image,
  x: Float,
  y: Float,
  x_place: XPlace,
  y_place: YPlace,
  img: Image,
) -> Image {
  let dx = case x_place {
    Center -> widthf(img) /. -2.0
    Left -> 0.0
    Right -> 0.0 -. widthf(img)
  }
  let dy = case y_place {
    Bottom -> 0.0 -. heightf(img)
    Middle -> heightf(img) /. -2.0
    Top -> 0.0
  }
  Combination(scene, translate(img, x +. dx, y +. dy))
  |> cropf(0.0, 0.0, widthf(scene), heightf(scene))
  |> fix_position
}

pub fn place_image_align(
  scene: Image,
  x: Int,
  y: Int,
  x_place: XPlace,
  y_place: YPlace,
  img: Image,
) -> Image {
  place_image_alignf(
    scene,
    int.to_float(x),
    int.to_float(y),
    x_place,
    y_place,
    img,
  )
}

pub fn place_linef(
  scene: Image,
  x1: Float,
  y1: Float,
  x2: Float,
  y2: Float,
  style: Style,
) -> Image {
  Combination(scene, Polygon(style, [Pointf(x1, y1), Pointf(x2, y2)]))
  |> cropf(0.0, 0.0, widthf(scene), heightf(scene))
  |> fix_position
}

pub fn place_line(
  scene: Image,
  x1: Int,
  y1: Int,
  x2: Int,
  y2: Int,
  style: Style,
) -> Image {
  place_linef(
    scene,
    int.to_float(x1),
    int.to_float(y1),
    int.to_float(x2),
    int.to_float(y2),
    style,
  )
}

pub fn place_polygonf(scene: Image, points: List(Pointf), style: Style) -> Image {
  Combination(scene, Polygon(style, points))
  |> cropf(0.0, 0.0, widthf(scene), heightf(scene))
  |> fix_position
}

pub fn place_polygon(scene: Image, points: List(Point), style: Style) -> Image {
  place_polygonf(scene, list.map(points, point_to_pointf), style)
}

pub fn put_imagef(scene: Image, x: Float, y: Float, img: Image) -> Image {
  place_imagef(scene, x, heightf(scene) -. y, img)
}

pub fn put_image(scene: Image, x: Int, y: Int, img: Image) -> Image {
  put_imagef(scene, int.to_float(x), int.to_float(y), img)
}

// **************************
// * SVG
// **************************

pub fn to_svg(img: Image) -> String {
  "<svg "
  <> attrib("width", widthf(img))
  <> attrib("height", heightf(img))
  <> "xmlns=\"http://www.w3.org/2000/svg\">\n"
  <> to_svg_(img, 1)
  <> "</svg>"
}

fn to_svg_(img: Image, level: Int) -> String {
  case img {
    Rectangle(style:, box: Box(center:, width:, height:, angle:)) -> {
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
    Ellipse(style:, box: Box(center:, width:, height:, angle:)) -> {
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
    Crop(box: Box(center:, width:, height:, angle:), image:) -> {
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
    Text(
      style:,
      box: Box(center:, width:, height:, angle:),
      text:,
      flip_horizontal:,
      flip_vertical:,
      font:,
    ) -> {
      let original_width = system.text_width(text, font.family, font.size)
      let original_height = system.text_height(text, font.family, font.size)
      let scale_x =
        width
        /. original_width
        *. {
          case flip_horizontal {
            True -> -1.0
            False -> 1.0
          }
        }
      let scale_y =
        height
        /. original_height
        *. {
          case flip_vertical {
            True -> -1.0
            False -> 1.0
          }
        }
      ident(level)
      <> "<text "
      <> attribs("dominant-baseline", "middle")
      <> attribs("text-anchor", "middle")
      <> attrib("x", 0.0)
      <> attrib("y", 0.0)
      <> attribs("font-family", font.family)
      <> attrib("font-size", font.size)
      <> attribs(
        "transform",
        translate_str(center.x, center.y)
          <> " "
          <> rotate_str(angle, Pointf(0.0, 0.0))
          <> " "
          <> scale_str(scale_x, scale_y),
      )
      <> style.to_svg(style)
      <> ">"
      <> text
      <> "</text>\n"
    }
  }
}

fn rotate_str(angle: Float, center: Pointf) -> String {
  "rotate("
  <> float.to_string(angle)
  <> " "
  <> float.to_string(center.x)
  <> " "
  <> float.to_string(center.y)
  <> ")"
}

fn scale_str(scale_x: Float, scale_y: Float) -> String {
  "scale(" <> float.to_string(scale_x) <> "," <> float.to_string(scale_y) <> ")"
}

fn translate_str(x: Float, y: Float) -> String {
  "translate(" <> float.to_string(x) <> "," <> float.to_string(y) <> ")"
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
