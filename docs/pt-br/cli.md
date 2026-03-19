# Executando um arquivo

Para executar um arquivo Gleam:

```sh
sgleam arquivo.gleam
```

O sgleam procura uma função `main`{.gleam} ou `smain`{.gleam} no arquivo. A função `main`{.gleam} não recebe argumentos:

```gleam
// ola.gleam
import gleam/io

pub fn main() {
  io.println("Olá mundo!")
}
```

```sh
$ sgleam ola.gleam
Olá mundo!
```

A função `smain`{.gleam} possui três assinaturas possíveis. Sem argumentos, funciona como `main`{.gleam}:

```gleam
// saudacao.gleam
import gleam/io

pub fn smain() {
  io.println("Olá!")
}
```

```sh
$ sgleam saudacao.gleam
Olá!
```

Recebendo uma `String`{.gleam}, a função recebe toda a entrada do usuário:

```gleam
// eco.gleam
import gleam/io

pub fn smain(entrada: String) {
  io.println("Você digitou: " <> entrada)
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
import gleam/io
import gleam/list

pub fn smain(linhas: List(String)) {
  io.println("Linhas: " <> int.to_string(list.length(linhas)))
}
```

```sh
$ printf "a\nb\nc" | sgleam conta.gleam
Linhas: 3
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
--- repl2_1.gleam ---
...
--- repl2_1.mjs ---
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
Failure at teste.gleam (dobro_examples:11)
  Actual  : 8
  Expected: 9
5 tests, 4 success(es), 1 failure(s) and 0 error(s).
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
| `sgleam help` | Exibe ajuda |

# Opções

| Opção | Descrição |
|-------|-----------|
| `-n` | Usar Number ao invés de BigInt para inteiros |
| `-q` | Não exibir mensagem de boas-vindas no REPL |
| `--version` | Exibir versão |
