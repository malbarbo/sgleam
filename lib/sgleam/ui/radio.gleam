import sgleam/ui/element.{type Element, type RadioRepr}

pub fn new(group: String, value: String, checked: Bool) -> RadioRepr(msg) {
  element.new_radio(group, value, checked)
}

pub fn on_select(radio: RadioRepr(msg), message: msg) -> RadioRepr(msg) {
  element.radio_on_select(radio, message)
}

pub fn enabled(radio: RadioRepr(msg), value: Bool) -> RadioRepr(msg) {
  element.radio_enabled(radio, value)
}

pub fn done(radio: RadioRepr(msg)) -> Element(msg) {
  element.radio_done(radio)
}
