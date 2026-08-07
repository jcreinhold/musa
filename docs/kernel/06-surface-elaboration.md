# 06 — Surface Elaboration

How the existing `.musa` surface language elaborates into the temporal kernel (course correction §30 Step 4, §19–21).
This document describes elaboration of the grammar **as it exists today** (prompts 02–06); it is not a surface redesign
(§31). The implementation is prompt 11 (`docs/prompts/11-kernel-elaboration.md`).

## The elaboration boundary

```text
source / musical HIR  (motifs, repeats, transforms, chords, provenance — preserved)
        ↓  elaborate / observe
finite temporal kernel  (flat timelines; the semantic quotient)
        ↓  adapt
ScoreSnapshot  (the score-specific projection backends already consume)
```

Normalization is a **semantic boundary, not the internal representation of every compiler pass** (§19). The HIR keeps
`repeat`, `loop`, motif references, and transformations for efficiency, editing, provenance, diagnostics, and structural
display; elaboration evaluates finite observations of them into kernel timelines. Nothing requires duplicating thousands
of nodes merely to obey the normalized model.

## Payload design (the central decision)

Score elaboration uses one payload type per fact domain. For the current grammar there is one essential payload:

```text
NotePayload {
    pitch: WrittenPitch        % written spelling, verbatim — never a MIDI number (roadmap §2)
    voice: VoiceIdentity       % (part, voice) source identity
    origin: Origin             % provenance: source span, declaration, expansion path
}
```

Decisions recorded against course correction §32:

- **Voice identity rides in payload metadata** — it is not a temporal primitive. The candidate answer to the §32 open
  question is "payload metadata + HIR structure"; prompt 11 gathers the evidence.
- **Provenance rides in the payload** (§20): the kernel quotient forgets production history, so history travels with
  each occurrence as data the kernel is opaque to. `Origin` never participates in temporal semantics; it participates in
  canonical payload serialization only as a stable, deterministic key (N3), so semantic equality can still distinguish
  occurrences a consumer must tell apart.
- **No duration field in the payload** — duration is temporal support (D0).

## Elaboration rules, per surface construct

| Surface construct | Elaboration |
| --- | --- |
| `note c5 1/4;` | One occurrence of `ScoreFact::Note` over the current position's span, carrying pitch, written duration, and any articulations; the voice cursor advances by `1/4`. |
| `rest 1/2;` | One occurrence of `ScoreFact::Rest` over the current position's span; the voice cursor advances by `1/2`. A *written* rest is notation an author asked for, and export and provenance both need it; what stays absent is unwritten silence (§2). |
| `chord (c5 e5 g5) 1/2;` | `overlay` of one `Note` occurrence per pitch over the same span (§30 Step 4). The projection regroups same-span, same-scope, same-origin note occurrences into `ScoreEventKind::Chord`. |
| voice body | `sequence` of its items in source order (cursor semantics = left-fold of successive extents). |
| part | `overlay` of its voice timelines. Voice identity is the fact's `Scope`, not a timeline of its own. |
| piece score | `overlay` of its part timelines: **one** `Timeline<ScoreFact>` per compilation. |
| `use motif(args);` | Binding + reference at the HIR level: the motif body elaborates with bound arguments; each resulting occurrence's `Origin` gains the `MotifApplication` step. Not a kernel concept. |
| `repeat n { … }` | HIR-level `sequence` of `n` evaluations (§19); each iteration's occurrences gain the `RepeatIteration(i)` provenance step. |
| `transpose up P5 { … }` | `map_payload` with the transposition function on `pitch` (§13); occurrences gain the `Transposition` provenance step. |
| `c4 1/4 ~;` (tie) | **No kernel construct, and no fact.** A tie says two written noteheads spell *one* occurrence, so elaboration merges the tied statement with its continuation on the spot: one occurrence, span the sum, `NotatedDuration` the compound spelling. Merging happens at every nesting level, so a tie inside a `retrograde` is gone before the block is reversed and needs no repair. A tie onto a different pitch, or with nothing after it, is a diagnostic. |
| `c4 1/4 accent staccato;` | Articulations are a **field of the note fact**, not facts of their own: a staccato dot has no extent and no identity apart from its note. The projection emits one `ArticulationMarking` per name, in written order, against the event's id. |
| `dynamic mf;` | A **point** occurrence of `ScoreFact::Dynamic` at the cursor, with no cursor advance. The projection resolves it to the first event at or after it in the same voice; nothing after it is a diagnostic. |
| `slur { … }` | The body elaborates unchanged, and one `ScoreFact::Slur` occurrence is **overlaid** over `[0, extent)` of it. Nothing is copied onto the notes. The projection emits a `SlurSpan` naming the first event at or after the region's start and the last ending at or before its end. `phrase` and `crescendo`/`diminuendo` work identically. |
| `tuplet n/d { … }` | The body elaborates with the voice's duration scale multiplied by `d/n`, so written values become exact rationals (`3/2` of eighths gives `1/12`). A `ScoreFact::Tuplet` occurrence is overlaid over the body, and the projection emits a `TupletSpan` carrying the unreduced `n/d`, which is what the backends need to print the bracket. A tuplet that would cross a barline is a diagnostic. |
| `performance { profile v { … } }` | **Nothing elaborates.** A profile is a reading of marks, not material: it produces no occurrence, occupies no time, and is carried on the snapshot beside the motif table for the performance layer to consult. Written marks stay written (§6.4). |
| `profile v;` inside a part | **A binding, not an occurrence.** It names which profile realizes this part; naming an undeclared one is a diagnostic. Both semantic paths read it through the same `part_metadata`, so it cannot drift between them. |
| `key c major;`, `meter 4/4;` | **Region occurrences** of `ScoreFact::Key`/`Meter`, scoped to the piece and spanning `[0, d]`. An unwritten meter still produces a fact — 4/4 governs a piece that never says so — while an unwritten key produces none, which is why the projection's `key` is an `Option` and its `meter` is not. |
| `section "A" at 9:1;`, `harmony { at 1:1 c; }` | **Point occurrences** of `ScoreFact::Section`/`Harmony` at the time the coordinate names. The coordinate is resolved against the *meter occurrence* and the timeline's own extent; naming a place the piece never reaches is a diagnostic. |
| `tempo 1/4 = 60;`, `tempo 1/4 = 90 at 9:1;` | **Not a fact, ever** (§22). Tempo is the performance layer's `Beat → Second` map; it stays on the snapshot's `TempoMap`. See "Tempo stays out" below. |
| piece | The one timeline **projected** into `ScoreSnapshot` (`musa-compiler/src/project.rs`): voices, context maps, and annotations alike. |

## The payload, and the adapter contract

A `ScoreFact` has exactly three axes, and they vary independently:

- **`scope`** — where in the score's *structure* the fact sits: one voice of one part, or the piece as a whole. Not
  where it is in time.
- **`kind`** — what is stated: a note, a rest, a slur, a phrase, a tuplet, a dynamic, a hairpin, a key, a meter, a
  section, a chord symbol.
- **`origin`** — why it exists (§20). Provenance stays above the kernel.

*Where it is in time is the occurrence's span*, and is never a field. Keeping the three apart is the point: a slur moves
in time without changing voice, a voice is renamed without moving anything, a dynamic changes from `mf` to `f` in place.

The projection (`project.rs`) visits the canonically ordered occurrences **once**, buckets them by scope, assigns
`EventId`s to note and rest facts in visit order, and resolves region facts to the event ids at their ends. It relies on
one invariant, stated on the function and asserted in debug builds: **a region fact's boundaries coincide with event
boundaries in its own scope**, because a region is built from the extent of the items it encloses. If that is ever
violated, the elaboration that violated it is the bug.

## Key, meter, harmony: the present shape

Course correction §21 places key/meter/harmony **regions** in the kernel as typed interval payloads whenever their
temporal extent matters — e.g. `modulate to C major { … }`. Prompt 40 put them there **before** the surface grew such a
construct, and that order was deliberate: a region that happens to cover the whole piece is not a special case, but a
piece-wide scalar called `MeterMap` is. Modelling meter as one region over `[0, d]` now means the later change adds
*more occurrences* rather than a second way to ask the same question (PoSD ch. 10).

So today:

- `Key { tonic, mode }` and `Meter { numerator, denominator }` are occurrences over `[0, d]`, scoped `Scope::Piece`.
- `Section { name }` and `Harmony { symbol }` are point occurrences at the time their `measure:beat` coordinate names.
- `KeyMap`, `MeterMap`, and the section and harmony lanes of `AnnotationStore` are **projections** of those
  occurrences, reproducing byte for byte what the direct lowerer emits — which is what `fixtures_have_full_parity`
  checks.

The promise of §21 — that these arrive "without any kernel change" — is therefore demonstrated rather than asserted:
prompt 40 touched no file in `musa-kernel`. When `modulate` or a mid-piece `meter` arrives, the elaboration emits a
region with a narrower span and nothing else changes; this document is extended, not repaired, at that prompt.

Two consequences worth stating, because a later reader will otherwise re-derive them:

- **Positions resolve against the meter *occurrence*, and against the timeline's own extent.** Neither is recomputed
  from the snapshot. The extent is exact rather than a maximum over event ends, and the two agree only because a
  written rest is an occurrence (prompt 39) — a piece that ends in silence ends where the silence ends, which
  `a_piece_that_ends_in_a_rest_ends_where_the_rest_ends` fixes as a fixture.
- **Piece-scoped facts sort first at a shared instant.** Their canonical key begins `*|*`, and `*` sorts before any
  part number, so the normal form prints the context a reader meets first.

## Tempo stays out (§22)

Tempo never elaborates into kernel occurrences and never rescales kernel time. It is the performance layer's monotone
map `Beat → Second` applied to symbolic positions at realization time (`07-backend-contract.md`). "Stretch the material"
(payload/time action, D5) and "perform the same material more slowly" (tempo map) remain different operations.

## Adapter contract (ScoreSnapshot)

The adapter projects normalized `Timeline[NotePayload]` values into the existing `ScoreSnapshot`:

- occurrences → `ScoreEvent`s (id assignment deterministic: canonical order per part, voice-major);
- same-span/same-voice/same-origin groups → `Chord`; singletons → `Note`;
- payload `voice` → part/voice lanes; payload `origin` → event `Origin` verbatim;
- extent per voice → `Voice::span`; part extent = max over voices;
- context maps and annotations → as the old lowerer emits them.

**Parity requirement (§30 Step 5):** for every fixture, the adapter's snapshot must equal the old lowerer's snapshot on
positions, durations, spelling, part/voice identity, multiplicity, ordering, and provenance. The old lowerer is the
regression oracle until prompt 12, and remains runnable permanently.

The phase-2 constructs above (tie, articulation, dynamic, slur, tuplet) are **kernel-path only**. The old lowerer is
frozen, so instead of re-implementing them it rejects them with `` `<construct>` needs the kernel elaboration path ``;
parity therefore still holds on everything the oracle accepts. `phase_two_constructs_are_kernel_only` in
`crates/musa-compiler/tests/elaboration.rs` pins that boundary so neither path can quietly drift across it.

## What elaboration must never do

- Introduce rest/silence occurrences to "fill" regions the author left empty (§2) — a `rest` the author wrote is
  material and elaborates to an occurrence; a gap is not.
- Push production history into kernel semantics (e.g. making equality motif-aware) (§20).
- Add kernel constructs because one surface feature is awkward — awkwardness is elaboration's problem (§34).
- Change the surface grammar to make elaboration easier (§35.9).
