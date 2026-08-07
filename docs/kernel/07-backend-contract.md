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

## Falsification duty (§33)

Consumers built against this contract are evidence for or against it. If several materially different musical examples
(the §33 corpus: chorale, canon, tuplets, meter/key change, accelerando, crescendo, loops, aleatory, live process)
require awkward or lossy lowering through this contract, the kernel is reconsidered as a whole — never patched per
example. Current status of the corpus is tracked in `08-open-questions.md`.
