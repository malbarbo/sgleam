import gleam/io
import sgleam/image.{
  beside, bitmap_data_uri, flip_horizontal, flip_vertical, to_svg,
}

pub fn main() {
  let pixels =
    bitmap_data_uri(
      "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAFklEQVR4nGP4z8DA8J+BkYHh////DAAe9gT9Ce00PgAAAABJRU5ErkJggg==",
      2.0,
      2.0,
    )
  beside(beside(pixels, flip_horizontal(pixels)), flip_vertical(pixels))
  |> to_svg
  |> io.println
}
