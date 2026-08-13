# 07 — Backend Contract

What downstream consumers of the event-track core may assume, and what they must never do. Backends today consume
`ScoreSnapshot` and `NotationPlan` rather than tracks directly; this contract applies to those projections as well,
because the projections preserve the core's guarantees.

## What consumers may assume

- **Normalized tracks only.** No consumer ever sees `follow`, `together`, or references — only flat, canonically ordered
  occurrence multisets with exact rational spans (`05-normalization.md`).
- **Exact positions.** Every span is a pair of exact rationals in the track's coordinate. No float, no rounding, no
  sample-frame quantization has occurred at this layer.
- **A stated coordinate.** Every track says whose time it measures, in its type. A consumer never has to infer whether a
  number is a written beat or a second, and never adds one to the other.
- **Multiset multiplicity.** Identical occurrences are distinct facts. A consumer that merges equal occurrences is
  destroying musical information (two performers, one note).
- **Ambient duration.** A track's duration may exceed every occurrence's end; the tail is real temporal extent, not an
  error.
- **Semantic equality.** Comparison, caching, and golden testing use the canonical form (N4–N6) and nothing else.
- **Opacity.** Payloads are typed and serializable storable data, but their musical meaning belongs to their own theory
  modules. A consumer interprets the payload types it understands and ignores nothing silently — unknown payload types
  are an explicit skip, never a misread.

## What consumers must decide themselves

- **Rest glyphs are a notation decision**. An uncovered region of a notated voice is filled with rest symbols by the
  *notation* layer (`NotationPlan`), choosing glyph shapes per meter and convention. The core stores nothing there, and
  no backend may claim the core "has rests."
- **Notation spelling stays verbatim.** Written pitch spelling passes through from the payload; backends do not respell
  (roadmap §6.3).
- **Layout/engraving is downstream.** The plan and the core are semantic, not typographic (roadmap §12.1).

## What performance does

Performance realization is a **coordinate conversion**. It supplies the monotone map
`tempo : WrittenTime → PhysicalTime` and applies it to symbolic positions:

```text
(s, e, a)  ↦  (tempo(s), tempo(e), a)
```

- Changing tempo **never rewrites** the written track. Ritardando/accelerando are tempo-map phenomena, not track edits.
- `stretch` (time scaling, D5) is the *other* operation — it changes the written track. Conflating the two is the bug
  this contract exists to prevent, and since prompt 127a the coordinate tag makes one class of that confusion a type
  error rather than a convention.
- The tempo map is piecewise-monotone; integration details live in the performance layer (prompt 15), not here.
- The conversion is **named and recorded**: it is a pass with a descriptor, origin paths, and a loss list
  (`../across-stages/02-derivation-diagrams.md`), not an implicit reinterpretation of the same numbers.

## What audio does

Audio is the other core value, not another kind of track. A running source is a **machine**
(`../across-stages/03-machine-calculus.md`): a finite description whose one step consumes one input and produces one
output, where one audio step is one sample frame. What it produces — an audio history — is not finite, is not a payload,
and is never an event track.

```text
event track → tempo/performance conversion → EventTrack<PerformedTime, Gesture>
            → schedule(format, policy, time map, track)
            → Machine<AudioFrameStep, …> → step → audio history → output
```

`schedule` is the **checked, named connection** between the two, and it is the only one. It records every conversion
decision it makes and fails rather than guessing. No DSP concept enters a track; no track crosses into a sample buffer.
The engine runs a prepared machine and reads scheduled frames — never tracks.

The machine is emphatically **not** outside the core language: the same total source language builds both values
(`../constitution.md` §9). What is outside is the *history*, because a history is coinductive and a source value is
finite.

### R1 — exact preparation and conditional frame equality

The realization boundary has the conceptual operation:

```text
prepare_execution :
  EventTrack<PerformedTime,Gesture> × Bindings × Seed × Options
  → Result PreparedMachine PrepareError.
```

The gesture track is the exact projection under its admitted payload schema. `Bindings`, `Seed`, `Options`, and the
complete `Result` each own a versioned canonical equality; `Options` includes every acceptance- or execution-affecting
choice, including sample rate, channel layout, batching policy, bounds, and deterministic quality policy.
Presentation-only lineage is produced separately and cannot affect `PreparedMachine`.

**R1.** Equal complete arguments to a pure deterministic `prepare_execution` return equal complete results. This is
function congruence, not a cache or hash assumption.

Equal successful preparation results yield equal output frames only with equal external input histories, equal initial
and allocation state, and primitives conforming to the deterministic step contract in
`../across-stages/03-machine-calculus.md`. Floating-point/device equality is no stronger than those primitive premises.
`prepare_lineage` separately relates the presentation gesture to machine anchors and records losses; it does not modify
execution.

A preparation cache stores the named operation version, every complete framed argument, and the complete result. A
digest locates candidates. A hit is returned only after exact complete-argument confirmation, so a digest collision
cannot change the answer.

## The preserved boundaries

- `ScoreSnapshot` is the score-specific interpretation of the normalized denotation. Note-specific assumptions live
  there, not in the core; if a score concept cannot be expressed through the adapter, the *snapshot* grows, not the
  core.
- The `ScoreSnapshot → NotationPlan → backend` pipeline is unchanged. Notation never learns about motif expansion,
  repetition semantics, source functions, loops, or transformations — it consumes already-resolved temporal facts.
- Provenance flows: `EventId`/`Origin` in the snapshot are the same provenance carried in occurrence payloads, so the
  editor's source-mapping contract (MEI `xml:id`, click-to-source) survives the core unchanged.

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

Constructors are `pub(crate)` throughout: a snapshot is produced by projecting a track and by nothing else. The one
exception is `KeyMap::new`, because naming a key is not a snapshot-building privilege — the MIDI entry buffer spells
incoming notes against a key it was handed, and holds no score.

## Questions consumers ask the core rather than answer privately (prompt 44)

Prompt 42 closed the *snapshot* so that guarantees travel by accessor. Prompt 44 does the same one layer down, for the
track itself. Before it, four places answered "what is in force here" and "what does this cover" — `project_piece`,
`project_regions`, `plan.rs`'s membership rebuild, and `lower_performance`'s dynamic scan — and they disagreed about end
instants, point occurrences, and coincident onsets. Two of them were keyed on *source position* rather than time, which
is correct only while every context fact spans the whole piece.

The guarantee the core now carries is simple, and is the reason these are queries rather than four conventions:

> **Two consumers asking the same question of the same track get the same answer.**

| Question | The operation that answers it |
| --- | --- |
| Which occurrences are in force at instant `t`? | `covering` (D10) |
| What value is in force at `t`, among the facts I care about? | `prevailing` (D11) |
| Where do the boundaries fall — end instants, points, ties? | The convention table in D10–D11, stated once |

Two things consumers may *not* assume. The queries return occurrences, never identities: `EventId` is the score layer's
invention and the core does not know it. And `prevailing` takes a selector, not a payload trait — the caller says which
facts are context-bearing, because "this is a key signature" is musical knowledge the core must not learn.

Consumers that must answer for *every* event keep their ordered sweep and honour the conventions rather than calling a
query per event; D10's performance rule says why, and benchmark P3 enforces it.

## Shape is normative; sampling is the consumer's (prompt 45)

A `Progress` in a payload (`03-denotational-semantics.md`) is a fact about the piece, and a conforming consumer must
honour it: the same breakpoints, evaluated the same way, in exact rationals. It serializes and it contributes to the
semantic hash, so two implementations that disagree about a shape are reading different pieces.

**Where** to sample the shape is not part of the contract. musa's performance layer samples a hairpin once per notated
event, at `u = index / (count − 1)`, because a hairpin is written around notes and the arrival should not depend on the
rhythm. A consumer that samples per onset, per frame, or per control-rate tick is equally conforming; it will agree at
the endpoints and may differ between them. That is the same latitude a consumer already has over tempo realization, and
it is stated here so nobody encodes musa's sampling choice as though it were the specification.

### A gradual tempo change is the second reader of that rule (prompt 73)

A `Tempo` payload may carry a `Ramp`: where the speed arrives, how far it reaches, and a `Progress` saying how the
change is spread across that reach. Two consumers read the same payload for genuinely different purposes, which is what
the rule above exists for.

- **The engraver samples it not at all.** No notation format has a continuous tempo, so the page prints the word where
  the ramp begins and the speed where it arrives, and the middle is the reader's. MEI, MusicXML and LilyPond each get a
  marking at both ends.
- **The performance layer integrates it exactly.** The rate is linear in *seconds per beat* along the shape, so the
  elapsed time over each of the shape's pieces is `(u₁ − u₀) · (s₀ + s₁) / 2` — the trapezoid rule, which for a
  piecewise-linear rate is not an approximation but the integral. Every value stays rational.
- **MIDI samples it, and picks its own density.** SMF has one set-tempo event or a run of them; musa's exporter writes
  **32 per whole note** of a ramp's reach. That number is the exporter's, stated here and not in the score, and a
  conforming consumer may choose another. It will agree at the endpoints and may differ between them.

Interpolating **beats per minute** instead would put the rate's reciprocal under the integral and leave the rationals,
which §4 forbids — so the exactness requirement decides a question that looks like a matter of taste. It happens to
agree with practice: an orchestra told to slow down evenly slows evenly in duration.

### A scope is the third reader, and the one MIDI cannot follow (prompt 75)

Every context fact carries a `Scope`, and `Meter` and `Tempo` inherit by `Override`: a part that states its own is read
at its own and every other scope reads the piece's. Polymeter and polytempo are therefore not an extension of this
format — a file written before either existed is already a legal file with every part in the piece's scope, and a file
with them differs only in which scope some occurrences carry.

What differs is what each consumer can do with it.

- **A consumer of the kernel file loses nothing.** The scope is in the occurrence, so "the meter for part 1" is a
  question the file answers.
- **MusicXML has a measure list per part and MEI does not.** MusicXML therefore carries divergent barlines exactly; MEI
  states each staff's meter on its `<staffDef>` but numbers `<measure>` for the score, so a piece whose barlines
  genuinely diverge is exported with a warning saying which measure numbering the document carries. LilyPond needs
  Timing moved from `Score` to `Staff`, which the export writes.
- **SMF has one tempo track and one time-signature track, and no scope at all.** A polytempo performance is exported by
  resolving every lane against its own map and writing every note at the frame it actually sounds. The file is therefore
  **sonically exact and notationally wrong**: it plays correctly, and the tempo it states is the piece's reference
  rather than any part's. Both losses are reported by the exporter. This is the one place in this document where a
  format is asked to say something it cannot, and the honest degradation is to be right about the sound and explicit
  about the notation.

## What a conforming consumer of a kernel file owes (prompt 48)

`examples/kernel/*.musa.kernel` is the corpus a second implementation is validated against. Reading one, a consumer owes
three things and nothing more:

1. **Refuse a version you do not know.** The first line is `% musa-kernel-2`. A file without it is not a kernel file,
   and a `musa-kernel-1` file is refused rather than migrated (`01-grammar.md`).
2. **Honour the shape, choose your own sampling.** Every rational in a file — a span, a scale factor, a duration, a
   hairpin's breakpoints — is exact and must stay exact. A consumer that reads a shape through `f64` and writes it back
   has produced a different piece, and the semantic hash will say so. Where to sample is still free, exactly as above.
3. **Agree on the meaning, not the spelling.** Two consumers conform when they evaluate a file to tracks with the same
   N4 canonical semantic value and therefore the same semantic hash (N6). N5 human text alone is insufficient. How they
   got there — whether they expanded `let` eagerly, kept the sharing, or restricted before evaluating — is their
   business, because the calculus's theorems (`10-term-calculus.md` T1–T5) say those choices cannot change the answer.

What a consumer does **not** owe: understanding the payload. A file's payloads are opaque strings typed by
`EventTrack[<Coordinate>, <PayloadType>]`, and a consumer that does not own that payload type may still check the file's
structure, report its duration, and compare two files' shapes. It simply cannot say what the music is — which is the
correct division, and the reason the core never learned music theory.

## A file is one realization of the work (prompt 66)

A piece may leave decisions to the performance: an open repeat count, a free duration, an order of fragments. Where it
does, `11-realization.md` puts the decision **above** the core — a seed plus an override set, supplied to elaboration —
so that by the time a term exists every choice is made and everything above stays true. Two consequences reach a
consumer, and only two:

1. **A `.musa.kernel` file is the projection of one realization, not of the work.** Its header says which realization
   produced it, and reproducing the file means being given the same one. A file of a piece that fixes everything has
   nothing extra to say, which is why every file written before prompt 66 stays valid.
2. **A consumer never chooses.** Choosing happened once, before the term existed. A consumer that draws a random number
   has produced a different piece, and the semantic hash will say so.

What a consumer does **not** owe, again: understanding the freedom. An open repeat reaches it as an ordinary occurrence
with a payload it may or may not know, on the same terms as every other payload.

## A notation export of a freedom is lossy, and says so (prompt 68)

Prompt 68 gives three of those freedoms a surface — a mobile's order, a freely-held duration, an improvised frame — and
none of MEI, `LilyPond`, or `MusicXML` has an element that means any of them. What each export therefore carries is:

- the **realized** music, written out in full, exactly as this compilation read the piece; and
- the **instruction** as a text direction over it — `any order — this reading: …`, `hold to 2`, `improvise over Dm7 |
  G7` — placed at the region's start, and at its end where it spans more than a measure.

That is a reading of the work rather than the work, so `render_notation` reports it: one warning per lost kind, per
render, on `RenderedNotation::warnings` and out of the CLI on stderr. **A silent lossy export is a contract violation**;
a loud one is the format's limit, honestly stated. A consumer reading such a file gets a complete, exactly-timed piece
and a human-readable note that it was one of several — which is the most either format can carry, and more than a bare
export would.

MIDI carries the realization and nothing else, and warns about nothing: a performance is exactly what MIDI is for.

The `.musa.kernel` file remains the lossless one. A consumer that needs the freedom itself reads the occurrence payload,
where the fragments, their chosen order, and the bounds of a held note all survive.

## A grace note is written, not read (prompt 71)

A grace note is a **point occurrence**: start equal to end, standing at the onset of the note it leans on, carrying a
written pitch, its own marks, and an `index` giving its place in the group. It has no written duration, and that is the
fact rather than an omission — so a consumer cannot read a duration off the page, and must not invent one and call it
the piece.

How long it sounds and whose time it takes are the **profile's** (`musa_compiler::GracePolicy`: `steal` and `from`). Two
profiles read the same page as an appoggiatura on the beat and an acciaccatura ahead of it, and the engraving is
byte-identical under both.

Every notation format offers to settle this for you, and a conforming backend declines all three offers:

| Format | The offer | What musa writes |
| --- | --- | --- |
| MEI | `@grace="acc"` / `"unacc"` — on the beat or ahead of it | `@grace="unknown"`, inside `<graceGrp attach="pre">` |
| `MusicXML` | `steal-time-previous` / `steal-time-following` | neither attribute; `<grace slash="yes"/>` and no `<duration>` |
| `LilyPond` | `\acciaccatura` / `\appoggiatura` | `\grace { … }` |

`LilyPond` is the one worth naming, because its vocabulary complects the two layers hardest: its only *neutral* command
is `\grace`, and the two named ones each add a slash, a slur, and a reading. musa writes `\grace` and lets the house
style do what a performer does. The written durations inside it (`c8`) are stem flags, not durations — MEI's `dur="8"`
and `MusicXML`'s `<type>eighth</type>` are the same instruction, and no consumer may read any of the three as time.

The order within a group is normative. Normalization (05, N2) sorts occurrences by span and then by payload key, and
every grace in a group shares a span — so the order lives in the payload's `index`, and a consumer that prints or plays
them in any other order is printing different music. `examples/graces.musa` and `examples/graces-reordered.musa` are the
conformance pair: identical but for two reversed groups, and their kernel terms differ.

## Falsification duty

Consumers built against this contract are evidence for or against it. If several materially different musical examples
(the falsification corpus: chorale, canon, tuplets, meter/key change, accelerando, crescendo, loops, aleatory, live
process) require awkward or lossy lowering through this contract, the core is reconsidered as a whole — never patched
per example. Current status of the corpus is tracked in `08-open-questions.md`.
