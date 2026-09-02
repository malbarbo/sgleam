import sgleam/ui/element.{type CheckboxRepr, type Element}

pub fn new(checked: Bool) -> CheckboxRepr(msg) {
  element.new_checkbox(checked)
}

pub fn on_change(
  checkbox: CheckboxRepr(msg),
  to_message: fn(Bool) -> msg,
) -> CheckboxRepr(msg) {
  element.checkbox_on_change(checkbox, to_message)
}

pub fn enabled(checkbox: CheckboxRepr(msg), value: Bool) -> CheckboxRepr(msg) {
  element.checkbox_enabled(checkbox, value)
}

pub fn done(checkbox: CheckboxRepr(msg)) -> Element(msg) {
  element.checkbox_done(checkbox)
}
