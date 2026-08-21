---
id: 162
slug: delete-the-module-layer
status: pending
depends_on: [161]
phase: 3
---

# Delete the ML Module Layer

## Task

`signature`, `structure`, `template structure` and `make` are an ML module system bolted onto a dependently typed
language that can already express everything they do. Delete them: a signature is a record type, a structure is a `let`,
a template structure is a function, and `make` is a call. **Seven declaration sites in the whole corpus.**

## Read

- `docs/rules/language/01-surface.md`, the module-layer sections prompt 145 marked deprecated-and-owned-by-161.
- The seven sites, found by `grep -rn 'signature\|structure' stdlib/ examples/`.
- `crates/musa-compiler/src/module.rs` — including its member walk, which reads a structure's `let` and `fn` members
  only and never registered a `data` member. That gap is why one law about structure-internal data was deleted rather
  than fixed during this overhaul.

## Design

**The translation, in four lines.**

| Was | Becomes |
| --- | --- |
| `signature S { … }` | `record S(…) { … }` |
| `structure X: S { … }` | `let X : S = S(…)` |
| `template structure F(A: S): T { … }` | `fn F(a : S) -> T { … }` |
| `make F(X)` | `F(X)` |

**The abstract-type question, answered here rather than assumed.** A signature's `data Hidden;` withholds a constructor.
A record field of type `Type` carries the same information — and it is exactly the construct that puts a record at
`Type 1`, which is why prompt 152 comes first and why this prompt is the one that *measures* whether the hierarchy was
needed. If the stdlib after this rewrite never needs a third level, say so in the commit; if it does, that is 151 paying
for itself.

**Sealing is module privacy, not a second mechanism.** What `signature` bought that a record does not is that a
constructor could be withheld. Musa already has module-private definitions and `Refusal::Private` already fires for
them; sealing becomes "do not export the constructor," which is one rule instead of a layer.

**Why the layer was wrong in the first place, recorded so it does not return.** *No sublanguage by subtraction* —
AGENTS.md's standing rule — cuts both ways. The module layer was a sublanguage by *addition*: a second, weaker
abstraction mechanism beside the one dependent records already provide, with its own scoping, its own matching rule, and
its own diagnostics, none of which composed with the rest of the language.

## Target

- `crates/musa-syntax`: the four keywords removed from the grammar; `editors/tree-sitter-musa` kept in step under the
  drift law.
- `crates/musa-compiler/src/module.rs`: deleted, its callers routed through ordinary resolution.
- `stdlib/`, `examples/`: the seven sites rewritten.
- `docs/rules/language/01-surface.md`, `docs/book/`: the sections removed.
- `crates/musa-compiler/tests/suite/`: the module-layer law suites deleted; the sealing law restated as a module-privacy
  law over a `data` declaration, which is the form that will still exist.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
! grep -rnE '\b(signature|structure|make)\b' stdlib/src examples --include=*.musa
pnpm -r test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

`pnpm -r test` is here because the grammar moves and the tree-sitter drift law is checked on the other build system.

Commit as `Delete the ML module layer`.

## Stop

- No new visibility mechanism. Module privacy exists; use it.
- No functor-like abstraction added back under another name.
