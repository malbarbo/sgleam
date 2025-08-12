import gleam/float
import gleam/int
import gleam/list
import sgleam/color.{type Color}
import sgleam/imagef
import sgleam/style.{type Style}
import sgleam/xplace.{type XPlace}
import sgleam/yplace.{type YPlace}

pub type Image =
  imagef.Image

pub type Point {
  Point(x: Int, y: Int)
}

fn point_to_pointf(p: Point) -> imagef.Point {
  imagef.Point(int.to_float(p.x), int.to_float(p.y))
}

fn pointf_to_point(p: imagef.Point) -> Point {
  Point(float.round(p.x), float.round(p.y))
}

pub const empty = imagef.empty

pub fn width(img: Image) -> Int {
  img |> imagef.width |> float.round
}

pub fn height(img: Image) -> Int {
  img |> imagef.height |> float.round
}

pub fn dimension(img: Image) -> #(Int, Int) {
  let #(width, height) = imagef.dimension(img)
  #(float.round(width), float.round(height))
}

pub fn center(img: Image) -> Point {
  img |> imagef.center |> pointf_to_point
}

// **************************
// * Basic images
// **************************

pub fn rectangle(width: Int, height: Int, style: Style) -> Image {
  imagef.rectangle(int.to_float(width), int.to_float(height), style)
}

pub fn square(side: Int, style: Style) -> Image {
  imagef.square(int.to_float(side), style)
}

pub fn ellipse(width: Int, height: Int, style: Style) -> Image {
  imagef.ellipse(int.to_float(width), int.to_float(height), style)
}

pub fn circle(radius: Int, style: Style) -> Image {
  imagef.circle(int.to_float(radius), style)
}

pub fn line(x: Int, y: Int, style: Style) -> Image {
  imagef.line(int.to_float(x), int.to_float(y), style)
}

pub fn add_line(
  img: Image,
  x1: Int,
  y1: Int,
  x2: Int,
  y2: Int,
  style: Style,
) -> Image {
  imagef.add_line(
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

pub fn triangle(side: Int, style: Style) -> Image {
  imagef.triangle(int.to_float(side), style)
}

pub fn right_triangle(side1: Int, side2: Int, style: Style) -> Image {
  imagef.right_triangle(int.to_float(side1), int.to_float(side2), style)
}

pub fn isosceles_triangle(
  side_length: Float,
  angle: Float,
  style: Style,
) -> Image {
  imagef.isosceles_triangle(side_length, angle, style)
}

pub fn rhombus(side_length: Int, angle: Int, style: Style) -> Image {
  imagef.rhombus(int.to_float(side_length), int.to_float(angle), style)
}

pub fn regular_polygon(
  side_length: Float,
  side_count: Int,
  style: Style,
) -> Image {
  imagef.regular_polygon(side_length, side_count, style)
}

pub fn polygon(points: List(Point), style: Style) -> Image {
  imagef.polygon(list.map(points, point_to_pointf), style)
}

pub fn add_polygon(img: Image, points: List(Point), style: Style) -> Image {
  imagef.add_polygon(img, list.map(points, point_to_pointf), style)
}

pub fn star_polygon(
  side_length: Int,
  side_count: Int,
  step_count: Int,
  style: Style,
) -> Image {
  imagef.star_polygon(int.to_float(side_length), side_count, step_count, style)
}

pub fn star(side_length: Int, style: Style) -> Image {
  imagef.star(int.to_float(side_length), style)
}

pub fn radial_start(
  point_count: Int,
  inner_radius: Int,
  outer_radius: Int,
  style: Style,
) -> Image {
  imagef.radial_start(
    point_count,
    int.to_float(inner_radius),
    int.to_float(outer_radius),
    style,
  )
}

// **************************
// * Transformations
// **************************

pub fn rotate(img: Image, angle: Float) -> Image {
  imagef.rotate(img, angle)
}

pub fn scale(img: Image, factor: Float) -> Image {
  imagef.scale(img, factor)
}

pub fn scale_xy(img: Image, x_factor: Float, y_factor: Float) -> Image {
  imagef.scale_xy(img, x_factor, y_factor)
}

pub fn flip_horizontal(img: Image) -> Image {
  imagef.flip_horizontal(img)
}

pub fn flip_vertical(img: Image) -> Image {
  imagef.flip_vertical(img)
}

pub fn frame(img: Image) -> Image {
  imagef.frame(img)
}

pub fn color_frame(img: Image, color: Color) -> Image {
  imagef.color_frame(img, color)
}

pub fn crop(
  img: Image,
  x: Float,
  y: Float,
  width: Float,
  height: Float,
) -> Image {
  imagef.crop(img, x, y, width, height)
}

pub fn crop_align(
  img: Image,
  x_place: XPlace,
  y_place: YPlace,
  crop_width: Int,
  crop_height: Int,
) -> Image {
  imagef.crop_align(
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
  imagef.combine(images, op)
}

pub fn above(a: Image, b: Image) -> Image {
  imagef.above(a, b)
}

pub fn above_align(x_place: XPlace, a: Image, b: Image) -> Image {
  imagef.above_align(x_place, a, b)
}

pub fn beside(a: Image, b: Image) -> Image {
  imagef.beside(a, b)
}

pub fn beside_align(y_place: YPlace, a: Image, b: Image) -> Image {
  imagef.beside_align(y_place, a, b)
}

pub fn overlay(top: Image, bottom: Image) -> Image {
  imagef.overlay(top, bottom)
}

pub fn overlay_align(
  x_place: XPlace,
  y_place: YPlace,
  top: Image,
  bottom: Image,
) -> Image {
  imagef.overlay_align(x_place, y_place, top, bottom)
}

pub fn overlay_offset(top: Image, x: Int, y: Int, bottom: Image) -> Image {
  imagef.overlay_offset(top, int.to_float(x), int.to_float(y), bottom)
}

pub fn overlay_align_offset(
  x_place: XPlace,
  y_place: YPlace,
  top: Image,
  x: Int,
  y: Int,
  bottom: Image,
) -> Image {
  imagef.overlay_align_offset(
    x_place,
    y_place,
    top,
    int.to_float(x),
    int.to_float(y),
    bottom,
  )
}

pub fn overlay_xy(top: Image, x: Int, y: Int, bottom: Image) -> Image {
  imagef.overlay_xy(top, int.to_float(x), int.to_float(y), bottom)
}

pub fn underlay(bottom: Image, top: Image) -> Image {
  imagef.underlay(bottom, top)
}

pub fn underlay_align(
  x_place: XPlace,
  y_place: YPlace,
  bottom: Image,
  top: Image,
) -> Image {
  imagef.underlay_align(x_place, y_place, bottom, top)
}

pub fn underlay_offset(bottom: Image, x: Int, y: Int, top: Image) -> Image {
  imagef.underlay_offset(bottom, int.to_float(x), int.to_float(y), top)
}

pub fn underlay_align_offset(
  x_place: XPlace,
  y_place: YPlace,
  bottom: Image,
  x: Int,
  y: Int,
  top: Image,
) -> Image {
  imagef.underlay_align_offset(
    x_place,
    y_place,
    bottom,
    int.to_float(x),
    int.to_float(y),
    top,
  )
}

pub fn underlay_xy(bottom: Image, x: Int, y: Int, top: Image) -> Image {
  imagef.underlay_xy(bottom, int.to_float(x), int.to_float(y), top)
}

// **************************
// * Placing
// **************************

pub fn empty_scene(width: Int, height: Int) -> Image {
  imagef.empty_scene(int.to_float(width), int.to_float(height))
}

pub fn empty_scene_color(width: Int, height: Int, color: Color) -> Image {
  imagef.empty_scene_color(int.to_float(width), int.to_float(height), color)
}

pub fn place_image(scene: Image, x: Int, y: Int, img: Image) -> Image {
  imagef.place_image(scene, int.to_float(x), int.to_float(y), img)
}

pub fn place_image_align(
  scene: Image,
  x: Int,
  y: Int,
  x_place: XPlace,
  y_place: YPlace,
  img: Image,
) -> Image {
  imagef.place_image_align(
    scene,
    int.to_float(x),
    int.to_float(y),
    x_place,
    y_place,
    img,
  )
}

pub fn place_line(
  scene: Image,
  x1: Int,
  y1: Int,
  x2: Int,
  y2: Int,
  style: Style,
) -> Image {
  imagef.place_line(
    scene,
    int.to_float(x1),
    int.to_float(y1),
    int.to_float(x2),
    int.to_float(y2),
    style,
  )
}

pub fn place_polygon(scene: Image, points: List(Point), style: Style) -> Image {
  imagef.place_polygon(scene, list.map(points, point_to_pointf), style)
}

pub fn put_image(scene: Image, x: Int, y: Int, img: Image) -> Image {
  imagef.put_image(scene, int.to_float(x), int.to_float(y), img)
}

// **************************
// * SVG
// **************************

pub fn to_svg(img: Image) -> String {
  imagef.to_svg(img)
}
