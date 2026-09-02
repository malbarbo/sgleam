import sgleam/ui/element.{type ButtonRepr, type Element}

pub fn new(label: String) -> ButtonRepr(msg) {
  element.new_button(label)
}

pub fn on_press(button: ButtonRepr(msg), message: msg) -> ButtonRepr(msg) {
  element.button_on_press(button, message)
}

pub fn enabled(button: ButtonRepr(msg), value: Bool) -> ButtonRepr(msg) {
  element.button_enabled(button, value)
}

pub fn done(button: ButtonRepr(msg)) -> Element(msg) {
  element.button_done(button)
}
