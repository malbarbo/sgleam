import gleam/io
import sgleam/image.{bitmap_data_uri, to_svg}

pub fn main() {
  bitmap_data_uri(
    "data:image/bmp;base64,Qk06AAAAAAAAADYAAAAoAAAAAQAAAAEAAAABABgAAAAAAAQAAAATCwAAEwsAAAAAAAAAAAAAAAD/AA==",
  )
  |> to_svg
  |> io.println
}
