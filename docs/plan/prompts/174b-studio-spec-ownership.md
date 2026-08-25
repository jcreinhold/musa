---
id: 174b
slug: studio-spec-ownership
status: done
depends_on: [167, 174, 174a]
phase: 3
---

# Bridge Checked Source Values Without Inventing a Host Language

> **Reopened by note 79 and reordered by note 80.** The first execution moved the legacy Rust `StudioSpec` from
> `musa-compiler` to `musa-dsp`. Note 79 established that ordinary source owns declarable sound semantics. Preparation
> then exposed a second fact: prompt 167's source graph is an intentionally small adapter trial and cannot yet represent
> the production path's patches, buses, sends, modulation, or full processor set. Deleting that path here would either
> reject accepted source or smuggle prompts 175–180 into this one. This prompt builds the checked-value bridge; prompt
> 180a performs the clean cutover after those declarations exist.

## Task

Give a successfully checked, evaluated, closed source value a narrow host-facing artifact that can cross from the
compiler to a sibling consumer without exposing the evaluator's `Value` or re-parsing printed text. Exercise it on
`std::sound::graph::StudioDescription`: decode the complete exact trial value into a read-only projection and prove the
projection agrees with the canonical source datum. Do not claim the legacy production `StudioSpec` has been replaced.

## Read

- `docs/rules/language/00-semantics.md`'s ownership test and `08-performance-and-sound.md` §0; notes
  [`79`](../../notes/research/language-design-closure/79-source-owns-the-sound-language.md) and
  [`80`](../../notes/research/language-design-closure/80-bridge-before-cutover.md).
- Prompt 167 and note 66, especially the measured source `StudioDescription`, its deliberately small vocabulary, and the
  explicitly missing bridge; `stdlib/src/{sound/graph,adapters/graph}.musa` in full.
- `musa-calculus::Datum`, `canonical`, `Literal`, `Payload`, and `Document::{term,value}`; especially
  `base_laws::what_is_not_canonical_data_reads_back_as_nothing`. Since prompt 157 a record literal is a saturated
  one-constructor application and therefore already reads as `Datum::Case`; “no record arm” means records do not get a
  duplicate representation.
- The legacy `StudioSpec` production callers, only to inventory what this prompt cannot yet replace and to pin a
  differential oracle for prompt 180a.
- Prompt 164 and note 61 for the builtin-ownership precedent.
- Peyton Jones chapter 3 and Ousterhout chapters 7–8: semantics-preserving source-to-substrate translation and the cost
  of adjacent layers exposing the same abstraction.

## Design

The bridge is generic canonical source data, not a public compiler HIR and not a studio AST. It is constructible only as
the result of checking and normalizing a closed storable source value in its declaration context. It carries the
qualified root type, source package/schema version, and a complete deterministic exact datum encoding. It contains no
closures, evaluator environment, source syntax, `Value`, pointer identity, display text, hash-only identity, or float.

Reuse the calculus's existing canonical-data vocabulary directly: records, enums, and lists all arrive as qualified
constructor cases, while their declaration says which fields those cases carry. Add only the smallest opaque checked
wrapper and exact literal-encoding hook that this caller needs; do not add a record or sound case to `Datum`, and do not
change the δ-rule firing domain. Laws must show existing canonical readback and δ reduction are byte- and
step-identical. Constructor names and variable children are framed, and a schema/version change changes the artifact
identity. A digest may index an artifact but exact bytes decide equality.

The `StudioDescription` projection is a structural decoder over that artifact. Its fields are private, its readers are
read-only, and there is no public field constructor, default, alias, validation table, or Rust-side inference. It
decodes every declaration, anchor, port kind, path, parameter, exact value, and list position. Source `validate` owns
graph validity; projection decoding owns only schema agreement and malformed-artifact refusal.

The existing Rust `StudioSpec` remains a temporary compatibility path in this prompt because it represents more than the
source trial. Mark it and its construction APIs internal/deprecated where that does not break repository callers,
inventory every remaining semantic difference, and add no new use. Prompt 175 adds the source vocabulary, 176 exact
quantities, 177–180 the performance/instrument/routing/control semantics, and 180a deletes the path and the
`musa-compiler -> musa-dsp` dependency.

## Target

- A generic opaque checked canonical-data artifact, produced by the existing checker/evaluator and usable without
  exposing calculus `Value` or re-parsing display text.
- A complete read-only projection of the prompt-167 `StudioDescription` trial, including schema/version and exact
  source-datum equivalence laws.
- Negative laws for wrong root type/version, malformed framing, incomplete fields, reordered declarations, and an
  unchecked or noncanonical value.
- An explicit production-gap inventory covering every legacy patch/bus/send/modulation/processor construct that blocks
  cutover, linked from prompt 180a.
- No new authoritative Rust declaration vocabulary and no new production dependency on `StudioSpec`.

## Check

```sh
cargo nextest run -p musa-calculus -p musa-compiler -p musa-dsp
cargo clippy --all-targets -p musa-calculus -p musa-compiler -p musa-dsp -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
cargo insta test --workspace --unreferenced=reject
rg -n "CheckedSource|Canonical.*Artifact|StudioDescription" crates/musa-calculus crates/musa-compiler crates/musa-dsp
```

Commit as `Bridge checked source values to host consumers`.

## Stop

- No production studio cutover or deletion of accepted patch/bus/send/modulation behavior; prompt 180a.
- No new processor, instrument, control, routing feature, or syntax spelling; prompts 175–180.
- No source evaluator, type checker, or resolver in `musa-dsp`; it receives checked canonical data.
- No public calculus `Value`, evaluator environment, editable projection, display-text parser, or sound-specific
  calculus case.
- No float conversion, scheduling, or machine instantiation; prompts 176 and 178 own those boundaries.
