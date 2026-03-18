# Imagens

O módulo `sgleam/image`{.gleam} permite criar, combinar e transformar imagens. As imagens são renderizadas como SVG.


## Importações

```gleam
import sgleam/image
import sgleam/fill
import sgleam/stroke
import sgleam/style
```


## Formas básicas

### Retângulos e quadrados

```gleam
image.rectangle(100, 50, fill.blue)
```

![](img:rectangle)

```gleam
image.square(80, fill.red)
```

![](img:square)

### Círculos e elipses

```gleam
image.circle(40, fill.green)
```

![](img:circle)

```gleam
image.ellipse(60, 30, fill.yellow)
```

![](img:ellipse)

### Triângulos

```gleam
image.triangle(80, fill.orange)
```

![](img:triangle)

```gleam
image.right_triangle(60, 40, fill.purple)
```

![](img:right_triangle)

### Outras formas

```gleam
image.rhombus(60, 45, fill.red)
```

![](img:rhombus)

```gleam
image.regular_polygon(40, 6, fill.blue)  // hexágono
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


## Variantes Int e Float

Todas as funções que recebem dimensões possuem duas versões:

- Versão `Int`{.gleam}: `image.circle(40, fill.red)`{.gleam}
- Versão `Float`{.gleam} (sufixo `f`): `image.circlef(40.5, fill.red)`{.gleam}


## Texto

```gleam
image.text("Olá!", 24, fill.black)
```

![](img:text)


## Estilização

### Preenchimento (fill)

```gleam
fill.red                    // cor nomeada
fill.rgb(255, 128, 0)       // cor RGB
fill.rgba(0, 0, 255, 0.5)   // cor com transparência
fill.none                   // sem preenchimento
```

### Contorno (stroke)

```gleam
stroke.black                  // cor nomeada
stroke.width(3)               // largura do contorno
stroke.rgb(255, 0, 0)         // cor RGB
stroke.dash_array([5, 3])     // linha tracejada
stroke.none                   // sem contorno
```

### Combinando estilos

Use `style.join`{.gleam} para combinar preenchimento e contorno:

```gleam
image.circle(40, style.join([fill.red, stroke.black, stroke.width(2)]))
```

![](img:style_join)


## Combinando imagens

### Lado a lado (beside)

```gleam
image.beside(
  image.square(40, fill.red),
  image.square(40, fill.blue),
)
```

![](img:beside)

### Empilhadas (above)

```gleam
image.above(
  image.square(40, fill.red),
  image.square(40, fill.blue),
)
```

![](img:above)

### Sobrepostas (overlay)

```gleam
image.overlay(
  image.circle(20, fill.red),
  image.square(60, fill.blue),
)
```

![](img:overlay)

### Combinando uma lista de imagens

```gleam
image.combine(
  [image.square(40, fill.red), image.square(40, fill.blue), image.square(40, fill.green)],
  image.beside,
)
```


## Cenas

Cenas são imagens com um sistema de coordenadas fixo. O eixo y cresce para baixo.

```gleam
image.empty_scene(200, 200)
|> image.place_image(100, 100, image.circle(20, fill.red))
```

![](img:scene)

A função `place_image`{.gleam} posiciona uma imagem pelo seu centro. Use `place_image_align`{.gleam} para controlar o alinhamento.


## Transformações

```gleam
image.rotate(img, 45)            // rotacionar 45 graus
image.scale(img, 2)              // dobrar o tamanho
image.scale_xy(img, 2, 1)        // escalar diferente em x e y
image.flip_horizontal(img)       // espelhar horizontalmente
image.flip_vertical(img)         // espelhar verticalmente
image.crop(img, 10, 10, 80, 80)  // recortar
```


## Propriedades

```gleam
image.width(img)      // largura (Int)
image.height(img)     // altura (Int)
image.dimension(img)  // #(largura, altura)
image.center(img)     // #(x, y) do centro
```


## Sistema de coordenadas

No sgleam, o sistema de coordenadas segue a convenção padrão de telas:

- O ponto (0, 0) fica no canto superior esquerdo
- O eixo x cresce para a direita
- O eixo y cresce para baixo
