import gleam/bool
import gleam/float
import gleam/int
import gleam/list
import sgleam/color.{type Color}
import sgleam/font.{type Font, Font}
import sgleam/math.{atan2, cos, cos_deg, pi, sin, sin_deg, sqrt}
import sgleam/style.{type Style}
import sgleam/system
import sgleam/xplace.{type XPlace, Center, Left, Right}
import sgleam/yplace.{type YPlace, Bottom, Middle, Top}

// TODO: freeze
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
// * Utilities
// **************************

fn positive(n: Float) -> Float {
  float.max(0.0, n)
}

fn mid(a: Float, b: Float) -> Float {
  { a -. b } /. 2.0
}

// **************************
// * PathCmd
// **************************

pub opaque type PathCmd {
  MoveTo(Pointf)
  LineTo(Pointf)
  QuadTo(control: Pointf, end: Pointf)
  CubicTo(c1: Pointf, c2: Pointf, end: Pointf)
  ArcTo(
    rx: Float,
    ry: Float,
    rotation: Float,
    large_arc: Bool,
    sweep: Bool,
    end: Pointf,
  )
}

pub fn move_to(x: Float, y: Float) -> PathCmd {
  MoveTo(Pointf(x, y))
}

pub fn line_to(x: Float, y: Float) -> PathCmd {
  LineTo(Pointf(x, y))
}

pub fn quad_to(cx: Float, cy: Float, x: Float, y: Float) -> PathCmd {
  QuadTo(Pointf(cx, cy), Pointf(x, y))
}

pub fn cubic_to(
  c1x: Float,
  c1y: Float,
  c2x: Float,
  c2y: Float,
  x: Float,
  y: Float,
) -> PathCmd {
  CubicTo(Pointf(c1x, c1y), Pointf(c2x, c2y), Pointf(x, y))
}

pub fn arc_to(
  rx: Float,
  ry: Float,
  rotation: Float,
  large_arc: Bool,
  sweep: Bool,
  x: Float,
  y: Float,
) -> PathCmd {
  ArcTo(rx, ry, rotation, large_arc, sweep, Pointf(x, y))
}

pub fn pathf(commands: List(PathCmd), closed: Bool, style: Style) -> Image {
  Path(style, commands, closed) |> fix_position
}

// **************************
// * Image
// **************************

pub opaque type Image {
  Path(style: Style, commands: List(PathCmd), closed: Bool)
  Combination(Image, Image)
  Crop(box: Box, image: Image)
  Text(style: Style, box: Box, text: String, font: Font)
  Bitmap(box: Box, image: system.Loaded)
}

/// The image of a rectangle under an affine map. `x_axis` goes from the
/// middle of the left side to the middle of the right side, and `y_axis`
/// from the middle of the top side to the middle of the bottom side. A flip
/// makes the two axes a mirror of a rotation.
type Box {
  Box(center: Pointf, x_axis: Pointf, y_axis: Pointf)
}

/// One image of each variant. The constructors of an opaque type stay out of
/// the exports of the module, so the FFI takes the classes from these.
@internal
pub fn variants() -> List(Image) {
  let box = box_of_size(0.0, 0.0)
  [
    empty,
    Combination(empty, empty),
    Crop(box, empty),
    Text(style.none, box, "", font.default()),
    Bitmap(box, system.load_image("")),
  ]
}

fn box_of_size(width: Float, height: Float) -> Box {
  Box(
    Pointf(width /. 2.0, height /. 2.0),
    Pointf(width, 0.0),
    Pointf(0.0, height),
  )
}

fn box_translate(box: Box, dx: Float, dy: Float) -> Box {
  Box(..box, center: point_translate(box.center, dx, dy))
}

fn box_box(box: Box) -> #(Pointf, Pointf) {
  let abs = float.absolute_value
  let dx = { abs(box.x_axis.x) +. abs(box.y_axis.x) } /. 2.0
  let dy = { abs(box.x_axis.y) +. abs(box.y_axis.y) } /. 2.0
  #(
    point_translate(box.center, 0.0 -. dx, 0.0 -. dy),
    point_translate(box.center, dx, dy),
  )
}

fn box_rotate(box: Box, center: Pointf, angle: Float) -> Box {
  let origin = Pointf(0.0, 0.0)
  Box(
    point_rotate(box.center, center, angle),
    point_rotate(box.x_axis, origin, angle),
    point_rotate(box.y_axis, origin, angle),
  )
}

fn box_scale(box: Box, x_factor: Float, y_factor: Float) -> Box {
  Box(
    Pointf(box.center.x *. x_factor, box.center.y *. y_factor),
    Pointf(box.x_axis.x *. x_factor, box.x_axis.y *. y_factor),
    Pointf(box.y_axis.x *. x_factor, box.y_axis.y *. y_factor),
  )
}

fn box_flip(box: Box, point_flip: fn(Pointf) -> Pointf) -> Box {
  Box(point_flip(box.center), point_flip(box.x_axis), point_flip(box.y_axis))
}

pub const empty = Path(style.none, [], True)

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
    Path(style:, commands:, closed:) ->
      Path(style, list.map(commands, cmd_translate(_, dx, dy)), closed)
    Combination(a, b) -> Combination(translate(a, dx, dy), translate(b, dx, dy))
    Crop(box:, image:) ->
      Crop(box: box_translate(box, dx, dy), image: translate(image, dx, dy))
    Text(box:, ..) -> Text(..img, box: box_translate(box, dx, dy))
    Bitmap(box:, ..) -> Bitmap(..img, box: box_translate(box, dx, dy))
  }
}

fn fix_position(img: Image) -> Image {
  let #(min, _) = box(img)
  case min == Pointf(0.0, 0.0) {
    True -> img
    False -> translate(img, 0.0 -. min.x, 0.0 -. min.y)
  }
}

// **************************
// * Bounding box
// **************************

fn box(img: Image) -> #(Pointf, Pointf) {
  case drawn_box(img) {
    Ok(box) -> box
    Error(Nil) -> #(Pointf(0.0, 0.0), Pointf(0.0, 0.0))
  }
}

/// The box of what `img` draws, or `Error(Nil)` when `img` holds no path
/// with commands. A part with nothing in it adds nothing to the box of a
/// combination, wherever a translation left it.
fn drawn_box(img: Image) -> Result(#(Pointf, Pointf), Nil) {
  case img {
    Path(commands: [], ..) -> Error(Nil)
    Path(commands:, ..) -> {
      let #(min_x, min_y, max_x, max_y) = commands_box(commands)
      Ok(#(Pointf(min_x, min_y), Pointf(max_x, max_y)))
    }
    Combination(a, b) ->
      case drawn_box(a), drawn_box(b) {
        Ok(#(amin, amax)), Ok(#(bmin, bmax)) ->
          Ok(#(
            Pointf(float.min(amin.x, bmin.x), float.min(amin.y, bmin.y)),
            Pointf(float.max(amax.x, bmax.x), float.max(amax.y, bmax.y)),
          ))
        Ok(box), Error(Nil) | Error(Nil), Ok(box) -> Ok(box)
        Error(Nil), Error(Nil) -> Error(Nil)
      }
    Crop(box:, ..) -> Ok(box_box(box))
    Text(box:, ..) -> Ok(box_box(box))
    Bitmap(box:, ..) -> Ok(box_box(box))
  }
}

/// The box of the path of `commands`, as sinteract draws it.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "path_box")
fn commands_box(commands: List(PathCmd)) -> #(Float, Float, Float, Float)

// **************************
// * PathCmd transforms
// **************************

fn cmd_translate(cmd: PathCmd, dx: Float, dy: Float) -> PathCmd {
  case cmd {
    MoveTo(p) -> MoveTo(point_translate(p, dx, dy))
    LineTo(p) -> LineTo(point_translate(p, dx, dy))
    QuadTo(c, e) ->
      QuadTo(point_translate(c, dx, dy), point_translate(e, dx, dy))
    CubicTo(c1, c2, e) ->
      CubicTo(
        point_translate(c1, dx, dy),
        point_translate(c2, dx, dy),
        point_translate(e, dx, dy),
      )
    ArcTo(rx, ry, rot, la, sw, e) ->
      ArcTo(rx, ry, rot, la, sw, point_translate(e, dx, dy))
  }
}

fn cmd_rotate(cmd: PathCmd, center: Pointf, angle: Float) -> PathCmd {
  case cmd {
    MoveTo(p) -> MoveTo(point_rotate(p, center, angle))
    LineTo(p) -> LineTo(point_rotate(p, center, angle))
    QuadTo(c, e) ->
      QuadTo(point_rotate(c, center, angle), point_rotate(e, center, angle))
    CubicTo(c1, c2, e) ->
      CubicTo(
        point_rotate(c1, center, angle),
        point_rotate(c2, center, angle),
        point_rotate(e, center, angle),
      )
    ArcTo(rx, ry, rot, la, sw, e) ->
      ArcTo(rx, ry, rot +. angle, la, sw, point_rotate(e, center, angle))
  }
}

fn cmd_scale(cmd: PathCmd, x_factor: Float, y_factor: Float) -> PathCmd {
  let sp = fn(p: Pointf) { Pointf(p.x *. x_factor, p.y *. y_factor) }
  case cmd {
    MoveTo(p) -> MoveTo(sp(p))
    LineTo(p) -> LineTo(sp(p))
    QuadTo(c, e) -> QuadTo(sp(c), sp(e))
    CubicTo(c1, c2, e) -> CubicTo(sp(c1), sp(c2), sp(e))
    ArcTo(rx, ry, rot, la, sw, e) -> {
      let #(rx, ry, rot) = arc_scale(rx, ry, rot, x_factor, y_factor)
      ArcTo(rx, ry, rot, la, sw, sp(e))
    }
  }
}

fn arc_scale(
  rx: Float,
  ry: Float,
  rotation: Float,
  x_factor: Float,
  y_factor: Float,
) -> #(Float, Float, Float) {
  // A rotated ellipse scaled along x and y is an ellipse with other axes.
  // The axes are the eigenvectors of A = M M^T, where M takes the unit
  // circle to the scaled ellipse.
  let ux = rx *. cos_deg(rotation) *. x_factor
  let uy = rx *. sin_deg(rotation) *. y_factor
  let vx = 0.0 -. ry *. sin_deg(rotation) *. x_factor
  let vy = ry *. cos_deg(rotation) *. y_factor
  let a = ux *. ux +. vx *. vx
  let b = ux *. uy +. vx *. vy
  let d = uy *. uy +. vy *. vy
  let new_rotation = 0.5 *. atan2(2.0 *. b, a -. d) *. 180.0 /. pi
  // Of the four axis directions, take the one closest to the old rotation,
  // so an arc that is not rotated stays that way.
  let turns = float.round({ rotation -. new_rotation } /. 90.0)
  let new_rotation = new_rotation +. 90.0 *. int.to_float(turns)
  let radius = fn(angle) {
    let c = cos_deg(angle)
    let s = sin_deg(angle)
    sqrt(float.max(a *. c *. c +. 2.0 *. b *. s *. c +. d *. s *. s, 0.0))
  }
  #(radius(new_rotation), radius(new_rotation +. 90.0), new_rotation)
}

fn cmd_flip(cmd: PathCmd, pf: fn(Pointf) -> Pointf) -> PathCmd {
  case cmd {
    MoveTo(p) -> MoveTo(pf(p))
    LineTo(p) -> LineTo(pf(p))
    QuadTo(c, e) -> QuadTo(pf(c), pf(e))
    CubicTo(c1, c2, e) -> CubicTo(pf(c1), pf(c2), pf(e))
    ArcTo(rx, ry, rot, la, sw, e) -> ArcTo(rx, ry, 0.0 -. rot, la, !sw, pf(e))
  }
}

fn points_to_path(points: List(Pointf), style: Style) -> Image {
  case points {
    [] -> Path(style, [], False)
    [p] -> Path(style, [MoveTo(p)], False)
    [first, second] -> Path(style, [MoveTo(first), LineTo(second)], False)
    [first, ..rest] ->
      Path(style, [MoveTo(first), ..list.map(rest, fn(p) { LineTo(p) })], True)
  }
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

// **************************
// * Basic images
// **************************

pub fn rectanglef(width: Float, height: Float, style: Style) -> Image {
  let width = positive(width)
  let height = positive(height)
  Path(
    style,
    [
      MoveTo(Pointf(0.0, 0.0)),
      LineTo(Pointf(width, 0.0)),
      LineTo(Pointf(width, height)),
      LineTo(Pointf(0.0, height)),
    ],
    True,
  )
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
  let w = positive(width)
  let h = positive(height)
  let rx = w /. 2.0
  let ry = h /. 2.0
  Path(
    style,
    [
      MoveTo(Pointf(w, ry)),
      ArcTo(rx, ry, 0.0, False, True, Pointf(0.0, ry)),
      ArcTo(rx, ry, 0.0, False, True, Pointf(w, ry)),
    ],
    True,
  )
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
  Path(style, [MoveTo(Pointf(0.0, 0.0)), LineTo(Pointf(x, y))], False)
  |> fix_position
}

pub fn line(x: Int, y: Int, style: Style) -> Image {
  linef(int.to_float(x), int.to_float(y), style)
}

// **************************
// * Polygons
// **************************

pub fn trianglef(side: Float, style: Style) -> Image {
  let side = positive(side)
  // side *. sqrt(3.0) /. 2.0
  let height = side *. 0.8660254037844386
  points_to_path(
    [Pointf(side /. 2.0, 0.0), Pointf(side, height), Pointf(0.0, height)],
    style,
  )
}

pub fn triangle(side: Int, style: Style) -> Image {
  trianglef(int.to_float(side), style)
}

pub fn right_trianglef(side1: Float, side2: Float, style: Style) -> Image {
  let side1 = positive(side1)
  let side2 = positive(side2)
  points_to_path(
    [Pointf(0.0, 0.0), Pointf(0.0, side2), Pointf(side1, side2)],
    style,
  )
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
  points_to_path(
    [
      Pointf(side_length *. sin_deg(hangle), side_length *. cos_deg(hangle)),
      Pointf(0.0, 0.0),
      Pointf(
        0.0 -. side_length *. sin_deg(hangle),
        side_length *. cos_deg(hangle),
      ),
    ],
    style,
  )
  |> fix_position
}

pub fn isosceles_triangle(side_length: Int, angle: Int, style: Style) -> Image {
  isosceles_trianglef(int.to_float(side_length), int.to_float(angle), style)
}

// Triangle by sides and angles. Uses HtDP vertex convention:
// A=top-left, B=top-right, C=bottom. Side a opposite A, etc.
// Side c (A→B) is placed horizontally at the top.
fn triangle_from_sides_angle(
  side_b: Float,
  side_c: Float,
  angle_a: Float,
  style: Style,
) -> Image {
  // Place A at origin, B at (side_c, 0)
  // C is at angle_a from A, distance side_b from A
  let cx = side_b *. cos_deg(angle_a)
  let cy = side_b *. sin_deg(angle_a)
  points_to_path([Pointf(0.0, 0.0), Pointf(side_c, 0.0), Pointf(cx, cy)], style)
  |> fix_position
}

// Solve triangle from two sides and included angle using law of cosines
fn solve_side(a: Float, b: Float, angle_c: Float) -> Float {
  sqrt(a *. a +. b *. b -. 2.0 *. a *. b *. cos_deg(angle_c))
}

// Solve angle from three sides using law of cosines
fn solve_angle(opposite: Float, adj1: Float, adj2: Float) -> Float {
  let cos_val =
    float.clamp(
      { adj1 *. adj1 +. adj2 *. adj2 -. opposite *. opposite }
        /. { 2.0 *. adj1 *. adj2 },
      -1.0,
      1.0,
    )
  // acos in degrees
  atan2(sqrt(1.0 -. cos_val *. cos_val), cos_val) *. 180.0 /. pi
}

/// Triangle by three side lengths.
pub fn triangle_sssf(
  side_a: Float,
  side_b: Float,
  side_c: Float,
  style: Style,
) -> Image {
  let side_a = positive(side_a)
  let side_b = positive(side_b)
  let side_c = positive(side_c)
  let angle_a = solve_angle(side_a, side_b, side_c)
  triangle_from_sides_angle(side_b, side_c, angle_a, style)
}

pub fn triangle_sss(
  side_a: Int,
  side_b: Int,
  side_c: Int,
  style: Style,
) -> Image {
  triangle_sssf(
    int.to_float(side_a),
    int.to_float(side_b),
    int.to_float(side_c),
    style,
  )
}

/// Triangle by side-angle-side (side a, angle B, side c).
pub fn triangle_sasf(
  side_a: Float,
  angle_b: Float,
  side_c: Float,
  style: Style,
) -> Image {
  let side_a = positive(side_a)
  let side_c = positive(side_c)
  let side_b = solve_side(side_a, side_c, angle_b)
  let angle_a = solve_angle(side_a, side_b, side_c)
  triangle_from_sides_angle(side_b, side_c, angle_a, style)
}

pub fn triangle_sas(
  side_a: Int,
  angle_b: Int,
  side_c: Int,
  style: Style,
) -> Image {
  triangle_sasf(
    int.to_float(side_a),
    int.to_float(angle_b),
    int.to_float(side_c),
    style,
  )
}

/// Triangle by side-side-angle (side a, side b, angle C).
pub fn triangle_ssaf(
  side_a: Float,
  side_b: Float,
  angle_c: Float,
  style: Style,
) -> Image {
  let side_a = positive(side_a)
  let side_b = positive(side_b)
  let side_c = solve_side(side_a, side_b, angle_c)
  let angle_a = solve_angle(side_a, side_b, side_c)
  triangle_from_sides_angle(side_b, side_c, angle_a, style)
}

pub fn triangle_ssa(
  side_a: Int,
  side_b: Int,
  angle_c: Int,
  style: Style,
) -> Image {
  triangle_ssaf(
    int.to_float(side_a),
    int.to_float(side_b),
    int.to_float(angle_c),
    style,
  )
}

/// Triangle by angle-angle-side (angle A, angle B, side c).
pub fn triangle_aasf(
  angle_a: Float,
  angle_b: Float,
  side_c: Float,
  style: Style,
) -> Image {
  let side_c = positive(side_c)
  let angle_c = 180.0 -. angle_a -. angle_b
  // Law of sines: side / sin(angle) = side_c / sin(angle_c)
  let ratio = side_c /. sin_deg(angle_c)
  let side_b = ratio *. sin_deg(angle_b)
  triangle_from_sides_angle(side_b, side_c, angle_a, style)
}

pub fn triangle_aas(
  angle_a: Int,
  angle_b: Int,
  side_c: Int,
  style: Style,
) -> Image {
  triangle_aasf(
    int.to_float(angle_a),
    int.to_float(angle_b),
    int.to_float(side_c),
    style,
  )
}

/// Triangle by angle-side-side (angle A, side b, side c).
pub fn triangle_assf(
  angle_a: Float,
  side_b: Float,
  side_c: Float,
  style: Style,
) -> Image {
  let side_b = positive(side_b)
  let side_c = positive(side_c)
  triangle_from_sides_angle(side_b, side_c, angle_a, style)
}

pub fn triangle_ass(
  angle_a: Int,
  side_b: Int,
  side_c: Int,
  style: Style,
) -> Image {
  triangle_assf(
    int.to_float(angle_a),
    int.to_float(side_b),
    int.to_float(side_c),
    style,
  )
}

/// Triangle by angle-side-angle (angle A, side b, angle C).
pub fn triangle_asaf(
  angle_a: Float,
  side_b: Float,
  angle_c: Float,
  style: Style,
) -> Image {
  let side_b = positive(side_b)
  let angle_b = 180.0 -. angle_a -. angle_c
  let ratio = side_b /. sin_deg(angle_b)
  let side_c = ratio *. sin_deg(angle_c)
  triangle_from_sides_angle(side_b, side_c, angle_a, style)
}

pub fn triangle_asa(
  angle_a: Int,
  side_b: Int,
  angle_c: Int,
  style: Style,
) -> Image {
  triangle_asaf(
    int.to_float(angle_a),
    int.to_float(side_b),
    int.to_float(angle_c),
    style,
  )
}

/// Triangle by side-angle-angle (side a, angle B, angle C).
pub fn triangle_saaf(
  side_a: Float,
  angle_b: Float,
  angle_c: Float,
  style: Style,
) -> Image {
  let side_a = positive(side_a)
  let angle_a = 180.0 -. angle_b -. angle_c
  let ratio = side_a /. sin_deg(angle_a)
  let side_b = ratio *. sin_deg(angle_b)
  let side_c = ratio *. sin_deg(angle_c)
  triangle_from_sides_angle(side_b, side_c, angle_a, style)
}

pub fn triangle_saa(
  side_a: Int,
  angle_b: Int,
  angle_c: Int,
  style: Style,
) -> Image {
  triangle_saaf(
    int.to_float(side_a),
    int.to_float(angle_b),
    int.to_float(angle_c),
    style,
  )
}

pub fn rhombusf(side_length: Float, angle: Float, style: Style) -> Image {
  let side_length = positive(side_length)
  // An angle above 180 makes the cosine negative.
  let height = float.absolute_value(2.0 *. side_length *. cos_deg(angle /. 2.0))
  let width = float.absolute_value(2.0 *. side_length *. sin_deg(angle /. 2.0))
  points_to_path(
    [
      Pointf(0.0, height /. 2.0),
      Pointf(width /. 2.0, 0.0),
      Pointf(width, height /. 2.0),
      Pointf(width /. 2.0, height),
    ],
    style,
  )
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

pub fn regular_polygon(
  side_length: Int,
  side_count: Int,
  style: Style,
) -> Image {
  regular_polygonf(int.to_float(side_length), side_count, style)
}

pub fn polygonf(points: List(Pointf), style: Style) -> Image {
  points_to_path(points, style) |> fix_position
}

pub fn polygon(points: List(Point), style: Style) -> Image {
  polygonf(list.map(points, point_to_pointf), style)
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
    True -> 90.0 +. 180.0 /. side_countf
    False -> -90.0
  }

  int.range(0, side_count, [], fn(acc, i) {
    let theta =
      alpha +. 360.0 *. int.to_float(i * step_count % side_count) /. side_countf
    [Pointf(radius *. cos_deg(theta), radius *. sin_deg(theta)), ..acc]
  })
  |> list.reverse
  |> points_to_path(style)
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

pub fn radial_starf(
  point_count: Int,
  inner_radius: Float,
  outer_radius: Float,
  style: Style,
) -> Image {
  let point_count = int.max(2, point_count)
  let inner_radius = positive(inner_radius)
  let outer_radius = positive(outer_radius)
  let alpha = case int.is_even(point_count) {
    True -> 90.0 +. 180.0 /. int.to_float(point_count)
    False -> -90.0
  }

  int.range(0, point_count, [], fn(acc, i) {
    let theta1 =
      alpha +. 360.0 *. int.to_float(i * 2) /. int.to_float(2 * point_count)
    let theta2 =
      alpha +. 360.0 *. int.to_float(i * 2 + 1) /. int.to_float(2 * point_count)
    [
      Pointf(inner_radius *. cos_deg(theta2), inner_radius *. sin_deg(theta2)),
      Pointf(outer_radius *. cos_deg(theta1), outer_radius *. sin_deg(theta1)),
      ..acc
    ]
  })
  |> list.reverse
  |> points_to_path(style)
  |> fix_position
}

pub fn radial_star(
  point_count: Int,
  inner_radius: Int,
  outer_radius: Int,
  style: Style,
) -> Image {
  radial_starf(
    point_count,
    int.to_float(inner_radius),
    int.to_float(outer_radius),
    style,
  )
}

pub fn pulled_regular_polygonf(
  side_length: Float,
  side_count: Int,
  pull: Float,
  angle: Float,
  style: Style,
) -> Image {
  let side_count = int.max(3, side_count)
  let side_countf = int.to_float(side_count)
  let radius = positive(side_length) /. { 2.0 *. sin_deg(180.0 /. side_countf) }
  let alpha = case int.is_even(side_count) {
    True -> 90.0 +. 180.0 /. side_countf
    False -> -90.0
  }
  let vertices =
    int.range(0, side_count, [], fn(acc, i) {
      let theta = alpha +. 360.0 *. int.to_float(i) /. side_countf
      [Pointf(radius *. cos_deg(theta), radius *. sin_deg(theta)), ..acc]
    })
    |> list.reverse
  case vertices {
    [first, ..] -> {
      let edges = pulled_edges(vertices, first, pull, angle)
      Path(style, [MoveTo(first), ..edges], True) |> fix_position
    }
    _ -> empty
  }
}

pub fn pulled_regular_polygon(
  side_length: Int,
  side_count: Int,
  pull: Float,
  angle: Float,
  style: Style,
) -> Image {
  pulled_regular_polygonf(
    int.to_float(side_length),
    side_count,
    pull,
    angle,
    style,
  )
}

fn pulled_edges(
  vertices: List(Pointf),
  first: Pointf,
  pull: Float,
  angle: Float,
) -> List(PathCmd) {
  case vertices {
    [] -> []
    [last] -> [edge_cubic(last, first, pull, angle)]
    [a, b, ..rest] -> [
      edge_cubic(a, b, pull, angle),
      ..pulled_edges([b, ..rest], first, pull, angle)
    ]
  }
}

fn edge_cubic(from: Pointf, to: Pointf, pull: Float, angle: Float) -> PathCmd {
  let dx = to.x -. from.x
  let dy = to.y -. from.y
  let dist = sqrt(dx *. dx +. dy *. dy)
  let edge_rad = atan2(dy, dx)
  let angle_rad = angle *. pi /. 180.0
  let c1 =
    Pointf(
      from.x +. pull *. dist *. cos(edge_rad +. angle_rad),
      from.y +. pull *. dist *. sin(edge_rad +. angle_rad),
    )
  let c2 =
    Pointf(
      to.x -. pull *. dist *. cos(edge_rad -. angle_rad),
      to.y -. pull *. dist *. sin(edge_rad -. angle_rad),
    )
  CubicTo(c1, c2, to)
}

// **************************
// * Wedge
// **************************

pub fn wedgef(radius: Float, angle: Float, style: Style) -> Image {
  wedge_path(radius, angle, style) |> fix_position
}

pub fn wedge(radius: Int, angle: Int, style: Style) -> Image {
  wedgef(int.to_float(radius), int.to_float(angle), style)
}

fn wedge_path(radius: Float, angle: Float, style: Style) -> Image {
  let r = positive(radius)
  let sweep_flag = angle <. 0.0
  let arcs = case float.absolute_value(angle) >=. 360.0 {
    // An arc that ends where it starts draws nothing, so a full turn takes
    // two halves.
    True -> [
      ArcTo(r, r, 0.0, False, sweep_flag, Pointf(0.0 -. r, 0.0)),
      ArcTo(r, r, 0.0, False, sweep_flag, Pointf(r, 0.0)),
    ]
    False -> {
      let large_arc = float.absolute_value(angle) >. 180.0
      let end = Pointf(r *. cos_deg(angle), 0.0 -. r *. sin_deg(angle))
      [ArcTo(r, r, 0.0, large_arc, sweep_flag, end)]
    }
  }
  Path(style, [MoveTo(Pointf(0.0, 0.0)), LineTo(Pointf(r, 0.0)), ..arcs], True)
}

// **************************
// * Text
// **************************

pub fn text_fontf(text: String, font: Font, style: Style) -> Image {
  let #(width, height) = system.text_metrics(text, font)
  Text(style, box_of_size(width, height), text, font)
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
// * Bitmap
// **************************

pub fn bitmap(path: String) -> Image {
  let #(width, height, image) = system.load_bitmap(path)
  Bitmap(box_of_size(width, height), image)
}

pub fn bitmap_data_uri(data_uri: String, width: Float, height: Float) -> Image {
  Bitmap(box_of_size(width, height), system.load_image(data_uri))
}

// **************************
// * Transformations
// **************************

pub fn rotatef(img: Image, angle: Float) -> Image {
  // the api for the user is counter clockwise, but the implementation is clockwise
  rotate_around(img, centerf(img), 0.0 -. angle) |> fix_position
}

pub fn rotate(img: Image, angle: Int) -> Image {
  rotatef(img, int.to_float(angle))
}

fn rotate_around(img: Image, center: Pointf, angle: Float) -> Image {
  case img {
    Path(style:, commands:, closed:) ->
      Path(style, list.map(commands, cmd_rotate(_, center, angle)), closed)
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
    Bitmap(box:, ..) -> Bitmap(..img, box: box_rotate(box, center, angle))
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
    // The path takes the scale point by point, but its stroke has one
    // width and one dash, so they take the mean of the two factors, as the
    // width does in 2htdp.
    Path(style:, commands:, closed:) ->
      Path(
        style.scale(style, { x_factor +. y_factor } /. 2.0),
        list.map(commands, cmd_scale(_, x_factor, y_factor)),
        closed,
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
    // The stroke of a text is as wide as the stroke of a path.
    Text(style:, box:, ..) ->
      Text(
        ..img,
        style: style.scale(style, { x_factor +. y_factor } /. 2.0),
        box: box_scale(box, x_factor, y_factor),
      )
    Bitmap(box:, ..) -> Bitmap(..img, box: box_scale(box, x_factor, y_factor))
  }
}

pub fn scale_xy(img: Image, x_factor: Int, y_factor: Int) -> Image {
  scale_xyf(img, int.to_float(x_factor), int.to_float(y_factor))
}

pub fn flip_horizontal(img: Image) -> Image {
  flip(img, point_flip_x) |> fix_position
}

pub fn flip_vertical(img: Image) -> Image {
  flip(img, point_flip_y) |> fix_position
}

fn flip(img: Image, point_flip: fn(Pointf) -> Pointf) -> Image {
  case img {
    Path(style:, commands:, closed:) ->
      Path(style, list.map(commands, cmd_flip(_, point_flip)), closed)
    Combination(a, b) -> Combination(flip(a, point_flip), flip(b, point_flip))
    Crop(box:, image:) ->
      Crop(box: box_flip(box, point_flip), image: flip(image, point_flip))
    Text(box:, ..) -> Text(..img, box: box_flip(box, point_flip))
    Bitmap(box:, ..) -> Bitmap(..img, box: box_flip(box, point_flip))
  }
}

pub fn frame(img: Image) -> Image {
  color_frame(img, color.black)
}

pub fn color_frame(img: Image, color: Color) -> Image {
  let w = widthf(img)
  let h = heightf(img)
  let frame_style = style.join([style.stroke(color), style.stroke_widthf(2.0)])
  cropf(overlay(rectanglef(w, h, frame_style), img), 0.0, 0.0, w, h)
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
  Crop(box_of_size(width, height), translate(img, 0.0 -. x, 0.0 -. y))
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
  // The offset is negative when the crop is larger than the image, which
  // puts the image at the right or bottom of the crop.
  let dx = case x_place {
    Left -> 0.0
    Center -> mid(widthf(img), crop_width)
    Right -> widthf(img) -. crop_width
  }
  let dy = case y_place {
    Top -> 0.0
    Middle -> mid(heightf(img), crop_height)
    Bottom -> heightf(img) -. crop_height
  }
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
  above_align(a, Center, b)
}

pub fn above_align(a: Image, x_place: XPlace, b: Image) -> Image {
  let #(dxa, dxb) = x_place_dx(x_place, widthf(a), widthf(b))
  Combination(translate(a, dxa, 0.0), translate(b, dxb, heightf(a)))
}

pub fn beside(a: Image, b: Image) -> Image {
  beside_align(a, Middle, b)
}

pub fn beside_align(a: Image, y_place: YPlace, b: Image) -> Image {
  let #(dya, dyb) = y_place_dy(y_place, heightf(a), heightf(b))
  Combination(translate(a, 0.0, dya), translate(b, widthf(a), dyb))
}

pub fn overlay(top: Image, bottom: Image) -> Image {
  overlay_align(top, Center, Middle, bottom)
}

pub fn overlay_align(
  top: Image,
  x_place: XPlace,
  y_place: YPlace,
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
  top: Image,
  x_place: XPlace,
  y_place: YPlace,
  x: Float,
  y: Float,
  bottom: Image,
) -> Image {
  overlay_align(top, x_place, y_place, translate(bottom, x, y))
}

pub fn overlay_align_offset(
  top: Image,
  x_place: XPlace,
  y_place: YPlace,
  x: Int,
  y: Int,
  bottom: Image,
) -> Image {
  overlay_align_offsetf(
    top,
    x_place,
    y_place,
    int.to_float(x),
    int.to_float(y),
    bottom,
  )
}

pub fn overlay_xyf(top: Image, x: Float, y: Float, bottom: Image) -> Image {
  Combination(translate(bottom, x, y), top) |> fix_position
}

pub fn overlay_xy(top: Image, x: Int, y: Int, bottom: Image) -> Image {
  overlay_xyf(top, int.to_float(x), int.to_float(y), bottom)
}

pub fn underlay(bottom: Image, top: Image) -> Image {
  overlay(top, bottom)
}

pub fn underlay_align(
  bottom: Image,
  x_place: XPlace,
  y_place: YPlace,
  top: Image,
) -> Image {
  overlay_align(top, x_place, y_place, bottom)
}

pub fn underlay_offsetf(
  bottom: Image,
  x: Float,
  y: Float,
  top: Image,
) -> Image {
  overlay(translate(top, x, y), bottom)
}

pub fn underlay_offset(bottom: Image, x: Int, y: Int, top: Image) -> Image {
  underlay_offsetf(bottom, int.to_float(x), int.to_float(y), top)
}

pub fn underlay_align_offsetf(
  bottom: Image,
  x_place: XPlace,
  y_place: YPlace,
  x: Float,
  y: Float,
  top: Image,
) -> Image {
  underlay_align(bottom, x_place, y_place, translate(top, x, y))
}

pub fn underlay_align_offset(
  bottom: Image,
  x_place: XPlace,
  y_place: YPlace,
  x: Int,
  y: Int,
  top: Image,
) -> Image {
  underlay_align_offsetf(
    bottom,
    x_place,
    y_place,
    int.to_float(x),
    int.to_float(y),
    top,
  )
}

pub fn underlay_xyf(bottom: Image, x: Float, y: Float, top: Image) -> Image {
  Combination(bottom, translate(top, x, y)) |> fix_position
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
  let frame_style = style.join([style.stroke(color), style.stroke_widthf(2.0)])
  cropf(rectanglef(width, height, frame_style), 0.0, 0.0, width, height)
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

pub fn place_imagesf(
  scene: Image,
  positions: List(Pointf),
  images: List(Image),
) -> Image {
  case positions, images {
    [pos, ..rest_pos], [img, ..rest_imgs] ->
      place_imagesf(place_imagef(scene, pos.x, pos.y, img), rest_pos, rest_imgs)
    _, _ -> scene
  }
}

pub fn place_images(
  scene: Image,
  positions: List(Point),
  images: List(Image),
) -> Image {
  place_imagesf(scene, list.map(positions, point_to_pointf), images)
}

pub fn place_images_alignf(
  scene: Image,
  positions: List(Pointf),
  x_place: XPlace,
  y_place: YPlace,
  images: List(Image),
) -> Image {
  case positions, images {
    [pos, ..rest_pos], [img, ..rest_imgs] ->
      place_images_alignf(
        place_image_alignf(scene, pos.x, pos.y, x_place, y_place, img),
        rest_pos,
        x_place,
        y_place,
        rest_imgs,
      )
    _, _ -> scene
  }
}

pub fn place_images_align(
  scene: Image,
  positions: List(Point),
  x_place: XPlace,
  y_place: YPlace,
  images: List(Image),
) -> Image {
  place_images_alignf(
    scene,
    list.map(positions, point_to_pointf),
    x_place,
    y_place,
    images,
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
  Combination(
    scene,
    Path(style, [MoveTo(Pointf(x1, y1)), LineTo(Pointf(x2, y2))], False),
  )
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

pub fn place_polygonf(
  scene: Image,
  points: List(Pointf),
  style: Style,
) -> Image {
  Combination(scene, points_to_path(points, style))
  |> cropf(0.0, 0.0, widthf(scene), heightf(scene))
  |> fix_position
}

pub fn place_polygon(scene: Image, points: List(Point), style: Style) -> Image {
  place_polygonf(scene, list.map(points, point_to_pointf), style)
}

pub fn place_curvef(
  scene: Image,
  x1: Float,
  y1: Float,
  angle1: Float,
  pull1: Float,
  x2: Float,
  y2: Float,
  angle2: Float,
  pull2: Float,
  style: Style,
) -> Image {
  let #(c1, c2) = curve_controls(x1, y1, angle1, pull1, x2, y2, angle2, pull2)
  Combination(
    scene,
    Path(
      style,
      [MoveTo(Pointf(x1, y1)), CubicTo(c1, c2, Pointf(x2, y2))],
      False,
    ),
  )
  |> cropf(0.0, 0.0, widthf(scene), heightf(scene))
  |> fix_position
}

pub fn place_curve(
  scene: Image,
  x1: Int,
  y1: Int,
  angle1: Int,
  pull1: Float,
  x2: Int,
  y2: Int,
  angle2: Int,
  pull2: Float,
  style: Style,
) -> Image {
  place_curvef(
    scene,
    int.to_float(x1),
    int.to_float(y1),
    int.to_float(angle1),
    pull1,
    int.to_float(x2),
    int.to_float(y2),
    int.to_float(angle2),
    pull2,
    style,
  )
}

pub fn place_wedgef(
  scene: Image,
  x: Float,
  y: Float,
  radius: Float,
  angle: Float,
  style: Style,
) -> Image {
  Combination(scene, translate(wedge_path(radius, angle, style), x, y))
  |> cropf(0.0, 0.0, widthf(scene), heightf(scene))
  |> fix_position
}

pub fn place_wedge(
  scene: Image,
  x: Int,
  y: Int,
  radius: Int,
  angle: Int,
  style: Style,
) -> Image {
  place_wedgef(
    scene,
    int.to_float(x),
    int.to_float(y),
    int.to_float(radius),
    int.to_float(angle),
    style,
  )
}

pub fn put_imagef(scene: Image, x: Float, y: Float, img: Image) -> Image {
  place_imagef(scene, x, heightf(scene) -. y, img)
}

pub fn put_image(scene: Image, x: Int, y: Int, img: Image) -> Image {
  put_imagef(scene, int.to_float(x), int.to_float(y), img)
}

// **************************
// * Adding
// **************************

pub fn add_linef(
  img: Image,
  x1: Float,
  y1: Float,
  x2: Float,
  y2: Float,
  style: Style,
) -> Image {
  Combination(
    img,
    Path(style, [MoveTo(Pointf(x1, y1)), LineTo(Pointf(x2, y2))], False),
  )
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

pub fn add_polygonf(img: Image, points: List(Pointf), style: Style) -> Image {
  Combination(img, points_to_path(points, style)) |> fix_position
}

pub fn add_polygon(img: Image, points: List(Point), style: Style) -> Image {
  add_polygonf(img, list.map(points, point_to_pointf), style)
}

pub fn add_curvef(
  img: Image,
  x1: Float,
  y1: Float,
  angle1: Float,
  pull1: Float,
  x2: Float,
  y2: Float,
  angle2: Float,
  pull2: Float,
  style: Style,
) -> Image {
  let #(c1, c2) = curve_controls(x1, y1, angle1, pull1, x2, y2, angle2, pull2)
  Combination(
    img,
    Path(
      style,
      [MoveTo(Pointf(x1, y1)), CubicTo(c1, c2, Pointf(x2, y2))],
      False,
    ),
  )
  |> fix_position
}

pub fn add_curve(
  img: Image,
  x1: Int,
  y1: Int,
  angle1: Int,
  pull1: Float,
  x2: Int,
  y2: Int,
  angle2: Int,
  pull2: Float,
  style: Style,
) -> Image {
  add_curvef(
    img,
    int.to_float(x1),
    int.to_float(y1),
    int.to_float(angle1),
    pull1,
    int.to_float(x2),
    int.to_float(y2),
    int.to_float(angle2),
    pull2,
    style,
  )
}

pub fn add_solid_curvef(
  img: Image,
  x1: Float,
  y1: Float,
  angle1: Float,
  pull1: Float,
  x2: Float,
  y2: Float,
  angle2: Float,
  pull2: Float,
  style: Style,
) -> Image {
  let #(c1, c2) = curve_controls(x1, y1, angle1, pull1, x2, y2, angle2, pull2)
  Combination(
    img,
    Path(style, [MoveTo(Pointf(x1, y1)), CubicTo(c1, c2, Pointf(x2, y2))], True),
  )
  |> fix_position
}

pub fn add_solid_curve(
  img: Image,
  x1: Int,
  y1: Int,
  angle1: Int,
  pull1: Float,
  x2: Int,
  y2: Int,
  angle2: Int,
  pull2: Float,
  style: Style,
) -> Image {
  add_solid_curvef(
    img,
    int.to_float(x1),
    int.to_float(y1),
    int.to_float(angle1),
    pull1,
    int.to_float(x2),
    int.to_float(y2),
    int.to_float(angle2),
    pull2,
    style,
  )
}

pub fn add_wedgef(
  img: Image,
  x: Float,
  y: Float,
  radius: Float,
  angle: Float,
  style: Style,
) -> Image {
  Combination(img, translate(wedge_path(radius, angle, style), x, y))
  |> fix_position
}

pub fn add_wedge(
  img: Image,
  x: Int,
  y: Int,
  radius: Int,
  angle: Int,
  style: Style,
) -> Image {
  add_wedgef(
    img,
    int.to_float(x),
    int.to_float(y),
    int.to_float(radius),
    int.to_float(angle),
    style,
  )
}

fn curve_controls(
  x1: Float,
  y1: Float,
  angle1: Float,
  pull1: Float,
  x2: Float,
  y2: Float,
  angle2: Float,
  pull2: Float,
) -> #(Pointf, Pointf) {
  let dist = sqrt({ x2 -. x1 } *. { x2 -. x1 } +. { y2 -. y1 } *. { y2 -. y1 })
  let c1 =
    Pointf(
      x1 +. pull1 *. dist *. cos_deg(angle1),
      y1 -. pull1 *. dist *. sin_deg(angle1),
    )
  let c2 =
    Pointf(
      x2 -. pull2 *. dist *. cos_deg(angle2),
      y2 +. pull2 *. dist *. sin_deg(angle2),
    )
  #(c1, c2)
}

// **************************
// * SVG
// **************************

pub fn to_svg(img: Image) -> String {
  image_to_svg(
    img,
    float.ceiling(round2(widthf(img))),
    float.ceiling(round2(heightf(img))),
  )
}

fn round2(v: Float) -> Float {
  int.to_float(float.round(v *. 100.0)) /. 100.0
}

/// sinteract draws `img` on a canvas of `width` by `height`.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "image_to_svg")
fn image_to_svg(img: Image, width: Float, height: Float) -> String
