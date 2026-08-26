# MIDI-to-notation transcription trial

**Status:** implementation and decision record for prompt 203. This page governs nothing. The keyboard workflow is
governed by [`docs/rules/desktop/10-keyboard-composition.md`](../../rules/desktop/10-keyboard-composition.md); prompts
204–205 implement the decisions measured here.

## Decision

Musa should use a deterministic, bounded top-K structural search, but it must not call its first candidate an answer.
The checked-in trial improved exact onsets from 45/58 to 54/58 and reduced review operations from 28 to 16. It did not
identify a probability model, a universal voice rule, or a reliable written-duration inference. The production object is
therefore a ranked explanation with local alternatives and constraints, not a confidence score.

Known transport/count-in time and musician-supplied beat/downbeat anchors are different in kind from another heuristic.
Across three exact-pinned ASAP performances, mapping into the supplied beat coordinate left a mean absolute residual of
1.144 trial ticks (about 4.8% of a beat) after matching 5,530 notes. Only 3,431 were already on the exact 24-tick grid.
The clock evidence removes global tempo drift; it does not decide how an expressive onset should be written.

The admitted search representation is an immutable candidate DAG with shared back-pointers. Each node states exact onset
and key-end choices, onset group and voice hypotheses, notation structure, derivation event ids, and a versioned cost
record. A candidate materializes its Musa/source preview only after ranking. This replaces the trial's intentionally
simple cloned vectors before prompt 204 makes a production API.

## Evidence and method

MIDI already contains discrete key and controller transitions. No audio onset detection, pitch tracking, source
separation, or DSP belongs in this problem. The trial keeps note-on, key release, pedal-extended sounding end, clock,
and intended written end separate.

The repository-owned corpus has ten executable `.musa` scores and canonical JSON traces. It varies known/free clocks,
straight and syncopated rhythm, triplets, systematic swing displacement, rubato, pickup and asymmetric meter, polyphonic
block/rolled attacks, crossing voices, repetition, a spurious note, pedal, articulation, silence, and unmeasured
material. Generated timing is labeled generated; it is not presented as human performance. The generator, decoder,
source compilation, model results, deterministic repeat, search bound, and hardware-independent QWERTY controller are
laws in `musa-project`.

Three rhythm baselines were compared:

1. independent rounding to a sixteenth grid;
2. a 96-state bounded structural lattice with timing, notation-complexity, transition, and grouping costs; and
3. an HMM-shaped variant with a steeper residual and prior weight.

Three voice/group baselines were compared: current vertical pitch order with a fixed 35 ms grouping window; greedy pitch
continuity with a tempo-relative window; and a bounded cost search with continuity, gap, new-voice, and crossing costs.
Voice labels are scored under optimal permutation. Crossings remain legal.

The full generated outputs are [`corpus.json`](../../../crates/musa-project/tests/fixtures/transcription/corpus.json)
and [`results.json`](../../../crates/musa-project/tests/fixtures/transcription/results.json). Raw external aggregates
and benchmark output are in [`data/203-asap-v1.1-results.json`](data/203-asap-v1.1-results.json) and
[`data/203-benchmark.txt`](data/203-benchmark.txt).

## Generated-corpus results

Nine fixtures make a metrical claim; the tenth deliberately requires source rather than an invented grid.

| Rhythm model | Exact onsets | Intended in top 5 | Exact key durations | Source-token edits | Review operations |
| --- | ---: | ---: | ---: | ---: | ---: |
| Nearest sixteenth | 45/58 | 7/9 | 44/58 | 27 | 28 |
| Structural DP | **54/58** | **8/9** | **47/58** | **23** | **16** |
| HMM-shaped | **54/58** | **8/9** | **47/58** | **23** | **16** |

| Rhythm model | Bar phase | Rest structure | Tie structure | Tuplet structure |
| --- | ---: | ---: | ---: | ---: |
| Nearest sixteenth | 9/9 | 6/9 | **8/9** | **8/9** |
| Structural DP | 9/9 | **7/9** | 7/9 | 7/9 |
| HMM-shaped | 9/9 | **7/9** | 7/9 | 7/9 |

The structural model is the useful representation, not an across-the-board winner. Its complexity prior improved rests
and harmed one tie and one tuplet reading. Cemgil et al.'s central lesson survives the small trial: timing fit and a
notation prior must coexist, while alternate readings remain real. The HMM-shaped weights produced byte-identical
rankings to the simpler structural model, so probability language buys nothing here and is rejected.

| Voice/group model | Voice labels | Grouping pairs | Review operations |
| --- | ---: | ---: | ---: |
| Fixed-window pitch order | 58/63 | 175/176 | 6 |
| Adaptive greedy continuity | **59/63** | **176/176** | **4** |
| Adaptive bounded cost | **59/63** | **176/176** | **4** |

The 40 ms rolled attack is one group at 120 BPM under the adaptive rule and is split by the fixed 35 ms baseline. The
four surviving voice corrections are all in the crossing texture. Neither pitch proximity nor a crossing penalty can
recover authorial voice identity from that performance alone. Nakamura et al.'s joint-rhythm result and Madsen and
Widmer's corpus-sensitive costs therefore support ranked joint candidates, not a hard no-crossing or piano-hand rule.

## What the errors look like

The swing fixture intends straight eighths with systematic performance displacement. On its first offbeat, the intended
source token is equivalent to `note@12/12`; independent sixteenth rounding hears the 70 ms delay as `note@18/12`.
Changing one token would make that onset locally closer and the phrase globally less faithful. The structural top-K can
retain the straight and displaced readings and ask once for the groove.

The triplet fixture's first interior onset is `note@8/8`. Sixteenth rounding proposes tick 6; the hierarchical search
retains tick 8 because the phrase repeats the ternary division. Conversely, the structural prior's preference for a
simple boundary loses one corpus tie and one tuplet structure. The right correction is an alternative candidate, not a
larger universal tuplet bonus.

The pedal fixture releases keys after 180 ms while CC64 holds sound as late as 1.7 s. Written quarter/eighth ends match
neither fact mechanically. The trial scores key duration and reports the mismatch; prompt 205 must consider subsequent
attacks and notation structure while keeping pedal extension separate. The spurious D-flat remains one local review
operation rather than being deleted as a “mistake.”

The crossing fixture needs four voice corrections under every top-one baseline. A Review choice such as “keep these two
lines crossing” is more informative than retuning proximity weights and is exactly local to the affected groups.

## Exact-pinned real performances

The optional adapter refuses any checkout whose `HEAD` is not ASAP v1.1 commit
`fad8d1e8078d0ae47ad2f280b5d022bd2de24784`. It copies no CC BY-NC-SA data. It reads three MIDI/annotation pairs, maps
score and performance into ASAP's aligned beat coordinate, then matches each performed note to the nearest unused
same-pitch score note within two beats. This is an adapter diagnostic, not a ground-truth note alignment; unmatched
notes and the rule are published rather than hidden.

| Performance | Performed/score notes | Matched | Exact 24-tick onset | Absolute tick error |
| --- | ---: | ---: | ---: | ---: |
| Bach, Fugue BWV 846, Shi05M | 754/762 | 739 | 585 | 335 |
| Mozart, Sonata 12/3, WuuE04M | 2,941/2,942 | 2,921 | 2,105 | 1,960 |
| Debussy, *Reflets dans l'eau*, Kleisen11M | 2,019/2,057 | 1,870 | 741 | 4,033 |
| **Total** | **5,714/5,761** | **5,530** | **3,431** | **6,328** |

The 96.8% match rate is sufficient to test clock normalization and insufficient to claim note-transcription accuracy.
The falling exact-grid rate from Bach to Debussy is also a warning against a repertoire-neutral “cleanest grid” prior.
ASAP's own documentation says its scores retain defects, 29 performances are not score-aligned, and voice/beaming/
tuplet production was not validated. Those limitations remain limitations in this report.

Modern learned results remain research references. Beyer and Dai report lower onset/offset error than classical HMMs;
Wachter, Murgul, and Heizmann report 97.3% onset F1 and 83.3% note-value accuracy with beat annotations. Both depend on
trained models, token/post-processing choices, supported-meter subsets, and datasets that cannot become Musa build or
production dependencies under the current license. The residual note-value error still requires Review. No model was
downloaded or admitted.

## Bounds and latency

The optimized benchmark ran on 2026-08-26 at working commit `30f507a4` on an Apple M4 Pro, 24 GiB RAM, macOS 26.6.1, and
Rust 1.98.0. Ten samples gave:

| Phrase | Median | Slowest | Largest retained abstract search |
| --- | ---: | ---: | ---: |
| Complete diverse corpus | 8.749 ms | 9.008 ms | 10,752 bytes |
| 32 regular notes | 7.117 ms | 7.318 ms | — |
| 64 regular notes | 16.75 ms | 16.95 ms | — |
| 128 regular notes | 43.96 ms | 44.27 ms | 102,912 bytes |
| 256 regular notes | 127.0 ms | 130.3 ms | — |

The 128-note run's maximum RSS was 15,138,816 bytes and macOS peak footprint was 9,879,936 bytes; both include the Rust
process and Divan. The abstract count includes candidate records and tick-vector capacity, not allocator metadata. The
trial intentionally does not optimize its cloned candidate paths. Prompt 204 must use shared back-pointers and preserve
the 96-state/128-note/50 ms boundary; a take over 128 notes is split only at a retained complete phrase boundary or
refused with the retained length. A 256-note monolith is not admitted by wishful benchmarking.

## Decisions fixed for prompts 204–205

- Candidate positions and durations are exact. Trial likelihood terms are normalized integers; checked source policy
  supplies exact weights. Floating point may measure physical-time residuals at the device edge but never represents a
  musical position or participates in an unstable tie-break.
- Hard evidence and musician constraints filter first. Remaining candidates order by exact weighted total, then the
  complete cost vector (timing, tempo smoothness, grouping/voice continuity, rests, ties, tuplets, other notation
  complexity), then canonical structural bytes. Every cost field and normalization has a version.
- Return at most five materialized candidates. Retain at most 96 states per search layer, accept at most 128 completed
  notes and four proposed voices, and keep candidate/back-pointer storage below 128 KiB on the 128-note law. The
  reference-host target is 50 ms; exceeding a published hard work bound refuses rather than returning a partial rank.
- Known transport time preserves its calibrated score origin; it is never rebased to the first captured note, because
  that destroys pickups. Free capture returns pulse/phase/meter hypotheses. Beat and downbeat taps are exact constraints
  over the same take and are preferred to additional inference.
- There is no calibrated probability threshold. A region needs review whenever the retained top candidates disagree on a
  bar/beat phase, onset group, written end, tie/rest/tuplet spelling, voice, or pitch spelling there. Free capture also
  asks for pulse/downbeat evidence until the retained structural candidates agree. Expose the disagreement, not a
  percentage.
- Ametric/unmeasured input returns “write source” unless the musician supplies a metrical scope. Unsupported policy
  subdivisions, more than four simultaneous proposed voices, an unsplittable phrase over 128 notes, incomplete note/
  pedal state, or an exhausted search bound are explicit refusals.
- The adaptive onset-group proposal starts at one twelfth of the local beat, clamped to 18–70 ms, and remains only a
  candidate cost. A fixed global window is rejected. Rolled chord, block chord, arpeggio, and separate line readings
  remain alternatives when their structural candidates survive.
- Voice and rhythm are completed jointly. New-voice and crossing costs are finite; an explicit voice-count/group/
  crossing constraint wins. The generated top-one target is at least 59/63 label matches and 176/176 grouping-pair
  matches, but crossing closure is top-K/local-review recall, not a false top-one threshold.
- Key release, pedal-extended sound, and written end remain three facts. No default inserts articulation, slur, dynamic,
  pedal mark, ornament, or “mistake” deletion. Pitch spelling reads checked destination context and preserves MIDI pitch
  identity; absence of tonal context does not mean C major.

These are corpus-relative admission thresholds. Prompt 209 still owes complete real-device workflow and correction-
count evidence before step entry can be removed.
