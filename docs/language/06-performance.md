# 06 — Elaboration Performance and Compatibility Baseline

Status: **governing for the prompt 93–142 migration**.

This is the before-picture for `docs/elaboration-language.md`. It measures the compiler that accepts only the old
surface language and fixes what that language means before its evaluator, type checker, and parser change. It is not a
claim that these numbers are universal: the compatibility manifests are exact, while timings are local evidence.

## Workloads

The historical `small`, `large`, and `shared` columns remain defined by `docs/kernel/09-performance.md`; adding a
scenario never edits them. Prompt 93 adds four generated, committed fixtures:

| workload | pressure | evaluated occurrences |
| --- | --- | ---: |
| `open-shape` | sixteen motifs placed twice under changing prevailing keys | 134 |
| `higher-order-shape` | the same 128 notes through nested `repeat`, `use`, `stretch`, `retrograde`, and `invert` | 130 |
| `declaration-heavy` | 48 local motifs, 16 fragments, and 64 motifs in four imported libraries, mostly unused | 17 |
| `audio-bridge` | two parts/profiles/patches, shared bus, modulation, hairpins, and articulations | 12 |

`crates/musa-compiler/tests/elaboration_fixture_generators.rs` is the readable source of all four fixtures and the four
import libraries. The test proves the committed text agrees with the generator. No proposed syntax occurs here.

## Measurements and honest seams

`cargo bench -p musa-compiler` uses divan's allocation profiler. Source strings, import maps, parsing for P2–P5, and
timeline production for P3–P5 are outside the timed closure.

| id | measured stage |
| --- | --- |
| P0 | parse only |
| P1 | end-to-end `compile`, the server-side portion of an edit |
| P2 | elaboration after parse, including term construction/evaluation and snapshot projection |
| P3 | projection from evaluated timelines into score events |
| P4 | canonicalization of the whole piece timeline |
| P5 | semantic hashing, including canonical traversal |

The current compiler has no credible independent seam between *closing a term* and *evaluating it*: the private
elaborator performs both before its benchmark sink receives timelines. Publishing a term or phase object solely to put
two extra rows in this table would change the API being baselined. Prompt 97 may create a real internal boundary; until
then P2 is the honest combined measurement. Divan reports allocator activity, including maximum live allocator bytes,
but not process peak RSS per phase. A process-level RSS sample would include the harness and all setup, so it is omitted
rather than mislabeled as a compiler-stage measurement.

## Prompt 93 baseline

Machine: Apple M4 Pro, arm64, macOS 26.5.1, release profile. Compiler samples are divan medians over 100 samples. The
range is retained in command output; timings below are rounded because differences smaller than the timer and scheduler
noise are not evidence. Allocation counts and bytes are exact per iteration.

| phase | workload | median | allocations | bytes allocated |
| --- | --- | ---: | ---: | ---: |
| P0 | open-shape | 17.55 µs | 175 | 18.35 KB |
| P0 | higher-order-shape | 5.33 µs | 96 | 7.55 KB |
| P0 | declaration-heavy | 37.34 µs | 279 | 28.76 KB |
| P0 | audio-bridge | 13.60 µs | 238 | 20.25 KB |
| P1 | open-shape | 304.3 µs | 7,959 | 415.1 KB |
| P1 | higher-order-shape | 227.1 µs | 3,754 | 255.5 KB |
| P1 | declaration-heavy | 463.7 µs | 15,814 | 724.7 KB |
| P1 | audio-bridge | 119.2 µs | 3,382 | 160.1 KB |
| P2 | open-shape | 283.6 µs | 7,785 | 396.8 KB |
| P2 | higher-order-shape | 212.1 µs | 3,659 | 247.9 KB |
| P2 | declaration-heavy | 405.0 µs | 15,536 | 695.9 KB |
| P2 | audio-bridge | 89.52 µs | 3,145 | 139.9 KB |
| P3 | open-shape | 21.97 µs | 928 | 91.31 KB |
| P3 | higher-order-shape | 26.97 µs | 1,173 | 113.1 KB |
| P3 | declaration-heavy | 3.05 µs | 133 | 14.55 KB |
| P3 | audio-bridge | 2.14 µs | 77 | 9.30 KB |
| P4 | open-shape | 50.53 µs | 1,043 | 105.8 KB |
| P4 | higher-order-shape | 66.17 µs | 1,291 | 125.0 KB |
| P4 | declaration-heavy | 5.74 µs | 149 | 15.66 KB |
| P4 | audio-bridge | 3.43 µs | 89 | 9.51 KB |
| P5 | open-shape | 93.27 µs | 926 | 76.61 KB |
| P5 | higher-order-shape | 134.7 µs | 1,038 | 85.50 KB |
| P5 | declaration-heavy | 11.78 µs | 135 | 11.99 KB |
| P5 | audio-bridge | 6.42 µs | 104 | 7.39 KB |

The declaration-heavy fixture is deliberately small after evaluation: its cost is resolver and diagnostic-table work,
not note production. The open/higher-order pair has the opposite purpose: similar denotations with different source
structure expose traversal or sharing regressions.

`cargo bench -p musa-audio --bench audio_bridge` measures a fresh prepared plan per sample, with plan preparation
outside the timed region. At 48 kHz, block size 128, and 240,000 rendered frames, the clean prompt-93 baseline is:

| median | observed range | samples | allocations | bytes allocated | maximum live allocator bytes |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 18.26 ms | 17.74–19.13 ms | 30 | 2 | 65.53 KB | 2.129 MB |

Those two allocations belong to the offline output collector; `RenderPlan::render` retains its separate real-time law.
The audio compatibility manifest also renders block sizes 64 and 256 because block-rate modulation makes the block size
part of the prepared plan's observable behavior.

## Compatibility oracle

`tests/fixtures/elaboration-compatibility.txt` fixes semantic hashes, normalized-kernel digests, diagnostic codes and
labels, Origin paths, `PartId`-bearing performance lanes, studio intent, and the existing MEI/LilyPond/MusicXML snapshot
corpora. `tests/fixtures/audio-bridge-backends.compat` additionally fixes the complete MEI, LilyPond, MusicXML, score
MIDI, performance MIDI, and WAV export bytes through `ProjectSession`. `tests/fixtures/audio-bridge-audio.compat` fixes
the prepared graph digest, lowering notes, scheduled events, block sizes, and float-WAV bytes at the narrower audio
owner. Update requires `UPDATE_ELABORATION_BASELINE=1`; an update is never a routine test repair and its diff must name
the prompt that authorizes the semantic change.

`tests/fixtures/elaboration-expected-changes.json` is the only exception list. Each defect belongs to exactly one of
prompts 124–129. The repairing prompt removes the entry and replaces the negative observation with a positive law; no
later prompt may preserve wrong sound by copying the old digest.

## Gates and budgets

The migration gate is relative: P1 or P2 moving more than 10% on any comparable workload requires a recorded rerun,
allocation comparison, and explicit justification in the implementing prompt. One machine's median is not an absolute CI
threshold. The end-to-end authority remains `docs/interface/06-performance.md`: B1 is ≤120 ms after debounce and B2 is
≤400 ms with the previous engraving visible. Even the declaration-heavy P1 is below 0.5 ms here, so these rows diagnose
the compiler; they do not replace B1/B2.

Prompt 96's resource-exhaustion cases are a separate suite. They must use generated bounded inputs, report the bound,
measure time-to-diagnostic and maximum allocation, and run outside these steady-state samples. A rejected cyclic or
oversized program is not averaged into successful compilation, and an operating-system kill is never reported as a
language diagnostic.

## Prompt 95 comparison

The private functional core was measured on the same Apple M4 Pro in the same release configuration, with 100 samples.
These workloads contain no new expression declarations, so this comparison isolates the cost imposed on existing Musa
programs by collecting an empty definition graph. The exact expression evaluator is exercised by the generated law
suite; prompt 123 owns accepted-envelope evaluator benchmarks.

| phase | workload | median | versus prompt 93 | allocations | bytes allocated |
| --- | --- | ---: | ---: | ---: | ---: |
| P1 | open-shape | 322.8 µs | +6.1% | 8,206 | 424.8 KB |
| P1 | higher-order-shape | 223.5 µs | −1.6% | 3,775 | 256.4 KB |
| P1 | declaration-heavy | 469.3 µs | +1.2% | 16,044 | 734.0 KB |
| P1 | audio-bridge | 107.8 µs | −9.6% | 3,388 | 160.4 KB |
| P2 | open-shape | 293.6 µs | +3.5% | 7,997 | 405.3 KB |
| P2 | higher-order-shape | 207.4 µs | −2.2% | 3,676 | 248.6 KB |
| P2 | declaration-heavy | 412.9 µs | +2.0% | 15,760 | 704.8 KB |
| P2 | audio-bridge | 84.98 µs | −5.1% | 3,151 | 140.1 KB |

No comparable P1/P2 median regressed by more than the 10% review threshold. The largest regression, open-shape P1, adds
247 allocations and 9.7 KB relative to prompt 93; its P2 movement is smaller, and both remain far below the interactive
budget. The negative audio-bridge deltas are ordinary run-to-run improvement, not a claimed optimization. Prompt 95
therefore adds no compatibility exception and does not alter any baseline digest.

## Prompt 96 finite-work curve and limits

Command: `cargo bench -p musa-compiler --bench pipeline -- finite_core`. Same Apple M4 Pro and release configuration;
medians are 100 samples. Source construction is outside the timed closure. The accepted workload performs exactly the
stated number of `nat_fold` iterations with a scalar accumulator; the rejected workload states 200,000 iterations and
measures time to the deterministic diagnostic, separately from successful compilation.

| fold iterations | result | median | allocations | bytes allocated | maximum live allocator bytes |
| ---: | --- | ---: | ---: | ---: | ---: |
| 0 | accepted | 18.32 µs | 552 | 30.31 KB | 12.38 KB |
| 1,000 | accepted | 172.3 µs | 5,553 | 466.3 KB | 12.41 KB |
| 10,000 | accepted | 1.571 ms | 50,553 | 4.390 MB | 12.41 KB |
| 50,000 | accepted | 7.795 ms | 250,553 | 21.83 MB | 12.41 KB |
| 200,000 | resource diagnostic | 9.853 µs | 278 | 16.80 KB | 12.41 KB |

The curve is linear over this deliberately allocation-heavy closure evaluator: about 0.155 µs and five transient
allocations per fold iteration. Total allocated bytes are churn, not retained size—the flat maximum-live column is the
reason the table reports both. Scheduler noise dominates the smallest rows; these local medians are design evidence, not
CI timing thresholds. Boundary tests, rather than timing, fix acceptance exactly.

Prompt 96 sets one internal deterministic meter with these language-version limits:

| metric | limit | evidence and intent |
| --- | ---: | --- |
| reduction steps | 200,000 | admits the measured 50,000-step fold (about 150,000 charged reductions) below 8 ms while rejecting a stated 200,000-step fold before its loop |
| constructed value nodes | 100,000 | admits useful finite collections but preflights `range(100001)` before allocation |
| logical value bytes | 1,048,576 | separately bounds dense exact values; 65,536 repeated ratios crosses it while remaining below the node limit |
| monomorphized prelude instances | 2,048 | far above ordinary declaration counts; a generated 2,049-call boundary fixture fixes the diagnostic |
| estimated music occurrences | 1,000,000 | reserves substantial headroom over the 1,572-occurrence large fixture; prompt 97 activates the charge and prompt 123 retunes from music-producing curves |

There is no public “make it bigger” compiler option: no current caller needs one, and exposing five implementation knobs
as language API would make builds disagree silently. Each rejection names the operation, metric, attempted count, and
limit with code `resource-limit`. Aggregate operations preflight their known shape before allocation; earlier private
values are discarded, no snapshot is returned, and `ProjectSession` retains its last-valid engraving and playback
artifacts. This is resource rejection of a finite program, never a nontermination diagnosis or timeout.
