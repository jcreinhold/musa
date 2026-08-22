---
id: 154
slug: implicit-arguments
status: in-progress
depends_on: [153]
phase: 3
---

# Make Implicit Arguments Real and Writable

## Task

An inferred binder is half real. An author writes `fn compose<A, B, C>(…)` and the lowering turns each type
parameter into a `Filling::Parameter` Π, which `advance` fills with a metavariable at every use. What is missing is
everything around it: the parameter's type cannot be stated, an argument cannot be supplied by the *name* of the binder
it fills, and a value whose own type is an inferred Π loses that type the moment it is used as a value. Prompt 145 wrote
the two spellings that close the first two gaps into `01-surface.md` §1's grammar and nothing implements them.

**Corrected against the code, and each correction is measured.**

- *An author can already write one.* `lower/items.rs`'s `type_parameters` and `Raw::parameter_pi` turn every `<A>` into
  an inferred binder; `stdlib/src/core.musa`'s `compose<A, B, C>` is one. What an author cannot write is the parameter's
  **type** — `type_parameters` hardcodes `Type 0` — which is what `01-surface.md` §1's `"{" IDENT (":" type)? "}"` is
  for.
- *Insertion at application exists* — `elab/spine.rs`'s `advance`, and this prompt does not re-implement it.
- *Insertion at check exists too* — `elab/check.rs`'s `abstracted`, which the suite already pins as "a term checked
  against an implicit Pi is abstracted, not switched". So the rule this prompt was written to add is present, and the
  Design's statement of it had its two types the wrong way round: it is the **expected** type being an inferred Π that
  inserts the λ.
- *The stopping rule is genuinely missing, and it does not stop a loop.* Because `abstracted` fires first, `id` used as
  a value η-expands to `λX. id X` and terminates. What it does instead is **refuse**: a term of type
  `{A : Type 0} → {B : Type 0} → A → A`, checked at that very type, is "could not determine a type parameter", because
  `abstracted` binds `A` and `B` as λs and `Switch` then inserts a fresh `?B` that nothing can solve. Measured in this
  repo before a line of 154 was written.

Prompt 153's repairs narrowed what is left here further: the two-pass deferral walk in `elab/spine.rs` stands alongside
the constraint queue rather than being replaced by it — `01-surface.md` §1.5 requires it and 153's second repair records
the measurement — so insertion at application keeps meeting it, unchanged.

## Read

- `crates/musa-calculus/src/kernel/term.rs`, `Filling` — **three** arms, not the two 147 named: 149 added
  `Constraint`, which carries `Storable` and no source program can write. This prompt adds no fourth.
- `/Users/jcreinhold/Code/Idris2/src/TTImp/Elab/App.idr` — insertion at application, and where it stops. Read for the
  stopping rule; the surrounding design is Idris's and musa's `abstracted` differs, as `Task` records.
- `docs/rules/language/01-surface.md` §1 — the two grammar lines prompt 145 added and left without prose:
  `type-param := IDENT | "{" IDENT (":" type)? "}"` and `arg := expr | "_" | "{" IDENT "=" expr "}"`. Read §1.5 with
  them: a type parameter is "solved by elaboration, never written", so *both* type-parameter spellings are inferred and
  the braces are what lets one state its type.
- `crates/musa-calculus/src/elaboration/elab/spine.rs` — `advance` and `Slot`: insertion at application, and where a
  named argument has to be placed.
- `crates/musa-calculus/src/elaboration/elab/check.rs` — `abstracted`: insertion at check, already written.
- `crates/musa-calculus/src/elaboration/raw.rs` — `RawShape::Call`, which is what an argument list becomes, and
  `Raw::parameter_app`, the positional written implicit the host's schemes already use.
- `crates/musa-syntax/src/parser/functions.rs` `type_params` and `crates/musa-syntax/src/parser/expressions.rs`
  `expr_arg_list` — the two parsers that reject the brace forms today.
- `crates/musa-compiler/src/lower/items.rs` `type_parameters` and `crates/musa-compiler/src/lower/values.rs`
  `arguments` — where each brace form has to arrive.

## Design

**Insertion is at application and at the end of checking, and both rules are already written.** When a function's type
is an inferred Π and the next written argument is not that argument, insert a metavariable — `advance`. When the
**expected** type is an inferred Π and the term's own form does not abstract that binder, insert a λ — `abstracted`.
This prompt adds neither; it adds the rule that says when the first one must *not* fire.

**Where insertion stops is the whole design.** Insertion at application stops when the head is being applied to no
further written arguments *and* the type the call is checked against is itself an inferred Π at the same binder. And
`abstracted` gains its companion, because in musa it is the rule that runs first: it does not abstract a binder the term
is about to supply itself. Without the pair, a value keeps none of its own scheme — `id` becomes `λX. id X`, which is
η-equal and harmless, and a phantom parameter becomes "could not determine a type parameter", which is not. That refusal
is the law, and it is what `implicit_laws.rs` states: a term of type `{A} → {B} → A → A` is accepted at its own type.

**A type parameter states its type in braces.** `<A>` is `<{A : Type 0}>`, which is what `type_parameters` already
builds; `<{n : Nat}>` is the form that has no other spelling, and it is what a dependent signature needs — an inferred
parameter that is not a type. Both are inferred: `01-surface.md` §1.5 says a type parameter is never written at a use
site, and §1's brace comment describes the form rather than contrasting it with the bare one.

**Named implicits, because inference genuinely cannot always reach.** Musa spells it `f({A = Nat}, x)` — an argument
list, so it rides on `RawShape::Call` rather than on a spine of `Raw::parameter_app`. The name is the *callee's binder
name*, which the lowering does not know and the core does, so unlike a `_` section it cannot be translated away in
`lower/values.rs`: it travels on `Call` and `advance` places it when it meets the binder that bears the name. A name no
inferred binder of the callee bears is refused, naming the function and listing the names it does bear.

**No `auto` implicits and no proof search.** Idris2's third `PiInfo` arm runs a search procedure to fill an argument.
Prompt 143 refused it: it is the trait system's dispatch problem re-entering through the type language, and 146 deleted
that system on measured evidence. `Filling` has three arms — `Written`, `Parameter`, and 149's `Constraint`, which
carries `Storable` and which no source program can write — and gains no fourth.

**The stdlib is the measurement.** After this prompt, a `.musa` signature can state an inferred parameter's type, and
the nine generic signatures in `stdlib/src` still elaborate with every type parameter inferred at every use site. If a
use site has to write one, inference is not working and the prompt is not done — that is a sharper check than any
synthetic test, and it is why 152 comes first.

## Target

- `crates/musa-calculus/src/elaboration/raw.rs`: a named inferred argument on `RawShape::Call`, and the constructor that
  builds one.
- `crates/musa-calculus/src/elaboration/elab/spine.rs`: `advance` places a named argument at the binder that bears the
  name, refuses a name no binder bears, and stops inserting at the walk's end when the expected type is an inferred Π.
- `crates/musa-calculus/src/elaboration/elab/check.rs`: `abstracted`'s companion — it does not abstract a binder the
  term supplies itself.
- `crates/musa-syntax`: `type_params` accepts `{ IDENT (":" type)? }`, `expr_arg_list` accepts `{ IDENT "=" expr }`.
- `crates/musa-compiler/src/lower/`: `type_parameters` reads the stated type instead of hardcoding `Type 0`, and
  `arguments` carries a named inferred argument through to `Call`.
- `stdlib/`: an inferred parameter's type stated where the brace form says more than the bare one; every existing
  signature keeps working with nothing written at a use site.
- `crates/musa-calculus/tests/suite/implicit_laws.rs`: the stopping rule as a test that is *refused* without it —
  `{A} → {B} → A → A` at its own type — plus the named argument and the name that names nothing.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- format --check stdlib/src
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

**The re-checker's obligation for this prompt**: an inserted implicit argument is an ordinary application in the
finished term, so the existing arm covers it — but the suite must contain a fixture whose implicits were *inserted*
rather than written, re-checked. Prompt 158 audits it.

Commit as `Make implicit arguments real and writable`.

## Stop

- No `auto`, no instance arguments, no search.
- No change to how explicit arguments elaborate.
- No change to what a *bare* `<A>` means. It stays an inferred parameter at `Type 0`; giving it a fresh level unknown
  instead is a level-inference change and belongs with 152's own work, not here.
- No change to `docs/rules/language/`. §1's two grammar lines and §1.5's rule are what this prompt implements.
