# The semantic pipeline's measured baseline

The prompt 39–43 migration moves every notated fact into one timeline per piece and turns `ScoreSnapshot` into a
projection of it. That is the largest change the semantic core will take, and the risk it carries is *allocation and
hashing*, not arithmetic. This file is the instrument: a baseline taken before the migration starts, and one row per
prompt after it, so a regression is a fact rather than an impression.

## Harness

`divan` (roadmap §15.10), because it runs under a plain `cargo bench` with no separate driver to install and reports
per-iteration allocation counts beside the timings. Benchmarks live in `crates/musa-compiler/benches/pipeline.rs`; the
phase seams they need are `#[doc(hidden)] musa_compiler::bench`, which exists for measurement only and is not part of
the compiler's interface.

```sh
cargo bench -p musa-compiler
```

## Workloads

The two reference workloads of `docs/interface/06-performance.md` §1:

| id | file | size |
| --- | --- | --- |
| small | `examples/glass-mountain.musa` | 18 occurrences |
| large | `tests/fixtures/large-score.musa` | 1560 occurrences — 100 bars × 4 parts, plus a coda of ties, slurs, tuplets, dynamics, and articulations |

The coda was added at prompt 38: the plain bars measure throughput, but without it a change to the expressive-notation
path — exactly what prompts 39–41 rewrite — would regress unmeasured.

## What is measured

| id | stage | why it is the right thing to watch |
| --- | --- | --- |
| P1 | `compile` end to end | B1's server-side share: a keystroke costs a compile |
| P2 | elaboration only, parse excluded | isolates what prompts 39–41 change |
| P3 | the snapshot projection (the adapter) | the stage prompt 39 creates, and the one most likely to regress |
| P4 | canonical form of the whole piece | what prompt 43's semantic identity pays on every edit |

## Baseline

Machine class: Apple M4 Pro laptop, macOS 26.5, `cargo bench` (release). Time is divan's **median** over 100 samples;
`docs/interface/06-performance.md` states its budgets as p95 over 20 trials, which is the *end-to-end* harness's
statistic — divan reports median, mean, fastest and slowest, and the median is what this table carries. Allocations are
per iteration and are exact rather than sampled.

| prompt | id | workload | median | allocations | bytes allocated |
| --- | --- | --- | --- | --- | --- |
| 38 | P1 | small | 61 µs | 2 015 | 142 KB |
| 38 | P1 | large | 1.23 ms | 46 117 | 3.33 MB |
| 38 | P2 | small | 41 µs | 1 767 | 120 KB |
| 38 | P2 | large | 1.05 ms | 44 416 | 3.14 MB |
| 38 | P3 | small | 2.0 µs | 72 | 11.5 KB |
| 38 | P3 | large | 150 µs | 4 755 | 700 KB |
| 38 | P4 | small | 37 µs | 367 | 16.5 KB |
| 38 | P4 | large | 1.70 ms | 21 993 | 1.26 MB |
| 39 | P1 | small | 57 µs | 2 024 | 133 KB |
| 39 | P1 | large | 1.33 ms | 50 824 | 3.45 MB |
| 39 | P2 | small | 41 µs | 1 776 | 112 KB |
| 39 | P2 | large | 1.19 ms | 49 123 | 3.25 MB |
| 39 | P3 | small | 3.3 µs | 127 | 11.1 KB |
| 39 | P3 | large | 220 µs | 7 852 | 853 KB |
| 39 | P4 | small | 38 µs | 367 | 13.1 KB |
| 39 | P4 | large | 2.13 ms | 30 934 | 1.09 MB |
| 40 | P1 | small | 55 µs | 2 036 | 137 KB |
| 40 | P1 | large | 1.26 ms | 50 836 | 3.75 MB |
| 40 | P2 | small | 39 µs | 1 788 | 116 KB |
| 40 | P2 | large | 1.09 ms | 49 135 | 3.55 MB |
| 40 | P3 | small | 3.1 µs | 127 | 11.1 KB |
| 40 | P3 | large | 208 µs | 7 852 | 853 KB |
| 40 | P4 | small | 29 µs | 367 | 17.6 KB |
| 40 | P4 | large | 1.56 ms | 30 934 | 1.51 MB |
| 41 | P1 | small | 68 µs | 2 036 | 137 KB |
| 41 | P1 | large | 1.33 ms | 50 836 | 3.75 MB |
| 41 | P2 | small | 43 µs | 1 788 | 116 KB |
| 41 | P2 | large | 1.15 ms | 49 135 | 3.55 MB |
| 41 | P3 | small | 3.4 µs | 127 | 11.1 KB |
| 41 | P3 | large | 224 µs | 7 852 | 853 KB |
| 41 | P4 | small | 30 µs | 367 | 17.6 KB |
| 41 | P4 | large | 1.72 ms | 30 934 | 1.51 MB |

Two things the baseline already says, recorded here rather than acted on (prompt 38 changes nothing it measures):

- **Elaboration is the pipeline.** P2 is ~85% of P1 on the large workload; parsing 1560 notes is not what costs.
- **Canonical form costs more than producing the score does.** P4 on the large workload exceeds P2, on a timeline that
  is already materialized — the `String` keys `Canonical` produces are the obvious suspect, and confirming or refuting
  that is prompt 43's job, with this row as its before.

Prompt 39's row, read against 38's: **P1 large +8%, P2 large +13%** — the latter over the block's 10% gate, and
declared as the prompt requires. The large workload's timeline now holds 1572 occurrences rather than 1560, because
slurs, tuplets, and hairpins are occurrences instead of tags copied onto notes, and elaboration additionally groups
occurrences into statements to merge ties. P3 grew for the same reason with the opposite sign: work that used to happen
during elaboration (rebuilding annotation spans from per-note tags) is now the projection's, which is where it belongs
and where it can be measured. Allocation is where the cost shows, as prompt 38 predicted it would.

The trade is the one the migration was for: a slur is stored once instead of once per note it covers, and the
`Vec<u32>` every note carried is gone. Prompt 43 is where the `Canonical` `String` keys — still the largest single
line in P4 — get their measurement.

Prompt 40's row, read against 39's: **P1 large −6%, P2 large −8%, P3 large −5%, P4 large −27%**. Nothing regressed, so
the block's gate is not engaged; the numbers are recorded because the P4 movement is a real change rather than noise.

The piece timeline gained two occurrences (a key and a meter) and the large workload's P1/P2 allocation counts moved by
twelve — the cost of putting the context maps in the timeline is, as §21 predicted, nothing. **P4 is where the work
went.** `Canonical::canonical_key` used to build its scope prefix with one `format!` and the whole key with another;
it now writes into a single `String` sized up front. Allocation *count* is identical (30 934 either way, because P3 and
P4 measure the voice lanes only), but reallocation is not: `grow` fell from 24 478 per iteration to 310. That is the
first of the two `Canonical` costs prompt 38 flagged; the `String` keys themselves are still there, and still prompt
43's to measure.

Prompt 41's row deleted code and measured nothing new: **every allocation count is identical to prompt 40's**, in all
four phases and both workloads, which is exactly what deleting a path `compile` never took should do. The timings run
5–8% above prompt 40's and should not be read as a regression — the three runs taken for this row were on a contended
machine (divan's slowest sample reached 158 ms against a 1.3 ms median, where prompt 40's worst sample was 1.7 ms), and
the medians above are the best of the three. When a row's allocation counts are unchanged and its timings are not, the
timings are the thing that is lying.

## The rule

The table is a record, not a gate — machines differ, and a row taken on another laptop is not comparable to this one.
The gate is **relative**: no prompt in the 39–48 block may regress **P1 or P2 on the large workload by more than 10%**
against the row before it without saying so in its "Repairs made while implementing" section and justifying the trade.

A prompt in this block is not done while its row is missing. Re-run the command above, append the rows, and keep the
machine class beside them if it differs.
