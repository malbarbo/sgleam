import sgleam/ui/element.{type Element, type TextAreaRepr}

pub fn new(value: String) -> TextAreaRepr(msg) {
  element.new_text_area(value)
}

pub fn placeholder(area: TextAreaRepr(msg), value: String) -> TextAreaRepr(msg) {
  element.text_area_placeholder(area, value)
}

pub fn rows(area: TextAreaRepr(msg), value: Int) -> TextAreaRepr(msg) {
  element.text_area_rows(area, value)
}

pub fn on_input(
  area: TextAreaRepr(msg),
  to_message: fn(String) -> msg,
) -> TextAreaRepr(msg) {
  element.text_area_on_input(area, to_message)
}

pub fn enabled(area: TextAreaRepr(msg), value: Bool) -> TextAreaRepr(msg) {
  element.text_area_enabled(area, value)
}

pub fn done(area: TextAreaRepr(msg)) -> Element(msg) {
  element.text_area_done(area)
}
