import sgleam/ui/element.{type Element, type TextRepr}

pub fn new(content: String) -> TextRepr {
  element.new_text(content)
}

pub fn done(text: TextRepr) -> Element(msg) {
  element.text_done(text)
}
