# 63. The barline quadratic

**Status: governs nothing.** `../../../plan/prompts/165d-the-bar-claims-prefix.md` is the prompt. This page is the
measurement that found the defect, the shape of the workload that could show it, and the numbers before and after.
Release builds on an arm64 macOS host, `musa check`, best of three runs of a timing loop.

## 1. The defect

Every `|` and every `bar { … }` raises a `fills_meter` claim, and a claim carried `before`: the whole prefix of the fold
it stood in, as one term. `Document::passage` is the only reader of that field in the workspace, and it read it by
elaborating the term and evaluating the track to take one rational off `.duration()`. So a voice of `n` bars elaborated
`1 + 2 + … + n` bars of music to answer `n` questions of the form "where does this bar begin".

Building the prefix was never the problem. `Placed` is a skew-binary counter: `place` merges equal-sized subtrees and
`built` folds its `O(log n)` stack entries into one term, so the subtrees are `Arc`-shared across every prefix the fold
hands out. Elaboration is what flattens that sharing — elaborating a term walks it whole and knows nothing about two
terms sharing an `Arc`.

## 2. The workload that could show it

The obvious workload could not. A voice of eight eighth notes a bar crosses `Budget::LANGUAGE`'s 200,000 steps at
twenty-seven bars, so timing fifty or a hundred bars of eighths times a *refusal*, not a compile — about 31 ms either
way, because the meter trips early and the wasted prefix work never happens.

The workload that could is one whole note a bar. Bars are the variable this is about; keeping the notes at one a bar
separates the two limits and reaches a hundred bars inside the same budget.

| bars | before | after | before, compile only | after, compile only |
| ---: | ---: | ---: | ---: | ---: |
| 25 | 38.7 ms | 19.0 ms | 28.7 ms | 9.0 ms |
| 50 | 125.6 ms | 32.0 ms | 115.6 ms | 22.0 ms |
| 100 | 490.3 ms | 59.9 ms | 480.3 ms | 49.9 ms |

Process start is 10 ms of every row, measured on a one-bar file; the compile-only columns subtract it. Before: 4.03×
then 4.15× the time for 2× the bars. After: 2.44× then 2.27× — linear with the counter's log factor. At a hundred bars
the compile is 9.6× faster.

Eight eighths a bar says the same thing over the range the budget leaves open, and is worth recording because it is what
real music looks like:

| bars | notes | before | after |
| ---: | ---: | ---: | ---: |
| 12 | 96 | 65.3 ms | 36.3 ms |
| 25 | 200 | 245.4 ms | 79.1 ms |
| 26 | 208 | 267.3 ms | 80.1 ms |

2.17× the bars for 4.66× the time before, 2.2× after.

## 3. The meter priced a quadratic as linear

This is the part worth keeping. The refusal boundary is identical program for program with the fix and without it —
twenty-six bars of eighths compiles in both builds, twenty-seven refuses in both at 200,001 steps of 200,000 — so the
quadratic was never charged a step. `docs/rules/language/02-core-calculus.md` §4's counters are exactly what a reader
would reach for to find a compile that takes half a second, and they would have found nothing. A deterministic budget
bounds what a program may *ask for*; it does not bound what the compiler does around the asking, and prefix measurement
was around it.

## 4. The fix

A claim carries what measures the prefix rather than the prefix: the pieces `Placed` already holds, recorded at the
moment the claim was raised. `Document::passage` sums their durations over a memo keyed by the address of each piece's
`Arc`. Each distinct piece is elaborated once, so a fold of `n` statements elaborates `O(n)` pieces whose sizes sum to
`O(n log n)` instead of `O(n²)`.

Two facts make it sound rather than lucky. `duration(follow(a, b)) = duration(a) + duration(b)` is the event track's own
equation for `sequence`, so measuring a prefix in pieces and measuring it whole are the same number by the ontology —
`a_prefix_measured_in_pieces_is_the_prefix_measured_whole` states it, and
`a_transformed_prefix_measures_the_same_in_pieces` states it again under each of the four transformation blocks, which
is the one place the design could quietly be wrong. And the memo is a cache, not an equality: an address only ever
*finds* a cached answer, so two structurally equal terms at different addresses cost a second elaboration and give the
same rational. The entry keeps its `Raw` alive, so the address cannot be freed and reused under the key.

`the_bars_of_one_voice_share_the_music_before_them` asserts the property the old fold could not have at any speed:
across forty claims, the distinct prefix pieces are strictly fewer than the pieces carried. One built term per claim
shares nothing, so a table keyed on it would have been all misses.

## 5. What this did not touch

Not the step charge, not the cost table, not `Budget::LANGUAGE`. `tests/fixtures/large-score.musa` — the fixture
`docs/rules/desktop/06-frame-budgets.md`'s B1 and B2 are measured against — still refuses on the step budget in 35 ms,
the same 35 ms before this fix and after, so it cannot time a compile at all until prompt 165 settles whether 200,000 is
the right ceiling. B2 against the real fixture waits there. This note is the wall clock alone.
