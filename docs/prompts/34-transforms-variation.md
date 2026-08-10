---
id: 34
slug: transforms-variation
status: done
depends_on: [12, 25]
phase: 3
---

# Transformations and Variation: stretch, retrograde, invert, specialization

## Task

Complete the transformation story on top of the temporal kernel: `stretch`, `retrograde`, and `invert` alongside
`transpose`; occurrence specialization (`use sigh() with { ... }`) so prompt 25's `Specialize` mode becomes real; and
the laws of roadmap §5.4 — including retrograde's anti-homomorphism — as executable property tests. Every transform is
an **elaboration-time function over timelines or payloads** (course correction §13–14); none adds a kernel constructor.

## Read

- Course correction §13 (payload mapping is functorial — transposition/inversion are payload maps, no `Transform` kernel
  node), §14 (time scaling is an external action — augmentation/diminution evaluate into ordinary kernel timelines, no
  permanent `Stretch` node), §19–20 (surface structure and provenance stay above the normalized kernel), §29 (do not add
  dedicated lowering cases for new musical concepts), §34 (the governing rule: semantic necessity only).
- Roadmap §5.4 (transformation laws; retrograde reverses order: `retro(a then b) = retro(b) then retro(a)`;
  transformations may fail, change shape, or require a pitch system), §7.2 (finite constructs list), §9 (occurrence
  specialization syntax and semantics), §17.2 (law tests).
- Prompt 12's canonical kernel elaboration path (transforms elaborate through it), prompt 25's
  `GeneratedEditMode::Specialize` stub.

## Design

- Language: `stretch 3:2 { ... }` (rational factor), `retrograde { ... }`, `invert axis <pitch> { ... }` (or
  `invert around c5` — pick and document). All composable with each other, `transpose`, `repeat`, and motif calls; all
  finite (§7.2). The parser accepts the constructs; **elaboration** (not the kernel) defines their meaning:
  - `stretch r` → the kernel's time-scaling action (course correction §14) applied during elaboration:
    `span(stretch(r, x)) = r · span(x)`, exact rationals.
  - `retrograde` → the derived time-reversal function on the elaborated finite timeline:
    `(d, E) ↦ (d, {(d−e, d−s, a)})`. It is a plain function in the elaboration module — the kernel needs no reversal
    primitive (record this in `docs/kernel/08-open-questions.md` as evidence for §34's "smallest complete basis").
  - `invert axis` → payload map mirroring pitch around the axis (chromatic with respelling preference — pick and
    document; spellings needing more than a double accidental are a **diagnostic**, not silent wrong notes; §5.4 allows
    meaningful failure).
- Provenance: `ExpansionStep` gains `Stretch(Ratio)`, `Retrograde`, `Inversion { axis }` — provenance paths live above
  the kernel and remain fully inspectable (§20, roadmap §8.3). The normalized kernel timeline itself forgets which
  transform produced it; that is the semantic quotient working as intended.
- Occurrence specialization: `use sigh() with { note 2 = d5; }` — an override map on the elaboration of that call.
  Overrides apply after the motif body elaborates, and the overridden occurrence's `Origin` records both the motif
  application and the override span. Resolution of `note 2`: ordinal within the call's elaboration. Diagnostics for
  out-of-range ordinals.
- Prompt 21's `Specialize` mode is now implemented in the GUI: clicking a generated note and changing it offers
  edit-definition vs specialize, and specialize rewrites the source to the `with { }` form (adding the `with` clause to
  an existing call, or merging into one that exists). This completes roadmap §9's editing story.
- Property tests (roadmap §17.2): `stretch(1, x) = x`; `retrograde(retrograde(x)) = x`;
  `retrograde(a then b) = retrograde(b) then retrograde(a)` — expressed as kernel `sequence` anti-homomorphism;
  `invert(invert(x)) = x` (when inversion succeeds); stretch distributes over `sequence`/`overlay` (course correction
  §14 already proves scaling does — test it at the elaboration level).

## Target

- `musa-language`/`musa-compiler`: syntax, elaboration semantics, provenance, diagnostics for the three transforms +
  specialization.
- `musa-project`/desktop: working `Specialize` edit mode.
- `examples/`: a variation fixture (theme + stretch + retrograde + inversion + one specialized occurrence).
- Tests: the law suite above; snapshot elaborations with full origin paths; specialization override tests incl.
  diagnostics.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/variation.musa
cd apps/musa-desktop && cargo tauri dev   # manual: specialize a motif occurrence from the score
```

Commit as `Add stretch, retrograde, invert, and occurrence specialization`.

## Repairs made while implementing

- **`stretch 3/2`, not `stretch 3:2`.** The language already spells exact ratios with `/` (durations, meters, tuplets);
  a second ratio syntax for one construct would be a dialect. An integer factor (`stretch 2`) is accepted as the whole
  number it is.
- **`invert around c5`, not `invert axis c5`.** It reads as English at the point of use, and `around` is a keyword only
  after `invert`.
- **Stretch renotates, it does not only rescale.** Augmentation is a notational act as well as a temporal one: a bar of
  quarters stretched by two is *written* as halves. `NotatedDuration::stretched` scales the value and its written pieces
  and respells them, which is what separates `stretch` from a tuplet (whose written values deliberately stay put while
  their sounding value changes).
- **Inversion is diatonic, then spelled.** The mirror keeps the letter distance and the semitone distance, so `c5 e5 g5`
  about `c5` becomes `c5 af4 f4` — an engraver's inversion, not a chromatic one that would write `gs4`. A result past a
  double accidental is a diagnostic naming the pitch and the axis (roadmap §5.4's meaningful failure).
- **Retrograde reverses ties.** A tie is a relation to the *next* sounding group, so reversing moves each mark back one
  group; double reversal restores the original, which is a law test. `retie` rebuilds occurrences rather than mutating
  payloads, because `Occurrence` exposes no payload mutation and does not need to.
- **The specialization ordinal counts events, not sounding notes.** The first draft skipped rests. That silently
  disagreed with the score inspector's `▸ note 3`, which counts every event of the occurrence — and a composer reading a
  position off the score and typing it into a `with` clause is naming the note they are looking at. Positions now match,
  and a position that is a rest or a chord is an error naming which.
- **A `with` clause cannot specialize a call that runs more than once.** The clause belongs to the call, so a `use`
  inside a `repeat` would change every run — the thing the mode exists to avoid. `EditImpact` carries `specializable` so
  the interface can say why the second answer is unavailable instead of offering it and failing.
- **Only a pitch can be specialized.** `ChangeDuration` with `Specialize` is refused by name: an override respells one
  note, and renotating one inside an occurrence would move every note after it.
- **`EditIntent::Specialize` is the language's job.** Adding, merging, and ordering `note n = p;` overrides is syntax
  work, so it lives in `musa-language::edits` beside the other intents; `musa-project` resolves provenance to
  `(call site, position)` and nothing more.
- **Retrograde needs no kernel primitive**, recorded as §34 evidence in `docs/kernel/08-open-questions.md`: the finite
  kernel's occurrences are already a materialized set, so reversal is a mapping over them.

## Stop

- No new kernel constructors for any of these (course correction §29, §34); a perceived need is a spec repair, committed
  first.
- No "variation" operators beyond these three (no random/humanize transforms — roadmap §7.2 finite + deterministic; §8.3
  rejects unstable semantics).
- No theory-driven transforms (neo-Riemannian etc. — roadmap §8.2 libraries, not core).
- No motif extraction improvements (prompt 25's version stands; refine only if the fixture exposes a bug).
