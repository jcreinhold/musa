# 06 — Elaboration Performance and Compatibility Baseline

Status: **governing for the prompt 93–171 migration**.

This is the before-picture for the elaboration language. It measures the compiler that accepts only the old surface
language and fixes what that language means before its evaluator, type checker, and parser change. It is not a claim
that these numbers are universal: the compatibility manifests are exact, while timings are local evidence.

## Workloads

The historical `small`, `large`, and `shared` columns remain defined by `docs/rules/kernel/09-performance.md`; adding a
scenario never edits them. Prompt 93 adds four generated, committed fixtures:

| workload | pressure | evaluated occurrences |
| --- | --- | ---: |
| `open-shape` | sixteen motifs placed twice under changing prevailing keys | 134 |
| `higher-order-shape` | the same 128 notes through nested `repeat`, `use`, `stretch`, `retrograde`, and `invert` | 130 |
| `declaration-heavy` | 48 local motifs, 16 fragments, and 64 motifs in four imported libraries, mostly unused | 17 |
| `audio-bridge` | two parts/profiles/patches, shared bus, modulation, hairpins, and articulations | 12 |

`crates/musa-compiler/tests/suite/elaboration_fixture_generators.rs` is the readable source of all four fixtures and the
four import libraries. The test proves the committed text agrees with the generator. No proposed syntax occurs here.

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

`tests/fixtures/elaboration-expected-changes.json` is the only exception list. Each defect names the one prompt that
repairs it, by slug rather than by rank — the ledger pointed at the wrong prompts for three insertions before that was
fixed, because a rank is a schedule and an identity must not be spelled as one. The repairing prompt removes the entry
and replaces the negative observation with a positive law; no later prompt may preserve wrong sound by copying the old
digest.

## Gates and budgets

The migration gate is relative: P1 or P2 moving more than 10% on any comparable workload requires a recorded rerun,
allocation comparison, and explicit justification in the implementing prompt. One machine's median is not an absolute CI
threshold. The end-to-end authority remains `docs/rules/desktop/06-performance.md`: B1 is ≤120 ms after debounce and B2
is ≤400 ms with the previous engraving visible. Even the declaration-heavy P1 is below 0.5 ms here, so these rows
diagnose the compiler; they do not replace B1/B2.

Prompt 96's resource-exhaustion cases are a separate suite. They must use generated bounded inputs, report the bound,
measure time-to-diagnostic and maximum allocation, and run outside these steady-state samples. A rejected cyclic or
oversized program is not averaged into successful compilation, and an operating-system kill is never reported as a
language diagnostic.

## Prompt 95 comparison

The private functional core was measured on the same Apple M4 Pro in the same release configuration, with 100 samples.
These workloads contain no new expression declarations, so this comparison isolates the cost imposed on existing Musa
programs by collecting an empty definition graph. The exact expression evaluator is exercised by the generated law
suite; prompt 127 owns accepted-envelope evaluator benchmarks.

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
| instantiated prelude entries | 2,048 | far above ordinary declaration counts; a generated 2,049-call boundary fixture fixes the diagnostic |
| estimated music occurrences | 1,000,000 | reserves substantial headroom over the 1,572-occurrence large fixture; prompt 97 activates the charge and prompt 127 retunes from music-producing curves |

There is no public “make it bigger” compiler option: no current caller needs one, and exposing five implementation knobs
as language API would make builds disagree silently. Each rejection names the operation, metric, attempted count, and
limit with code `resource-limit`. Aggregate operations preflight their known shape before allocation; earlier private
values are discarded, no snapshot is returned, and `ProjectSession` retains its last-valid engraving and playback
artifacts. This is resource rejection of a finite program, never a nontermination diagnosis or timeout.

## Prompt 127 workloads

Prompt 93's four fixtures pressure the surface language. Prompt 127 adds five more, generated by the same test
(`crates/musa-compiler/tests/suite/elaboration_fixture_generators.rs`) and committed beside them, each written to
isolate one layer the elaboration language added. `the_pressure_workloads_compile_and_denote_what_they_claim` fixes what
each one denotes, because a benchmark on a fixture that stopped compiling measures the diagnostic path silently and
forever.

| workload | pressure | denoted events | evaluated occurrences |
| --- | --- | ---: | ---: |
| `core-pressure` | a 64-deep call chain, a 512-element fold, a named function value, and `compose_music` | 8 | 8 |
| `template-pressure` | a functor made eight times, read by a voice template that repeats each instance under two scales, plus one source import | 272 | 281 |
| `analysis-pressure` | 64 bars of four-part harmony over a constructed tonal vocabulary | 448 | 448 |
| `kernel-pressure` | 32 typed quote holes bound in one term, each placed three ways | 352 | 352 |
| `kernel-pressure.musa.kernel` | a kernel document of 16 phrases × 32 occurrences, each placed three ways | 1,280 | 1,280 |

The two columns are counted at different layers — denoted events are what the score snapshot publishes, evaluated
occurrences are what the kernel timeline holds — and they agree everywhere except `template-pressure`, where the
timeline carries nine more. Both are printed by `cargo bench -p musa-compiler` before the tables, so no timing row can
be read without the size of the workload behind it. `core-pressure` denotes eight notes on purpose: everything expensive
in it is the expression layer, so an evaluator regression cannot hide behind note production. `analysis-pressure` trips
the repeated-bar lint on purpose too — the lint pass is part of what a keystroke pays, and a workload that avoided
tripping it would measure a compiler nobody runs.

Five more measurements join the table for them:

| id | measured stage |
| --- | --- |
| P6 | the tonal reading of a whole piece |
| P7 | every analysis kind at once — what a reader opening the analysis panel pays |
| K0 | a `.musa.kernel` document compiled as itself |
| E0–E4 | format, parser recovery, an edit in the first bar, an edit in the last bar, the name under the cursor |
| S0–S2 | identical calls, distinct arguments, and the same music hand-hoisted |

E2 and E3 exist as a pair: the compiler is not incremental, so if an edit at the head ever costs differently from an
edit at the tail, something has started depending on where the edit landed.

## The two sharing gaps, measured

The compiler elaborates a motif body once per entry in an internal table keyed by everything the body's facts depend on.
Through prompt 126 that key included the **call span**, so two calls shared a body only when they were written in the
same place — which no two calls are. Prompt 127 measured what that cost before deciding anything.

The measurement is not a timing. Duplication is visible in the term the compiler prints: a body elaborated twice is two
`let shared` bindings holding the same occurrences. `cargo bench -p musa-compiler` reports both counts for every
workload and every sharing shape, so the timings below can be read against the size of the term behind them.

`S0` scales identical calls at distinct sites, which is the **call-site gap**. `S1` and `S2` are the same music written
two ways — the argument-independent tail inside the parameterized body, and hoisted out of it by hand — which is the
**full-laziness gap**. All three use a sixteen-note body; `S1` and `S2` walk seven letters over five octaves, so above
35 calls the arguments repeat, as they do in a real piece that calls one motif five hundred times.

| shape | calls | bindings before | bindings after | term bytes before | term bytes after |
| --- | ---: | ---: | ---: | ---: | ---: |
| identical calls | 8 | 8 | **1** | 16,399 | **2,833** |
| identical calls | 32 | 32 | **1** | 64,848 | **4,726** |
| identical calls | 128 | 128 | **1** | 259,019 | **12,601** |
| identical calls | 512 | 512 | **1** | 1,036,887 | **44,741** |
| distinct arguments | 8 | 8 | 8 | 17,432 | 17,432 |
| distinct arguments | 32 | 32 | 32 | 68,954 | 68,954 |
| distinct arguments | 128 | 128 | **35** | 275,413 | **83,096** |
| distinct arguments | 512 | 512 | **35** | 1,102,529 | **115,606** |
| hand-hoisted | 8 | 16 | **9** | 18,404 | **4,756** |
| hand-hoisted | 32 | 64 | **33** | 72,986 | **12,536** |
| hand-hoisted | 128 | 256 | **36** | 291,797 | **28,835** |
| hand-hoisted | 512 | 1,024 | **36** | 1,170,119 | **94,850** |

Two facts in that table decided the prompt.

**The call-site gap was the whole cost.** Bindings grew with the number of call sites in every shape — one per site, and
two per iteration in the hand-hoisted spelling — so the term grew linearly in how often the music was *asked for* rather
than in how much music there was. At 512 calls of one sixteen-note phrase the compiler built a megabyte of term to say a
thing that fits in 44 KB.

**The full-laziness gap was a consequence of it, not a second problem.** Before the fix, hoisting the
argument-independent tail by hand made matters *worse* — 1,024 bindings against 512, because each iteration now made two
calls and each call was its own site. A composer following the standard advice for sharing would have been penalised for
it. After the fix the same hoist behaves as written: 36 bindings against 35, the one extra being the tail it hoisted.

The committed fixtures move too. `open-shape` binds sixteen bodies instead of thirty-two and prints an 11,897-byte term
instead of a 20,059-byte one; it is sixteen motifs placed twice, and placing a motif twice is now one body. So does the
example the gap was found in: `examples/changes.musa` bound one twenty-two-occurrence body twice under two names, and
now binds it once, with five references carrying five distinct call sites in their marks.

## What the call span was standing in for

The span could not simply be dropped. A body reads the scale in force *at the call* — the innermost `in scale`, or else
the key latest at the cursor — and the key recorded the piece's `scale` setting but not that lexical pitch context.
Distinct spans were what kept two readings of one saved phrase apart, and
`scale_context_laws.rs::one_bound_phrase_elaborates_differently_under_two_scales` fails the moment the span goes on its
own: `use subject` under C major and under C dorian must produce `e5` and `eb5`.

So the key names the effective pitch context directly, and the site itself is carried where it belongs — at the
*reference*, in its mark. `crates/musa-compiler/tests/suite/sharing_laws.rs` fixes that statement in four parts:

- `identical_calls_at_distinct_sites_elaborate_one_body` — four `use`s of one motif, one binding.
- `a_parameterized_body_is_one_binding_per_distinct_argument` — eight calls, two arguments, two bindings.
- `one_body_read_under_two_scales_is_two_bodies` — the same motif under two scales is two pieces of music.
- `a_shared_body_carries_no_call_site_and_each_reference_carries_its_own` — three references, three distinct call sites
  in the printed marks.

Against the prompt-126 compiler the first three fail, at 4, 8, and 4 bindings respectively.

Three things a shared body would otherwise stop doing once per call had to be accounted for, and the prompt required
each to be shown unchanged or re-charged at the reference:

**Provenance** is re-charged at the reference. The body prints once with a placeholder where the call would be, and
every reference states its own site, scope, and expansion steps in its mark (`docs/rules/kernel/10-term-calculus.md`
T6). `shared_instantiations_are_closed_and_keep_definition_and_call_provenance` already fixed this shape for two uses of
one binding; it now covers every call of every motif. The compatibility oracle's Origin paths are byte-identical.

**Diagnostics** are unchanged. A diagnostic reported from inside a body was never reported once per call — resolution
reports it where the defect is written — and the oracle's ordered diagnostics are byte-identical across the corpus.

**The output meter** is charged at the boundary and nowhere else, which is what makes sharing invisible to it. This is
the one place where the first implementation of this prompt was wrong, and the error is worth recording: charging the
table's *hits* looked like "charge the program, not the compiler's cleverness", and was in fact a double charge. A
640,000-occurrence piece written as 160 calls of one motif was refused, where the prompt-126 compiler accepted it and
the corrected one accepts it again. `one_more_call_of_a_shared_body_charges_one_more_body` fixes the rate rather than
the total: one more call charges exactly one more body, and a meter that charged the sharing table instead of the music
would report the same crossing point for both call counts and a difference of zero.

## Why Chapter 23's caution does not transfer

Peyton Jones (1987) Chapter 15 defines full laziness and Chapter 23 warns that it interacts badly with space behaviour:
a hoisted subexpression is retained for the lifetime of the enclosing closure, so a program that would have recomputed a
cheap value instead holds a large one alive. That warning is about *lazy* evaluation of a language with unbounded
structures. Musa's core is strict, total, and finite: a body is elaborated to a value with a known occurrence count, the
meter bounds that count before anything is allocated, and no binding outlives the piece's term. There is no thunk to
retain and no unevaluated tail to grow. The Chapter 23 hazard is therefore absent by construction rather than avoided by
care — which is worth saying out loud, because inheriting a caution one's calculus does not earn is how a design
acquires machinery it cannot justify.

What *does* transfer is §23.2.1's observation that whether the sharing is found depends on how the source was written.
That is why the sharing is a `let` in the printed term rather than an invisible memo: a performance property that turns
on a syntactic accident must be inspectable. `musa kernel` prints it, and the benchmark counts it.

## The decision

**The call-site gap is closed**, by keying on the effective pitch context instead of the call span. It is a
transformation of the term, not a cache: nothing is retained between compilations, nothing needs invalidating, and the
cache-key obligations above do not apply.

**The full-laziness gap is recorded as immaterial and left alone.** With the call-site gap closed, the residual
duplication of a parameterized body is bounded by its number of *distinct arguments* — 35 in a workload of 512 calls,
because the written pitch vocabulary is finite — rather than by the duration of the piece. Hoisting the
argument-independent tail by hand buys 18% of the term's bytes at 512 calls (94,850 against 115,606) and costs 21% more
elaboration time, because it doubles the call sites. Automatic hoisting would add a term-rewriting pass, an Origin step,
and a new way for two compilers to disagree, to recover a fraction of a term that is already an order of magnitude
smaller than it was. The number says no.

## Prompt 127 pipeline comparison

Machine: Apple M4 Pro, arm64, macOS 26.5.1, release profile, divan medians over 100 samples. The two sides were measured
sequentially on an otherwise idle machine, from the same benchmark code: the *before* column is the prompt-126 compiler
in a worktree with prompt 127's fixtures, generators, and benchmark file copied in, so the only difference between the
columns is the compiler. Allocation counts are exact per iteration; medians are not, and the table says so below where
the two disagree.

| stage | workload | before | after | Δ median | allocations before → after |
| --- | --- | ---: | ---: | ---: | ---: |
| P1 | open-shape | 581.5 µs | 531.5 µs | −8.6% | 16,321 → 14,432 |
| P1 | higher-order-shape | 262.0 µs | 259.8 µs | −0.8% | 4,584 → 4,478 |
| P1 | shared | 1.909 ms | 1.891 ms | −0.9% | 29,208 → 29,212 |
| P1 | large | 4.551 ms | 4.449 ms | −2.2% | 138,419 → 138,419 |
| P1 | declaration-heavy | 1.476 ms | 1.450 ms | −1.8% | 51,521 → 51,525 |
| P1 | audio-bridge | 133.6 µs | 133.6 µs | ±0.0% | 4,227 → 4,227 |
| P2 | open-shape | 562.9 µs | 510.6 µs | −9.3% | 16,112 → 14,223 |
| P2 | higher-order-shape | 273.5 µs | 254.6 µs | −6.9% | 4,485 → 4,379 |
| P2 | shared | 1.946 ms | 1.928 ms | −0.9% | 29,049 → 29,053 |
| P2 | large | 4.372 ms | 4.384 ms | +0.3% | 137,961 → 137,961 |
| P2 | declaration-heavy | 1.571 ms | 1.408 ms | −10.4% | 51,236 → 51,240 |
| P2 | audio-bridge | 121.2 µs | 115.4 µs | −4.8% | 3,990 → 3,990 |

`open-shape` is the only committed fixture whose allocation count moves materially, and it moves by exactly what the
term table predicts: sixteen fewer bodies, 11.6% fewer allocations, and about 9% less time in both P1 and P2. The
`audio-bridge` baseline is preserved: identical allocation counts, identical bytes, no expected-change entry.

Every other movement in that table is noise, and the allocation column is how one can tell. Medians on this machine
swing by ±10% between runs at fixed allocation counts — `declaration-heavy` P2's −10.4% comes with four *more*
allocations, and the largest raw swings in the full run (`template-pressure` P1 at −38%, `kernel-pressure` P2 at −39%)
come with allocation counts identical to within 20 parts per million and fastest-sample deltas under 5%. Prompt 93's
migration gate asks for a recorded rerun and an allocation comparison whenever P1 or P2 moves more than 10%; this is
that comparison, and it says the compiler did not change on those workloads. **No retained regression.** No workload's
allocation count rose by more than 0.01%, and none rose at all outside the ±4-allocation jitter of the resolver's own
tables.

The sharing workloads are where the change is meant to show, and they show it:

| shape | calls | before | after | Δ median | allocations before → after |
| --- | ---: | ---: | ---: | ---: | ---: |
| identical calls | 8 | 287.6 µs | 210.5 µs | −26.8% | 7,147 → 4,297 |
| identical calls | 32 | 1.046 ms | 678.7 µs | −35.1% | 25,046 → 12,426 |
| identical calls | 128 | 4.290 ms | 2.614 ms | −39.1% | 96,634 → 44,940 |
| identical calls | 512 | 18.33 ms | 10.67 ms | −41.8% | 382,975 → 174,991 |
| distinct arguments | 8 | 326.2 µs | 316.8 µs | −2.9% | 8,182 → 8,190 |
| distinct arguments | 32 | 1.187 ms | 1.135 ms | −4.4% | 28,765 → 28,797 |
| distinct arguments | 128 | 4.752 ms | 3.406 ms | −28.3% | 110,656 → 69,211 |
| distinct arguments | 512 | 20.60 ms | 12.16 ms | −41.0% | 438,147 → 225,436 |
| hand-hoisted | 8 | 380.6 µs | 284.4 µs | −25.3% | 9,775 → 6,934 |
| hand-hoisted | 32 | 1.531 ms | 979.1 µs | −36.0% | 34,726 → 22,141 |
| hand-hoisted | 128 | 6.329 ms | 3.636 ms | −42.6% | 134,089 → 76,760 |
| hand-hoisted | 512 | 27.51 ms | 14.72 ms | −46.5% | 531,468 → 294,425 |

The distinct-argument rows at 8 and 32 calls are the mechanism stated exactly: below 35 calls every argument is
different, nothing repeats, and the allocation count is unchanged to within eight allocations. Sharing starts when the
music does.

The absolute numbers matter as much as the deltas. Compiling 512 calls of a sixteen-note phrase costs 10.7 ms — a piece
with 8,192 notes written the way a composer would actually write it, well inside a keystroke's budget. Before, it cost
18.3 ms and a megabyte of term.

## Prompt 127 budgets and scaling variables

These are the language block's resource budgets. They are *reviewed* against measurement, not asserted as CI thresholds:
one machine's median is evidence about a design, and a timing test that fails on a busy laptop teaches nothing. Each
names the variable it scales in, because a budget without one is a number that stops meaning anything as soon as a piece
grows.

| budget | scales in | measured | headroom |
| --- | --- | ---: | --- |
| L1 parse (P0) | source bytes | 6.9–11.6 µs/KB across every fixture from 511 B to 16.5 KB | parsing is never the interactive cost |
| L2 compile (P1) | evaluated occurrences, then declarations | 2.8 µs per occurrence on `large`; 4.4 ms for 1,572 occurrences | a 100-bar 4-part score is 4% of B1 |
| L3 elaborate (P2) | distinct motif bodies × occurrences per body | 10.7 ms for 8,192 occurrences from one shared body | linear in occurrences, no longer in call sites |
| L4 analysis (P6) | sounding slices × chord vocabulary × surviving keys | 13–47 µs per occurrence; 8.1 ms for 448 events of four-part harmony | the one stage whose constant is not a property of the note count |
| L5 every analysis (P7) | L4, plus voice pairs for counterpoint | 9.9 ms on the same piece | opening the analysis panel is one B4 |
| L6 kernel document (K0) | occurrences in the file | 1.2 ms for a 38 KB, 1,280-occurrence document | interchange is not a slow path |
| L7 format (E0) | source bytes | 1.2 ms for 16.5 KB | format-on-save is not a wait |
| L8 recovery (E1) | source bytes, to first diagnostic | 171 µs on a 16.5 KB document with an unclosed brace | a half-typed document costs less than a valid one |
| L9 point query (E4) | references, not occurrences | 4.6 ns | hover and completion do not compile |

Two of those deserve their number spelled out. **L2 and L3 are the same measurement seen twice**: on every workload P1
minus P2 is the parse, and P2 is where the piece is built. The interactive budget that matters is B1's 120 ms, and the
large case — the fixture B1 and B2 are stated on — spends 3.7% of it.

**L4 is the stage whose cost is not read off the note count.** Per occurrence it ranges from 13 µs on `open-shape` to 47
µs on `higher-order-shape` across the committed corpus — a factor of 3.6 between two fixtures that denote 134 and 130
events. The spread is harmonic density, not duration: every slice is fitted against every chord in the vocabulary rooted
on every sounding class, and then against every degree of every surviving key, so a piece with more notes sounding at
once costs more per note. `analysis-pressure` exists to hold that constant still at 18 µs per occurrence for ordinary
four-part harmony; a vocabulary that grows shows up here and nowhere else.

The resource meter's limits are unchanged by this prompt, and one of them is now measured rather than reserved:

| metric | limit | prompt 127's evidence |
| --- | ---: | --- |
| estimated music occurrences | 1,000,000 | a 640,000-occurrence piece compiles; the rejection is deterministic, names `elaborating the piece timeline`, and reports the attempted count. Sharing does not move the threshold: one more call charges exactly one more body |

## Hot-path repair

The gate above says a P1 or P2 move over 10% requires a recorded rerun and an allocation comparison. That gate was not
being held, and the reason is worth recording before the numbers are: **the benchmark binary could not run.**
`bench::sharing_source` went on emitting a parameter default for fifty-three commits after the language removed
defaults, so `main` panicked in its own preamble and every P0–P7, K0, E0–E4 and S0–S2 measurement was unreachable. Two
workloads had drifted well past 10% in that window. A gate with no live baseline is not a gate, so the generated sharing
shapes now carry the same compile-and-denote assertion the committed pressure fixtures have had since they were written
(`the_sharing_shapes_compile_and_denote_what_they_claim`).

Four repairs followed, each measured alone: an index behind `Derivation::find` and the anchor interner, which were
linear scans and therefore quadratic in distinct anchors; a split of the expansion context into borrowed material and
the nesting that changes, which stopped every nested block from copying a contextual value and the document's named
values; a retained source string on `ParsedDocument` and a both-ends walk in `trimmed_span`, which stopped two stages
re-deriving what the parser had already been handed.

Machine: Apple M4 Pro, arm64, macOS 26.5.1, release profile, divan medians over 100 samples, both sides measured
sequentially on an otherwise idle machine from the same benchmark code. Command:

```sh
cargo bench -p musa-compiler
```

| stage | workload | before | after | Δ median | allocations before → after |
| --- | --- | ---: | ---: | ---: | ---: |
| P1 | kernel-pressure | 10.34 ms | 2.116 ms | −79.5% | 372,287 → 45,712 |
| P1 | shared | 6.027 ms | 2.604 ms | −56.8% | 35,685 → 35,139 |
| P1 | large | 5.788 ms | 4.853 ms | −16.2% | 147,686 → 135,574 |
| P1 | audio-bridge | 161.4 µs | 138.4 µs | −14.3% | 4,773 → 4,013 |
| P1 | small | 202.8 µs | 177.1 µs | −12.7% | 5,523 → 4,782 |
| P1 | declaration-heavy | 1.649 ms | 1.482 ms | −10.1% | 54,924 → 49,451 |
| P1 | template-pressure | 10.04 ms | 9.251 ms | −7.9% | 270,637 → 250,314 |
| P1 | open-shape | 632.0 µs | 586.3 µs | −7.2% | 16,062 → 14,212 |
| P1 | higher-order-shape | 355.3 µs | 366.5 µs | +3.2% | 5,593 → 5,222 |
| P1 | analysis-pressure | 3.106 ms | 3.320 ms | +6.9% | 86,803 → 81,947 |
| P1 | core-pressure | 3.518 ms | 3.801 ms | +8.0% | 105,960 → 102,699 |
| P2 | kernel-pressure | 10.10 ms | 1.897 ms | −81.2% | 369,923 → 43,346 |
| P2 | shared | 5.905 ms | 2.568 ms | −56.5% | 35,197 → 34,649 |
| P2 | audio-bridge | 117.8 µs | 93.30 µs | −20.8% | 4,043 → 3,281 |
| P2 | large | 5.100 ms | 4.168 ms | −18.3% | 139,580 → 127,466 |
| P2 | small | 148.5 µs | 126.2 µs | −15.0% | 4,685 → 3,942 |
| P2 | open-shape | 575.4 µs | 510.7 µs | −11.2% | 15,018 → 13,166 |
| P2 | declaration-heavy | 1.501 ms | 1.349 ms | −10.1% | 53,010 → 47,535 |
| P2 | template-pressure | 9.968 ms | 9.150 ms | −8.2% | 269,617 → 249,292 |
| P2 | higher-order-shape | 328.5 µs | 345.6 µs | +5.2% | 5,305 → 4,932 |
| P2 | analysis-pressure | 2.854 ms | 2.810 ms | −1.5% | 83,642 → 78,784 |
| P2 | core-pressure | 3.289 ms | 3.245 ms | −1.3% | 103,112 → 99,849 |

**Every workload's allocation count fell.** That is what makes the table readable: the three positive medians —
`higher-order-shape` in both stages and `analysis-pressure` and `core-pressure` in P1 — each come with *fewer*
allocations than before, and each is contradicted by the same workload's other stage. By this document's own rule they
are the machine, not the compiler. Run-to-run spread at a fixed binary reached 9% on the sub-millisecond workloads here,
which is why no claim in the repair commits rests on a median under 10% without an allocation column beside it.

The three rows that are not noise say what each repair bought and where:

- **kernel-pressure, −80% and 88% fewer allocations.** Both the derivation index and the context split land here, and
  the split is the larger half: this is the one workload that builds nested contextual values and kernel quotes, so it
  entered new material at every level, and every entry copied a `Music` tree and an `IndexMap` of the document's named
  values.
- **shared, −57% at an unchanged allocation count.** Purely the derivation index. Each of this workload's occurrences
  carries a `RepeatIteration` and a `MotifApplication` step, so it mints anchors that a flat score never mints, and
  three linear scans per event over a growing table is quadratic in exactly those.
- **small, audio-bridge, declaration-heavy, open-shape, −10% to −21% with allocations down by the same proportion.** The
  CST repairs. The smallest workloads gain the most, which is the signature of a per-statement fixed cost rather than
  anything that scales with the music.

`template-pressure` and `core-pressure` are the two workloads left with a five-figure allocation count and no repair
aimed at them; they are where a further measurement should start.

## Prompt 127 public surface and dependencies

No new crate, and no compiler internal became public. The sharing change is entirely inside
`crates/musa-compiler/src/elaborate.rs`. The three items the benchmarks needed — `Sharing`, `sharing_source`, and
`DISTINCT_ROOTS` — live in `musa_compiler::bench`, which is `#[doc(hidden)]`, exists only so the phases `compile` runs
together can be timed apart, and is called by nothing outside `benches/`. The module-design audit
(`bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-compiler`) reports those three items and nothing
else new.
