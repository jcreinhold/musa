---
id: 147
slug: term-and-binder-collapse
status: in-progress
depends_on: [145]
phase: 3
---

# Collapse Seventeen Shapes to Fourteen: One Binder Node, One Literal Node

## Task

`Shape` has seventeen variants, and three groups of them are one variant wearing different hats. Collapse `Lam`/`Pi`/
`Let` into one `Bind` node carrying a `Binder`, collapse `Lit`/`Numeral` into one `Lit` node carrying a `Constant`,
rename `Hole` to `Meta`, and fix the naming while the file is open. **Mechanical: no semantics change and no program's
acceptance moves.**

The other half of `02-core-calculus.md` §1's seven — `Const`/`Def`/`Base`/`Builtin` becoming one `Named` whose
reduction behaviour the *context* answers for — is prompt 147a. It was in this prompt and is not any more; the repair
note at the end of **Design** says what the evidence was.

## Read

- `crates/musa-calculus/src/term.rs` — the seventeen, and the doc comments that already say which ones overlap.
- `crates/musa-calculus/src/{eval,quote,convert}.rs` — the three traversals that shrink.
- `/Users/jcreinhold/Code/Idris2/src/Core/TT/Term.idr` — twelve constructors; musa keeps seven, and the five it drops
  are dropped for reasons prompt 143 records.

## Design

**What this prompt collapses.**

```rust
enum Shape {
    Bind { name: Name, binder: Binder, body: Term },   // was Lam, Pi, Let
    Lit  (Constant),                                   // was Lit, Numeral
    Meta (…),                                          // was Hole
    …                                                  // Const, Def, Base, Builtin: 147a
}

enum Binder   { Lam, Pi { filling: Filling, ty: Term }, Let { ty: Term, value: Term } }
enum Constant { Payload(base::Literal), Numeral(family::Numeral) }
```

**Where each of the seventeen goes.** `Lam`/`Pi`/`Let` → `Bind` + `Binder`. `Lit`/`Numeral` → `Lit(Constant)`. `Hole` →
`Meta`. `Const`/`Def`/`Base`/`Builtin` → `Named` + `Definition`, at 147a. `RecordType`/`Record`/`Project` stay for now —
157 moves them, and moving them here would need families, which do not exist yet. `Indexed` stays for now — 151 deletes
it, separately, because that deletion has an argument attached and this one must not. `Var`/`App`/`Universe` unchanged.
Seventeen become fourteen here and eleven at 147a.

**Why `Bind` saves nothing and is still right.** Three constructors become one plus a three-way tag: no net saving in
variants. The saving is that "go under a binder" is written once instead of three times, across `eval`, `quote`, `zonk`,
and the shrink traversals — which is exactly where the crate's binder bugs would live. `Binder::outer` is what makes
that one arm possible: it answers which subterms are read *outside* the binder, which is the whole of what differs
between a λ, a Π, and a `let` as far as a structural walk is concerned.

**`Binder::Lam` carries nothing.** The sketch this prompt was written from gave a λ a `filling` and a `ty`; the term
language has never had either. A λ's domain is read off the Π it is checked against — that is what bidirectional
checking *is* — and a filling is a question about who writes an argument, which a λ does not take. Giving the arm two
fields no constructor could fill would be a form with an unreachable state in it.

**`Constant` keeps its two arms and the reason is a rule, not taste.** Two rules depend on the split: §3 compares a
numeral *as a number*, and ι decrements on a numeral, which is what lets `Nat`'s eliminator fire without unfolding a
tower of `Succ`. `Numeral` holds `crate::family::Numeral` rather than re-spelling its two fields, because that struct
already exists, already carries `{ family, count }`, and already owns the equality the rule needs.

**The naming, decided.** `Hole` → `Meta`: `Hole` is surface syntax and `EventsHole` already takes the word at the
surface; `MetaSource` already exists in the crate, so the codebase had agreed and the term language had not. `PiInfo` →
`Filling`, because the question it answers is who writes the argument; the arm names stay `Written`/`Parameter`/
`Constraint`, which prompt 146 left as the three fillings that exist. `Index` keeps its name; `DbIndex` is rejected —
the `Db` reads as *database*, and the pair `Index`/`Level` already says which side of the index/level duality each is
on. `DbLevel` and `Depth` merge into one `Level`: they are a position and a count into the same environment, and
carrying both invites using one where the other is meant. `Sort` takes the universe word so `Level` means one thing;
152 gives `Sort` its contents. `show.rs`'s private `Level` becomes `Precedence`, which is what it always was.
`family::Binder` becomes `family::Parameter`, because one crate cannot spell a telescope position and a term's binder
the same way and expect the mix-up to be caught — the same argument `Level`/`Sort` is made on.

**Repaired at implementation (prompt 147a split out).** This prompt asked, in one commit, for both the collapse above
and the relocation of reduction behaviour into the context. Implementation evidence says those are two prompts:

- A recursor's motive universe is part of a term's identity — `family::Constant`'s `PartialEq` says so, and
  `case.rs`'s `motive_level` mints `Nat.elim` at `Type 1` whenever a `match` computes a type. A `Named { name, role }`
  whose `Role` is `Function | Constructor | TypeConstructor` cannot tell those two terms apart, so the sketch as
  written moves acceptance — which this prompt's own oracle forbids. Fixing it needs either a fourth role or
  level-polymorphic recursors, and the second is prompt 152, which this prompt's **Stop** excludes.
- `Definition::Compiled(CaseTree)` cannot hold what a definition holds today — an elaborated value and its type — and
  case trees are prompt 155's, stubbed until then. The arm that would carry today's definitions is not in the sketch.
- Reaching a definition by name at reduction time needs the definition table to reach `eval`, `apply`, and every
  closure applied under `quote` and `convert`. That is an architecture this prompt does not describe and cannot be
  read into it.

None of that touches the collapse above, which is mechanical exactly as claimed. 147a states the relocation with those
three answered.

## Target

- `crates/musa-calculus/src/term.rs`: `Bind` + `Binder`, `Lit(Constant)`, `Meta`, `Filling`, `Index`, `Level`, and
  `Sort` (two-point, unchanged in behaviour until 152).
- `crates/musa-calculus/src/{eval,quote,convert,show,elab/}`: the traversals, rewritten against the new shape — one
  binder arm where there were three, one literal arm where there were two.
- `crates/musa-calculus/src/sort.rs`: `level.rs` renamed, holding `Sort`.
- No change to `crates/musa-compiler` beyond what the renamed variants require; the facade does not move.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

**The oracle is exact acceptance.** This prompt changes no program's verdict. `--run-ignored all` must show the same
failure list, byte for byte, that prompt 166's Check records — same tests, same messages, same counts. A message that
changes wording is acceptable only where a type name in it changed; a *count* that changes means a semantics change
crept in and the prompt is not done.

Commit as `Collapse the term language's binders and literals`.

## Stop

- No `Named` and no `Definition` (147a): `Const`, `Def`, `Base`, and `Builtin` stay exactly as they are.
- No `Indexed` deletion (151), no records-as-data (157), no case trees (155), no metavariable solving (153), no
  universe change (152).
- No new diagnostics. Wording repairs only where a renamed type appears in a message.
