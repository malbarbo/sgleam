# Executando um arquivo

Para executar um arquivo Gleam:

```sh
sgleam arquivo.gleam
```

O sgleam procura uma função `smain`{.gleam} ou `main`{.gleam} no arquivo. O sgleam imprime o que a `smain`{.gleam} devolve. Uma `String`{.gleam} sai como está, uma imagem aparece como no REPL, um mundo ou uma animação executa, e qualquer outro valor sai como o Gleam o escreve:

```gleam
// ola.gleam
pub fn smain() {
  "Olá mundo!"
}
```

```sh
$ sgleam ola.gleam
Olá mundo!
```

A função `smain`{.gleam} também pode receber a entrada do usuário. Recebendo uma `String`{.gleam}, a função recebe toda a entrada:

```gleam
// eco.gleam
pub fn smain(entrada: String) {
  "Você digitou: " <> entrada
}
```

```sh
$ echo "teste" | sgleam eco.gleam
Você digitou: teste
```

Recebendo uma `List(String)`{.gleam}, a função recebe a entrada dividida em linhas:

```gleam
// conta.gleam
import gleam/int
import gleam/list

pub fn smain(linhas: List(String)) {
  "Linhas: " <> int.to_string(list.length(linhas))
}
```

```sh
$ printf "a\nb\nc" | sgleam conta.gleam
Linhas: 3
```

Recebendo um `Env`{.gleam}, do módulo `sgleam/env`{.gleam}, a função recebe o que o programa conhece ao começar. `env.args`{.gleam} dá os argumentos depois do arquivo, `env.random`{.gleam} dá um gerador de números aleatórios, e `env.time`{.gleam} dá os milissegundos desde 1970-01-01 00:00 UTC:

```gleam
// ola.gleam
import gleam/string
import sgleam/env.{type Env}

pub fn smain(env: Env) {
  "Olá, " <> string.join(env.args(env), " e ") <> "!"
}
```

```sh
$ sgleam ola.gleam Ana Bia
Olá, Ana e Bia!
```

Um argumento que começa com `-` vai depois de `--`, como em `sgleam run ola.gleam -- -x`. Sem isso, o sgleam lê o argumento como uma opção dele.

A função `main`{.gleam} não recebe argumentos, e o sgleam não imprime o que ela devolve.

Para ver um valor no meio de um programa, use `echo`{.gleam}. Ele imprime o arquivo e a linha, e depois o valor:

```gleam
// dobro.gleam
pub fn smain() {
  let x = echo 2 * 3
  x * 2
}
```

```sh
$ sgleam dobro.gleam
dobro.gleam:3
6
12
```


# Números aleatórios

O módulo `sgleam/random`{.gleam} sorteia números a partir de um gerador. Um gerador é um valor, então o mesmo gerador sempre dá o mesmo número. Cada função devolve o número junto com o próximo gerador, e o sorteio seguinte usa o novo:

```gleam
// dados.gleam
import sgleam/env.{type Env}
import sgleam/random

pub fn smain(env: Env) {
  let gen = env.random(env)
  let #(a, gen) = random.int(gen, 1, 6)
  let #(b, _) = random.int(gen, 1, 6)
  #(a, b)
}
```

`env.random`{.gleam} dá um gerador com uma semente diferente a cada execução. `random.new(semente)`{.gleam} cria um com uma semente fixa, e a mesma semente dá a mesma sequência. `random.int(gen, de, ate)`{.gleam} sorteia um `Int`{.gleam} entre `de` e `ate`, os dois incluídos. `random.float(gen)`{.gleam} sorteia um `Float`{.gleam} de 0.0 até 1.0, sem incluir o 1.0. `random.pick(gen, itens)`{.gleam} sorteia um item de uma lista, e `random.shuffle(gen, itens)`{.gleam} coloca uma lista em ordem aleatória.

Um teste dá ao programa um gerador com respostas conhecidas. `random.mock`{.gleam} recebe as chamadas que o programa faz, em ordem, cada uma com o resultado que devolve:

```gleam
import sgleam/check
import sgleam/random.{type Generator}

pub fn lance(gen: Generator) -> #(Int, Generator) {
  let #(a, gen) = random.int(gen, 1, 6)
  let #(b, gen) = random.int(gen, 1, 6)
  #(a + b, gen)
}

pub fn lance_examples() {
  let gen =
    random.mock([random.int_returns(1, 6, 2), random.int_returns(1, 6, 5)])
  check.eq(lance(gen).0, 7)
}
```

`random.float_returns`{.gleam}, `random.pick_returns`{.gleam} e `random.shuffle_returns`{.gleam} descrevem as outras chamadas. Cada sorteio precisa casar com a primeira chamada da lista, com os mesmos argumentos. Uma chamada diferente, ou um sorteio depois do fim da lista, para o programa com uma mensagem que diz o que o mock esperava. Para uma função que recebe um `Env`{.gleam}, `env.fake()`{.gleam} dá um env sem argumentos, com `random.new(0)`{.gleam} e tempo 0, e `env.with_args`{.gleam}, `env.with_random`{.gleam} e `env.with_time`{.gleam} o alteram:

```gleam
env.fake() |> env.with_random(random.mock([random.int_returns(1, 6, 2)]))
```


# Imagens e mundos

Um mundo, ou uma animação, mostra seus quadros em uma janela, e fechar a janela encerra o programa. Quando nenhuma janela abre, como em uma sessão ssh, os quadros aparecem no terminal, e `Ctrl+c` encerra o programa. Se o terminal também não mostra imagens, cada quadro é impresso como `Image`. O REPL mostra uma imagem no terminal, ou imprime `Image` quando o terminal não mostra imagens.

Enquanto um mundo aparece no terminal, o que o programa imprime, como um `echo`{.gleam}, vai para as cinco linhas abaixo do quadro. Quando o mundo termina, o último quadro fica na tela com essas linhas embaixo.

A opção `-d` escolhe onde as imagens aparecem. `-d window` mostra os quadros de um mundo apenas em uma janela, e o REPL continua mostrando suas imagens no terminal. `-d kitty`, `-d sixel` e `-d blocks` usam apenas o terminal, com os gráficos do Kitty, com Sixel ou com blocos coloridos, que qualquer terminal com true color mostra. Com `-d kitty` e `-d sixel`, o sgleam consulta o terminal antes e para com um erro quando o terminal não suporta o modo. `-d blocks` não consulta, então um terminal sem true color mostra cores erradas. `-d none` não mostra imagens e imprime `Image` no lugar delas.

```sh
sgleam -d blocks bola.gleam
```


# Modo interativo (REPL)

Para entrar no modo interativo:

```sh
sgleam
```

No REPL, você pode digitar expressões, definições (variáveis, funções, tipos) e comandos:

```gleam-repl
> 1 + 2
3
> let x = 10
10
> x * 2
20
```

Também é possível carregar um arquivo, tornando as definições disponíveis no REPL.
Por exemplo, dado o arquivo `dobro.gleam`:

```gleam
import sgleam/check

pub fn dobro(x: Int) -> Int {
  x * 2
}

pub fn dobro_examples() {
  check.eq(dobro(0), 0)
  check.eq(dobro(3), 6)
}
```

Podemos usar a função `dobro`{.gleam} no REPL:

```sh
sgleam repl dobro.gleam
```

```gleam-repl
> dobro(5)
10
> dobro(3) + 1
7
```


## Comandos do REPL

`:quit` — Sai do REPL (ou `Ctrl+d`).

`:type` — Mostra o tipo de uma expressão sem avaliá-la:

```gleam-repl
> :type 1 + 2
Int
> :type [1, 2, 3]
List(Int)
```

`:debug` — Ativa/desativa o modo debug, que mostra o código Gleam e JavaScript gerado antes da execução:

```gleam-repl
> :debug
Debug mode on.
> let x = 10
--- repl1.gleam ---
...
--- repl1_1.gleam ---
...
--- repl1_1.mjs ---
...
10
> :debug
Debug mode off.
```


## Importações no REPL

Importações são suportadas e mescladas automaticamente:

```gleam-repl
> import gleam/int.{to_string}
> to_string(42)
"42"
> import gleam/int.{add}
> add(1, 2)
3
```


# Testes

Para executar os testes de um arquivo:

```sh
sgleam test arquivo.gleam
```

Os testes são funções cujo nome termina com `_examples` e usam o módulo `sgleam/check`{.gleam}.

Por exemplo, dado o arquivo `teste.gleam`:

```gleam
import sgleam/check

pub fn soma_examples() {
  check.eq(1 + 1, 2)
  check.eq(2 + 3, 5)
}

pub fn dobro_examples() {
  check.eq(2 * 0, 0)
  check.eq(2 * 3, 6)
  check.eq(2 * 4, 9)
}
```

```sh
sgleam test teste.gleam
```

```
Running tests...
  teste.gleam/dobro_examples
    Failure at line 11
      Actual  : 8
      Expected: 9
  teste.gleam/soma_examples
5 tests, 4 successes, 1 failure and 0 errors.
```

Neste caso, o teste `check.eq(2 * 4, 9)`{.gleam} falhou porque `2 * 4`{.gleam} é `8`{.gleam}, não `9`{.gleam}.


# Formatação

Para formatar o código fonte:

```sh
sgleam format arquivo.gleam
```

Ou para formatar a entrada padrão:

```sh
sgleam format < arquivo.gleam
```


# Verificação

Para verificar se o código compila corretamente (verificação de tipos e erros de sintaxe) sem executá-lo:

```sh
sgleam check arquivo.gleam
```

Se não houver erros, nenhuma saída é produzida. Caso contrário, os erros são exibidos.


# Comandos

| Comando | Descrição |
|---------|-----------|
| `sgleam [arquivo]` | Executa o arquivo (atalho para `sgleam run`) |
| `sgleam repl [arquivo]` | Modo interativo (REPL) |
| `sgleam run arquivo [args]` | Executa o arquivo, com argumentos para `smain(env)` |
| `sgleam test arquivo` | Executa os testes |
| `sgleam format [arquivos]` | Formata o código (lê stdin se nenhum arquivo for dado) |
| `sgleam check arquivo` | Verifica o código (apenas compilação) |
| `sgleam help [comando]` | Exibe a ajuda do programa ou de um comando |

# Opções

| Opção | Descrição |
|-------|-----------|
| `-n` | Usar Number ao invés de BigInt para inteiros |
| `-d MODO` | Mostrar imagens em `window`, `kitty`, `sixel`, `blocks` ou `none` |
| `--players N` | Jogar um mundo multijogador com N jogadores, cada um em uma janela |
| `-q` | Não exibir mensagem de boas-vindas no REPL |
| `--version` | Exibir versão |
