---
id: 146
slug: delete-the-trait-system
status: pending
depends_on: [145]
phase: 3
---

# Delete the Trait System, and Put Type-Directed Disambiguation in Its Place

## Task

Delete `class.rs` (421 lines), `dictionary.rs` (1,170), and the trait region of `musa-compiler`'s prelude (~521), and
rewrite the three stdlib traits as ordinary records. Add type-directed name disambiguation, which is what the trait
system was actually being used for. **This prompt is independent of every other B-phase prompt** and can be worked first
or last; it is placed first because it removes ~2,100 lines the later prompts would otherwise have to carry through
every refactor.

## Read

- `crates/musa-calculus/src/class.rs`, `dictionary.rs` — what is being deleted.
- `crates/musa-compiler/src/prelude.rs`, the trait region — in particular `eq_instances()`, whose five instance bodies
  are the five δ-builtins with no λ around them, and `iterable_list()`, which hand-writes `fold_from_end` as a `rec` +
  `match` in about forty lines of Rust AST builder.
- `stdlib/src/algebra.musa` — the three taxes, in that file's own words.
- `/Users/jcreinhold/Code/Idris2/src/TTImp/Elab/Ambiguity.idr` — the disambiguation model.
- `docs/rules/language/01-surface.md` §1.4 as prompt 145 left it — "the grammar keeps `trait`, `impl`, and `where` until
  that prompt runs" names *this* prompt, so the surface forms come out here.
- `crates/musa-syntax/src/parser/declarations.rs`, `syntax_kind.rs`, `keywords.rs`, and
  `editors/tree-sitter-musa/grammar.js` — the four places the surface forms are written down, held together by the drift
  law.

## Design

**The measurement that justifies deletion.** Six traits. 82 call sites. **Zero trait-constrained signatures** — not one
function in the corpus is polymorphic over a trait. Two `Iterable` instances (`List`, `Option`). Three derived methods
with zero call sites. `Eq`'s five instances *are* five builtins. A dispatch mechanism with nothing to dispatch on is a
name-resolution mechanism wearing a costume, and that is what replaces it.

**What replaces it, in three parts.**

*Structures become records.* `Group<G>` becomes `record Group(G : Type) { unit : G, compose : G -> G -> G, inverse : G
-> G }`, and an instance becomes a value. This is strictly more than the trait had: a record is first-class, so a
function may take two groups, return one, or store one in a list. `algebra.musa`'s own comments name all three things
the trait could not do; a record does them without a mechanism.

*Overloading becomes disambiguation.* `==` at five types is five names in scope and a checker that already knows the
expected type. This is local bidirectional elaboration — it needs nothing from prompt 153 — and its failure mode is a
diagnostic listing the candidates and the type that ruled each out, which is strictly better than "no instance found".

*The rest becomes macros.* Anything genuinely wanting dispatch on an open set is a macro's job, and prompt 160 is where
that lands.

**The grammar goes with the mechanism, and this is the repair.** The prompt as first written deleted the elaboration and
left `trait`, `where`, and the constraint syntax standing in the parser. `01-surface.md` §1.4 — governing since prompt
145 — says the grammar keeps them only until this prompt runs, so leaving them is drift of exactly the kind the root
`AGENTS.md` forbids, and the alternative is worse: a lowerer that parses a `trait` in order to refuse it is a
transitional diagnostic the very next prompt would delete. `trait`, the `where` clause, and the constraint production
come out of `musa-syntax` and out of the tree-sitter grammar in the same commit as the mechanism they spelled.

`impl` **stays**, and that is why this is not a keyword sweep: an `impl` block without a trait head declares inherent
items in a type's namespace (`01-surface.md` §1.4's closing paragraph), which is a namespace and not a dispatch
mechanism. What comes out of the `impl` production is its optional trait head and its optional `where`.

**The `fold` question, answered concretely.** Deleting `Iterable` does not mean hand-writing `list_fold`, `nat_fold`,
`option_fold`. Prompt 156 gives every declared family a generated eliminator, so the fold for a family *is* its
recursor, generated. `iterable_list()`'s forty lines of Rust go away entirely — they were a hand-written catamorphism
standing in for the one the family already implies.

**What must not regress.** `Bool` and `Nat` have no `Eq` instance today, on the stated ground that they are declared
families and ι already answers. That reasoning survives and generalizes: after this prompt, *every* declared family
answers equality by its own eliminator, and the five δ-builtins stay δ-builtins for the five base types.

## Target

- `crates/musa-calculus`: `class.rs` and `dictionary.rs` deleted; every `pub(crate)` path into them removed;
  `Shape`/`Form` variants for dictionaries and instances removed.
- `crates/musa-compiler/src/prelude.rs`: the trait region deleted.
- `crates/musa-compiler`: a disambiguation module — candidates by name, filtered by expected type, one diagnostic.
- `stdlib/src/algebra.musa`: `Group`, `Action`, `Torsor` as records, with the carrier first as a field-order convention
  rather than a resolution key.
- `stdlib/`, `examples/`: every `impl` and every trait-method call site rewritten. The 82 sites are the measurement.
- `crates/musa-compiler/tests/suite/`: `trait_laws.rs` and `operator_laws.rs`'s trait sections deleted; a disambiguation
  law suite added.
- `crates/musa-syntax`: `trait`, the `where` clause, and the constraint production removed from the keyword table, the
  syntax kinds, the parser, the typed AST, the highlighter, and the formatter; `impl`'s trait head removed and its
  inherent form kept; `trait_syntax_laws.rs` deleted.
- `editors/tree-sitter-musa/`: the same three productions removed, the corpus refreshed, and the drift law green.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
! grep -rn 'trait\b' stdlib/src --include=*.musa
! test -f crates/musa-calculus/src/class.rs
! test -f crates/musa-calculus/src/dictionary.rs
! grep -rn 'TraitKw\|WhereKw\|TraitDecl' crates/musa-syntax/src
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

`--run-ignored all` may still show the staff class prompt 166 owns and nothing else. Any new red is this prompt's.

Commit as `Delete the trait system`.

## Stop

- No change to the term language. The collapse is 147 and mixing them makes both diffs unreadable.
- No new record features. Records stay exactly what they are today until 157 turns them into data.
- No proof search, no instance arguments, no "one small dispatch case". That is the appendage this prompt removes.
- No removal of the `impl` keyword. The inherent form is a namespace and `01-surface.md` §1.4 keeps it by name.
