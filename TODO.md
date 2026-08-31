- Adicionar um modo de live programming (https://nullprogram.com/blog/2014/12/23/)
- MQuickJS (https://github.com/bellard/mquickjs) como segundo motor: descartado
  em 2026-08-19. É ES5 estrito, sem class, let/const, módulos, Map/Set, BigInt
  nem arrow, então nada do JS gerado pelo Gleam roda. O motor é 3,8x menor no
  wasm, mas é só ~22% do sgleam.wasm (teto de ~13% de economia). O lado Rust
  seria barato: o trait Engine já serve. Reabrir se ganhar class e módulos.
