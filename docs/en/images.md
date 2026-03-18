# Images

The `sgleam/image`{.gleam} module allows you to create, combine, and transform images. Images are rendered as SVG.


## Imports

```gleam
import sgleam/image
import sgleam/fill
import sgleam/stroke
import sgleam/style
```


## Basic shapes

### Rectangles and squares

```gleam
image.rectangle(100, 50, fill.blue)
```

![](img:rectangle)

```gleam
image.square(80, fill.red)
```

![](img:square)

### Circles and ellipses

```gleam
image.circle(40, fill.green)
```

![](img:circle)

```gleam
image.ellipse(60, 30, fill.yellow)
```

![](img:ellipse)

### Triangles

```gleam
image.triangle(80, fill.orange)
```

![](img:triangle)

```gleam
image.right_triangle(60, 40, fill.purple)
```

![](img:right_triangle)

### Other shapes

```gleam
image.rhombus(60, 45, fill.red)
```

![](img:rhombus)

```gleam
image.regular_polygon(40, 6, fill.blue)  // hexagon
```

![](img:hexagon)

```gleam
image.star(50, fill.gold)
```

![](img:star)

```gleam
image.radial_star(5, 20, 50, fill.orange)
```

![](img:radial_star)


## Int and Float variants

All functions that take dimensions have two versions:

- `Int`{.gleam} version: `image.circle(40, fill.red)`{.gleam}
- `Float`{.gleam} version (suffix `f`): `image.circlef(40.5, fill.red)`{.gleam}


## Text

```gleam
image.text("Hello!", 24, fill.black)
```

![](img:text)


## Styling

### Fill

```gleam
fill.red                    // named color
fill.rgb(255, 128, 0)       // RGB color
fill.rgba(0, 0, 255, 0.5)   // color with transparency
fill.none                   // no fill
```

### Stroke (outline)

```gleam
stroke.black                  // named color
stroke.width(3)               // stroke width
stroke.rgb(255, 0, 0)         // RGB color
stroke.dash_array([5, 3])     // dashed line
stroke.none                   // no stroke
```

### Combining styles

Use `style.join`{.gleam} to combine fill and stroke:

```gleam
image.circle(40, style.join([fill.red, stroke.black, stroke.width(2)]))
```

![](img:style_join)


## Combining images

### Side by side (beside)

```gleam
image.beside(
  image.square(40, fill.red),
  image.square(40, fill.blue),
)
```

![](img:beside)

### Stacked (above)

```gleam
image.above(
  image.square(40, fill.red),
  image.square(40, fill.blue),
)
```

![](img:above)

### Overlaid (overlay)

```gleam
image.overlay(
  image.circle(20, fill.red),
  image.square(60, fill.blue),
)
```

![](img:overlay)

### Combining a list of images

```gleam
image.combine(
  [image.square(40, fill.red), image.square(40, fill.blue), image.square(40, fill.green)],
  image.beside,
)
```


## Scenes

Scenes are images with a fixed coordinate system. The y-axis increases downward.

```gleam
image.empty_scene(200, 200)
|> image.place_image(100, 100, image.circle(20, fill.red))
```

![](img:scene)

The `place_image`{.gleam} function positions an image by its center. Use `place_image_align`{.gleam} to control alignment.


## Transformations

```gleam
image.rotate(img, 45)            // rotate 45 degrees
image.scale(img, 2)              // double the size
image.scale_xy(img, 2, 1)        // scale differently in x and y
image.flip_horizontal(img)       // mirror horizontally
image.flip_vertical(img)         // mirror vertically
image.crop(img, 10, 10, 80, 80)  // crop
```


## Properties

```gleam
image.width(img)      // width (Int)
image.height(img)     // height (Int)
image.dimension(img)  // #(width, height)
image.center(img)     // #(x, y) of center
```


## Coordinate system

In sgleam, the coordinate system follows the standard screen convention:

- The point (0, 0) is at the top-left corner
- The x-axis increases to the right
- The y-axis increases downward
