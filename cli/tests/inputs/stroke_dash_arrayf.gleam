import sgleam/stroke

pub fn smain() {
  echo stroke.dash_arrayf([2.0, 3.0]) == stroke.dash_array([2, 3])
  echo stroke.dash_arrayf([-1.5, 2.5]) == stroke.dash_arrayf([0.0, 2.5])
}
