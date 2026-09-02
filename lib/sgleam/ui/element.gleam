import gleam/list
import gleam/option.{type Option, None, Some}

pub type Element(msg) {
  Text(TextRepr)
  Button(ButtonRepr(msg))
  TextInput(TextInputRepr(msg))
  TextArea(TextAreaRepr(msg))
  Checkbox(CheckboxRepr(msg))
  Radio(RadioRepr(msg))
  Picker(PickerRepr(msg))
  Link(LinkRepr(msg))
  Row(LayoutRepr(msg))
  Column(LayoutRepr(msg))
}

pub opaque type TextRepr {
  TextRepr(content: String)
}

pub opaque type ButtonRepr(msg) {
  ButtonRepr(label: String, on_press: Option(msg), enabled: Bool)
}

pub opaque type TextInputRepr(msg) {
  TextInputRepr(
    value: String,
    placeholder: Option(String),
    on_input: Option(fn(String) -> msg),
    on_submit: Option(fn(String) -> msg),
    enabled: Bool,
  )
}

pub opaque type TextAreaRepr(msg) {
  TextAreaRepr(
    value: String,
    placeholder: Option(String),
    rows: Int,
    on_input: Option(fn(String) -> msg),
    enabled: Bool,
  )
}

pub opaque type CheckboxRepr(msg) {
  CheckboxRepr(checked: Bool, on_change: Option(fn(Bool) -> msg), enabled: Bool)
}

pub opaque type RadioRepr(msg) {
  RadioRepr(
    group: String,
    value: String,
    checked: Bool,
    on_select: Option(msg),
    enabled: Bool,
  )
}

pub opaque type Choice {
  Choice(label: String, value: String)
}

pub opaque type PickerRepr(msg) {
  PickerRepr(
    choices: List(Choice),
    selected: Option(String),
    on_select: Option(fn(String) -> msg),
    enabled: Bool,
  )
}

pub opaque type LinkRepr(msg) {
  LinkRepr(label: String, uri: String, on_press: Option(msg))
}

type LayoutDirection {
  Horizontal
  Vertical
}

pub opaque type LayoutRepr(msg) {
  LayoutRepr(children: List(Element(msg)), direction: LayoutDirection)
}

// Text

pub fn new_text(content: String) -> TextRepr {
  TextRepr(content: content)
}

pub fn text_done(repr: TextRepr) -> Element(msg) {
  Text(repr)
}

pub fn text_data(repr: TextRepr) -> String {
  repr.content
}

// Button

pub fn new_button(label: String) -> ButtonRepr(msg) {
  ButtonRepr(label: label, on_press: None, enabled: True)
}

pub fn button_on_press(repr: ButtonRepr(msg), message: msg) -> ButtonRepr(msg) {
  ButtonRepr(..repr, on_press: Some(message))
}

pub fn button_enabled(repr: ButtonRepr(msg), enabled: Bool) -> ButtonRepr(msg) {
  ButtonRepr(..repr, enabled: enabled)
}

pub fn button_done(repr: ButtonRepr(msg)) -> Element(msg) {
  Button(repr)
}

pub fn button_data(repr: ButtonRepr(msg)) -> #(String, Option(msg), Bool) {
  #(repr.label, repr.on_press, repr.enabled)
}

// Text input

pub fn new_text_input(value: String) -> TextInputRepr(msg) {
  TextInputRepr(
    value: value,
    placeholder: None,
    on_input: None,
    on_submit: None,
    enabled: True,
  )
}

pub fn text_input_placeholder(
  repr: TextInputRepr(msg),
  placeholder: String,
) -> TextInputRepr(msg) {
  TextInputRepr(..repr, placeholder: Some(placeholder))
}

pub fn text_input_on_input(
  repr: TextInputRepr(msg),
  to_message: fn(String) -> msg,
) -> TextInputRepr(msg) {
  TextInputRepr(..repr, on_input: Some(to_message))
}

pub fn text_input_on_submit(
  input: TextInputRepr(msg),
  to_message: fn(String) -> msg,
) -> TextInputRepr(msg) {
  TextInputRepr(..input, on_submit: Some(to_message))
}

pub fn text_input_enabled(
  repr: TextInputRepr(msg),
  enabled: Bool,
) -> TextInputRepr(msg) {
  TextInputRepr(..repr, enabled: enabled)
}

pub fn text_input_done(repr: TextInputRepr(msg)) -> Element(msg) {
  TextInput(repr)
}

pub fn text_input_data(
  repr: TextInputRepr(msg),
) -> #(
  String,
  Option(String),
  Option(fn(String) -> msg),
  Option(fn(String) -> msg),
  Bool,
) {
  #(repr.value, repr.placeholder, repr.on_input, repr.on_submit, repr.enabled)
}

// Text area

pub fn new_text_area(value: String) -> TextAreaRepr(msg) {
  TextAreaRepr(
    value: value,
    placeholder: None,
    rows: 3,
    on_input: None,
    enabled: True,
  )
}

pub fn text_area_placeholder(
  repr: TextAreaRepr(msg),
  placeholder: String,
) -> TextAreaRepr(msg) {
  TextAreaRepr(..repr, placeholder: Some(placeholder))
}

pub fn text_area_rows(repr: TextAreaRepr(msg), rows: Int) -> TextAreaRepr(msg) {
  TextAreaRepr(..repr, rows: rows)
}

pub fn text_area_on_input(
  repr: TextAreaRepr(msg),
  to_message: fn(String) -> msg,
) -> TextAreaRepr(msg) {
  TextAreaRepr(..repr, on_input: Some(to_message))
}

pub fn text_area_enabled(
  repr: TextAreaRepr(msg),
  enabled: Bool,
) -> TextAreaRepr(msg) {
  TextAreaRepr(..repr, enabled: enabled)
}

pub fn text_area_done(repr: TextAreaRepr(msg)) -> Element(msg) {
  TextArea(repr)
}

pub fn text_area_data(
  repr: TextAreaRepr(msg),
) -> #(String, Option(String), Int, Option(fn(String) -> msg), Bool) {
  #(repr.value, repr.placeholder, repr.rows, repr.on_input, repr.enabled)
}

// Checkbox

pub fn new_checkbox(checked: Bool) -> CheckboxRepr(msg) {
  CheckboxRepr(checked: checked, on_change: None, enabled: True)
}

pub fn checkbox_on_change(
  repr: CheckboxRepr(msg),
  to_message: fn(Bool) -> msg,
) -> CheckboxRepr(msg) {
  CheckboxRepr(..repr, on_change: Some(to_message))
}

pub fn checkbox_enabled(
  repr: CheckboxRepr(msg),
  enabled: Bool,
) -> CheckboxRepr(msg) {
  CheckboxRepr(..repr, enabled: enabled)
}

pub fn checkbox_done(repr: CheckboxRepr(msg)) -> Element(msg) {
  Checkbox(repr)
}

pub fn checkbox_data(
  repr: CheckboxRepr(msg),
) -> #(Bool, Option(fn(Bool) -> msg), Bool) {
  #(repr.checked, repr.on_change, repr.enabled)
}

// Radio

pub fn new_radio(group: String, value: String, checked: Bool) -> RadioRepr(msg) {
  RadioRepr(
    group: group,
    value: value,
    checked: checked,
    on_select: None,
    enabled: True,
  )
}

pub fn radio_on_select(repr: RadioRepr(msg), message: msg) -> RadioRepr(msg) {
  RadioRepr(..repr, on_select: Some(message))
}

pub fn radio_enabled(repr: RadioRepr(msg), enabled: Bool) -> RadioRepr(msg) {
  RadioRepr(..repr, enabled: enabled)
}

pub fn radio_done(repr: RadioRepr(msg)) -> Element(msg) {
  Radio(repr)
}

pub fn radio_data(
  repr: RadioRepr(msg),
) -> #(String, String, Bool, Option(msg), Bool) {
  #(repr.group, repr.value, repr.checked, repr.on_select, repr.enabled)
}

// Choice

pub fn new_choice(label: String, value: String) -> Choice {
  Choice(label: label, value: value)
}

pub fn choice_data(choice: Choice) -> #(String, String) {
  #(choice.label, choice.value)
}

// Picker

pub fn new_picker(choices: List(Choice)) -> PickerRepr(msg) {
  PickerRepr(choices: choices, selected: None, on_select: None, enabled: True)
}

pub fn picker_selected(
  repr: PickerRepr(msg),
  selected: String,
) -> PickerRepr(msg) {
  PickerRepr(..repr, selected: Some(selected))
}

pub fn picker_on_select(
  repr: PickerRepr(msg),
  to_message: fn(String) -> msg,
) -> PickerRepr(msg) {
  PickerRepr(..repr, on_select: Some(to_message))
}

pub fn picker_enabled(repr: PickerRepr(msg), enabled: Bool) -> PickerRepr(msg) {
  PickerRepr(..repr, enabled: enabled)
}

pub fn picker_done(repr: PickerRepr(msg)) -> Element(msg) {
  Picker(repr)
}

pub fn picker_data(
  repr: PickerRepr(msg),
) -> #(List(Choice), Option(String), Option(fn(String) -> msg), Bool) {
  #(repr.choices, repr.selected, repr.on_select, repr.enabled)
}

// Link

pub fn new_link(label: String, uri: String) -> LinkRepr(msg) {
  LinkRepr(label: label, uri: uri, on_press: None)
}

pub fn link_on_press(repr: LinkRepr(msg), message: msg) -> LinkRepr(msg) {
  LinkRepr(..repr, on_press: Some(message))
}

pub fn link_done(repr: LinkRepr(msg)) -> Element(msg) {
  Link(repr)
}

pub fn link_data(repr: LinkRepr(msg)) -> #(String, String, Option(msg)) {
  #(repr.label, repr.uri, repr.on_press)
}

// Layout

pub fn new_row(children: List(Element(msg))) -> LayoutRepr(msg) {
  LayoutRepr(children: children, direction: Horizontal)
}

pub fn new_column(children: List(Element(msg))) -> LayoutRepr(msg) {
  LayoutRepr(children: children, direction: Vertical)
}

pub fn layout_done(repr: LayoutRepr(msg)) -> Element(msg) {
  case repr.direction {
    Horizontal -> Row(repr)
    Vertical -> Column(repr)
  }
}

pub fn layout_data(repr: LayoutRepr(msg)) -> List(Element(msg)) {
  repr.children
}

// Message mapping

fn map_string_handler(
  handler: Option(fn(String) -> a),
  transform: fn(a) -> b,
) -> Option(fn(String) -> b) {
  option.map(handler, fn(to_message) {
    fn(value) { transform(to_message(value)) }
  })
}

fn map_bool_handler(
  handler: Option(fn(Bool) -> a),
  transform: fn(a) -> b,
) -> Option(fn(Bool) -> b) {
  option.map(handler, fn(to_message) {
    fn(value) { transform(to_message(value)) }
  })
}

pub fn map(element: Element(a), transform: fn(a) -> b) -> Element(b) {
  case element {
    Text(repr) -> Text(repr)

    Button(repr) ->
      Button(ButtonRepr(..repr, on_press: option.map(repr.on_press, transform)))

    TextInput(repr) ->
      TextInput(
        TextInputRepr(
          ..repr,
          on_input: map_string_handler(repr.on_input, transform),
          on_submit: map_string_handler(repr.on_submit, transform),
        ),
      )

    TextArea(repr) ->
      TextArea(
        TextAreaRepr(
          ..repr,
          on_input: map_string_handler(repr.on_input, transform),
        ),
      )

    Checkbox(repr) ->
      Checkbox(
        CheckboxRepr(
          ..repr,
          on_change: map_bool_handler(repr.on_change, transform),
        ),
      )

    Radio(repr) ->
      Radio(RadioRepr(..repr, on_select: option.map(repr.on_select, transform)))

    Picker(repr) ->
      Picker(
        PickerRepr(
          ..repr,
          on_select: map_string_handler(repr.on_select, transform),
        ),
      )

    Link(repr) ->
      Link(LinkRepr(..repr, on_press: option.map(repr.on_press, transform)))

    Row(repr) ->
      Row(
        LayoutRepr(..repr, children: list.map(repr.children, map(_, transform))),
      )

    Column(repr) ->
      Column(
        LayoutRepr(..repr, children: list.map(repr.children, map(_, transform))),
      )
  }
}
