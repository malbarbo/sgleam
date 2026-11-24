import sgleam/check
import tests/coverage/words

pub fn sign(n: Int) -> String {
  case n < 0 {
    True -> words.negative()
    False -> "positive"
  }
}

pub fn sign_examples() {
  check.eq(sign(-3), "negative")
}
