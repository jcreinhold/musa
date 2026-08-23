---
id: 160
slug: macros-as-functions
status: pending
depends_on: [159]
phase: 3
---

# Macros as Ordinary Total Functions Over One Evaluator

## Task

Musa's case for existing is that a composer can write music-theoretic transformations as language, so the macro layer
matters more than any other part of this overhaul. **It is built.** Prompts 138–159 delivered every clause of what this
prompt was written to deliver, and none of them was this one. Finish it the only way that is left: make the claim
*checkable in one place*, and write the chapter that teaches it.

## Read

- `docs/rules/language/11-quotation.md` after 145 and 159, and
  [`55-cat-stays-a-base-type.md`](../../notes/research/language-design-closure/55-cat-stays-a-base-type.md) for why
  `Cat` is not a family.
- [`56-the-macro-layer-is-already-one-thing.md`](../../notes/research/language-design-closure/56-the-macro-layer-is-already-one-thing.md)
  — the audit behind this repair, claim by claim with the prompt that met each.
- `/Users/jcreinhold/Code/Idris2/src/TTImp/Reflect.idr` and `Core/Reflect.idr` — roughly 3,200 lines of Reify/Reflect,
  which is the thing musa is *not* building. Read it to see the size of what one evaluator saves.
- `crates/musa-compiler/src/phase/adapter.rs`'s `read_adapter_module` and `phase/transform.rs`'s `expand_syntax` — the
  two functions that make "expansion is evaluation" a fact rather than a plan.
- `docs/rules/across-stages/` — the `Original` provenance a macro must not lose.
- `docs/book/src/SUMMARY.md` and `docs/book/src/concepts/provenance.md` — where the chapter goes, and the voice it is
  written in.

## Design

**One macro system, not two.** Idris2 carries syntax rewriting *and* elaborator reflection: a `%macro` runs in an `Elab`
monad, and every type that crosses between the object language and the macro language needs a Reify and a Reflect
instance. Musa needs neither, because a macro is a function over ordinary values and the evaluator that runs it is the
one that already normalizes everything else. **This is the one place musa ends up simpler than the reference, and it
comes directly from having committed to a single theory.** That paragraph is the thesis of the chapter this prompt
writes.

**The scope repair, recorded.** *Repaired before implementation; note 56 is the measurement.* The Task read "Finish it",
and eight of eight of the properties it names are already held and lawed — ordinary function, one evaluator, typed
quotation, splicing, hygiene, provenance, totality, phase conservativity. Two of its sentences describe a language musa
does not have:

- *"After 159, `c` is an index a `match` can refine."* 159 established the opposite. `Cat` is a base type with literal
  values and `Syntax` has no eliminator, so there is no `match` on a syntax value for an index to be refined in. The
  category-refinement law cannot be written; note 55 §5 states the condition that would re-open it.
- *"the builtins 159 removed no longer needed."* 159 removed a coercion rule and *added* a row. Nothing in `expand/` is
  holding a builtin it no longer needs.

And "the quotation vocabulary as library functions" is not available either: the vocabulary is eighteen δ-builtins and
two traversals, and a δ-rule is a host `fn` pointer. What survives is the half that was always the point — the claim,
and teaching it.

**A claim spread over six files is not checkable.** Eight laws each true about its own construct is not the same
artifact as "the macro layer is one thing, and here is how you would falsify that". The evaluator claim especially: it
is asserted in a module doc, and nothing fails if a second evaluation path is added tomorrow. One law file gathers the
eight, each stated as the property rather than the mechanism, each with the negative control that makes it falsifiable
— and the evaluator claim gets a *structural* control, over the source text, in the shape `boundary_laws.rs` already
uses in `musa-calculus`: expansion reaches the core through `document::elaborate` and `musa_calculus::normalize`, and
no other evaluation entry point exists for it to reach.

**Hygiene, stated as a property rather than a mechanism.** A name introduced inside a quotation is distinct from any
name at the splice site, and a name captured from the splice site resolves there. The laws exist; what the gathering
adds is that they are read as one claim with the others.

**Totality is what makes this safe.** A macro is checked like any other function, so it terminates, so expansion
terminates. There is no `%macro` escape hatch and no partiality, which is why musa can afford to run macros with the
ordinary evaluator instead of a sandboxed one.

**Provenance survives expansion.** Every node a macro builds carries an `Original` that points at what the author wrote,
and `SourceInfo` has no eliminator, so an adapter can neither read provenance nor fabricate it. This is the property the
Origin view depends on.

## Target

- `crates/musa-compiler/tests/suite/macro_closure_laws.rs`, declared in `tests/suite/main.rs`: the eight claims of note
  56 §2 as one table, each with its negative control, plus the source-text law that expansion has one evaluation entry
  point. Where a claim is already lawed elsewhere, the row cites that law by name and adds the control it is missing
  rather than copying it.
- `docs/book/src/concepts/macros.md` and its `SUMMARY.md` entry: what an adapter is, `expand`'s signature, the quote and
  splice forms, hygiene and what it costs an author (`forget`, and why a name may not end in `_g0`), what provenance
  buys, and the Idris2 comparison as the closing argument. Written against `stdlib/src/adapters/doubled.musa`, which is
  small enough to show whole.
- `docs/plan/code-map/spec-to-implementation-map.md`, "Quotation in the core": status **implemented** rather than
  "implemented, unreached", and the **Owes** paragraph replaced — 142 and 159 discharged it.
- `crates/musa-compiler/src/lower/quotes/laws.rs` and `crates/musa-compiler/src/registry.rs`: the three sentences note
  56 §5 lists, each of which describes the compiler before prompt 142's cutover.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

`--run-ignored all` carries the 30 staff-budget failures prompt 166 owns; the count must not move.

Commit as `Make macros ordinary total functions`.

## Stop

- No elaborator reflection, no `Elab` monad, no Reify/Reflect, no quoting of core terms.
- No macro that can fail to terminate, and no annotation that would let one.
- **No `Syntax` family and no category refinement.** Note 55 measured both and note 55 §5 names what would re-open
  them; neither is this prompt.
- No new quotation vocabulary, in `stdlib/` or anywhere. The eighteen rows and two traversals are what there is.
