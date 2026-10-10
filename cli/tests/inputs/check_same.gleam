import sgleam/check
import sgleam/fill
import sgleam/image

pub fn same_examples() {
  let r = image.rectangle(20, 10, fill.red)
  check.same(image.rotate(r, 90), image.rectangle(10, 20, fill.red))
  check.same(image.square(10, fill.red), image.square(10, fill.blue))
}
