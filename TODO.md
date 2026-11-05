- Adicionar um modo de live programming (https://nullprogram.com/blog/2014/12/23/)
- MQuickJS (https://github.com/bellard/mquickjs) como segundo motor: descartado
  em 2026-08-19. É ES5 estrito, sem class, let/const, módulos, Map/Set, BigInt
  nem arrow, então nada do JS gerado pelo Gleam roda. O motor é 3,8x menor no
  wasm, mas é só ~22% do sgleam.wasm (teto de ~13% de economia). O lado Rust
  seria barato: o trait Engine já serve. Reabrir se ganhar class e módulos.
- Fazer o `sgleam server` funcionar no Windows. O sinteract só abre a sessão
  no fd 3 e no fd 4, que existem no unix e no wasi, então no Windows o
  primeiro mundo falha com `StageError::NoSession`.
- Bug: no navegador, uma recursão que não é de cauda passa de 1.500 a 2.000
  níveis e esgota a pilha do V8, que lança `RangeError` de dentro do wasm. O
  QuickJS compilado para WASI não verifica a pilha (`quickjs.c:2761`), então
  o `JS_SetMaxStackSize` do engine não vale lá. O simplecode recria a
  instância e o aluno perde as entradas do REPL. No nativo o limite é 88.000.
  O plano do wasm tira o QuickJS do navegador.
