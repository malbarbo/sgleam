# sgleam sobre o backend WebAssembly do gleam

Proposta. Nada foi implementado. No navegador, o sgleam deixa de rodar o
JavaScript gerado pelo Gleam no QuickJS e passa a rodar o WebAssembly que o
backend do fork `malbarbo/gleam` gera (branch `wasm-array`). O `wasm/` de
hoje, que é o sgleam inteiro com o QuickJS compilado para `wasm32-wasip1`,
sai. O nativo continua no QuickJS por enquanto.

Três revisores conferiram a primeira versão em 2026-11-05, contra o código
e com experimentos. Os programas dos experimentos estão no scratchpad da
sessão (`review-backend`, `review-student`). Esta versão incorpora o que
eles acharam.

## Como é hoje

- O `engine` compila Gleam para JavaScript e roda o resultado no QuickJS
  (`rquickjs`). O REPL compila cada entrada como um módulo e o carrega sobre
  os anteriores.
- As bibliotecas de `lib/sgleam/` chamam o Rust por 32 `@external(javascript,
  ...)` no `sgleam_ffi.mjs`, que chama as funções que `add_sgleam`
  (`engine/src/quickjs.rs`) registra no objeto global `sgleam`.
- O `sgleam_ffi.mjs` faz mais que os externals. Ele tem a cola do REPL
  (`try_main`, `repl_memo`, `has_var`, `repl_print`), o gancho
  `gleam_inspect` do fork da stdlib (commit `67cf194`), que escreve
  `//world` e as cores e estilos, os caminhos do usuário no `echo` e nos
  erros, o rastro de um estouro de pilha pelos source maps, e o `check` e os
  testes, que pegam um `panic` com try/catch e seguem.
- O `check` usa uma reescrita do gerador de JavaScript que só o fork do
  sgleam tem (`compiler-core/src/javascript/expression.rs:1975`), e os
  `*_examples` são achados por reflexão no JavaScript.
- O `wasm/` exporta o engine para o navegador na API do `ENGINE.md` do
  simplecode, com o `font_face` das fontes do sinteract.

## O que o backend do gleam gera

- Um módulo por programa, com o programa inteiro. O compilador monomorfiza o
  código, copia para dentro do módulo as funções do `builtins-wasm` que o
  programa usa e dá ao módulo a própria memória linear, de 17 páginas.
- Os valores do Gleam são valores do WasmGC. A memória linear serve de
  rascunho para o Rust do `builtins-wasm`, que a usa como uma arena. Cada
  chamada de cola marca a arena antes e a solta depois.
- O `link.rs` não é um ligador. Ele lê um binário só do `builtins-wasm`, por
  `include_bytes`, e conta com uma memória, um global (o ponteiro da pilha)
  e dados em endereços fixos, sem relocação.
- Só os pacotes `gleam_stdlib` e `gleam_wasm_runtime` podem usar o módulo
  `"builtins"` (`reaches_builtins`, `webassembly.rs:6669`). Um external com
  outro nome de módulo vira uma função de JavaScript, com conversões de
  `externref`, e o módulo passa a precisar de um host JavaScript.
- Um módulo só com WASI importa `fd_write` e `proc_exit`, e mais o que o
  programa usar. Um `panic`, um `todo` ou um `let assert` escrevem
  `let_assert at src/a.gleam:4` e a mensagem no fd 2, e chamam
  `proc_exit(1)`. Nada disso dá trap.
- O `echo` escreve no fd 2, sempre com cor, e com o caminho relativo ao
  pacote.
- O host precisa de WasmGC e tail calls. O código gerado não usa exceções,
  mas o desenho do `check` abaixo pode passar a usar.
- O sgleam usa as strings `UTF8`, que são o padrão. As `JSString` pedem os
  builtins `wasm:js-string`, que o host do simplecode não dá.
- A função de início só configura os builtins e cria as strings e as
  constantes, sem WASI. O `main` roda pelo export `_start`.
- O `gleam-core` com o backend compila para `wasm32-wasip1` sem mudança
  (`cargo build -p gleam-core --target wasm32-wasip1`), desde que o binário
  do `builtins-wasm` já esteja feito.

## A proposta

### Uma instância de runtime e uma instância por entrada

O `sgleam.wasm` tem o compilador, as fontes do `font_face` e um binário de
runtime embutido. O runtime é o `builtins-wasm` junto com o Rust do sgleam,
num cdylib só, com a memória linear e o estado que dura entre chamadas. O
engine instancia o runtime uma vez por sessão.

Cada programa e cada entrada do REPL viram um módulo sem memória própria. O
módulo importa do runtime a memória e as funções do Rust, pelo tipo 1 do
`wasm_instantiate`. O WASI do host já sabe ler a memória importada. Assim
uma entrada não custa 1,1 MB de memória, e duzentas entradas não custam
duzentas memórias.

Isso pede ao fork um modo em que o `link.rs` importa as funções do runtime
em vez de copiá-las para dentro do módulo.

### O Rust do sgleam

Os externals de `lib/sgleam/` passam a ser `@external(webassembly,
"builtins", ...)`. O crate `sgleam-builtins` depende do `builtins-wasm` como
biblioteca e gera o cdylib do runtime. O fork recebe pelo
`WebAssemblyConfig` os bytes do runtime e a lista dos pacotes que podem usar
o módulo `"builtins"`, que passa a incluir o `sgleam`.

A arena é o alocador global do `builtins-wasm`, e cada chamada a solta no
fim. O estado que dura entre chamadas, como a sessão do mundo, o relógio e
os bitmaps de um `Loaded`, precisa de um alocador próprio fora da arena.

O sinteract abre a sessão no fd 3 e no fd 4 só com `target_os = "wasi"`, e
usa `std::fs::File` e `std::time::Instant`, que não existem em
`wasm32-unknown-unknown`. O runtime chama o WASI direto, pelo crate `wasi`,
como o `builtins-wasm/src/io.rs` já faz, e o sinteract ganha um caminho sem
o `std::time`.

O ABI da cola só passa números, `String`, `List`, tuplas e `Result`. Uma
`Image` continua uma árvore Gleam, e o `==` continua estrutural
(`check_image_trees`). O Gleam escreve a árvore num formato plano antes de
chamar o Rust. O commit 2 do `IMAGE.md`, que troca a árvore por uma classe
opaca do rquickjs, precisa ser reescrito para o wasm.

O `random.coerce` é a identidade no JavaScript (`random.gleam:217`). Com a
monomorfização, ele não tem equivalente, e o mock do `random.gleam` precisa
de outro desenho.

### O REPL

O backend monomorfiza, então a entrada N+1 não pode chamar uma função
genérica da entrada N num tipo novo. Cada entrada compila de novo, como
fonte, todas as definições do REPL (`fn`, `type`, `const`), como o
`build_source` já faz. Só os valores dos `let` passam de uma instância para
outra.

O fork ganha um tipo de import novo, com um nome de módulo como `repl:N`,
que importa um valor com o tipo GC dele, sem conversão e sem tornar o
módulo um módulo de host JavaScript. São cerca de 100 a 200 linhas no
`host.rs` e na validação.

O WasmGC identifica dois grupos recursivos iguais pela estrutura. O que o
backend emite serve, mas o `rec_group_from` (`webassembly.rs:2784`) ordena
os membros de um grupo pela ordem de criação. Com `type A { A(fn() -> B) }`
e `type B { B(fn() -> A) }`, o `A` de um programa vira o `B` do outro. O
fork passa a ordenar os membros de cada grupo pelo nome qualificado.

Uma variável de tipo sem ligação vira `Never`. Um `let id = fn(a) { a }`
sai como `fn(Never) -> Never` e não pode ser importado depois num tipo
concreto. O primeiro uso fixa o tipo (veja as decisões).

O sgleam compila hoje o código do usuário, a stdlib e o `lib/sgleam` como um
pacote só, com o nome `""` (`engine/src/gleam.rs:148`). Com o backend, a
stdlib passa a ser o pacote `gleam_stdlib`, e o REPL guarda os `Module`
tipados de todas as dependências, não só as `ModuleInterface`.

Um `panic` numa entrada volta como `EXIT` com código 1. O REPL o trata como
erro daquela entrada, e a sessão continua.

### O `check` e os testes

Hoje um `panic` dentro de um `check` não encerra o programa. No wasm, o
`panic` chama `proc_exit`. O fork ganha três coisas:

- a reescrita do `check`, que hoje só existe no gerador de JavaScript;
- uma lista dos `*_examples` exportada pelo módulo, no lugar da reflexão;
- um `panic` que o `check` consegue pegar. A proposta é o `panic` lançar uma
  exceção wasm com uma tag, e o `check` a pegar com `try_table`. Assim o
  host precisa de exceções. A alternativa é um `wasm_call` por teste, que
  não serve para vários `check` dentro de uma mesma função.

### O que o aluno vê

- O `echo` e os erros dizem o caminho que o aluno deu. O fork recebe os
  caminhos como opção, e o REPL recebe a linha da entrada.
- A cor do `echo` e dos erros segue o `color_stdout` e o `color_stderr`. O
  fork recebe a decisão como opção, e o `echo` volta para o stdout.
- O cabeçalho do erro mantém a forma do sgleam (`Error at a.gleam (f:3)`),
  e não o `let_assert at`. O `assert` mostra o operador e os dois lados, e
  o `let assert` mostra o valor, como hoje. O fork precisa guardar esses
  valores.
- O inspect escreve os rótulos dos campos, e o gancho do inspect escreve
  `//world`, as cores e os estilos como hoje.
- O estouro de pilha vira a mensagem do sgleam (`Stack overflow` e as
  funções), tirada da seção de nomes, sem os 20 endereços do trap. O wasm
  não tem linhas, então a mensagem diz só a função.
- O wasm é de 10 a 30 vezes mais rápido. `fib(30)` sai em 0,02 s no
  wasmtime, contra 0,55 s no sgleam nativo.

### A pilha

No Node e no Deno, uma recursão que não é de cauda passa de 5.000 níveis e
falha antes de 10.000 com o código frio. Depois que o V8 otimiza a função,
ela chega a 70.000. Uma entrada do REPL é sempre um módulo novo, então o
aluno vê sempre o caso frio. O navegador de hoje falha antes, entre 2.000 e
4.000, e o sgleam nativo chega a 88.000. A pilha do worker do navegador não
pode ser configurada. O limite precisa ser medido no worker real do
simplecode, no Chrome e no Firefox, e documentado.

### A API do `env` no simplecode

Combinada com o agente do `sc` em 2026-11-05 e escrita por ele no
`ENGINE.md` e no `src/wasm_host.ts` do simplecode, ainda sem commit lá.

- `wasm_instantiate(bytes, len, slots, n, out) -> status`. Cada import tem
  um slot `{u32 kind, i32 value}`, na ordem dos imports. O tipo 0 é o padrão
  do host, o 1 é o handle em `value`, o 2 é um global i32 constante e o 3 é
  um global i32 mutável novo. O detalhe de uma falha fica em `*out`.
- `wasm_export(inst, name, len) -> handle`, com 0 se o nome não existe. A
  instância 0 é a do próprio engine. Cada resposta é um handle novo, então
  o engine guarda um handle por export.
- `wasm_call(func, sig, sig_len, vals) -> status`. A assinatura é
  `params:results` com `i j f d r`.
- `wasm_error`, `wasm_drop`, e `wasm_frames`, `wasm_memory_read` e
  `wasm_memory_write`, que o sgleam não usa.
- O status é `OK`, `TRAP` (com o tipo em `vals[0]`), `EXIT` (com o código
  do `proc_exit`), `STOP` e `ERROR`.
- O tipo 0 vale para o WASI, para o `env.check_interrupt` e para o
  `env.wasm_stack`. Uma função do WASI que o host não tem responde `ENOSYS`.

Falta uma coisa para o sgleam. Uma entrada que mostra uma imagem chama o
`env.draw_svg`, que hoje é só do engine e lê a memória dele. O tipo 0 em
`env.draw_svg` passa a ler a memória do módulo que chama, como o WASI faz.

### O Stop

Um global conta as entradas de função, e quando chega a zero o código chama
o `env.check_interrupt` e volta a contar. O código Gleam gerado não tem
laços, e uma chamada de cauda (`return_call`) passa pela entrada da função
chamada, então a entrada de função basta. Os laços estão só nos builtins,
como `_i64_to_str`, e uma chamada longa de um builtin não para no meio. O
fork ganha uma opção para emitir a contagem.

Com 1, o código dá `unreachable`, e o engine chama o `check_interrupt` para
separar o Stop de um estouro de pilha ou de outro trap. Se o Stop não tiver
resposta em um segundo, o host encerra o worker e abre outro, e o REPL
avisa que as entradas se perderam.

## Passos

1. O host do simplecode dá o `env.draw_svg` no tipo 0, lendo a memória do
   módulo que chama. O agente do `sc` já o fez, sem commit no simplecode.
2. No fork, o runtime pelo `WebAssemblyConfig`, a lista de pacotes que podem
   usar `"builtins"`, e o modo em que o módulo importa o runtime.
3. No sgleam, a stdlib como o pacote `gleam_stdlib`, e um arquivo sem as
   bibliotecas do sgleam roda no `wasmtime`. Os testes de
   `cli/tests/inputs` que não as usam passam, com os snapshots revistos.
4. No fork, os caminhos, a cor, o cabeçalho e os valores dos erros, os
   rótulos e o gancho do inspect, a reescrita do `check`, a lista dos
   `*_examples`, o `panic` que pode ser pego e a opção das verificações de
   estouro do `Int`.
5. O `sgleam-builtins`, com o alocador do estado e o sinteract sem
   `std::time`, e os externals de `lib/sgleam/`, junto com o `IMAGE.md`
   reescrito.
6. No fork, o import `repl:N`, a ordem dos grupos recursivos e a opção do
   Stop. Antes do REPL, medir no navegador o tempo de compilação de uma
   entrada, já que o compilador roda dentro do `sgleam.wasm`, e a memória
   de uma sessão longa.
7. O REPL com uma instância por entrada.
8. O `wasm/` sobre as primitivas do `env`, com o `font_face`, e os testes do
   Deno com um host que as implementa. Medir a pilha no worker real.
9. Remover do `wasm/` o `rquickjs`. O nativo continua com ele.

## Decisões

- O nativo fica para depois de medir o tempo de compilação de uma entrada
  no wasmtime (decidido em 2026-11-05). O navegador muda primeiro.
- O `Float` tem 64 bits (decidido em 2026-11-05).
- O `Int` tem 64 bits, e um estouro é um erro com o arquivo e a linha, como
  um `panic` (decidido em 2026-11-05). O padrão do sgleam hoje é BigInt, que
  é exato, então `fat(30)`, que funciona hoje, passa a parar com o erro.
  As verificações são uma opção do backend, desligada por padrão, e o
  sgleam a liga. Com ela, o código confere a soma, a subtração, a
  multiplicação, o `MIN / -1`, o `absolute_value` e os deslocamentos, e o
  `float.truncate` e o `float.round` de um valor fora do alcance. O
  `int.parse` de um número grande continua `Error(Nil)`.
- Um `let` do REPL cujo tipo ainda é genérico, como `let xs = []`, tem o
  tipo fixado pelo primeiro uso (decidido em 2026-11-05). Depois de
  `[1, ..xs]`, `["a", ..xs]` é um erro de tipo. Hoje o sgleam aceita os dois.
  A entrada que fixa o tipo calcula a expressão de novo nesse tipo, uma vez,
  e as seguintes importam esse valor.
- A ordem de um `dict` pode mudar entre o terminal e a página enquanto o
  nativo usa o QuickJS (decidido em 2026-11-05). Ela já muda entre os
  backends do Gleam, e o sgleam não faz nada a respeito.
- O REPL, o `echo` e o `string.inspect` escrevem os rótulos dos campos,
  como `Ponto(x: 1, y: 2.0)` (decidido em 2026-11-05).
- O `:debug` sai do REPL do navegador (decidido em 2026-11-05). O nativo
  continua mostrando o `.mjs`.
- O `:time` mede só a chamada que roda a entrada, sem a compilação e a
  instanciação, como hoje mede só a execução no QuickJS
  (`engine/src/repl.rs:677`).
