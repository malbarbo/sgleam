//// What `echo` and `string.inspect` print on the WebAssembly target. The
//// compilation of the user's package calls `print` for each `echo`, and
//// `show` for each `string.inspect`.

import gleam/dynamic.{type Dynamic}
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/string
import gleam/wasm/reflect.{type Field, Field}
import sgleam/color.{type Color}
import sgleam/style
import sgleam/wasi

/// Prints the place of an `echo`, its message, and `value` on stdout, as
/// the JavaScript target prints them.
pub fn print(
  file: String,
  line: Int,
  message: Option(String),
  value: Dynamic,
) -> Nil {
  // The path of a module starts at the root of the package, and the user
  // gave the path from `src`.
  let file = case file {
    "src/" <> file -> file
    file -> file
  }
  let message = case message {
    Some(message) -> " " <> message
    None -> ""
  }
  wasi.println(file <> ":" <> int.to_string(line) <> message)
  wasi.println(show(value))
}

/// The text of `value`, as the JavaScript target shows it. A color or a
/// style shows as an expression the user could write, at any depth.
pub fn show(value: Dynamic) -> String {
  case reflect.view(value) {
    reflect.Record(type_: reflect.Type("sgleam/color", "Color"), ..) ->
      color.to_source(to_color(value))
    reflect.Record(type_: reflect.Type("sgleam/style", "Style"), ..) ->
      style.to_source(to_style(value))
    reflect.Record(constructor:, fields: [_, ..] as fields, ..) ->
      constructor
      <> "("
      <> string.join(list.map(fields, show_field), ", ")
      <> ")"
    reflect.Tuple(items) ->
      "#(" <> string.join(list.map(items, show), ", ") <> ")"
    reflect.Cons(..) ->
      "[" <> string.join(list.map(items(value), show), ", ") <> "]"
    reflect.Entry(..) -> {
      // The runtime sorts the entries by the text of their keys, so the
      // text does not depend on the order of insertion.
      let entries =
        entries(value)
        |> list.map(fn(entry) { #(show(entry.0), show(entry.1)) })
        |> list.sort(fn(a, b) { string.compare(a.0, b.0) })
        |> list.map(fn(entry) { "#(" <> entry.0 <> ", " <> entry.1 <> ")" })
      "dict.from_list([" <> string.join(entries, ", ") <> "])"
    }
    _ -> reflect.inspect(value)
  }
}

fn show_field(field: Field) -> String {
  case field {
    Field(label: Some(label), value:) -> label <> ": " <> show(value)
    Field(label: None, value:) -> show(value)
  }
}

/// The items of the list `value`.
fn items(value: Dynamic) -> List(Dynamic) {
  case reflect.view(value) {
    reflect.Cons(first:, rest:) -> [first, ..items(rest)]
    _ -> []
  }
}

/// The keys and the values of the dict `value`.
fn entries(value: Dynamic) -> List(#(Dynamic, Dynamic)) {
  case reflect.view(value) {
    reflect.Entry(key:, value:, rest:) -> [#(key, value), ..entries(rest)]
    _ -> []
  }
}

// The functions below read a value of a type of sgleam, whose fields the
// type fixes, so a field of another kind is a bug of this module.

fn to_color(value: Dynamic) -> Color {
  case reflect.view(value) {
    reflect.Record(constructor: "Rgb", fields: [r, g, b, a], ..) ->
      color.rgba(
        to_int(r.value),
        to_int(g.value),
        to_int(b.value),
        to_float(a.value),
      )
    _ -> color.none
  }
}

fn to_style(value: Dynamic) -> style.Style {
  let assert reflect.Record(fields: [f1, f2, f3, f4, f5, f6, f7, f8, f9], ..) =
    reflect.view(value)
  style.new(
    to_option(f1.value, to_color),
    to_option(f2.value, to_float),
    to_option(f3.value, to_string),
    to_option(f4.value, to_color),
    to_option(f5.value, to_string),
    to_option(f6.value, to_string),
    to_option(f7.value, to_float),
    to_option(f8.value, to_float),
    to_option(f9.value, fn(value) { list.map(items(value), to_float) }),
  )
}

fn to_option(value: Dynamic, to: fn(Dynamic) -> a) -> Option(a) {
  case reflect.view(value) {
    reflect.Record(constructor: "Some", fields: [field], ..) ->
      Some(to(field.value))
    _ -> None
  }
}

fn to_int(value: Dynamic) -> Int {
  let assert reflect.Int(n) = reflect.view(value)
  n
}

fn to_float(value: Dynamic) -> Float {
  let assert reflect.Float(x) = reflect.view(value)
  x
}

fn to_string(value: Dynamic) -> String {
  let assert reflect.String(text) = reflect.view(value)
  text
}
