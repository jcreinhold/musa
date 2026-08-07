---
id: 09
slug: temporal-kernel
status: done
depends_on: [08]
phase: 1
---

# Temporal Kernel Implementation

## Task

Implement `musa-kernel`: the finite temporal algebra specified in `docs/kernel/` — exact rational time, typed
occurrences, sequence, overlay, ambient extension, restriction, payload mapping, time scaling, normalization, and
semantic equality — as a deep module with a narrow facade. Independent of the surface language: no source-syntax
changes, no compiler changes (elaboration is prompt 11).

## Read

- `docs/kernel/03-denotational-semantics.md` and `05-normalization.md` (the normative definitions; prompt 08).
- Course correction §§3–17 (the algebra), §26 (compiler architecture: the kernel's public interface is approximately
  `construct/check timeline`, `sequence`, `overlay`, `restrict`, `normalize`, `compare`, `map payloads`, `scale time` —
  internals hidden), §29 ("do not split each concept into its own microcrate").
- `musa-compiler/src/time.rs` for the existing rational-time conventions (the kernel uses the same `num-rational`
  exactness discipline but is its own crate — payloads, not the kernel, carry musical meaning; §12).

## Design

- New workspace crate `crates/musa-kernel`, depending only on `num-rational` (+ `serde` if the spec's serialization
  needs it). It must not depend on `musa-language` or `musa-compiler`: `Timeline<A>` is generic over its payload and the
  kernel never learns what a `Note` is (§12).
- Public surface (doc-commented with invariants before implementation, per conventions):

  ```rust
  pub struct Beat(Ratio<i64>);            // exact musical time position
  pub struct Span { start: Beat, end: Beat }   // half-open [start, end), start <= end
  pub struct Occurrence<A> { span: Span, payload: A }
  pub struct Timeline<A> { /* private: extent + occurrence multiset */ }

  pub fn timeline<A>(extent: Beat, occurrences: Vec<Occurrence<A>>)
      -> Result<Timeline<A>, KernelError>;   // checks 0 <= s <= e <= extent
  pub fn sequence<A>(parts: Vec<Timeline<A>>) -> Timeline<A>;
  pub fn overlay<A>(parts: Vec<Timeline<A>>) -> Timeline<A>;
  pub fn extend<A>(t: &Timeline<A>, new_extent: Beat) -> Result<Timeline<A>, KernelError>;
  pub fn restrict<A>(t: &Timeline<A>, window: Span) -> RestrictedView<'_, A>;
  pub fn map_payload<A, B>(t: &Timeline<A>, f: impl Fn(&A) -> B) -> Timeline<B>;
  pub fn scale<A>(t: &Timeline<A>, factor: Ratio<i64>) -> Timeline<A>;   // factor > 0
  pub fn normalize<A: Canonical>(t: &Timeline<A>) -> Timeline<A>;
  ```

  Composition expressions with named references (the `sequence`/`overlay` tree of §24) are an HIR concern; the kernel
  stores only flat timelines — construction IS normalization, so `normalize` is a re-canonicalization of occurrence
  order. If implementation evidence shows a real need to retain un-normalized trees, repair the spec first (prompt 08's
  `05-normalization.md`), then implement.
- Occurrences form a multiset: equal occurrences never collapse (§6). Canonical order: start, end, then payload ordering
  — define a `Canonical` trait (`fn canonical_key(&self) -> String` or `Ord` bound; pick one, document) so semantic
  equality and hashing are well-defined for arbitrary payloads.
- `restrict` returns the observation representation of course correction §17: each observed occurrence reports
  `whole_span` and `visible_span` — cropping never rewrites where an occurrence began.
- Semantic equality: `Timeline::semantic_eq` comparing canonical forms (extent + sorted occurrence multiset), not
  pointer/insertion identity.
- Canonical serialization to the `05-normalization.md` text form (`Display`), deterministic, for golden tests and
  semantic hashes. No parser (deferred, `08-open-questions.md`).
- `KernelError` names violations: occurrence outside extent, non-positive scale factor, shrinking `extend`.

## Target

- `crates/musa-kernel/`: the types and functions above, all laws documented on the functions that satisfy them.
- Workspace wiring: `Cargo.toml` members; `musa-kernel` has no dependents yet (prompt 11 adds the first).
- Tests: unit tests for construction checks, sequence/overlay/restrict/extend/scale semantics, canonical ordering,
  multiset preservation, and serialization determinism. The law suite is prompt 10.

## Check

```sh
cargo nextest run -p musa-kernel
cargo clippy --all-targets -p musa-kernel -- -D warnings
cargo fmt --check
```

Commit as `Add musa-kernel temporal algebra`.

## Stop

- No surface-language changes, no compiler integration, no `ScoreSnapshot` changes.
- No `Pattern`, infinite timelines, aleatory choice, or monadic `join` (§16, §18, §32).
- No kernel-file parser; serialization only.
- No musical payload types in this crate — not even `Note` (§12: payloads are supplied by consumers).
