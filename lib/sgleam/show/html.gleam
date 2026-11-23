import gleam/dict.{type Dict}
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/string

import sgleam/show/element.{
  type Choice, type Element, Button, Checkbox, Column, Link, Picker, RadioGroup,
  Row, Text, TextArea, TextInput,
}
import sgleam/show/event.{type Handler}

pub fn to_html(element: Element(msg)) -> String {
  let #(html, _) = render(element)
  html
}

pub fn render(element: Element(msg)) -> #(String, Dict(Int, Handler(msg))) {
  let #(html, handlers, _) = render_element(element, 0, 0, dict.new())

  #(html, handlers)
}

pub fn to_html_page(element: Element(msg)) -> String {
  let #(html, _, _) = render_element(element, 1, 0, dict.new())

  "<!DOCTYPE html>\n"
  <> "<html lang=\"pt-BR\">\n"
  <> "<head>\n"
  <> "  <meta charset=\"UTF-8\">\n"
  <> "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n"
  <> "  <title>sgleam/show</title>\n"
  <> "</head>\n"
  <> "<body>\n"
  <> html
  <> "\n</body>\n"
  <> "</html>"
}

fn render_element(
  element: Element(msg),
  level: Int,
  next: Int,
  handlers: Dict(Int, Handler(msg)),
) -> #(String, Dict(Int, Handler(msg)), Int) {
  case element {
    Text(repr) -> {
      let content = element.text_data(repr)

      render_text_tag("span", content, level, next, handlers)
    }

    Button(repr) -> {
      let #(label, message, enabled) = element.button_data(repr)

      let #(event_attr, handlers, after) =
        register_message_handler("click", message, next, handlers)

      #(
        indent(level)
          <> "<button"
          <> disabled_attr(enabled)
          <> event_attr
          <> ">"
          <> escape(label)
          <> "</button>",
        handlers,
        after,
      )
    }

    TextInput(repr) -> {
      let #(value, placeholder, on_input, on_submit, enabled) =
        element.text_input_data(repr)

      let #(input_attr, handlers, after_input) =
        register_string_handler("input", on_input, next, handlers)

      let #(submit_attr, handlers, after_submit) =
        register_string_handler("submit", on_submit, after_input, handlers)

      #(
        indent(level)
          <> "<input value=\""
          <> escape(value)
          <> "\""
          <> option_attr("placeholder", placeholder)
          <> disabled_attr(enabled)
          <> input_attr
          <> submit_attr
          <> ">",
        handlers,
        after_submit,
      )
    }

    TextArea(repr) -> {
      let #(value, placeholder, rows, on_input, enabled) =
        element.text_area_data(repr)

      let #(event_attr, handlers, after) =
        register_string_handler("input", on_input, next, handlers)

      #(
        indent(level)
          <> "<textarea"
          <> option_attr("placeholder", placeholder)
          <> " rows=\""
          <> int.to_string(rows)
          <> "\""
          <> disabled_attr(enabled)
          <> event_attr
          <> ">"
          <> escape(value)
          <> "</textarea>",
        handlers,
        after,
      )
    }

    Checkbox(repr) -> {
      let #(checked, on_change, enabled) = element.checkbox_data(repr)

      let #(event_attr, handlers, after) =
        register_bool_handler("check", on_change, next, handlers)

      #(
        indent(level)
          <> "<input type=\"checkbox\""
          <> bool_attr("checked", checked)
          <> disabled_attr(enabled)
          <> event_attr
          <> ">",
        handlers,
        after,
      )
    }

    RadioGroup(repr) -> render_radio_group(repr, level, next, handlers)

    Picker(repr) -> render_picker(repr, level, next, handlers)

    Link(repr) -> {
      let #(label, uri, message) = element.link_data(repr)

      let #(event_attr, handlers, after) =
        register_message_handler("click", message, next, handlers)

      #(
        indent(level)
          <> "<a href=\""
          <> escape(uri)
          <> "\""
          <> case element.link_opens_new_tab(repr) {
          True -> " target=\"_blank\" rel=\"noopener noreferrer\""
          False -> ""
        }
          <> event_attr
          <> ">"
          <> escape(label)
          <> "</a>",
        handlers,
        after,
      )
    }

    Row(repr) -> render_layout(repr, "row", level, next, handlers)

    Column(repr) -> render_layout(repr, "column", level, next, handlers)
  }
}

fn render_text_tag(
  tag: String,
  content: String,
  level: Int,
  next: Int,
  handlers: Dict(Int, Handler(msg)),
) -> #(String, Dict(Int, Handler(msg)), Int) {
  #(
    indent(level) <> "<" <> tag <> ">" <> escape(content) <> "</" <> tag <> ">",
    handlers,
    next,
  )
}

fn render_radio_group(
  repr,
  level: Int,
  next: Int,
  handlers: Dict(Int, Handler(msg)),
) {
  let #(options, enabled, label) = element.radio_group_data(repr)
  // Reserve an id even for empty, disabled, or handlerless groups.
  let name = "sgleam-radio-group-" <> int.to_string(next)
  let #(body, handlers, after) =
    render_radio_options(
      options,
      name,
      enabled,
      level + 1,
      0,
      next + 1,
      handlers,
      False,
      [],
    )
  #(
    indent(level)
      <> "<div role=\"radiogroup\""
      <> option_attr("aria-label", label)
      <> " style=\"display: flex; flex-direction: column; align-items: flex-start; gap: 4px;\">\n"
      <> body
      <> "\n"
      <> indent(level)
      <> "</div>",
    handlers,
    after,
  )
}

// Tail recursive, since a group can hold thousands of options.
fn render_radio_options(
  options: List(#(String, Bool, Option(msg))),
  name: String,
  enabled: Bool,
  level: Int,
  index: Int,
  next: Int,
  handlers: Dict(Int, Handler(msg)),
  selected: Bool,
  done: List(String),
) -> #(String, Dict(Int, Handler(msg)), Int) {
  case options {
    [] -> #(done |> list.reverse |> string.join("\n"), handlers, next)
    [#(label, checked, message), ..rest] -> {
      // At most one checked radio, even if a caller repeats a value.
      let checked = checked && !selected
      let message = case enabled {
        True -> message
        False -> None
      }
      let #(attr, handlers, after) =
        register_radio_handler(message, next, handlers)
      let item =
        indent(level)
        <> "<label><input type=\"radio\" name=\""
        <> name
        <> "\" value=\""
        <> int.to_string(index)
        <> "\""
        <> bool_attr("checked", checked)
        <> disabled_attr(enabled)
        <> attr
        <> "><span>"
        <> escape(label)
        <> "</span></label>"
      render_radio_options(
        rest,
        name,
        enabled,
        level,
        index + 1,
        after,
        handlers,
        selected || checked,
        [item, ..done],
      )
    }
  }
}

fn render_picker(
  repr,
  level: Int,
  next: Int,
  handlers: Dict(Int, Handler(msg)),
) {
  let #(choices, selected, on_select, enabled) = element.picker_data(repr)

  let #(event_attr, handlers, after) =
    register_string_handler("select", on_select, next, handlers)

  let options =
    choices
    |> list.map(render_choice(_, selected, level + 1))
    |> string.join("\n")

  #(
    indent(level)
      <> "<select"
      <> disabled_attr(enabled)
      <> event_attr
      <> ">\n"
      <> options
      <> "\n"
      <> indent(level)
      <> "</select>",
    handlers,
    after,
  )
}

fn render_choice(
  choice: Choice,
  selected: Option(String),
  level: Int,
) -> String {
  let #(label, value) = element.choice_data(choice)

  let is_selected = case selected {
    Some(current) -> current == value
    None -> False
  }

  indent(level)
  <> "<option value=\""
  <> escape(value)
  <> "\""
  <> bool_attr("selected", is_selected)
  <> ">"
  <> escape(label)
  <> "</option>"
}

fn render_layout(
  repr,
  direction: String,
  level: Int,
  next: Int,
  handlers: Dict(Int, Handler(msg)),
) {
  let children = element.layout_data(repr)

  let #(body, handlers, after) =
    render_children(children, level + 1, next, handlers, [])

  #(
    indent(level)
      <> "<div style=\"display: flex; gap: 4px; box-sizing: border-box; flex-direction: "
      <> direction
      <> case direction {
      "column" -> "; align-items: flex-start"
      _ -> ""
    }
      <> ";\">\n"
      <> body
      <> "\n"
      <> indent(level)
      <> "</div>",
    handlers,
    after,
  )
}

// Tail recursive, since a column can hold thousands of children.
fn render_children(
  children: List(Element(msg)),
  level: Int,
  next: Int,
  handlers: Dict(Int, Handler(msg)),
  done: List(String),
) -> #(String, Dict(Int, Handler(msg)), Int) {
  case children {
    [] -> #(done |> list.reverse |> string.join("\n"), handlers, next)

    [child, ..rest] -> {
      let #(html, handlers, after) =
        render_element(child, level, next, handlers)

      render_children(rest, level, after, handlers, [html, ..done])
    }
  }
}

fn register_message_handler(
  name: String,
  message: Option(msg),
  next: Int,
  handlers: Dict(Int, Handler(msg)),
) {
  let handler = option.map(message, event.constant)

  register_handler(name, handler, next, handlers)
}

fn register_string_handler(
  name: String,
  to_message: Option(fn(String) -> msg),
  next: Int,
  handlers: Dict(Int, Handler(msg)),
) {
  let handler = option.map(to_message, event.from_string)

  register_handler(name, handler, next, handlers)
}

fn register_bool_handler(
  name: String,
  to_message: Option(fn(Bool) -> msg),
  next: Int,
  handlers: Dict(Int, Handler(msg)),
) {
  let handler = option.map(to_message, event.from_bool)

  register_handler(name, handler, next, handlers)
}

fn register_radio_handler(
  message: Option(msg),
  next: Int,
  handlers: Dict(Int, Handler(msg)),
) {
  let handler =
    option.map(message, fn(message) {
      event.from_string(fn(_value) { message })
    })

  register_handler("select", handler, next, handlers)
}

fn register_handler(
  name: String,
  handler: Option(Handler(msg)),
  next: Int,
  handlers: Dict(Int, Handler(msg)),
) -> #(String, Dict(Int, Handler(msg)), Int) {
  case handler {
    None -> #("", handlers, next)

    Some(handler) -> {
      let handlers = dict.insert(handlers, next, handler)

      #(
        " data-on-" <> name <> "=\"" <> int.to_string(next) <> "\"",
        handlers,
        next + 1,
      )
    }
  }
}

fn option_attr(name: String, value: Option(String)) -> String {
  case value {
    None -> ""

    Some(value) -> " " <> name <> "=\"" <> escape(value) <> "\""
  }
}

fn bool_attr(name: String, value: Bool) -> String {
  case value {
    True -> " " <> name
    False -> ""
  }
}

fn disabled_attr(enabled: Bool) -> String {
  case enabled {
    True -> ""
    False -> " disabled"
  }
}

fn indent(level: Int) -> String {
  string.repeat(" ", level * 2)
}

fn escape(value: String) -> String {
  value
  |> string.replace("&", "&amp;")
  |> string.replace("<", "&lt;")
  |> string.replace(">", "&gt;")
  |> string.replace("\"", "&quot;")
  |> string.replace("'", "&#39;")
}
