import gleam/dict.{type Dict}
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/string

import sgleam/ui/element.{
  type Choice, type Element, Button, Checkbox, Column, Link, Picker, Radio, Row,
  Text, TextArea, TextInput,
}
import sgleam/ui/event.{type Handler}

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
  <> "  <title>sgleam/ui</title>\n"
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
        register_message_handler("press", message, next, handlers)

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
        register_bool_handler("toggle", on_change, next, handlers)

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

    Radio(repr) -> {
      let #(group, value, checked, message, enabled) = element.radio_data(repr)

      let #(event_attr, handlers, after) =
        register_radio_handler(message, next, handlers)

      #(
        indent(level)
          <> "<input type=\"radio\""
          <> " name=\""
          <> escape(group)
          <> "\""
          <> " value=\""
          <> escape(value)
          <> "\""
          <> bool_attr("checked", checked)
          <> disabled_attr(enabled)
          <> event_attr
          <> ">",
        handlers,
        after,
      )
    }

    Picker(repr) -> render_picker(repr, level, next, handlers)

    Link(repr) -> {
      let #(label, uri, message) = element.link_data(repr)

      let #(event_attr, handlers, after) =
        register_message_handler("press", message, next, handlers)

      #(
        indent(level)
          <> "<a href=\""
          <> escape(uri)
          <> "\""
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

fn render_picker(repr, level: Int, next: Int, handlers: Dict(Int, Handler(msg))) {
  let #(choices, selected, on_select, enabled) = element.picker_data(repr)

  let #(event_attr, handlers, after) =
    register_string_handler("selection", on_select, next, handlers)

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

fn render_choice(choice: Choice, selected: Option(String), level: Int) -> String {
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
    render_children(children, level + 1, next, handlers)

  #(
    indent(level)
      <> "<div style=\"display: flex; box-sizing: border-box; flex-direction: "
      <> direction
      <> ";\">\n"
      <> body
      <> "\n"
      <> indent(level)
      <> "</div>",
    handlers,
    after,
  )
}

fn render_children(
  children: List(Element(msg)),
  level: Int,
  next: Int,
  handlers: Dict(Int, Handler(msg)),
) {
  case children {
    [] -> #("", handlers, next)

    [child, ..rest] -> {
      let #(html, handlers, after_child) =
        render_element(child, level, next, handlers)

      let #(rest_html, handlers, after) =
        render_children(rest, level, after_child, handlers)

      let separator = case rest_html {
        "" -> ""
        _ -> "\n"
      }

      #(html <> separator <> rest_html, handlers, after)
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

  register_handler("selection", handler, next, handlers)
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
        " data-sgleam-" <> name <> "=\"" <> int.to_string(next) <> "\"",
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
