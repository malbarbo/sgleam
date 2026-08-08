# Redefinition in a REPL

A REPL has to answer a question the compiler never asks: what happens when an
input redefines a name that is already there. The answer is a language
decision, not an implementation detail. This note records what GHCi and OCaml
do, measured rather than recalled, and how sgleam arrives at the same rule.

Measured with GHC 9.6.6 and OCaml 5.3.0.

## The rule they share

**An input is a scope. A definition shadows, it does not replace. Whatever was
defined earlier stays bound to what it saw at the time.**

Nothing ever breaks retroactively, so no REPL here refuses a redefinition. The
price is that a shadowed type needs a name of its own in the output: GHCi
qualifies it with the module of the input that defined it (`Ghci1.T`), OCaml
appends a suffix (`t/2`), sgleam does as GHCi does (`repl1.T`). All three do
this only for a name that was shadowed.

## Behaviour

| | GHCi | OCaml | sgleam |
|---|---|---|---|
| redefine a type while values of it exist | allowed | allowed | allowed |
| those values afterwards | live, as `Ghci1.T` | live, as `t/2` | live, as `repl1.T` |
| mixing old and new | type error, both definition sites | type error, plus a hint | type error, naming the old one |
| value read by an earlier function | the old one | the old one | the old one |
| function called by an earlier function | the old one | the old one | the old one |
| recursive call in a redefinition | the new one | `let rec`: new, `let`: previous | the new one |
| mutual recursion in one input | `:{ … :}` | `let rec … and …` | plain, side by side |
| mutual recursion across inputs | not in scope | unbound | unbound |
| mutually recursive types, one input | `:{ … :}` | `type … and …` | plain, side by side |
| the same across inputs | not in scope | unbound | unknown type |
| redefine one type of such a pair | allowed, the other keeps the old one | same | same |
| the shadowed one, reached qualified | in scope | not writable | in scope |
| `const` distinct from `let` | absent | absent | present |

## Transcripts

Redefining a type, GHCi, OCaml and sgleam:

    > data T = A | B deriving Show   | type t = A | B;;      | type T { A B }
    > let x = A                      | let x = A;;           | let x = A
    > data T = C deriving Show       | type t = C;;          | type T { C }
    > :type x                        | x;;                   | :type x
    x :: Ghci1.T                     | - : t/2 = A           | repl1.T

Using the two together is what fails, and the message names both:

    Couldn't match expected type ‘T’ with actual type ‘Ghci1.T’
      NB: ‘T’ is defined at <interactive>:3:1-24
          ‘Ghci1.T’ is defined at <interactive>:1:1-28

OCaml has a message for this case:

    The value "x" has type "t/2" but an expression was expected of type "t"
      Hint: The type "t" has been defined multiple times in this toplevel
      session. Some toplevel values still refer to old versions of this type.

sgleam names the old one the same way, in its own error shape:

    Expected type:

        T

    Found type:

        repl1.T

Redefining one type of a mutually recursive pair shows the same rule at the
level of a type: after `data A = MkA B` and `data B = MkB Int` are defined
together, redefining `B` alone leaves `A` pointing at the old one. The mark is
put only on the name that was shadowed — `x :: A`, unqualified, still holds a
field of type `Ghci1.B`:

    Couldn't match expected type ‘Ghci1.B’ with actual type ‘B’
    There is no constructor "MkB2" within type "b/2"     (OCaml)

A redefinition does not reach back into what already exists — the three agree:

    let x = 1        fn f(y) { x + y }        let x = 100        f(1)   → 2
    fn g(n) { n+1 }  fn h(n) { g(n) * 10 }    fn g(n) { n+100 }  h(1)   → 20

OCaml is the only one that lets the user pick what a recursive call in a
redefinition means: `let rec fat n = … fat …` calls the new one, the same
without `rec` calls the previous one.

## How sgleam gets there

Gleam already identifies a type the way GHCi does: `Type::Named` carries the
module it was defined in, and two types of the same name in different modules
are distinct. Nothing had to be invented — the identity had to be kept.

Keeping the compiled type is not by itself enough: the next input is compiled
from generated source, and source can only name a type as `module.Name`. The
type must therefore live in a module that survives. Saving it and giving each
input its own module are the same move, not two options.

So input *N* gets a module `replN`, written once and never regenerated, holding
its types, its functions and its constants — everything the user wrote as
source. The REPL records, per name, the module that defines it; the module
generated for the next input imports those names instead of re-emitting them.
Two live modules can define `A`, and the one that was shadowed keeps working
for everything built on it.

A `let` cannot go there, and for a reason particular to Gleam: it is a computed
value, and Gleam has no computed binding at module level. It stays a slot,
reached by a module constant, `const x: A(Int) = load(0)`. But that constant
carries the *name* of the type as text, and the text has to be written in a
scope where it still means the right thing — which is why it goes into a
companion module `replN_M_vals`, written right after the run, once. Rendered in
the scope of the input that bound it and never re-read anywhere else, it cannot
be rewritten by a later definition. What it costs is one extra module per input
that binds a value; what it buys is that no annotation the REPL writes is ever
read twice.

The naming of a shadowed type costs nothing: Gleam already prints the qualified
name only for the one whose plain name is taken, and the REPL prints a type
through the generated module's own name map. Which is where the message comes
from, and it is the same message the other two give:

    Expected type:

        Val

    Found type:

        repl1.Val

Every definition of an input is made public, since a later input reaches it by
import. `pub` goes in front of what the user wrote, which keeps its columns, so
an error in it still lands where the user wrote it. The rule to state is that a
REPL definition has no visibility, which is also true of GHCi and OCaml, where
the prompt has no export list at all. What it costs is one diagnostic a file
would give and the REPL therefore never gives — a public type exposing a
private one.

Two consequences worth stating as rules rather than accidents:

- the definitions of one input are compiled together and see only the inputs
  before it. This is what lets them be mutually recursive, and it is why a
  function cannot read a `let` of its own input;
- the shadowed name is reachable qualified, `repl1.g()`, and writing it is what
  brings its module in. GHCi asks for no import either, and the name is one the
  user read in an error before writing it.

Left untouched: mutual recursion still has to happen within one input — which
all three already agree on.
