import sgleam/ui/element.{type Choice, type Element, type PickerRepr}

pub fn choice(label: String, value: String) -> Choice {
  element.new_choice(label, value)
}

pub fn new(choices: List(Choice)) -> PickerRepr(msg) {
  element.new_picker(choices)
}

pub fn selected(picker: PickerRepr(msg), value: String) -> PickerRepr(msg) {
  element.picker_selected(picker, value)
}

pub fn on_select(
  picker: PickerRepr(msg),
  to_message: fn(String) -> msg,
) -> PickerRepr(msg) {
  element.picker_on_select(picker, to_message)
}

pub fn enabled(picker: PickerRepr(msg), value: Bool) -> PickerRepr(msg) {
  element.picker_enabled(picker, value)
}

pub fn done(picker: PickerRepr(msg)) -> Element(msg) {
  element.picker_done(picker)
}
