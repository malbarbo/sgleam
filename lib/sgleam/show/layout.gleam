import sgleam/show/element.{type Element, type LayoutRepr}

pub fn row(children: List(Element(msg))) -> LayoutRepr(msg) {
  element.new_row(children)
}

/// Stacks children vertically, aligned to the start at their natural width.
pub fn column(children: List(Element(msg))) -> LayoutRepr(msg) {
  element.new_column(children)
}

pub fn done(layout: LayoutRepr(msg)) -> Element(msg) {
  element.layout_done(layout)
}
