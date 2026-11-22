# RadioGroup com builder

O módulo `sgleam/show/radio_group` constrói um grupo inteiro. Os valores podem ser
Strings, Ints ou variantes de um tipo próprio. Use valores distintos comparáveis,
sem funções. O nome HTML do grupo é gerado pelo renderizador, não pelo aluno.

```gleam
import sgleam/show/radio_group

// model.size tem tipo Size; SizeSelected recebe Size.
radio_group.new()
  |> radio_group.select(model.size)
  |> radio_group.label("Tamanho")
  |> radio_group.option(Small, "Pequeno")
  |> radio_group.option(Medium, "Médio")
  |> radio_group.option(Large, "Grande")
  |> radio_group.on_select(SizeSelected)
  |> radio_group.done
```

`new()` inicia sem seleção. Use `select(Medium)` para marcar Médio. O método
pode vir antes ou depois das opções; a última chamada a `select` prevalece.
Ele apenas descreve a seleção exibida, sem disparar `on_select` ou alterar o modelo.

Se o modelo permite nenhuma seleção, mantenha `Option` no estado e só chame
`select` quando houver um valor:

```gleam
let group = case model.size {
  None -> radio_group.new()
  Some(value) -> radio_group.new() |> radio_group.select(value)
}

group
|> radio_group.option(Small, "Pequeno")
|> radio_group.option(Medium, "Médio")
|> radio_group.on_select(SizeSelected)
|> radio_group.done
```

Nesse caso importe `None` e `Some` de `gleam/option`. No update,
`SizeSelected(size)` guarda `Some(size)`. Para limpar, guarde `None` e reconstrua
o componente na view sem chamar `select`. Clicar na opção já selecionada não
limpa o grupo: o comportamento é o de radios HTML nativos.

## Assinaturas

```gleam
new() -> RadioGroup(value, msg)
select(group: RadioGroup(value, msg), value: value) -> RadioGroup(value, msg)
option(group: RadioGroup(value, msg), value: value, label: String) -> RadioGroup(value, msg)
on_select(group: RadioGroup(value, msg), handler: fn(value) -> msg) -> RadioGroup(value, msg)
enabled(group: RadioGroup(value, msg), enabled: Bool) -> RadioGroup(value, msg)
label(group: RadioGroup(value, msg), label: String) -> RadioGroup(value, msg)
done(group: RadioGroup(value, msg)) -> Element(msg)
```

`option` preserva a ordem das chamadas. Cada opção tem um texto associado por
label HTML, portanto o texto também ativa o controle. `label` dá um nome
acessível ao grupo (aria-label), sem acrescentar um título visual; para um título
visível, use text. As opções aparecem em coluna.

`enabled(False)` desabilita todas as opções e não registra handlers. `on_select`
recebe o valor tipado, nunca uma String convertida do DOM. O builder transforma
os valores em mensagens ao finalizar; essas mensagens ficam no runtime Gleam.
O navegador só transmite o identificador do handler e um índice de opção.
`element.map` também transforma mensagens de RadioGroup.

Um grupo vazio é permitido. `select` com valor ausente não marca nenhuma opção.
Valores repetidos devem ser evitados; o renderizador marca no máximo a primeira
ocorrência selecionada. Identificadores de grupo são únicos na árvore renderizada,
inclusive para grupos vazios, sem handler ou desabilitados. A árvore deve ser
renderizada de uma só vez; fragmentos renderizados separadamente não compartilham
um alocador de nomes.

O antigo módulo `sgleam/show/radio` foi removido. Use `sgleam/show/radio_group`,
usando `new()` sem seleção e `select(value)` para selecionar uma opção.
Não é preciso informar group/value String nem calcular checked em cada opção.

Veja [radio_groups.gleam](../../examples/radio_groups.gleam), com dois grupos
independentes, valores String, seleção inicial vazia e botão para limpar, e
[show_gallery.gleam](../../examples/show_gallery.gleam).

## Organização interna

Assim como os outros componentes, o módulo radio_group só expõe funções que
delegam para element. Os tipos opacos RadioGroup(value, msg) (builder) e
RadioGroupRepr(msg) (representação final), junto das funções que manipulam seus
campos, ficam em element.gleam. Para anotar o tipo do builder, importe
`type RadioGroup` de `sgleam/show/element`. A sintaxe de construção não muda.
