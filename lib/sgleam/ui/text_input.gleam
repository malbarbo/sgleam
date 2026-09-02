import sgleam/ui/element.{type Element, type TextInputRepr}

pub fn new(value: String) -> TextInputRepr(msg) {
  element.new_text_input(value)
}

pub fn placeholder(
  input: TextInputRepr(msg),
  value: String,
) -> TextInputRepr(msg) {
  element.text_input_placeholder(input, value)
}

pub fn on_input(
  input: TextInputRepr(msg),
  to_message: fn(String) -> msg,
) -> TextInputRepr(msg) {
  element.text_input_on_input(input, to_message)
}

pub fn on_submit(
  input: TextInputRepr(msg),
  to_message: fn(String) -> msg,
) -> TextInputRepr(msg) {
  element.text_input_on_submit(input, to_message)
}

pub fn enabled(input: TextInputRepr(msg), value: Bool) -> TextInputRepr(msg) {
  element.text_input_enabled(input, value)
}

pub fn done(input: TextInputRepr(msg)) -> Element(msg) {
  element.text_input_done(input)
}
