import gleam/io
import gleam/string

pub fn main() {
  let n = 300
  let unit = 2
  show(<<1, 2>>)
  show(<<n:size(16)>>)
  show(<<n:size(unit)-unit(8)>>)
  show(<<1.5:float-size(32)>>)
  show(<<"hi":utf16>>)

  show(match_byte(<<7, 8>>))
  show(match_sized(<<1, 2>>))
  show(match_signed(<<255>>))
  show(match_var_size(<<1, 2>>, 16))
  show(match_rest(<<10, 20, 30>>))
  show(match_len_prefixed(<<2, 9, 9, 7>>))
  show(match_bits(<<0b11_000000>>))
  show(match_literal(<<1, 42>>))
}

fn match_byte(b: BitArray) -> Int {
  case b {
    <<x, _>> -> x
    _ -> -1
  }
}

fn match_sized(b: BitArray) -> Int {
  case b {
    <<x:size(16)>> -> x
    _ -> -1
  }
}

fn match_signed(b: BitArray) -> Int {
  case b {
    <<x:size(8)-signed>> -> x
    _ -> -1
  }
}

fn match_var_size(b: BitArray, size: Int) -> Int {
  case b {
    <<x:size(size)>> -> x
    _ -> -1
  }
}

fn match_rest(b: BitArray) -> BitArray {
  case b {
    <<_, rest:bits>> -> rest
    _ -> <<>>
  }
}

fn match_len_prefixed(b: BitArray) -> BitArray {
  case b {
    <<len, payload:size(len)-unit(8)-bytes, _:bits>> -> payload
    _ -> <<>>
  }
}

fn match_bits(b: BitArray) -> Int {
  case b {
    <<a:size(2), _:size(6)>> -> a
    _ -> -1
  }
}

fn match_literal(b: BitArray) -> Int {
  case b {
    <<1, x>> -> x
    _ -> -1
  }
}

fn show(value: a) -> Nil {
  io.println(string.inspect(value))
}
