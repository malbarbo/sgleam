# Executando um arquivo

Para executar um arquivo Gleam:

```sh
sgleam arquivo.gleam
```

O sgleam procura uma função `smain`{.gleam} ou `main`{.gleam} no arquivo. O sgleam imprime o que a `smain`{.gleam} devolve. Uma `String`{.gleam} sai como está, e qualquer outro valor sai como o Gleam o escreve:

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

A função `main`{.gleam} não recebe argumentos, e o sgleam não imprime o que ela devolve. Ela serve para um programa que mostra imagens ou executa um mundo.

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
| `sgleam run arquivo` | Executa o arquivo |
| `sgleam test arquivo` | Executa os testes |
| `sgleam format [arquivos]` | Formata o código (lê stdin se nenhum arquivo for dado) |
| `sgleam check arquivo` | Verifica o código (apenas compilação) |
| `sgleam help [comando]` | Exibe a ajuda do programa ou de um comando |

# Opções

| Opção | Descrição |
|-------|-----------|
| `-n` | Usar Number ao invés de BigInt para inteiros |
| `-q` | Não exibir mensagem de boas-vindas no REPL |
| `--version` | Exibir versão |
