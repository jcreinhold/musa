---
id: 11
slug: kernel-elaboration
status: done
depends_on: [10]
phase: 1
---

# Surface Elaboration Through the Kernel

## Task

Give the existing surface language a second semantic path: elaborate the current grammar (notes, rests, chords, voices,
parts, motifs, `repeat`, `transpose`) through the temporal kernel and adapt the result back into the existing
`ScoreSnapshot`, then differentially validate it against the prompt-05/06 direct lowerer on every fixture. The old
lowerer stays as the regression oracle; this prompt adds the new path **alongside** it — switching is prompt 12.

## Read

- `docs/kernel/06-surface-elaboration.md` (normative elaboration rules; prompt 08) and `07-backend-contract.md`.
- Course correction §19 (surface structure is preserved in the HIR; normalization is a semantic boundary, not the
  working representation), §20 (provenance above the semantic quotient — provenance rides in payload metadata), §27
  (`ScoreSnapshot` as the score-specific interpretation of the normalized denotation), §30 Steps 4–5, §33 (falsification
  corpus).
- Prompt 05's `lower.rs` (the oracle), prompt 06's expansion pass (motif/repeat/transpose semantics that must be
  reproduced exactly), prompt 09's `musa-kernel` surface.

## Design

- `musa-compiler` gains a dependency on `musa-kernel` and a private elaboration module. Payload design is the central
  decision — follow `06-surface-elaboration.md`: each note occurrence's payload carries its `WrittenPitch`, its
  part/voice identity (the §32 candidate decision: identity as payload metadata, not a temporal primitive), and its
  `Origin` (provenance above the kernel, §20). Chords elaborate to simultaneous per-pitch occurrences; the adapter
  regroups them into `ScoreEventKind::Chord` for the snapshot (record this regrouping rule in the module docs).
- Elaboration shape per construct (surface → kernel):
  - voice body → `sequence` of its items; a rest item contributes ambient extent with **no** note occurrence (§2);
  - chord → `overlay` of one occurrence per pitch over the same span;
  - part → `overlay` of its voices (identity in payloads keeps lanes separable downstream);
  - `transpose` → `map_payload` (§13); `repeat` → `sequence` of n evaluations at the HIR level, observed into the kernel
    (§19); motif calls → binding/reference + provenance, evaluated like their bodies;
  - key/meter/tempo → snapshot maps as today (they are context, not occurrences, in the current surface grammar —
    `06-surface-elaboration.md` §21's typed-interval-payload view is the future shape; note the gap in
    `08-open-questions.md` if one appears).
- `CompileOptions` gains an internal elaboration switch (default: the old path until prompt 12). `compile` runs the
  selected path; the differential harness runs both.
- Differential testing (§30 Step 5): for all `examples/*.musa` and a proptest-generated corpus of valid sources,
  old-path and new-path snapshots must agree on: event positions and durations (exact), pitch spelling, part/voice
  identity, event multiplicity, ordering, and origin spans/paths (provenance mapping). Report mismatches with both event
  sets printed; a mismatch is a bug in the new path until proven otherwise.
- Falsification corpus (§33), first instalment, as fixtures with hand-checked kernel normal-form snapshots:
  1. *Twinkle Twinkle* (sequential pitches and rests) — new `examples/twinkle.musa`;
  2. the existing `counterpoint.musa` (synchronized voices);
  3. a canon built from motif + transpose + delay-by-rest (reuse, delay, overlay) — new `examples/canon.musa`.
  Tuplets, meter/key change, accelerando, glissando, loops, aleatory, and live process stay in
  `08-open-questions.md` until the surface grammar can express them.

## Target

- `crates/musa-compiler/`: elaboration module + kernel-payload types + snapshot adapter + options switch.
- `examples/twinkle.musa`, `examples/canon.musa` compiling identically under both paths.
- Tests: the differential suite (fixtures + generated corpus) and the three falsification snapshots.
- `docs/kernel/08-open-questions.md` updated with anything the elaboration exposed (e.g. chord regrouping, empty-voice
  extents).

## Check

```sh
cargo nextest run -p musa-compiler
cargo clippy --all-targets -p musa-compiler -- -D warnings
cargo fmt --check
for f in examples/*.musa; do cargo run -p musa -- check "$f"; done   # all pass under the default path
```

Commit as `Elaborate surface constructs through the temporal kernel`.

## Stop

- Do not remove or change the prompt-05/06 lowering path; it is the oracle until prompt 12 and stays runnable after.
- Do not change the surface grammar to make elaboration easier — elaborate what exists (§35.9).
- No new surface constructs (no tuplets, no meter changes) to chase the remaining falsification examples.
- No performance work on the new path; parity first.
