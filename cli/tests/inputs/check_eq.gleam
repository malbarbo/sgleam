import sgleam/check

fn ignore_examples() {
  check.eq(1 + 1, 2)
}

pub fn string_examples() {
  check.eq("string" <> "examples", "stringexamples")
  check.eq("wrong" <> " string", "not this")
}

pub fn arithmetic_examples() {
  check.eq(4 + 2 * 5, 14)
  check.eq(1.2 /. 0.0, 0.0)
}
