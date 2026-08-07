---
id: 42
slug: snapshot-projection
status: done
depends_on: [41]
phase: 3
---

# `ScoreSnapshot` Becomes an Interface

## Task

`ScoreSnapshot` and its parts are public structs with public fields: `parts`, `tempo_map`, `meter_map`, `key_map`,
`annotations`, `motifs`, `profiles`, and inside them `Part::voices`, `Voice::events`, `ScoreEvent::onset`, and the rest.
Since prompt 40 those fields are no longer where the data lives — they are a projection of the timeline, materialized
eagerly because the field type demands it. Close the type: private fields, accessors that say what a caller wants rather
than how it is stored, and a projection free to become lazy when prompt 48 measures that it should.

This is the deep-module change that makes prompts 39–40 durable. Without it, every consumer is still coupled to a layout
decision, and the next representation change is another workspace-wide edit.

## Read

- PoSD ch. 4 (deep modules), ch. 5 (information hiding), ch. 7 (different layer, different abstraction — the snapshot is
  the *score* layer's abstraction over the timeline, and it should speak score, not storage). The red flags this prompt
  clears: *public fields expose layout*, *public interface mirrors storage*, *information leakage*.
- Course correction §27 (the snapshot is the score-specific interpretation of the normalized denotation),
  `docs/kernel/07-backend-contract.md` (what consumers may assume — this prompt makes those assumptions enforceable
  rather than conventional).
- Every consumer, before designing the accessors: `crates/musa-render/src/plan.rs`, `ly.rs`, `mei.rs`, `musicxml.rs`,
  `midi.rs`; `crates/musa-compiler/src/performance.rs`; `crates/musa-project/src/facts.rs`, `edit.rs`, `export.rs`,
  `midi.rs`, `snapshot.rs`. The accessor set is designed *from the calls that exist*, not from the fields.

## Design

### Design it twice, on the real call sites

PoSD ch. 11 and the module-design rule for a non-trivial boundary: write down two accessor sets before implementing
either.

- **Set A — field-for-field accessors.** `parts()`, `meter()`, `key()`, `annotations()`. Mechanical, small diff,
  and it preserves every existing awkwardness: `plan.rs` still loops `annotations.slurs()` and re-joins them to events
  by id, `facts.rs` still walks `parts → voices → events` to answer "what is at this time".
- **Set B — question-shaped accessors.** The operations consumers actually perform: iterate the events of a part in
  order; find the event at a time in a voice; ask what marks attach to an event; ask what regions cover an event; ask
  the piece's extent and its meter at a time.

Choose per operation, not wholesale, and record the choice. The bar: an accessor earns its place if two or more real
call sites become simpler, or if it removes a rebuild a caller is doing today. An accessor that only renames a field is
Set A and should stay Set A — do not invent a query nobody asks.

The strongest candidates for Set B, from the reading above, are the two rebuilds that recur across backends: *marks
attached to an event* (each of MEI, LilyPond, MusicXML re-derives this from `annotations` by id) and *regions covering
an event* (slur, phrase, tuplet, hairpin). Those are the projection's job — it has the timeline and the spans — and
pushing them down deletes the same loop from four backends (PoSD ch. 8, pull complexity downward; the red flag is
*repetition*).

### Keep the shapes public where they are values

`ScoreEvent`, `EventId`, `WrittenPitch`, `NotatedDuration`, `Clef`, `DynamicMark`, `ArticulationMark`, `ChordSymbol`
are values a consumer reads and matches on; they stay public with public fields where the field *is* the value. The
types that close are the containers whose layout is a decision: `ScoreSnapshot`, `Part`, `Voice`, `PartMap`,
`AnnotationStore`, `KeyMap`, `MeterMap`.

### `AnnotationStore` is the one to watch

It is currently eight parallel `Vec`s with eight accessors, filled by a projection that walks one ordered timeline. If
Set B absorbs its two real queries, it may not need to exist as a type at all. Decide deliberately: either it stays as
the "all annotations, by kind" view (with a stated reason — the outline pane and the export backends do want kind-major
access), or it dissolves into snapshot accessors. Do not leave it as a struct that exists because it used to.

### No behaviour change

Same values, same order, same ids. The diff is types and call sites. `cargo insta test --unreferenced=reject` across the
workspace is the proof.

## Target

- `crates/musa-compiler/src/score.rs`: fields private; the chosen accessor set, each with a doc comment stating its
  invariant; constructors `pub(crate)`.
- `crates/musa-compiler/src/project.rs`: projection updated; any Set B query implemented here, where the spans are.
- `crates/musa-render`, `crates/musa-project`, `apps/musa-desktop` (Rust side): call sites migrated.
- `docs/kernel/07-backend-contract.md`: a short section stating that the guarantees are now carried by the snapshot's
  interface, naming the accessors that carry each one.
- `docs/kernel/09-performance.md`: this prompt's row.

## Repairs made while implementing

**The Set B motivation above was wrong about where the duplication is.** The Design section claims *marks attached to
an event* is re-derived "from `annotations` by id" in each of MEI, LilyPond and MusicXML. It is not: those three
backends consume `NotationPlan`, and `NotationPlan` is built once, in `musa-render/src/plan.rs`. Exactly two places in
the workspace read `annotations` and rebuild anything from it, and they do the *same* rebuild two different ways:

- `plan.rs::Marks::collect` walks `from.0..=to.0` for phrases, hairpins and tuplets — raw id arithmetic that assumes
  contiguity without saying so, and that silently invents ids for events that do not exist if a region ever spans two
  voices;
- `performance.rs::hairpin_curves` scans a voice twice with `position()` per hairpin, per voice — quadratic in the
  hairpin count and re-run for every voice in the part.

So the Set B accessor that earned its place is neither of the two the prompt guessed. It is **one**:
`ScoreSnapshot::events_in(from, to) -> &[ScoreEvent]` — the events a region annotation covers. Both call sites above
became a single `for event in score.events_in(a, b)`, and `hairpin_curves` additionally collapsed from once-per-voice
to once-per-piece, since an event belongs to exactly one voice and the map is keyed by event id.

**Everything else is Set A, on purpose.** `parts()`, `meter()`, `key()`, `annotations()`, `motifs()`, `profiles()`,
`tempo()`, `Part::{id,name,clef,voices,voice,voice_name}`, `Voice::events`. Each renames a field and nothing more; by
the prompt's own bar ("two or more call sites become simpler") none of them earns a query, and inventing one would be
the speculative generality the Stop section forbids. Two near-Set-A accessors do slightly more than rename and are
recorded as such: `Voice::span` already existed as a method, and `Part::span` replaces
`part.voices.values().map(Voice::span).max().unwrap_or_default()` — one call site today, but a fold over a container
whose layout just became private, so the caller can no longer write it.

**`AnnotationStore` stays**, with the reason the Design section asked for: it is kind-major, and kind-major is what
its callers want — the outline pane lists sections, MEI writes dynamics as a lane, LilyPond emits articulations per
note. Only region membership was the duplicated question, and `events_in` answers that. It is not a struct that exists
because it used to.

**`KeyMap::new` is public, not `pub(crate)`.** `musa-project`'s MIDI entry buffer spells incoming notes against a key
it was handed and holds no score; naming a key is not a snapshot-building privilege. Every other constructor
(`ScoreSnapshot`'s mutators, `Part::new`, `Voice::new`, `MeterMap::new`) is `pub(crate)`: a snapshot is produced by
projecting a timeline and by nothing else.

**`TempoMap` did not close.** It is not in the Design section's list, and it is a value — a beat, a bpm, and a list of
changes — that `plan.rs` and `performance.rs` read field-wise. Closing it would have been a rename with no invariant
behind it.

**No snapshot changed and no allocation count moved.** 429 tests, every golden byte-identical, and P1–P4 allocation
counts identical to prompts 40 and 41 — see `docs/kernel/09-performance.md` for why this prompt's timings are not read
as a regression.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npm test
for f in examples/*.musa; do cargo run -p musa-cli -- check "$f"; done
cargo bench -p musa-compiler
```

Commit as `Close the score snapshot behind its interface`.

## Stop

- No representation change. The projection still materializes eagerly; laziness is prompt 48, and only if measured.
- No new information in the snapshot — no field gains a value it did not have.
- No accessor without a caller in this prompt. "A backend might want it" is prompt 48's problem, with evidence.
- No UI-facing (TypeScript) API change; the Tauri command boundary's JSON shape stays exactly as prompt 21 fixed it.
