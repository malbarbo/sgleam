import gleam/string
import sgleam/color
import sgleam/fill
import sgleam/stroke
import sgleam/style

pub fn smain() {
  echo #(color.red, color.cyan, color.rgb(10, 20, 30), color.none)
  echo color.rgba(255, 0, 0, 0.5)
  echo #(style.none, fill.none, fill.opacity(5), fill.opacityf(0.3))
  echo [stroke.dashed, stroke.dash_array([1, 2]), stroke.linecap_round]
  echo style.scale(stroke.dash_array([1, 2]), 1.5)
  string.inspect(
    style.join([fill.rgba(1, 2, 3, 0.25), stroke.blue, stroke.widthf(2.5)]),
  )
}
