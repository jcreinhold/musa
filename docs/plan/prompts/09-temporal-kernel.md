---
id: 09
slug: event-track
status: done
depends_on: [08]
phase: 1
---

# Temporal Kernel Implementation

## Task

Implement `musa-events`: the finite temporal algebra specified in `docs/rules/events/` — exact rational time, typed
occurrences, sequence, overlay, ambient extension, restriction, payload mapping, time scaling, normalization, and
semantic equality — as a deep module with a narrow facade. Independent of the surface language: no source-syntax
changes, no compiler changes (elaboration is prompt 11).

## Read

- `docs/rules/events/03-denotational-semantics.md` and `05-normalization.md` (the normative definitions; prompt 08).
- `docs/rules/events/03-denotational-semantics.md` and `04-algebraic-laws.md` (the algebra),
  `docs/plan/code-map/stage-pipeline.md` (the event track's public interface is approximately
  `construct/check timeline`, `sequence`, `overlay`, `restrict`, `normalize`, `compare`, `map payloads`, `scale time` —
  internals hidden), §29 ("do not split each concept into its own microcrate").
- `musa-compiler/src/time.rs` for the existing rational-time conventions (the event track uses the same `num-rational`
  exactness discipline but is its own crate — payloads, not the event track, carry musical meaning; §12).

## Design

- New workspace crate `crates/musa-events`, depending only on `num-rational` (+ `serde` if the spec's serialization
  needs it). It must not depend on `musa-syntax` or `musa-compiler`: `Timeline<A>` is generic over its payload and the
  event track never learns what a `Note` is (§12).
- Public surface (doc-commented with invariants before implementation, per conventions):

  ```rust
  pub struct Beat(Ratio<i64>);            // exact musical time position
  pub struct Span { start: Beat, end: Beat }   // half-open [start, end), start <= end
  pub struct Occurrence<A> { span: Span, payload: A }
  pub struct Timeline<A> { /* private: extent + occurrence multiset */ }

  pub fn timeline<A>(extent: Beat, occurrences: Vec<Occurrence<A>>)
      -> Result<Timeline<A>, EventsError>;   // checks 0 <= s <= e <= extent
  pub fn sequence<A>(parts: Vec<Timeline<A>>) -> Timeline<A>;
  pub fn overlay<A>(parts: Vec<Timeline<A>>) -> Timeline<A>;
  pub fn extend<A>(t: &Timeline<A>, new_extent: Beat) -> Result<Timeline<A>, EventsError>;
  pub fn restrict<A>(t: &Timeline<A>, window: Span) -> RestrictedView<'_, A>;
  pub fn map_payload<A, B>(t: &Timeline<A>, f: impl Fn(&A) -> B) -> Timeline<B>;
  pub fn scale<A>(t: &Timeline<A>, factor: Ratio<i64>) -> Timeline<A>;   // factor > 0
  pub fn normalize<A: Canonical>(t: &Timeline<A>) -> Timeline<A>;
  ```

  Composition expressions with named references (the `sequence`/`overlay` tree of §24) are an HIR concern; the event track
  stores only flat timelines — construction IS normalization, so `normalize` is a re-canonicalization of occurrence
  order. If implementation evidence shows a real need to retain un-normalized trees, repair the spec first (prompt 08's
  `05-normalization.md`), then implement.
- Occurrences form a multiset: equal occurrences never collapse (§6). Canonical order: start, end, then payload ordering
  — define a `Canonical` trait (`fn canonical_key(&self) -> String` or `Ord` bound; pick one, document) so semantic
  equality and hashing are well-defined for arbitrary payloads.
- `restrict` returns the observation representation of `docs/rules/events/03-denotational-semantics.md`: each observed
  occurrence reports `whole_span` and `visible_span` — cropping never rewrites where an occurrence began.
- Semantic equality: `Timeline::semantic_eq` comparing canonical forms (extent + sorted occurrence multiset), not
  pointer/insertion identity.
- Canonical serialization to the `05-normalization.md` text form (`Display`), deterministic, for golden tests and
  semantic hashes. No parser (deferred, `08-open-questions.md`).
- `EventsError` names violations: occurrence outside extent, non-positive scale factor, shrinking `extend`.

## Target

- `crates/musa-events/`: the types and functions above, all laws documented on the functions that satisfy them.
- Workspace wiring: `Cargo.toml` members; `musa-events` has no dependents yet (prompt 11 adds the first).
- Tests: unit tests for construction checks, sequence/overlay/restrict/extend/scale semantics, canonical ordering,
  multiset preservation, and serialization determinism. The law suite is prompt 10.

## Check

```sh
cargo nextest run -p musa-events
cargo clippy --all-targets -p musa-events -- -D warnings
cargo fmt --check
```

Commit as `Add musa-events temporal algebra`.

## Stop

- No surface-language changes, no compiler integration, no `ScoreSnapshot` changes.
- No `Pattern`, infinite timelines, aleatory choice, or monadic `join` (§16, §18, §32).
- No events-file parser; serialization only.
- No musical payload types in this crate — not even `Note` (§12: payloads are supplied by consumers).
