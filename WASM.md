# sgleam sobre o backend WebAssembly do gleam

Proposta. Nada foi implementado. O sgleam deixa de rodar o JavaScript gerado
pelo Gleam no QuickJS e passa a rodar o WebAssembly que o backend do fork
`malbarbo/gleam` gera (branch `wasm-array`). O `wasm/` de hoje, que é o
sgleam inteiro com o QuickJS compilado para `wasm32-wasip1`, sai.

## Como é hoje

- O `engine` compila Gleam para JavaScript e roda o resultado no QuickJS
  (`rquickjs`). O REPL compila cada entrada como um módulo e o carrega sobre
  os anteriores.
- As bibliotecas de `lib/sgleam/` chamam o Rust por 32 `@external(javascript,
  ...)` no `sgleam_ffi.mjs`, que chama as funções que `add_sgleam`
  (`engine/src/quickjs.rs`) registra no objeto global `sgleam`.
- O `wasm/` exporta o engine para o navegador na API do `ENGINE.md` do
  simplecode, e o QuickJS roda dentro do mesmo módulo.

## O que o backend do gleam gera

- Um módulo por programa, com o programa inteiro. O compilador monomorfiza o
  código, liga dentro do módulo as funções do `builtins-wasm` que o programa
  usa e dá ao módulo a própria memória linear.
- Os valores do Gleam são valores do WasmGC (structs e arrays). A memória
  linear serve só de rascunho para o Rust do `builtins-wasm`.
- O módulo importa só `wasi_snapshot_preview1`. Um `@external(webassembly,
  "x.mjs", ...)` fora da stdlib importa uma função de JavaScript, e o
  simplecode não tem como dar isso a um módulo qualquer.
- O host precisa de WasmGC, tail calls e exceções. O V8, o SpiderMonkey, o
  JSC e o wasmtime têm os três.

## A proposta

### O compilador dentro do `sgleam.wasm`

O `sgleam.wasm` passa a ter o compilador e mais nada. Ele compila o arquivo e
cada entrada do REPL para um módulo wasm e pede ao host que o instancie e o
chame. O host faz isso pelas primitivas genéricas do `env` que o `sc` já
usa (abaixo). O `sgleam.wasm` perde o QuickJS e fica com o `gleam-core`, o
`walrus` e o binário do `builtins-wasm` embutido.

### O REPL

Cada entrada continua um módulo, como hoje no JavaScript. A entrada N+1
importa da instância da entrada N as funções que devolvem os valores das
variáveis, com o tipo GC delas. O WasmGC identifica dois grupos recursivos
iguais em módulos diferentes, então o tipo de um valor da entrada N é o
mesmo tipo na entrada N+1 desde que o compilador emita o tipo sempre igual.
Nenhum valor GC passa pelo host. A entrada mostra o próprio resultado pelo
`fd_write`.

Isso pede ao fork um tipo wasm determinado pelo nome qualificado do tipo
Gleam. Hoje o backend usa o nome impresso, que não é injetivo (item 1 do
`WASM_REVIEW.md` do gleam). A correção que a revisão propõe serve aos dois.

Cada entrada carrega a própria cópia do que usa da stdlib e do
`builtins-wasm`. O tamanho e o tempo de compilação de uma entrada precisam
ser medidos antes do passo 4.

### As bibliotecas do sgleam

Os 32 externals de `lib/sgleam/` não podem continuar em JavaScript. A
proposta é um crate `sgleam-builtins`, compilado para
`wasm32-unknown-unknown` como o `builtins-wasm`, com o que o FFI faz hoje em
Rust: as imagens do `sinteract::image`, o SVG, a cena, a semente e o
relógio. O fork passa a ligar ao programa as funções de mais de um binário
de builtins, e o sgleam dá o seu. Os externals de `lib/sgleam/` viram
`@external(webassembly, "sgleam", ...)`.

O ABI da cola só passa números, `String`, `List`, tuplas e `Result`. Uma
`Image` é uma árvore Gleam, então o Gleam a escreve num formato plano antes
de chamar o Rust. Isso conversa com o `IMAGE.md`, que também leva as
imagens para o `sinteract::image`, e as duas mudanças devem ser feitas
juntas.

O mundo usa a sessão do sinteract nos fd 3 e fd 4 pelo WASI, como hoje. O
programa a abre pelo `fd_write` do próprio módulo.

### A API do `env` no simplecode

Combinada com o agente do `sc` em 2026-11-05. O `ENGINE.md` de hoje permite
no `env` só o `check_interrupt` e o `draw_svg`. Ele passa a ter também as
primitivas genéricas que o `sc.wasm` já importa (`sc/src/loader/host.rs`), e
a exigir do host WasmGC, tail calls e exceções. O agente do `sc` escreve a
seção do `ENGINE.md` e o `env.ts` do simplecode.

- `wasm_instantiate(bytes, len, slots, n, out) -> status`. Cada import tem
  um slot `{u32 kind, i32 value}`, na ordem dos imports. O tipo 0 é o padrão
  do host, o 1 é o handle em `value` (um export de outra instância, como
  vem), o 2 é um global i32 constante e o 3 é um global i32 mutável novo.
- `wasm_export(inst, name, len) -> handle`, com 0 se o nome não existe. A
  instância 0 é a do próprio engine.
- `wasm_call(func, sig, sig_len, vals) -> status`. A assinatura é
  `params:results` com `i j f d r`, e os resultados ficam em `vals`.
- `wasm_error(buf, cap)` dá a mensagem do último `ERROR`.
- `wasm_drop(handle)` solta uma instância, um export ou um global.
- O status é `OK`, `TRAP` (com o tipo em `vals[0]`, 1 para interrupção),
  `EXIT` (com o código do `proc_exit`), `STOP` e `ERROR`.
- `wasm_frames`, `wasm_memory_read` e `wasm_memory_write` são do `sc`. O
  sgleam não os usa.

O REPL liga a entrada N+1 à entrada N pelo tipo 1, com os handles dos
exports da entrada N. O tipo 0 num import de `wasi_snapshot_preview1` é o
WASI do host. Num módulo que não importa memória, como os do Gleam, o host
liga esse WASI à `memory` que o módulo exporta, depois de instanciá-lo.
Assim a função de início do módulo não pode usar o WASI, e o sgleam não
precisa disso, porque chama o `run` da entrada depois. Todas as instâncias
dividem os fd 0, 1 e 2 com o engine. O tipo 0 em `env.check_interrupt` é o
`check_interrupt` do host.

### O Stop

O código gerado consulta o `env.check_interrupt` sem chamar o host a cada
passo. Um global conta as entradas de função, e quando chega a zero o código
chama o `check_interrupt` e volta a contar. O Gleam não tem laços, então a
entrada de função basta. Com 1, o código dá `unreachable`, o `wasm_call`
volta com um trap, e o `sgleam.wasm` chama o `check_interrupt` para separar
o Stop de um erro. Um host que para a chamada sozinho, como o wasmtime com
épocas, devolve `STOP`, e o engine trata os dois casos do mesmo jeito. O
fork ganha uma opção para emitir a contagem.

Se o Stop não tiver resposta em cerca de um segundo, o host encerra o
worker, abre outro e carrega de novo o código do editor, como o Run faz. O
REPL avisa que as entradas se perderam. O host não refaz as entradas, porque
isso repetiria a saída e as leituras do stdin delas.

### O nativo

Duas escolhas:

- O `sgleam` nativo roda o mesmo wasm no wasmtime, e o QuickJS sai do
  projeto. A linguagem tem uma semântica só. O `Int` é inteiro de 64 bits
  nos dois lados, e não um `Number` do JavaScript em um e inteiro no outro.
  O `sc` já roda um wasmtime mínimo com JIT.
- O nativo continua no QuickJS, e só o navegador muda. É menos trabalho
  agora, e o mesmo programa pode dar resultados diferentes na página e no
  terminal.

A recomendação é a primeira.

## Passos

1. O `sc` escreve a API do `env` no `ENGINE.md` e no `env.ts` do
   simplecode, e o sgleam a testa com o próprio backend.
2. No fork, compilar o `gleam-core` com o backend wasm para
   `wasm32-wasip1`, e corrigir os bugs altos do `WASM_REVIEW.md`.
3. `sgleam` roda um arquivo sem as bibliotecas do sgleam pelo wasmtime, e
   os testes de `cli/tests/inputs` que não as usam passam.
4. O `sgleam-builtins` e os externals de `lib/sgleam/`, junto com o
   `IMAGE.md`.
5. O REPL com uma instância por entrada.
6. O `wasm/` sobre as primitivas do `env`, e os testes do Deno com um host
   que as implementa.
7. Remover o `rquickjs` e o `sgleam_ffi.mjs`.

## Questões abertas

- O nativo vai para o wasmtime?
- O `Int` de 64 bits e o `Float` de 64 bits como padrão do sgleam?
- O `string.inspect` e o `echo` do backend wasm escrevem os valores como o
  JavaScript? Os snapshots de `cli/tests` dependem disso.
