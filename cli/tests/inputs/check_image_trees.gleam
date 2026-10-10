import sgleam/check
import sgleam/fill
import sgleam/image
import sgleam/xplace
import sgleam/yplace

// Pictures built in different ways that give the same tree. A change to
// how an image is stored must keep each pair equal under `==`. The shapes
// are rectangles, because the box of a curve has rounding noise.
pub fn same_tree_examples() {
  let a = image.rectangle(10, 20, fill.red)
  let b = image.square(20, fill.blue)
  let c = image.square(30, fill.green)
  check.eq(image.beside(a, b), image.beside_align(a, yplace.Top, b))
  check.eq(image.beside(a, b), image.underlay_offset(a, 15, 0, b))
  check.eq(image.beside_align(a, yplace.Top, b), image.underlay_xy(a, 10, 0, b))
  check.eq(
    image.scale(image.beside(a, b), 2),
    image.beside(image.scale(a, 2), image.scale(b, 2)),
  )
  check.eq(
    image.flip_vertical(image.beside(a, b)),
    image.beside(image.flip_vertical(a), image.flip_vertical(b)),
  )
  check.eq(image.overlay_offset(a, -5, 0, b), image.underlay_offset(b, 5, 0, a))
  check.eq(image.overlay(a, c), image.overlay_offset(a, 0, 0, c))
  check.eq(image.overlay(b, a), image.underlay_xy(a, -5, 0, b))
  check.eq(
    image.overlay(a, c),
    image.overlay_align(a, xplace.Center, yplace.Middle, c),
  )
}
