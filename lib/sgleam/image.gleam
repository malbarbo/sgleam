import gleam/bool
import gleam/float
import gleam/int
import gleam/list
import gleam/string
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
  Text(
    style: Style,
    box: Box,
    text: String,
    flip_vertical: Bool,
    flip_horizontal: Bool,
    font: Font,
  )
  Bitmap(box: Box, data_uri: String)
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
  let center = Pointf(box.center.x *. x_factor, box.center.y *. y_factor)
  Box(
    ..box,
    center: center,
    width: box.width *. x_factor,
    height: box.height *. y_factor,
  )
}

fn box_flip(box: Box, point_flip: fn(Pointf) -> Pointf) -> Box {
  Box(..box, center: point_flip(box.center), angle: 0.0 -. box.angle)
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
  case img {
    Path(commands: [], ..) -> #(Pointf(0.0, 0.0), Pointf(0.0, 0.0))
    Path(commands: [first, ..rest], ..) -> {
      let p = cmd_endpoint(first)
      path_box(rest, p, p.x, p.y, p.x, p.y)
    }
    Combination(a, b) -> {
      let #(amin, amax) = box(a)
      let #(bmin, bmax) = box(b)
      #(
        Pointf(float.min(amin.x, bmin.x), float.min(amin.y, bmin.y)),
        Pointf(float.max(amax.x, bmax.x), float.max(amax.y, bmax.y)),
      )
    }
    Crop(box:, ..) -> box_box(box)
    Text(box:, ..) -> box_box(box)
    Bitmap(box:, ..) -> box_box(box)
  }
}

fn path_box(
  commands: List(PathCmd),
  prev: Pointf,
  min_x: Float,
  min_y: Float,
  max_x: Float,
  max_y: Float,
) -> #(Pointf, Pointf) {
  case commands {
    [] -> #(Pointf(min_x, min_y), Pointf(max_x, max_y))
    [cmd, ..rest] -> {
      let #(min_x, min_y, max_x, max_y) =
        cmd_box(cmd, prev, min_x, min_y, max_x, max_y)
      path_box(rest, cmd_endpoint(cmd), min_x, min_y, max_x, max_y)
    }
  }
}

fn cmd_box(
  cmd: PathCmd,
  prev: Pointf,
  min_x: Float,
  min_y: Float,
  max_x: Float,
  max_y: Float,
) -> #(Float, Float, Float, Float) {
  case cmd {
    MoveTo(p) -> update_bounds(p, min_x, min_y, max_x, max_y)
    LineTo(p) -> update_bounds(p, min_x, min_y, max_x, max_y)
    QuadTo(c, e) -> quad_box(prev, c, e, min_x, min_y, max_x, max_y)
    CubicTo(c1, c2, e) -> cubic_box(prev, c1, c2, e, min_x, min_y, max_x, max_y)
    ArcTo(rx, ry, rot, la, sw, e) ->
      arc_box(prev, rx, ry, rot, la, sw, e, min_x, min_y, max_x, max_y)
  }
}

fn update_bounds(
  p: Pointf,
  min_x: Float,
  min_y: Float,
  max_x: Float,
  max_y: Float,
) -> #(Float, Float, Float, Float) {
  #(
    float.min(min_x, p.x),
    float.min(min_y, p.y),
    float.max(max_x, p.x),
    float.max(max_y, p.y),
  )
}

// Quadratic bezier bounding box
fn quad_box(
  p0: Pointf,
  c: Pointf,
  e: Pointf,
  min_x: Float,
  min_y: Float,
  max_x: Float,
  max_y: Float,
) -> #(Float, Float, Float, Float) {
  let #(min_x, min_y, max_x, max_y) =
    update_bounds(e, min_x, min_y, max_x, max_y)
  let #(min_x, max_x) = quad_axis_extrema(p0.x, c.x, e.x, min_x, max_x)
  let #(min_y, max_y) = quad_axis_extrema(p0.y, c.y, e.y, min_y, max_y)
  #(min_x, min_y, max_x, max_y)
}

fn quad_axis_extrema(
  p0: Float,
  c: Float,
  e: Float,
  min: Float,
  max: Float,
) -> #(Float, Float) {
  // B'(t) = 0 => t = (p0 - c) / (p0 - 2c + e)
  let denom = p0 -. 2.0 *. c +. e
  case denom == 0.0 {
    True -> #(min, max)
    False -> {
      let t = { p0 -. c } /. denom
      case t >. 0.0 && t <. 1.0 {
        True -> {
          let v = quad_at(t, p0, c, e)
          #(float.min(min, v), float.max(max, v))
        }
        False -> #(min, max)
      }
    }
  }
}

fn quad_at(t: Float, p0: Float, c: Float, e: Float) -> Float {
  let mt = 1.0 -. t
  mt *. mt *. p0 +. 2.0 *. mt *. t *. c +. t *. t *. e
}

// Cubic bezier bounding box
fn cubic_box(
  p0: Pointf,
  c1: Pointf,
  c2: Pointf,
  e: Pointf,
  min_x: Float,
  min_y: Float,
  max_x: Float,
  max_y: Float,
) -> #(Float, Float, Float, Float) {
  let #(min_x, min_y, max_x, max_y) =
    update_bounds(e, min_x, min_y, max_x, max_y)
  let #(min_x, max_x) = cubic_axis_extrema(p0.x, c1.x, c2.x, e.x, min_x, max_x)
  let #(min_y, max_y) = cubic_axis_extrema(p0.y, c1.y, c2.y, e.y, min_y, max_y)
  #(min_x, min_y, max_x, max_y)
}

fn cubic_axis_extrema(
  p0: Float,
  c1: Float,
  c2: Float,
  e: Float,
  min: Float,
  max: Float,
) -> #(Float, Float) {
  // B'(t) = 3[a*t^2 + b*t + c] where:
  let a = 0.0 -. p0 +. 3.0 *. c1 -. 3.0 *. c2 +. e
  let b = 2.0 *. { p0 -. 2.0 *. c1 +. c2 }
  let c = c1 -. p0
  case a == 0.0 {
    True ->
      // Linear: t = -c/b
      case b == 0.0 {
        True -> #(min, max)
        False -> {
          let t = 0.0 -. c /. b
          case t >. 0.0 && t <. 1.0 {
            True -> {
              let v = cubic_at(t, p0, c1, c2, e)
              #(float.min(min, v), float.max(max, v))
            }
            False -> #(min, max)
          }
        }
      }
    False -> {
      let disc = b *. b -. 4.0 *. a *. c
      case disc <. 0.0 {
        True -> #(min, max)
        False -> {
          let sq = sqrt(disc)
          let t1 = { 0.0 -. b +. sq } /. { 2.0 *. a }
          let t2 = { 0.0 -. b -. sq } /. { 2.0 *. a }
          let #(min, max) = case t1 >. 0.0 && t1 <. 1.0 {
            True -> {
              let v = cubic_at(t1, p0, c1, c2, e)
              #(float.min(min, v), float.max(max, v))
            }
            False -> #(min, max)
          }
          case t2 >. 0.0 && t2 <. 1.0 {
            True -> {
              let v = cubic_at(t2, p0, c1, c2, e)
              #(float.min(min, v), float.max(max, v))
            }
            False -> #(min, max)
          }
        }
      }
    }
  }
}

fn cubic_at(t: Float, p0: Float, c1: Float, c2: Float, e: Float) -> Float {
  let mt = 1.0 -. t
  mt
  *. mt
  *. mt
  *. p0
  +. 3.0
  *. mt
  *. mt
  *. t
  *. c1
  +. 3.0
  *. mt
  *. t
  *. t
  *. c2
  +. t
  *. t
  *. t
  *. e
}

// Elliptical arc bounding box (SVG spec F.6.5)
fn arc_box(
  p1: Pointf,
  rx: Float,
  ry: Float,
  phi_deg: Float,
  large_arc: Bool,
  sweep: Bool,
  p2: Pointf,
  min_x: Float,
  min_y: Float,
  max_x: Float,
  max_y: Float,
) -> #(Float, Float, Float, Float) {
  let #(min_x, min_y, max_x, max_y) =
    update_bounds(p2, min_x, min_y, max_x, max_y)
  let rx = float.absolute_value(rx)
  let ry = float.absolute_value(ry)
  case rx == 0.0 || ry == 0.0 {
    True -> #(min_x, min_y, max_x, max_y)
    False -> {
      let phi = phi_deg *. pi /. 180.0
      let cos_phi = cos(phi)
      let sin_phi = sin(phi)
      // Step 1: compute (x1', y1')
      let dx = { p1.x -. p2.x } /. 2.0
      let dy = { p1.y -. p2.y } /. 2.0
      let x1p = cos_phi *. dx +. sin_phi *. dy
      let y1p = 0.0 -. sin_phi *. dx +. cos_phi *. dy
      // Correct radii if too small
      let lambda = x1p *. x1p /. { rx *. rx } +. y1p *. y1p /. { ry *. ry }
      let #(rx, ry) = case lambda >. 1.0 {
        True -> {
          let s = sqrt(lambda)
          #(rx *. s, ry *. s)
        }
        False -> #(rx, ry)
      }
      // Step 2: compute center'
      let num =
        float.max(
          0.0,
          rx
            *. rx
            *. ry
            *. ry
            -. rx
            *. rx
            *. y1p
            *. y1p
            -. ry
            *. ry
            *. x1p
            *. x1p,
        )
      let den = rx *. rx *. y1p *. y1p +. ry *. ry *. x1p *. x1p
      let sq = case den == 0.0 {
        True -> 0.0
        False -> sqrt(num /. den)
      }
      let sign = case large_arc == sweep {
        True -> -1.0
        False -> 1.0
      }
      let cxp = sign *. sq *. rx *. y1p /. ry
      let cyp = sign *. sq *. { 0.0 -. ry } *. x1p /. rx
      // Step 3: compute center
      let cx = cos_phi *. cxp -. sin_phi *. cyp +. { p1.x +. p2.x } /. 2.0
      let cy = sin_phi *. cxp +. cos_phi *. cyp +. { p1.y +. p2.y } /. 2.0
      // Step 4: compute theta1 and dtheta
      let theta1 =
        angle_vec(1.0, 0.0, { x1p -. cxp } /. rx, { y1p -. cyp } /. ry)
      let dtheta_raw =
        angle_vec(
          { x1p -. cxp } /. rx,
          { y1p -. cyp } /. ry,
          { 0.0 -. x1p -. cxp } /. rx,
          { 0.0 -. y1p -. cyp } /. ry,
        )
      let dtheta = case sweep {
        False ->
          case dtheta_raw >. 0.0 {
            True -> dtheta_raw -. 2.0 *. pi
            False -> dtheta_raw
          }
        True ->
          case dtheta_raw <. 0.0 {
            True -> dtheta_raw +. 2.0 *. pi
            False -> dtheta_raw
          }
      }
      // Find extrema: x at theta_x + k*pi, y at theta_y + k*pi
      let theta_x = atan2(0.0 -. ry *. sin_phi, rx *. cos_phi)
      let theta_y = atan2(ry *. cos_phi, rx *. sin_phi)
      arc_check_extrema(
        cx,
        cy,
        rx,
        ry,
        cos_phi,
        sin_phi,
        theta1,
        dtheta,
        theta_x,
        theta_y,
        min_x,
        min_y,
        max_x,
        max_y,
      )
    }
  }
}

fn arc_check_extrema(
  cx: Float,
  cy: Float,
  rx: Float,
  ry: Float,
  cos_phi: Float,
  sin_phi: Float,
  theta1: Float,
  dtheta: Float,
  theta_x: Float,
  theta_y: Float,
  min_x: Float,
  min_y: Float,
  max_x: Float,
  max_y: Float,
) -> #(Float, Float, Float, Float) {
  // Check x-extrema at theta_x + k*pi for k in -1, 0, 1, 2
  let #(min_x, min_y, max_x, max_y) =
    list.fold([-1, 0, 1, 2], #(min_x, min_y, max_x, max_y), fn(b, k) {
      let theta = theta_x +. int.to_float(k) *. pi
      case angle_in_range(theta, theta1, dtheta) {
        True -> {
          let #(px, _) = ellipse_point(cx, cy, rx, ry, cos_phi, sin_phi, theta)
          let #(bmin_x, bmin_y, bmax_x, bmax_y) = b
          #(float.min(bmin_x, px), bmin_y, float.max(bmax_x, px), bmax_y)
        }
        False -> b
      }
    })
  // Check y-extrema at theta_y + k*pi for k in -1, 0, 1, 2
  list.fold([-1, 0, 1, 2], #(min_x, min_y, max_x, max_y), fn(b, k) {
    let theta = theta_y +. int.to_float(k) *. pi
    case angle_in_range(theta, theta1, dtheta) {
      True -> {
        let #(_, py) = ellipse_point(cx, cy, rx, ry, cos_phi, sin_phi, theta)
        let #(bmin_x, bmin_y, bmax_x, bmax_y) = b
        #(bmin_x, float.min(bmin_y, py), bmax_x, float.max(bmax_y, py))
      }
      False -> b
    }
  })
}

fn angle_in_range(theta: Float, theta1: Float, dtheta: Float) -> Bool {
  case dtheta >=. 0.0 {
    True -> {
      let t = normalize_angle(theta -. theta1)
      t <=. dtheta
    }
    False -> {
      let t = normalize_angle(theta1 -. theta)
      t <=. float.absolute_value(dtheta)
    }
  }
}

fn normalize_angle(a: Float) -> Float {
  let two_pi = 2.0 *. pi
  case a <. 0.0 {
    True -> normalize_angle(a +. two_pi)
    False ->
      case a >=. two_pi {
        True -> normalize_angle(a -. two_pi)
        False -> a
      }
  }
}

fn ellipse_point(
  cx: Float,
  cy: Float,
  rx: Float,
  ry: Float,
  cos_phi: Float,
  sin_phi: Float,
  theta: Float,
) -> #(Float, Float) {
  let ct = cos(theta)
  let st = sin(theta)
  #(
    cx +. rx *. ct *. cos_phi -. ry *. st *. sin_phi,
    cy +. rx *. ct *. sin_phi +. ry *. st *. cos_phi,
  )
}

fn angle_vec(ux: Float, uy: Float, vx: Float, vy: Float) -> Float {
  atan2(ux *. vy -. uy *. vx, ux *. vx +. uy *. vy)
}

// **************************
// * PathCmd transforms
// **************************

fn cmd_endpoint(cmd: PathCmd) -> Pointf {
  case cmd {
    MoveTo(p) -> p
    LineTo(p) -> p
    QuadTo(_, end) -> end
    CubicTo(_, _, end) -> end
    ArcTo(_, _, _, _, _, end) -> end
  }
}

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
    ArcTo(rx, ry, rot, la, sw, e) ->
      ArcTo(rx *. x_factor, ry *. y_factor, rot, la, sw, sp(e))
  }
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
  let height = 2.0 *. side_length *. cos_deg(angle /. 2.0)
  let width = 2.0 *. side_length *. sin_deg(angle /. 2.0)
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
  let x1 = r
  let y1 = 0.0
  let x2 = r *. cos_deg(angle)
  let y2 = 0.0 -. r *. sin_deg(angle)
  let large_arc = float.absolute_value(angle) >. 180.0
  let sweep_flag = angle <. 0.0
  Path(
    style,
    [
      MoveTo(Pointf(0.0, 0.0)),
      LineTo(Pointf(x1, y1)),
      ArcTo(r, r, 0.0, large_arc, sweep_flag, Pointf(x2, y2)),
    ],
    True,
  )
}

// **************************
// * Text
// **************************

pub fn text_fontf(text: String, font: Font, style: Style) -> Image {
  let css = font.to_css(font)
  let width = system.text_width(text, css)
  let height = system.text_height(text, css)
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
// * Bitmap
// **************************

pub fn bitmap(path: String) -> Image {
  let #(width, height, data_uri) = system.load_bitmap(path)
  Bitmap(Box(Pointf(width /. 2.0, height /. 2.0), width, height, 0.0), data_uri)
}

pub fn bitmap_data_uri(data_uri: String, width: Float, height: Float) -> Image {
  Bitmap(Box(Pointf(width /. 2.0, height /. 2.0), width, height, 0.0), data_uri)
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
    Path(style:, commands:, closed:) ->
      Path(style, list.map(commands, cmd_scale(_, x_factor, y_factor)), closed)
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
    Bitmap(box:, ..) -> Bitmap(..img, box: box_scale(box, x_factor, y_factor))
  }
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
    Path(style:, commands:, closed:) ->
      Path(style, list.map(commands, cmd_flip(_, point_flip)), closed)
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
  "<svg "
  <> attrib("width", float.ceiling(round2(widthf(img))))
  <> attrib("height", float.ceiling(round2(heightf(img))))
  <> "xmlns=\"http://www.w3.org/2000/svg\">\n"
  <> to_svg_(img, 1)
  <> "</svg>"
}

fn round2(v: Float) -> Float {
  int.to_float(float.round(v *. 100.0)) /. 100.0
}

fn to_svg_(img: Image, level: Int) -> String {
  case img {
    Path(style:, commands:, closed:) -> {
      let aligned = style.outline_offset(style) >. 0.0
      let path_d = commands_to_d(commands, aligned)
      let path_d = case closed {
        True -> path_d <> " Z"
        False -> path_d
      }
      indent(level)
      <> "<path "
      <> attribs("d", path_d)
      <> style.to_svg(style)
      <> "/>\n"
    }
    Combination(a, b) ->
      indent(level)
      <> "<g>\n"
      <> to_svg_(a, level + 1)
      <> to_svg_(b, level + 1)
      <> indent(level)
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
      indent(level)
      <> "<defs>"
      <> "<clipPath "
      <> attribs("id", clipid)
      <> ">"
      <> rect
      <> "</clipPath>"
      <> "</defs>\n"
      <> indent(level)
      <> "<g "
      <> attribs("clip-path", "url(#" <> clipid <> ")")
      <> ">\n"
      <> to_svg_(image, level + 1)
      <> indent(level)
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
      let css = font.to_css(font)
      let original_width = system.text_width(text, css)
      let original_height = system.text_height(text, css)
      let x_offset = system.text_x_offset(text, css)
      let y_offset = system.text_y_offset(text, css)
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
      indent(level)
      <> "<text "
      <> attribs("dominant-baseline", "alphabetic")
      <> attribs("text-anchor", "start")
      <> attrib("x", x_offset)
      <> attrib("y", y_offset)
      <> attribs("font-family", font.family)
      <> attrib("font-size", font.size)
      <> attribs("font-style", font.font_style_to_svg(font.font_style))
      <> attribs("font-weight", font.font_weight_to_svg(font.font_weight))
      <> case font.underline {
        True -> attribs("text-decoration", "underline")
        False -> ""
      }
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
    Bitmap(box: Box(center:, width:, height:, angle:), data_uri:) -> {
      indent(level)
      <> "<image "
      <> attribs("href", data_uri)
      <> attrib("x", center.x -. width /. 2.0)
      <> attrib("y", center.y -. height /. 2.0)
      <> attrib("width", width)
      <> attrib("height", height)
      <> case angle != 0.0 {
        True -> attribs("transform", rotate_str(angle, center))
        False -> ""
      }
      <> "/>\n"
    }
  }
}

fn commands_to_d(commands: List(PathCmd), aligned: Bool) -> String {
  // For outline-only paths, round coordinates to pixel boundaries
  // (floor + 0.5) to match HtDP's aligned drawing mode.
  let c = case aligned {
    True -> align
    False -> fs
  }
  commands
  |> list.map(cmd_to_d(_, c))
  |> string.join(" ")
}

fn cmd_to_d(cmd: PathCmd, c: fn(Float) -> String) -> String {
  case cmd {
    MoveTo(p) -> "M " <> c(p.x) <> " " <> c(p.y)
    LineTo(p) -> "L " <> c(p.x) <> " " <> c(p.y)
    QuadTo(ctrl, e) ->
      "Q " <> c(ctrl.x) <> " " <> c(ctrl.y) <> " " <> c(e.x) <> " " <> c(e.y)
    CubicTo(c1, c2, e) ->
      "C "
      <> c(c1.x)
      <> " "
      <> c(c1.y)
      <> " "
      <> c(c2.x)
      <> " "
      <> c(c2.y)
      <> " "
      <> c(e.x)
      <> " "
      <> c(e.y)
    ArcTo(rx, ry, rot, la, sw, e) ->
      "A "
      <> fs(rx)
      <> " "
      <> fs(ry)
      <> " "
      <> fs(rot)
      <> " "
      <> bool01(la)
      <> " "
      <> bool01(sw)
      <> " "
      <> c(e.x)
      <> " "
      <> c(e.y)
  }
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "float_to_string_6")
fn fs(v: Float) -> String

/// Pixel-aligned coordinate for HtDP's aligned drawing mode.
/// Rounds to nearest integer (floor) then adds 0.5 to center
/// the 1px stroke on the pixel boundary. A small epsilon
/// compensates for floating-point rounding errors (e.g.
/// 9.999999999999998 should be treated as 10.0).
const fp_epsilon = 1.0e-10

fn align(v: Float) -> String {
  fs(float.floor(v +. fp_epsilon) +. 0.5)
}

fn bool01(b: Bool) -> String {
  case b {
    True -> "1"
    False -> "0"
  }
}

fn rotate_str(angle: Float, center: Pointf) -> String {
  "rotate(" <> fs(angle) <> " " <> fs(center.x) <> " " <> fs(center.y) <> ")"
}

fn scale_str(scale_x: Float, scale_y: Float) -> String {
  "scale(" <> fs(scale_x) <> "," <> fs(scale_y) <> ")"
}

fn translate_str(x: Float, y: Float) -> String {
  "translate(" <> fs(x) <> "," <> fs(y) <> ")"
}

fn indent(level: Int) -> String {
  string.repeat(" ", 2 * level)
}

fn attrib(name: String, value: Float) -> String {
  name <> "=\"" <> fs(value) <> "\" "
}

fn attribs(name: String, value: String) -> String {
  name <> "=\"" <> value <> "\" "
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "next_clip_id")
fn next_clip_id() -> Int
