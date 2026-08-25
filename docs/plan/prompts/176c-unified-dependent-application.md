---
id: 176c
slug: unified-dependent-application
status: done
depends_on: [170, 176b]
phase: 3
---

# Give Dependent Application One Surface Form

## Task

Repair the source language before prompt 177 writes another indexed family. Remove the surviving split between
angle-bracket “type-parameter application” and parenthesized value/index application: Musa has one dependent function
space and therefore one application syntax. Preserve the genuine inductive-family distinction between declaration-wide
uniform arguments and constructor-selected indices in the family declaration and eliminator, not at use sites. Replace
the constructor-only `: (indices...)` abbreviation with a complete constructor result type.

This prompt deliberately amends the candidate language specification before changing code. It is a clean source break
inside the unreleased language pass; no compatibility parser, formatter mode, or dual spelling survives it.

## Read

- `docs/rules/{README,constitution,obligations}.md`;
  `docs/rules/language/{00-semantics,01-surface,02-core-calculus}.md`, especially the one-theory amendment and the rule
  that binder filling is not a second Π.
- Prompts 143, 144, 153, 156, 170, and 176b; note 53's one-theory argument and note 74's pattern-unification closure.
- `crates/musa-syntax/src/parser/{types,declarations}.rs`, `crates/musa-compiler/src/lower/types.rs`, and
  `crates/musa-calculus/src/{elaboration/declare.rs,kernel/family/}`. The current lowerer explicitly collapses
  `AppliedType` and `IndexedType` into the same core application.
- The local Agda manual `~/Code/agda/doc/user-manual/language/data-types.lagda.rst`: parameters occur before the family
  signature and indices in it, while constructor results name the complete family application. Take the distinction, not
  Agda's whitespace application syntax.
- Peyton Jones chapters 2–5 and 8–9: application is an operation of the term language; structured constructors end in
  the type they construct; type checking, not punctuation, checks an argument against its domain.

## Design

### One application

`F(a, b)` is the only source application, whether `F` returns data, a type, or another function. `List(Nat)`,
`Result(Text, Nat)`, `Pc(12)`, `Vec(Nat, n)`, and `map(function, values)` share one CST/application lowering path and
one core spine. The domain of the reached Π decides whether an argument checks at `Type`, `Nat`, or another type. Delete
`<...>` declaration/application syntax, `AppliedType`/`IndexedType`, and every lowering rule that classifies an argument
as a type or value before checking it.

An inferred binder remains a property of a Π because it changes what a caller writes. Spell it in the ordinary binder
list as `{A: Type}` and retain the existing explicit filling `{A = Nat}`. Thus `fn identity({A: Type}, value: A) -> A`
is called as `identity(value)` or, when needed, `identity({A = Nat}, value)`. This is inference versus writing, not type
versus value application.

### Uniform family arguments and indices

A family declaration writes its uniform arguments in the declaration's ordinary binder list and its remaining dependent
signature after `:`:

```musa
data Vec(A: Type): (length: Nat) -> Type {
    Nil: Vec(A, 0),
    Cons(n: Nat, head: A, tail: Vec(A, n)): Vec(A, successor(n)),
}
```

`A` is scoped once over every constructor and fixed in every result. `length` belongs to the family signature; a
constructor supplies its value in its complete result, and matching may refine it. A family with no uniform arguments
may begin its signature immediately: `data ControlKey: (kind: ControlKind) -> Type { ... }`. An unindexed `data`,
`enum`, or `record` may omit the redundant `: Type` and constructor result exactly where the result is uniquely
determined.

Do not add an `index` keyword or infer uniformity from constructor bodies. Treating every family argument as an index
would change motives and require constructors to rebind declaration-wide context; inferring uniformity would make a
constructor edit silently change the eliminator. The existing core parameter/index representation remains and is
validated against the complete written constructor result.

### Clean migration

Migrate every committed `.musa` source, fixture, generated-source writer, formatter snapshot, teaching example, and
pending prompt. Historical completed prompts remain records, but governing/candidate rules and all future directives
must teach only the surviving syntax. Record the amendment under `docs/notes/research/language-design-closure/` and add
its six required answers to `docs/rules/README.md`; update the code map.

The hand parser owns the syntax. Move tree-sitter-musa, its corpus, queries where node names change, and all three
generated grammar files in the same commit. The drift law must compare the new syntax against the real lexer/parser.

## Target

- Candidate-language amendment, research record, code-map update, and repaired pending prompt cone.
- One source application CST and lowering path; no angle-bracket parameter syntax and no type/value application split.
- Ordinary inferred binders inside declaration parameter lists, using the existing pattern-unification filling rules.
- Family signatures separating uniform arguments from indices, with complete indexed-constructor result types.
- Full standard-library/example/test/generated-source migration with formatter and diagnostic laws for the clean break.
- Up-to-date tree-sitter grammar, corpus, queries, generated parser files, and token drift law.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && npm test
! rg -n '[A-Za-z][A-Za-z0-9_:]*<' stdlib examples crates/musa-compiler/tests editors/tree-sitter-musa/test -g '*.musa'
! rg -n 'AppliedType|IndexedType|DataChosen' crates/musa-syntax crates/musa-compiler editors/tree-sitter-musa
```

Commit as `Unify dependent application in Musa source`.

## Stop

- No change to core β/η/conversion, pattern-unification discipline, family elimination, positivity, or totality.
- No inference of uniform parameters from constructor bodies and no encoding of every uniform parameter as an index.
- No compatibility alias for `<...>` or `: (indices...)`; this is the language pass's clean break.
- No new musical vocabulary, performance policy, event-track operation, machine primitive, or runtime feature.
- No editor-extension release or external-repository pin update; this prompt owns the in-repository grammar artifact.
