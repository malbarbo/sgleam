import gleam/list
import sgleam/check.{eq}

const same = check.eq

pub fn unqualified_examples() {
  eq(1, 1)
  eq(1, 2)
}

pub fn argument_examples() {
  list.all([True, False], check.true)
}

pub fn variable_examples() {
  let f = check.false
  f(True)
}

pub fn constant_examples() {
  same(1, 2)
}
