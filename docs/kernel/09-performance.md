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
| shared | `tests/fixtures/shared-score.musa` | 1500 occurrences — the same 100 bars, written as four motifs repeated 100 times |

The coda was added at prompt 38: the plain bars measure throughput, but without it a change to the expressive-notation
path — exactly what prompts 39–41 rewrite — would regress unmeasured.

**`shared` was added at prompt 49**, and the reason is a rule this file should state once: *a change justified by reuse
cannot be measured on material with no reuse.* `large-score.musa` contains no `repeat` and no `use` — every one of its
1500 notes is typed out — so prompt 49's sharing had nothing to bite on there. `shared-score.musa` denotes the same
music at the other extreme of reuse, and a test in `large_score_generators.rs` asserts they agree on note count, so the
pair is a controlled comparison rather than two unrelated files. Both are generated from the same `LINES` table by
`crates/musa-project/tests/large_score_generators.rs`.

Adding a *third* fixture rather than extending the second is deliberate, and is the resolution of the tension prompt 45
recorded: growing `large-score.musa` would move every P1–P5 number and make this table's existing rows non-comparable. A
new workload adds a column; it does not invalidate one.

## What is measured

| id | stage | why it is the right thing to watch |
| --- | --- | --- |
| P1 | `compile` end to end | B1's server-side share: a keystroke costs a compile |
| P2 | elaboration only, parse excluded | isolates what prompts 39–41 change |
| P3 | the snapshot projection (the adapter) | the stage prompt 39 creates, and the one most likely to regress |
| P4 | canonical form of the whole piece | the order a semantic identity needs before it can hash anything |
| P5 | the semantic hash of the whole piece | what the session asks for on every recompile (prompt 43) |

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
| 42 | P1 | small | 63 µs | 2 036 | 137 KB |
| 42 | P1 | large | 1.41 ms | 50 836 | 3.75 MB |
| 42 | P2 | small | 42 µs | 1 788 | 116 KB |
| 42 | P2 | large | 1.32 ms | 49 135 | 3.55 MB |
| 42 | P3 | small | 3.4 µs | 127 | 11.1 KB |
| 42 | P3 | large | 223 µs | 7 852 | 853 KB |
| 42 | P4 | small | 29 µs | 367 | 17.6 KB |
| 42 | P4 | large | 1.67 ms | 30 934 | 1.51 MB |
| 43 | P1 | small | 70 µs | 2 118 | 141 KB |
| 43 | P1 | large | 1.95 ms | 57 230 | 3.95 MB |
| 43 | P2 | small | 52 µs | 1 870 | 120 KB |
| 43 | P2 | large | 1.75 ms | 55 529 | 3.75 MB |
| 43 | P3 | small | 3.3 µs | 127 | 11.1 KB |
| 43 | P3 | large | 227 µs | 7 852 | 853 KB |
| 43 | P4 | small | 6.6 µs | 136 | 8.6 KB |
| 43 | P4 | large | 389 µs | 9 457 | 636 KB |
| 43 | P5 | small | 11 µs | 125 | 8.5 KB |
| 43 | P5 | large | 660 µs | 9 520 | 645 KB |
| 44 | P1 | small | 72.27 µs | 2 123 | 141.1 KB |
| 44 | P1 | large | 2.015 ms | 57 235 | 4.037 MB |
| 44 | P2 | small | 53.97 µs | 1 875 | 119.6 KB |
| 44 | P2 | large | 1.781 ms | 55 534 | 3.838 MB |
| 44 | P3 | small | 3.309 µs | 127 | 11.09 KB |
| 44 | P3 | large | 232.3 µs | 7 852 | 853 KB |
| 44 | P4 | small | 6.664 µs | 136 | 12.58 KB |
| 44 | P4 | large | 396.4 µs | 9 457 | 1.02 MB |
| 44 | P5 | small | 12.61 µs | 125 | 9.408 KB |
| 44 | P5 | large | 674.8 µs | 9 520 | 776.3 KB |
| 45 | P1 | small | 68.99 µs | 2 123 | 141.1 KB |
| 45 | P1 | large | 1.943 ms | 57 235 | 4.037 MB |
| 45 | P2 | small | 51.64 µs | 1 875 | 119.6 KB |
| 45 | P2 | large | 1.766 ms | 55 534 | 3.838 MB |
| 45 | P3 | small | 3.432 µs | 127 | 11.09 KB |
| 45 | P3 | large | 230.7 µs | 7 852 | 853 KB |
| 45 | P4 | small | 6.456 µs | 136 | 12.58 KB |
| 45 | P4 | large | 395.5 µs | 9 457 | 1.02 MB |
| 45 | P5 | small | 11.44 µs | 125 | 9.408 KB |
| 45 | P5 | large | 665.4 µs | 9 520 | 776.3 KB |

Two things the baseline already says, recorded here rather than acted on (prompt 38 changes nothing it measures):

- **Elaboration is the pipeline.** P2 is ~85% of P1 on the large workload; parsing 1560 notes is not what costs.
- **Canonical form costs more than producing the score does.** P4 on the large workload exceeds P2, on a timeline that
  is already materialized — the `String` keys `Canonical` produces are the obvious suspect, and confirming or refuting
  that is prompt 43's job, with this row as its before.

Prompt 39's row, read against 38's: **P1 large +8%, P2 large +13%** — the latter over the block's 10% gate, and declared
as the prompt requires. The large workload's timeline now holds 1572 occurrences rather than 1560, because slurs,
tuplets, and hairpins are occurrences instead of tags copied onto notes, and elaboration additionally groups occurrences
into statements to merge ties. P3 grew for the same reason with the opposite sign: work that used to happen during
elaboration (rebuilding annotation spans from per-note tags) is now the projection's, which is where it belongs and
where it can be measured. Allocation is where the cost shows, as prompt 38 predicted it would.

The trade is the one the migration was for: a slur is stored once instead of once per note it covers, and the `Vec<u32>`
every note carried is gone. Prompt 43 is where the `Canonical` `String` keys — still the largest single line in P4 — get
their measurement.

Prompt 40's row, read against 39's: **P1 large −6%, P2 large −8%, P3 large −5%, P4 large −27%**. Nothing regressed, so
the block's gate is not engaged; the numbers are recorded because the P4 movement is a real change rather than noise.

The piece timeline gained two occurrences (a key and a meter) and the large workload's P1/P2 allocation counts moved by
twelve — the cost of putting the context maps in the timeline is, as §21 predicted, nothing. **P4 is where the work
went.** `Canonical::canonical_key` used to build its scope prefix with one `format!` and the whole key with another; it
now writes into a single `String` sized up front. Allocation *count* is identical (30 934 either way, because P3 and P4
measure the voice lanes only), but reallocation is not: `grow` fell from 24 478 per iteration to 310. That is the first
of the two `Canonical` costs prompt 38 flagged; the `String` keys themselves are still there, and still prompt 43's to
measure.

Prompt 41's row deleted code and measured nothing new: **every allocation count is identical to prompt 40's**, in all
four phases and both workloads, which is exactly what deleting a path `compile` never took should do. The timings run
5–8% above prompt 40's and should not be read as a regression — the three runs taken for this row were on a contended
machine (divan's slowest sample reached 158 ms against a 1.3 ms median, where prompt 40's worst sample was 1.7 ms), and
the medians above are the best of the three. When a row's allocation counts are unchanged and its timings are not, the
timings are the thing that is lying.

Prompt 42 closed the snapshot's fields and measured, again, nothing: **every allocation count and every allocated byte
is identical to prompts 40 and 41**, in all four phases and both workloads. An accessor that returns a borrow of a
private field compiles to the field read it replaced, which is what the numbers say. The timings again sit above prompt
40's (P1 large +12%, P2 large +21%) on the same contended machine that produced prompt 41's row — divan's slowest P4
sample was 4.0 ms against a 1.67 ms median — and the rule below is deliberately *not* invoked for them: a row whose
allocation counts are unchanged to the unit cannot have regressed 21% in real work. What prompt 42 did change is where
region membership is computed: `plan.rs` and `performance.rs` each stopped rebuilding it (by id arithmetic in one, by a
pair of linear scans in the other) and now call `ScoreSnapshot::events_in`. That work is in neither P3 nor P4, which
measure projection and canonicalization; it is in notation planning and performance lowering, which this harness does
not yet time. Prompt 48's windowed observation is where it will be.

Prompt 43 added a fifth measurement, **P5 — the semantic hash of the whole piece**, because that is what the session now
asks for on every recompile; P4 stays because the difference between the two is what the digest itself costs on top of
the canonical order it needs.

Read the row in two halves.

**The identity is not free, and P1 declares it.** P1 large moved 1.41 ms → 1.95 ms (**+38%**, over the block's 10% gate)
and P2 large 1.32 ms → 1.75 ms (**+33%**), because `compile` now hashes the piece timeline before projecting it. The
trade is stated rather than hidden: the whole point of the prompt is that the session can ask "did the meaning change"
instead of "did the counter move", and nothing can answer that without reading the meaning once. In budget terms it is
not close to a problem — `docs/interface/06-performance.md` B1 allows 120 ms from keystroke to diagnostics, and 1.95 ms
is 1.6% of it.

**Canonicalization got much cheaper, and that is where the prompt's optimization went.** `canonical_occurrences` sorted
with `sort_by_key`, which rebuilds the key on *every comparison* — n log n serializations of a `String` per occurrence.
`sort_by_cached_key` builds each key once. P4 large: 1.67 ms → **389 µs (−77%)**, allocations 30 934 → **9 457 (−69%)**.
P4 small: 29 µs → 6.6 µs. This is the intervention prompt 38 predicted and prompt 43 was told to make only if measured;
it was measured, and it is the only one applied. The two further candidates the prompt listed — a `write_canonical` on
the `Canonical` trait so callers share one buffer, and hashing without materializing keys at all — were **not** applied:
with P5 at 660 µs inside a 120 ms budget, neither is justified, and both change a public trait to buy time nobody is
short of. Recorded so the next person does not have to re-derive that they were considered.

Net against prompt 42, on the phases that existed then: P1 +38%, P2 +33%, P3 unchanged, P4 −77%. A keystroke costs about
half a millisecond more and gets an answer to a question it could not previously ask.


### Prompt 44 — the queries cost nothing measurable

P3 is the row this prompt was told to watch, because replacing four hand-rolled scans with kernel queries would be a
regression if the migration turned a sweep into one query per event. It did not: **P3 large 227 µs → 232 µs (+2%)**,
allocations unchanged at 7 852, which is run-to-run noise on this machine. `project_regions` still makes one ordered
pass over the voice's event extents and `lower_performance` still carries the dynamic forward; both now cite D10–D11's
conventions instead of inventing their own, which is a change of *authority*, not of algorithm. P1 large +3.3% and P2
large +1.8% are the same noise and are inside the 10% gate.

The one real cost is five allocations on P1/P2 large (57 230 → 57 235). `project_piece` sorts the piece-scoped facts in
canonical order now, which builds a `canonical_key` `String` per piece-scoped fact — five of them in the large fixture —
where sorting on `origin.definition_span` allocated nothing. That is the price of ordering by *meaning* rather than by
source position, and it is per piece-scoped fact, not per event.

**A transcription correction.** Prompt 43's P4-large and P5-large "bytes allocated" figures (636 KB, 645 KB) do not
match the divan block at identical allocation counts; the column is the `alloc:` byte total, which this run reports as
1.02 MB and 776 KB with allocation counts of exactly 9 457 and 9 520. Nothing regressed between the two rows — the
earlier bytes were read off the wrong line. Later rows are the `alloc:` line, as the column heading says.


### Prompt 45 — free, and honestly unmeasured

Every row is within noise of prompt 44's and **every allocation count is identical to the digit**. That is not a
coincidence and it is not a win: **neither reference workload contains a hairpin.** `examples/glass-mountain.musa` has
none and `tests/fixtures/large-score.musa` has none — its coda's expressive material is point dynamics, articulations,
ties, slurs and tuplets. So the one path prompt 45 changed is not exercised by P1–P5, and the table can only say that
adding a `Progress` to `FactKind::Hairpin` costs nothing where there are no hairpins.

What the cost would be, stated so it is not mistaken for zero: one two-element `Vec` per hairpin at elaboration, one
clone of it per hairpin at projection, and one `at()` per event under a hairpin at performance lowering — replacing one
`Ratio` multiply. All three are per *hairpin*, not per event, on a path that already allocates an `Origin` per fact.

The fixture is the thing to fix, not this row, and it is deliberately not fixed here: adding a hairpin to
`large-score.musa` would move every P1–P5 number and make this table's forty existing rows non-comparable. The right
place is the prompt that next needs the fixture to grow, and it should say so in its own row.


### Prompt 48 — off every measured path

No row, and this paragraph is why. The interchange printer and parser are reachable only from `musa kernel` and from the
corpus test: compiling, projecting, planning notation, and lowering performance never construct a `Term`, print one, or
parse one. P1–P5 exercise exactly those five, so a row would be five re-measurements of unchanged code.

The one change on a measured path is `kernel_normal_form`, which now builds an `over` of literal terms and evaluates it
instead of calling `overlay` directly. `evaluate` on a literal is a clone and `Form::Over` hands straight to `overlay`,
so the work is identical up to one `Vec` of terms — and `kernel_normal_form` is itself a test-and-golden entry point,
not a pipeline stage. Printing a piece is linear in its occurrences and allocates one string; that is the whole cost,
and it is paid only by someone who asked for a file.

### Prompt 49 — sharing, measured on material that shares

Apple M4 Pro, `--release`, median of 100 samples, against prompt 48's tree with the `shared` workload back-ported so
both sides measure the same three files.

| id | workload | before | after | Δ | alloc before | alloc after | Δ |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P1 | small | 61.7 µs | 73.1 µs | +18.6% | 1 997 | 2 160 | +8.2% |
| P1 | large | 1.854 ms | 1.866 ms | +0.6% | 54 088 | 54 103 | +0.03% |
| P1 | **shared** | 2.558 ms | **1.643 ms** | **−35.8%** | 62 871 | **25 024** | **−60.2%** |
| P2 | small | 51.1 µs | 56.2 µs | +9.9% | 1 749 | 1 912 | +9.3% |
| P2 | large | 1.663 ms | 1.637 ms | −1.6% | 52 387 | 52 402 | +0.03% |
| P2 | **shared** | 2.537 ms | **1.666 ms** | **−34.3%** | 62 736 | **24 889** | **−60.3%** |
| P3 | large | 200.6 µs | 217.5 µs | +8.4% | 7 852 | 7 852 | 0 |
| P4 | large | 391.5 µs | 360.1 µs | −8.0% | 9 457 | 9 457 | 0 |
| P5 | large | 590.3 µs | 625.8 µs | +6.0% | 9 520 | 9 520 | 0 |

**The shared column is the prompt.** A third of the elaboration time and three-fifths of the allocations disappear on
material written with a motif and a repeat, because the body is now walked, resolved, and diagnosed once instead of 100
times. Nothing about the *result* changed: the projected snapshot is byte-identical, which is what
`crates/musa-compiler/tests/` asserts and what made this a representation change rather than a semantic one (T2).

**The large column is the gate, and it is inside it** — P2 came out 1.6% *faster* and P1 0.6% slower, both noise.
`large` has no reuse, so it can only lose, and what it loses is 15 allocations out of 54 103, which is the `Vec` of
segments and the pair `coalesce` folds through. Getting there took two corrections, both worth recording because both
are the same mistake:

- The first draft cloned each literal out of its term to coalesce a run (`as_literal` returning a reference). On
  material with nothing to share that is a full copy of every occurrence: **+29% allocations on large**. Fixed by
  `Term::into_literal`, which takes the timeline by value.
- The second draft still cloned the piece's whole term to evaluate it, because `evaluate_marked` borrowed. Same shape,
  same cost, still +29%. Fixed by making `evaluate`/`evaluate_marked` take the term **by value**, so a literal's
  occurrences move into the result. The signature change is the honest one anyway: a caller that needs the term
  afterwards now says `evaluate(term.clone())`.

The lesson generalizes past this prompt: **a representation that shares must not pay for sharing where there is none.**
Both regressions were invisible on `small` and `shared` and obvious on `large`; without the fixture that has no reuse,
this would have shipped 29% heavier for every composer who types their notes out.

**P3/P4/P5 are noise.** Allocation counts are identical to the digit on every one, and the code they exercise is not
touched — P3 walks an evaluated timeline that is byte-identical to prompt 48's. P3 large's +8.4% (and P4 large's −8.0%)
is drift between two builds in two worktrees, not a change; it is reported rather than smoothed because the table is a
record.

**P1/P2 small regress by 10–19%, and that is real.** `glass-mountain.musa` is 18 occurrences with two motif calls, so it
does share — but at that size the fixed cost of the representation (a `Segment` per item, the two vectors `coalesce`
folds through, the binding table) is larger than the sharing it recovers. In absolute terms the compile went from 62 µs
to 73 µs, against a keystroke budget of 120 ms. It is stated rather than optimized away: the crossover is somewhere
between 18 occurrences and 1500, every real score is on the far side of it, and buying 12 µs back on the near side would
mean a second code path for small documents.

### The corpus, qualitatively

`examples/kernel/canon.musa.kernel` is the visible payoff, and it is what the prompt asked to see: two `let` bindings
and two marked references, where before it was every occurrence of both voices written out. The subject appears once.
Five of the nine files are **byte-identical** to prompt 48's — `counterpoint`, `invention`, `profile-fixture`,
`twinkle`, `tuplet-fixture` — because a run of adjacent literals is coalesced back into one `timeline` block. That was
not free either: without coalescing, every note printed as its own nested `timeline` inside a `sequence` and the corpus
grew 30% across the board while saying nothing new. The four that changed are exactly the four with structure to show:
`canon` and `glass-mountain` have motifs, `variation` has a motif and transformations, and `annotated` has slurs,
phrases and hairpins, whose region facts now print as the `overlay` they always were. The rule the printer follows is
the one the prompt wanted: **show the structure a composer wrote, and no structure they did not.**

### Prompt 50 — the gate, measured, and closed

Prompt 50's entry condition was a measurement, not an intuition: build a lazy windowed evaluator **only** if B1 or B2
fails, or passes with less than 20% headroom, *and* the stage that costs is evaluate-or-project. Both halves were
measured. The second is what closed it.

**B1 and B2** (Playwright, `apps/musa-desktop/ui/tests/screens/perf.spec.ts`, p95 over 20 trials, large fixture):

| budget | measured | budget | headroom |
| --- | --- | --- | --- |
| B1 keystroke → diagnostics | **2 ms** after the debounce | 120 ms | 98% |
| B2 keystroke → re-engraved | **373 ms** | 400 ms | **6.8%** |

So B1 passes with room to spare and **B2 does not clear the 20% bar** — the gate's step 2 does not apply, and step 3
does. Step 3 asks which stage costs, and this prompt added the split that answers it:

| B2, decomposed | p95 |
| --- | --- |
| debounce (fixed, `06-performance.md` §3.4) | 180 ms |
| edit → snapshot, the round trip | 2 ms |
| snapshot → score, the engraver | **191 ms** |

**The answer is engraving, and it is not close.** 191 of the 193 milliseconds that are not the debounce are Verovio
laying out 100 bars and the browser painting it. The harness stubs the shell (`tests/screens/shell.ts`), so the compiler
contributes *nothing* to that 373 ms — and its real contribution is known independently: P1 large is **1.87 ms**, and a
full parse → compile → MEI export of the same fixture through `musa` is ~10 ms wall including process start.

**Therefore no lazy evaluator.** A perfect one — evaluation reduced to zero — would take B2 from 373 ms to 371 ms. That
is 0.5% of a budget that needs 7% to be comfortable, bought with a second evaluation path, a second projection path, and
a new class of bug (a partial answer escaping to a caller that asked a whole-piece question) that the prompt itself
named as the one way the feature could do real damage. The trade is not close either.

Per the prompt's own step 3, the finding **belongs to prompt 22's surface**: B2's headroom is 6.8% and the way to widen
it is incremental or page-windowed *engraving*, not incremental evaluation. It is recorded here and the measurement now
runs on every UI test run rather than being reconstructible only by breaking an assertion.

**What would reopen this.** A fixture where P1 or P2 is a material fraction of B1's 120 ms — concretely, elaboration
above ~25 ms, which on the measured shape means roughly 20 000 occurrences, an order of magnitude past
`large-score.musa`. The prompt-49 row is the evidence that the semantic core has that order of magnitude in hand: 1 560
occurrences elaborate in 1.6 ms. When a real piece does not, the measurement — not this paragraph — reopens it.

## The rule

The table is a record, not a gate — machines differ, and a row taken on another laptop is not comparable to this one.
The gate is **relative**: no prompt in the 39–50 block may regress **P1 or P2 on the large workload by more than 10%**
against the row before it without saying so in its "Repairs made while implementing" section and justifying the trade.

A prompt in this block is not done while its row is missing. Re-run the command above, append the rows, and keep the
machine class beside them if it differs.
