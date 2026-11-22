import sgleam/show/element.{type Element, type LinkRepr}

pub fn new(label: String, uri: String) -> LinkRepr(msg) {
  element.new_link(label, uri)
}

pub fn on_press(link: LinkRepr(msg), message: msg) -> LinkRepr(msg) {
  element.link_on_press(link, message)
}

/// Opens navigation in a new tab when enabled. Defaults to False.
pub fn new_tab(link: LinkRepr(msg), enabled: Bool) -> LinkRepr(msg) {
  element.link_new_tab(link, enabled)
}

pub fn done(link: LinkRepr(msg)) -> Element(msg) {
  element.link_done(link)
}
