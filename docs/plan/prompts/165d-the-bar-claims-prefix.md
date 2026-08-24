---
id: 165d
slug: the-bar-claims-prefix
status: in-progress
depends_on: [165b, 165c]
phase: 3
---

# A Bar Reads Its Own Onset, Not the Music Before It

## Task

Every `|` and every `bar { … }` raises a `fills_meter` claim, and a claim carries `before` — the whole prefix of the
fold it stands in — so that `document.rs` can read one rational off it. Elaborating a hundred-bar voice therefore
elaborates the prefix a hundred times, and the wall clock is quadratic in the barlines: twenty-five bars of whole notes
compile in **39 ms**, fifty in **126 ms**, one hundred in **490 ms**. Make a claim cost its duration, which is what its
own doc comment already says it costs.

## Read

- [`crates/musa-compiler/src/lower/notation/statement.rs`](../../../crates/musa-compiler/src/lower/notation/statement.rs)'s
  `folded`, at the four lines that fill a claim's prefix:

  ```rust
  if self.claims.len() > raised {
      let before = placed.built(origin);
      for claim in self.claims.iter_mut().skip(raised) {
          let inside = claim.before.clone();
          claim.before = applied(origin, Raw::hosted(origin, "follow"), [before.clone(), inside]);
      }
  }
  ```

- [`crates/musa-compiler/src/lower/notation/raw.rs`](../../../crates/musa-compiler/src/lower/notation/raw.rs)'s
  `Placed`. It is a skew-binary counter: `place` merges equal-sized subtrees, and `built` folds the O(log N) stack
  entries into one term. So *building* a prefix is cheap and the subtrees are shared by `Arc` across every prefix the
  fold hands out. Nothing here is quadratic.
- [`crates/musa-compiler/src/document.rs`](../../../crates/musa-compiler/src/document.rs)'s `passage`, which is the only
  reader of `Claimed::before` in the workspace:

  ```rust
  let before = self.track(&claimed.before)?;
  …
  at: musa_score::MusicalTime::new(before.duration().as_ratio()),
  ```

  It elaborates and evaluates an entire prefix track — every occurrence in it — and reads `.duration()`. Its own doc
  comment says "a bar pays for its duration and nothing else", and that sentence is what this prompt makes true.
- [`crates/musa-compiler/src/lower/notation/mod.rs`](../../../crates/musa-compiler/src/lower/notation/mod.rs)'s
  `Claimed`, whose `before` field is documented as "The music standing before the passage. Its duration is where the
  passage starts" — the field already knows it is read for one number.
- `statement.rs`'s `transforming`, which maps `under` over `claim.before` as well as `claim.passage`, and its doc
  comment's reason: "`before` is read only for its duration, so a `stretch` changes where the passage begins".
- [`docs/rules/events/`](../../rules/events/README.md) on `sequence`: the duration of one track placed after another is
  the sum of the two durations. That equation is what lets a prefix be measured in pieces, and it is the only semantic
  fact this prompt relies on. `transpose`, `stretch`, `retrograde`, and `invert` — the four blocks `transforming` wraps
  — each distribute over `follow` in the sense that matters here: the duration of the transformed whole is the
  transformed sum of the parts' durations.
- The `rust-performance` skill's workflow, and the measurements below, which were taken with it.

## Design

**The measurement, so the fix is not an intuition.** Release build, arm64 macOS, `musa check` on a generated file
holding one `piece` with one `part` and one `voice`, each bar a `|` and its notes. Best of three runs of a loop; process
start is inside the number, and a one-bar file costs 10 ms of it, which the compile-only column subtracts.

One whole note per bar — the shape that separates bars from notes, since the step budget admits far more bars than it
admits notes:

| bars | wall clock | compile only |
| ---: | ---: | ---: |
| 25 | 38.7 ms | 28.7 ms |
| 50 | 125.6 ms | 115.6 ms |
| 100 | 490.3 ms | 480.3 ms |

Doubling the bars quadruples the time — 4.03× and then 4.15× — which is the definition of the defect. Eight eighth notes
per bar says the same thing across the range the step budget leaves open:

| bars | notes | wall clock | compile only |
| ---: | ---: | ---: | ---: |
| 12 | 96 | 65.3 ms | 55.2 ms |
| 25 | 200 | 245.4 ms | 235.3 ms |
| 26 | 208 | 267.3 ms | 257.2 ms |

2.17× the bars for 4.66× the time. Twenty-six bars of eighths is the ceiling: twenty-seven refuses at 200,001 steps of
`Budget::LANGUAGE`'s 200,000, which is why the bar-count evidence above is written in whole notes.

**The step meter does not see it.** The refusal boundary is the same program for program with this fix and without it —
twenty-six bars of eighths compiles and twenty-seven refuses in both builds — so the quadratic was never a step, and no
counter in `02-core-calculus.md` §4 would have found it. That is worth stating on its own, because §4's counters are
what a reader would reach for to find exactly this.

**`large-score.musa` is not the workload here.**
[`tests/fixtures/large-score.musa`](../../../tests/fixtures/large-score.musa) is the fixture
[`docs/rules/desktop/06-frame-budgets.md`](../../rules/desktop/06-frame-budgets.md)'s B1 and B2 are measured against,
and today it refuses on the step budget in 35 ms — the same 35 ms with this fix and without. It cannot time a compile
until prompt 165 settles the ceiling, so this prompt is measured on the synthetic voices above, and B2 against the real
fixture waits for that prompt.

**Where it is.** A claim's `before` is the fold's whole prefix as a term, and `Document::passage` elaborates each one
separately. Bar `k`'s prefix holds `k` bars of notes, so the total elaborated is `1 + 2 + … + N` bars for a voice of
`N`. The construction is not the problem — `Placed`'s stack shares subtrees — the *elaboration* is, because elaborating
a term walks it whole and knows nothing about two terms sharing an `Arc`.

**The fix is to stop asking for the track.** `before` is read for `duration` and nothing else, so a claim should carry
what measures the prefix rather than the prefix. Take the prefix in the pieces `Placed` already holds it in: a claim
records the stack's subtrees at the moment it was raised, and `Document::passage` sums their durations. Two things then
happen at once. Each subtree is elaborated once and cached by the identity of its `Arc`, so a fold of `N` statements
elaborates `O(N)` subtrees whose sizes sum to `O(N log N)` instead of `O(N²)`; and the claim stops depending on there
being a single term for the prefix at all, which is what made `built` get called per claim.

Sum, not concatenate: `duration(follow(a, b)) = duration(a) + duration(b)` is the event track's own equation for
`sequence`, so measuring a prefix in pieces and measuring it whole are the same number by the ontology rather than by an
implementation coincidence. State that as a law rather than trusting it.

`transforming` maps each of the four transformation blocks over a claim's terms. With `before` in pieces, it maps over
each piece. That is sound for the same equation read one step further: each of the four commutes with `follow` in the
way that decides a duration, so transforming the pieces and summing equals transforming the whole and measuring. A law
for that too, on all four blocks, because it is the one place this design could quietly be wrong.

**A memo keyed on `Arc` identity is a cache and must be one.** It answers a question about a term, the answer is a pure
function of that term, and the identity is only ever used to *find* a cached answer — never to decide that two terms are
equal. Two structurally equal terms with different pointers cost a second elaboration and give the same answer, so a
miss is slow and never wrong. Say so where the cache is, the way prompt 165c's does.

**What this prompt does not do.** It does not touch the step charge, the cost table, or `Budget::LANGUAGE`. Every
program above refuses or compiles at exactly the bar it did before, because the meter never priced the quadratic; making
the ceiling admit more music is prompt 165's, and so is deciding whether 200,000 is the right number for a corpus that
runs from `examples/in-c.musa` to `large-score`. This prompt is the wall clock alone.

## Target

- `Claimed::before` carrying the prefix in the pieces `Placed` holds it in rather than as one built term, with the
  reason written where the field is.
- `Lowering::folded` no longer calling `Placed::built` once per claim raised.
- `Document::passage` summing the pieces' durations, over a cache keyed by the identity of each piece's `Arc`, with the
  cache's soundness argued beside it — a miss is a second elaboration and never a different answer.
- `Lowering::transforming` mapping each transformation over each piece.
- A law that a prefix measured in pieces and a prefix measured whole are one duration, on a fold long enough that the
  pieces are several.
- A law that the four transformation blocks agree with that measurement — `transpose`, `stretch`, `retrograde`, `invert`
  — since the claim's onset under each is what a reader would doubt.
- A law that a voice of one hundred bars places every claim at its own bar — one whole note per bar, the shape today's
  step budget admits at that length, and the length at which the fold took 490 ms and the fix takes 60 ms. It asserts
  the onsets and not the clock, because a wall-clock assertion is a slower machine's false failure.
- A law that the claims of one voice share the pieces of their prefixes — that the distinct pieces across every claim
  are fewer than the pieces counted with multiplicity. One built term per claim shares nothing, so this is the line the
  fold could not cross, and it is what makes the cache in `passage` worth having.
- The measurement recorded as a numbered note under
  [`docs/notes/research/language-design-closure/`](../../notes/research/language-design-closure/): the table above, the
  command, the machine, the after numbers, and the sentence about the meter pricing a quadratic as linear.
- `docs/plan/code-map/` rows for whatever moved.

## Check

```sh
cargo build --workspace
cargo nextest run --workspace
cargo nextest run -p musa-compiler -p musa-project --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-compiler --bench pipeline -- p1_compile
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Let a bar read its own onset`.

## Stop

- No change to what any program means. A claim's onset is the same rational before and after; only what is elaborated to
  find it moves.
- No change to the step charge, `Budget::LANGUAGE`, or `02-core-calculus.md` §4. The quadratic was never priced by the
  meter, so removing it moves no counter — and the cost table is prompt 165's question, argued there with the whole
  corpus in front of it.
- No change to the claim vocabulary: `fills_meter` is raised where it was raised, about the passage it was about.
- No structural-equality cache. The identity of an `Arc` finds a cached answer; deciding two terms are the same term is
  a different claim and this prompt does not make it.
- No new elaborator API in `musa-calculus`. The sharing this exploits is `musa-compiler`'s own, in terms `musa-compiler`
  built, and a memo inside the trusted crate is a much larger claim than this measurement supports.
- No `#[ignore]` on a fast test, and no test that asserts a wall-clock number a slower machine would fail.
