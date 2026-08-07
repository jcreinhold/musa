# 07 — Backend Contract

What downstream consumers of the temporal kernel may assume, and what they must never do. Backends today consume
`ScoreSnapshot` and `NotationPlan` rather than kernel timelines directly; this contract applies to those projections as
well, because the projections preserve the kernel's guarantees (course correction §27–28).

## What consumers may assume

- **Normalized timelines only.** No consumer ever sees `sequence`, `overlay`, or references — only flat, canonically
  ordered occurrence multisets with exact rational spans (§25, `05-normalization.md`).
- **Exact positions.** Every span is a pair of exact rationals in beats. No float, no rounding, no sample-frame
  quantization has occurred at this layer.
- **Multiset multiplicity.** Identical occurrences are distinct facts. A consumer that merges equal occurrences is
  destroying musical information (two performers, one note) (§6).
- **Ambient extent.** A timeline's extent may exceed every occurrence's end; the tail is real temporal extent, not an
  error (§9).
- **Semantic equality.** Comparison, caching, and golden testing use the canonical form (N4–N6) and nothing else.
- **Opacity.** Payloads are typed and serializable, but their musical meaning belongs to their own theory modules (§12).
  A consumer interprets the payload types it understands and ignores nothing silently — unknown payload types are an
  explicit skip, never a misread.

## What consumers must decide themselves

- **Rest glyphs are a notation decision** (§2). An uncovered region of a notated voice is filled with rest symbols by
  the *notation* layer (`NotationPlan`), choosing glyph shapes per meter and convention. The kernel stores nothing
  there, and no backend may claim the kernel "has rests."
- **Notation spelling stays verbatim.** Written pitch spelling passes through from the payload; backends do not respell
  (roadmap §6.3).
- **Layout/engraving is downstream.** The plan and the kernel are semantic, not typographic (roadmap §12.1).

## What performance does (§22)

Performance realization supplies the monotone map `tempo : Beat → Second` and applies it to symbolic positions:

```text
(s, e, a)  ↦  (tempo(s), tempo(e), a)
```

- Changing tempo **never rewrites** the symbolic timeline. Ritardando/accelerando are tempo-map phenomena, not timeline
  edits.
- `stretch` (time scaling, D5) is the *other* operation — it changes the symbolic timeline. Conflating the two is the
  §22 bug this contract exists to prevent.
- The tempo map is piecewise-monotone; integration details live in the performance layer (prompt 15), not here.

## What audio does (§23)

Audio is a separate semantic layer: signals `Signal : PhysicalTime → Sample`. It meets the kernel at the
**realization/instrument boundary**:

```text
temporal kernel → tempo/performance → physical musical events → instruments → audio signals → DSP → output
```

No DSP concept enters the kernel; no kernel concept enters a sample buffer. The engine consumes scheduled physical
events, never timelines.

## The preserved boundaries (§27–28)

- `ScoreSnapshot` is the score-specific interpretation of the normalized denotation. Note-specific assumptions live
  there, not in the kernel; if a score concept cannot be expressed through the adapter, the *snapshot* grows, not the
  kernel (§27).
- The `ScoreSnapshot → NotationPlan → backend` pipeline is unchanged. Notation never learns about motif expansion,
  repetition semantics, source functions, loops, or transformations — it consumes already-resolved temporal facts (§28).
- Provenance flows: `EventId`/`Origin` in the snapshot are the same provenance carried in occurrence payloads, so the
  editor's source-mapping contract (MEI `xml:id`, click-to-source) survives the kernel unchanged (§20).

## The interface that carries the guarantees (prompt 42)

The assumptions above used to be conventions: `ScoreSnapshot` and its parts were public structs with public fields, and
a consumer could read the layout and rely on it. Since prompt 42 the containers are closed, and each guarantee is
carried by a named accessor rather than by a field a caller happens to find:

| Guarantee | The accessor that carries it |
| --- | --- |
| A part's voices, and which voice is which | `Part::voices`, `Part::voice`, `Part::voice_name` |
| A voice's events, in onset order, contiguous in id | `Voice::events` |
| How long a voice or a part is written to last | `Voice::span`, `Part::span` |
| The events a region annotation is *about* | `ScoreSnapshot::events_in` |
| The piece's key and meter, present whether or not written | `ScoreSnapshot::key`, `ScoreSnapshot::meter` |
| Markings by kind, in position order | `ScoreSnapshot::annotations` |

`events_in` is the one accessor that is a question rather than a field. A slur, phrase, tuplet or hairpin names only the
events at its ends; the events between them are what the mark is about, and every consumer needs them. The rule that
answers it — a region lies inside one voice, and a voice's events are contiguous in id — is a *projection* invariant, so
the projection states it once instead of `plan.rs` re-deriving it by id arithmetic and `performance.rs` re-deriving it
by a pair of linear scans, each with its own way of being wrong at a voice boundary.

`AnnotationStore` stays, deliberately. It is kind-major (`slurs()`, `dynamics()`, `sections()`, …), and kind-major is
what its callers want: the outline pane lists sections, MEI writes dynamics as a lane, LilyPond emits articulations per
note. Only region *membership* was the duplicated question, and `events_in` answers that.

Constructors are `pub(crate)` throughout: a snapshot is produced by projecting a timeline and by nothing else. The one
exception is `KeyMap::new`, because naming a key is not a snapshot-building privilege — the MIDI entry buffer spells
incoming notes against a key it was handed, and holds no score.

## Questions consumers ask the kernel rather than answer privately (prompt 44)

Prompt 42 closed the *snapshot* so that guarantees travel by accessor. Prompt 44 does the same one layer down, for the
timeline itself. Before it, four places answered "what is in force here" and "what does this cover" — `project_piece`,
`project_regions`, `plan.rs`'s membership rebuild, and `lower_performance`'s dynamic scan — and they disagreed about end
instants, point occurrences, and coincident onsets. Two of them were keyed on *source position* rather than time, which
is correct only while every context fact spans the whole piece.

The guarantee the kernel now carries is simple, and is the reason these are queries rather than four conventions:

> **Two consumers asking the same question of the same timeline get the same answer.**

| Question | The operation that answers it |
| --- | --- |
| Which occurrences are in force at instant `t`? | `Timeline::covering` (D10) |
| What value is in force at `t`, among the facts I care about? | `Timeline::prevailing` (D11) |
| Where do the boundaries fall — end instants, points, ties? | The convention table in D10–D11, stated once |

Two things consumers may *not* assume. The queries return occurrences, never identities: `EventId` is the score layer's
invention and the kernel does not know it (§12). And `prevailing` takes a selector, not a payload trait — the caller
says which facts are context-bearing, because "this is a key signature" is musical knowledge the kernel must not learn.

Consumers that must answer for *every* event keep their ordered sweep and honour the conventions rather than calling a
query per event; D10's performance rule says why, and benchmark P3 enforces it.

## Shape is normative; sampling is the consumer's (prompt 45)

A `Progress` in a payload (`03-denotational-semantics.md`) is a fact about the piece, and a conforming consumer must
honour it: the same breakpoints, evaluated the same way, in exact rationals. It serializes and it contributes to the
semantic hash, so two implementations that disagree about a shape are reading different pieces.

**Where** to sample the shape is not part of the contract. musa's performance layer samples a hairpin once per notated
event, at `u = index / (count − 1)`, because a hairpin is written around notes and the arrival should not depend on the
rhythm. A consumer that samples per onset, per frame, or per control-rate tick is equally conforming; it will agree at
the endpoints and may differ between them. That is the same latitude a consumer already has over tempo realization
(§22), and it is stated here so nobody encodes musa's sampling choice as though it were the specification.

## What a conforming consumer of a kernel file owes (prompt 48)

`examples/kernel/*.kernel` is the corpus a second implementation is validated against. Reading one, a consumer owes
three things and nothing more:

1. **Refuse a version you do not know.** The first line is `% musa-kernel-1`. A file without it is not a kernel file.
2. **Honour the shape, choose your own sampling.** Every rational in a file — a span, a scale factor, a duration, a
   hairpin's breakpoints — is exact and must stay exact. A consumer that reads a shape through `f64` and writes it back
   has produced a different piece, and the semantic hash will say so. Where to sample is still free, exactly as above.
3. **Agree on the meaning, not the spelling.** Two consumers conform when they evaluate a file to timelines with the
   same normal form (N5) and therefore the same semantic hash (N6). How they got there — whether they expanded `let`
   eagerly, kept the sharing, or restricted before evaluating — is their business, because the calculus's theorems
   (`10-term-calculus.md` T1–T5) say those choices cannot change the answer.

What a consumer does **not** owe: understanding the payload. A file's payloads are opaque strings typed by
`Timeline[<PayloadType>]`, and a consumer that does not own that payload type may still check the file's structure,
report its extent, and compare two files' shapes. It simply cannot say what the music is — which is the correct
division, and the reason the kernel never learned music theory.

## Falsification duty (§33)

Consumers built against this contract are evidence for or against it. If several materially different musical examples
(the §33 corpus: chorale, canon, tuplets, meter/key change, accelerando, crescendo, loops, aleatory, live process)
require awkward or lossy lowering through this contract, the kernel is reconsidered as a whole — never patched per
example. Current status of the corpus is tracked in `08-open-questions.md`.
