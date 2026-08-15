---
id: 138
slug: typed-syntax
status: pending
depends_on: [137]
phase: 3
---

# Give Syntax a Category, and the Phase API Its Types

## Task

Replace the phase's one untyped `Syntax` with the indexed family `Syntax<Cat>` over `Expr`, `Item`, `Pattern`, and
`TokenTree`; replace the stringly-typed token-kind and delimiter arguments in the phase registry with real enums; and
implement the derived-provenance representation `Derived { origin, quotation, path }` that prompts 139 and 140 mint.
Nothing an adapter writes should still be a string that the compiler parses back.

## Read

- `docs/rules/language/11-quotation.md` as prompt 131 wrote it — `Syntax<Cat>`'s index, the derived-identity law, and
  the list of what survives of the sealed-step recursor. This prompt implements the type; 139 and 140 implement the
  forms that build and match it.
- `docs/rules/language/00-semantics.md` §2 as 131 amended it — the phase environment `Σφ`, the phase-local types, and
  the two-descent rule.
- `crates/musa-compiler/src/core.rs`'s `SYNTAX_OWNERSHIP` — 14 phase operations, and the ones that take a kind or a
  delimiter as `Text`: `syntax_token`, `syntax_group`, `syntax_identifier`, `syntax_number`, and `syntax_built`. Read
  what each hides, because the ownership entry is the argument for the operation existing at all and a retyped operation
  still owes one.
- `stdlib/src/adapters/staff.musa`'s dispatch table and every `text_equal(kind, "…")` in it — the call sites this prompt
  makes ill-typed rather than merely discouraged.
- `crates/musa-compiler/src/syntax.rs` and `derivation.rs`, and prompts [127da](127da-path-aware-syntax.md) and
  [127db](127db-derivation-graph.md) — the path and derivation-graph representations that already exist. `Derived`
  grafts onto that graph; it does not start a second one.
- `docs/rules/across-stages/04-identity-and-realization.md` — derived identity is an across-stage contract, so the
  representation has to be expressible in its vocabulary. If it is not, that is a governing-document repair and a stop,
  not a quiet extension.

## Design

**The index is the point.** Today an adapter builds a group and then finds out at expansion time whether an expression
was wanted; with `Syntax<Cat>` that is a type error at the construction site. The four categories are exactly the four
the surface grammar distinguishes, and there is deliberately no fifth: a category that no program in prompt 132's trial
needed is a category to add later with evidence.

**There is no cast between categories.** Not `unsafe`, not compiler-only, not "just for the parser". A conversion that
exists will be used, and every use of it is the untyped `Syntax` back again with extra steps. Where a genuine change of
category is meant — an expression used as an item's body — it is a constructor of the target category taking the source
category, which is a fact about the grammar and belongs in the family's declaration.

**`TokenKind` and `Delimiter` become enums**, which is only possible now because prompt 136 gave enums namespaced
constructors: the phase's `Paren` and the surface's `Paren` can coexist. Each enum is generated from the same lexer
table `musa-language` already owns rather than hand-written beside it, so the drift law covers it: a token kind the
lexer knows and the phase does not is a build failure, not a silent gap.

**Provenance is a representation, not a convention.** `Derived { origin, quotation, path }` grafts onto 127db's
derivation graph: `origin` is the node the quote was written at, `quotation` identifies the quote, `path` is the
position within it. The identity law is the one 131 stated — same origin, same quotation, same path is the same derived
node, and nothing else is — and it is testable *here*, before any syntax can be quoted, by minting derived nodes
directly. Do that: a law that only becomes testable two prompts later is a law nobody checks.

**`syntax_built`'s role integer stops being an argument.** It was the author's job because nothing else knew the
position; now the path does. The operation keeps its ownership entry — it still hides the derivation graph — but its
signature loses the hand-allocated number. Adapters that still pass one fail to compile, and that is the intended
migration: prompt 145 rewrites the only one.

**What the registry looks like afterwards** is part of this prompt's output, not an accident of it: state the 14
entries' new signatures, say which hid nothing once the types were real, and mark those for deletion in 143 rather than
deleting them here — 139 and 140 still need a working phase API to build on.

**Laws.** A category mismatch is a type error, tested per pair. `TokenKind` covers the lexer's table exactly, tested by
generation. Derived identity is injective in all three components and stable across re-elaboration. The existing
`recurse_syntax` and `run_syntax_step` laws from prompt 127dcfaf still hold at every category. The phase's types remain
non-storable — `Syntax<Cat>` never crosses the expansion boundary as data.

## Target

- `Syntax<Cat>` in the phase environment, with the four categories and their constructors, and every `SYNTAX_OWNERSHIP`
  entry retyped.
- Generated `TokenKind` and `Delimiter` enums, tied to `musa-language`'s lexer table by a drift test.
- `Derived { origin, quotation, path }` grafted onto the derivation graph, with its identity law tested directly.
- `syntax_built` without its role-integer argument.
- The registry survey: new signatures, and the entries marked for deletion in 143 with the reason.
- `crates/musa-compiler/tests/suite/` cases for the category laws, the generation drift test, and derived identity.
- `docs/plan/code-map/` rows.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-language -p musa-compiler -p musa-core
cargo nextest run --workspace
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-core -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

`stdlib/src/adapters/staff.musa` must still expand: this prompt changes `syntax_built`'s signature, so the adapter's
call sites move with it, mechanically and without redesign. A rewrite of the adapter here is prompt 145's work done in
the wrong commit.

Commit as `Give syntax a category, and the phase API its types`.

## Stop

- No quotation form and no pattern form. Prompts 139 and 140.
- No cast, coercion, or escape between categories, under any spelling or visibility.
- No fifth category, no user-declared category, and no category parameterized by anything.
- No deletion of `recurse_syntax`, `run_syntax_step`, or the derived `fold_syntax`. Prompt 131 said they survive.
- No deletion of registry entries. Prompt 143.
- No rewrite of `stdlib/src/adapters/staff.musa` beyond the mechanical signature change. Prompt 145 measures the
  rewrite, and a partial rewrite here would corrupt that measurement.
