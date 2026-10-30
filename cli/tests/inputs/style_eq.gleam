import sgleam/fill
import sgleam/image
import sgleam/stroke
import sgleam/style

pub fn smain() {
  echo style.join([fill.red, fill.blue]) == fill.red
  echo style.join([stroke.blue, fill.red])
    == style.join([fill.red, stroke.blue])
  echo style.join([fill.red, style.none]) == fill.red
  echo style.join([]) == style.none
  echo style.join([stroke.width(2), fill.red, stroke.blue, stroke.width(5)])
  echo image.circle(10, style.join([stroke.blue, fill.red]))
    == image.circle(10, style.join([fill.red, stroke.blue]))
  style.scale(style.join([stroke.dashed, stroke.width(2)]), 2.0)
}
