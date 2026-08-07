---
id: 40
slug: context-facts
status: pending
depends_on: [39]
phase: 3
---

# Key, Meter, and Score-Level Annotations as Occurrences

## Task

Finish what prompt 39 started: move the last temporal facts that live outside the timeline into it. Key and meter become
region occurrences spanning the piece; sections and chord symbols become point occurrences at the time they mark.
`KeyMap`, `MeterMap`, and the section/harmony arms of `AnnotationStore` become projections. After this prompt there is
exactly one temporal representation in the compiler, and course correction §21's promise — that key/meter/harmony
regions arrive "without any kernel change" — is demonstrated rather than asserted.

Tempo does **not** move. That is not an omission; see below.

## Read

- Course correction §21 (key/meter/harmony as typed interval payloads once extent matters), §22 (tempo is the
  performance layer's `Beat → Second` map and never kernel material).
- `docs/kernel/08-open-questions.md` **Q8** — this prompt is its resolution: the working stance was "context maps until
  the surface gives them extent"; this prompt gives them kernel extent *without* waiting for the surface, and records
  why that was the right order.
- `docs/kernel/07-backend-contract.md` (what consumers may assume — unchanged by this prompt, and that is the point).
- `crates/musa-compiler/src/score.rs`: `KeyMap`, `MeterMap`, `TempoMap`, `SectionMark`, `HarmonyMark`.
- `crates/musa-compiler/src/elaborate.rs`: `elaborate_annotations`, `piece_extent`, `resolve_position` — the last
  functions that compute temporal facts outside the timeline.
- `crates/musa-project/src/facts.rs` (`OutlineFacts` reads sections and phrases), `crates/musa-render/src/plan.rs`
  (measures are computed from `MeterMap`).

## Design

### Why key and meter become regions now, when the surface has no `modulate`

Because a region that happens to cover the whole piece is not a special case, and a piece-wide scalar is. Today
`MeterMap` is a single numerator and denominator with "Map" in its name; when prompt 36-era syntax gives meter extent,
either the type grows a second representation or every consumer learns two ways to ask the same question. Modelling it
as one region over `[0, d]` now means the later change adds *more occurrences*, not a new kind of thing — PoSD ch. 10,
the special case defined out of existence before it is written.

New `FactKind` variants, all score-scoped (`Scope::Piece`):

| Variant | Span | Replaces |
| --- | --- | --- |
| `Key { tonic, mode }` | `[0, d]` today; the region it governs later | `ScoreSnapshot::key_map` |
| `Meter { numerator, denominator }` | `[0, d]` today | `ScoreSnapshot::meter_map` |
| `Section { name }` | a point | `AnnotationStore::sections` |
| `Harmony { symbol }` | a point today; a region when a chord's duration is written | `AnnotationStore::harmony` |

The projection reproduces `KeyMap`/`MeterMap`/`SectionMark`/`HarmonyMark` exactly as they are emitted today, including
`None` for an absent key.

### Tempo stays out, deliberately and visibly

§22 is unambiguous: tempo is a map from symbolic time to physical time, applied at realization; it never rescales the
timeline. `stretch` changes the music, a tempo change changes the performance of it, and the kernel must not offer a
place where those two could be confused. Prompt 36's piecewise `TempoMap` therefore stays exactly where it is, in the
snapshot and the performance layer.

Write this as a comment on `TempoMap` naming §22, because the next reader of this block will otherwise ask why tempo was
the one thing left behind and answer the question wrong.

### Positions resolve against a fact, not a side table

`resolve_position` currently reads `snapshot.meter_map` to turn `measure:beat` into time, and `piece_extent` scans every
event to find where the piece ends. Both now read the timeline: the meter occurrence, and the timeline's own extent —
which is what an ambient extent *is*, and is exact rather than a maximum over event ends. Watch for one behaviour
change: a piece whose last voice ends with a rest has an extent today equal to the last event's end, and that stays
true, because a rest is an occurrence (prompt 39). Confirm with a fixture rather than reasoning.

The "the piece ends before `9:1`" diagnostic must keep firing on the same inputs; it is tested by prompt 35's suite.

### Ordering inside the canonical form

Score-level facts now sort into the same canonical order as notes. `Canonical::canonical_key` must place them
deterministically — key and meter before the notes at the same instant, since that is how a reader encounters them —
and the canonical serialization (`kernel_normal_form`) gains them. Existing kernel-normal-form goldens **will** change;
re-record them in this commit and say so, since every other golden in the repo must not.

## Target

- `crates/musa-compiler/src/elaborate.rs`: the four new `FactKind` variants; `elaborate_annotations` produces
  occurrences; `piece_extent` deleted in favour of the timeline extent; `resolve_position` reads the meter fact.
- `crates/musa-compiler/src/project.rs`: context maps and score-level annotations projected.
- `crates/musa-compiler/tests/snapshots/`: kernel-normal-form goldens re-recorded (only these).
- `docs/kernel/06-surface-elaboration.md`: the `key`/`meter`/`tempo` row rewritten; the "future shape" section becomes
  the present shape, with tempo's exclusion stated.
- `docs/kernel/08-open-questions.md`: **Q8 resolved** — state the resolution and delete the open question.
- `docs/kernel/09-performance.md`: this prompt's row.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test -p musa-render --unreferenced=reject     # backend goldens unchanged
for f in examples/*.musa; do cargo run -p musa-cli -- check "$f"; done
cargo run -p musa-cli -- render examples/annotated.musa --to mei -o /tmp/a.mei
cargo bench -p musa-compiler
grep -n "piece_extent" crates/musa-compiler/src/*.rs | wc -l   # 0
```

Commit as `Elaborate key, meter, and score annotations as occurrences`.

## Stop

- Tempo does not become a fact. Not as a point, not as a region, not "for symmetry".
- No new surface syntax — no `modulate`, no mid-piece `meter`. This prompt changes representation only; the grammar
  freeze (§35.1) holds.
- No change to `NotationPlan`, MEI, LilyPond, MusicXML, or the desktop.
- Do not resolve Q4 (continuous curves) in passing; it needs the surface design prompt 36 owns.
