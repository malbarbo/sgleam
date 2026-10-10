import sgleam/check
import sgleam/fill
import sgleam/image
import sgleam/stroke

// Pictures that draw the same pixels, and pictures that do not, whatever
// their trees.
pub fn same_examples() {
  let r = image.rectangle(20, 10, fill.red)
  let s = image.square(10, fill.green)
  let t = image.triangle(30, fill.red)
  check.true(image.same(r, r))
  check.true(image.same(image.rotate(r, 90), image.rectangle(10, 20, fill.red)))
  check.true(image.same(image.beside(s, s), image.overlay_xy(s, 10, 0, s)))
  check.true(image.same(image.flip_horizontal(image.flip_horizontal(t)), t))
  check.true(image.same(image.overlay(s, image.square(10, fill.red)), s))
  check.true(image.same(image.empty, image.empty))
}

pub fn not_same_examples() {
  let r = image.rectangle(20, 10, fill.red)
  check.false(image.same(r, image.rectangle(20, 10, fill.blue)))
  check.false(image.same(r, image.rectangle(21, 10, fill.red)))
  check.false(image.same(
    image.square(10, stroke.black),
    image.square(10, fill.black),
  ))
  check.false(image.same(
    image.text("a", 20, fill.black),
    image.text("b", 20, fill.black),
  ))
}
