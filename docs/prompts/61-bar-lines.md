---
id: 61
slug: bar-lines
status: done
depends_on: [57, 58]
phase: 3
---

# Where the Barlines Fall

## Task

Measure numbering becomes a **function of the meters in force** rather than a division by one number. Today every
measure:beat conversion in the workspace divides by `MeterMap::measure_len()`, a scalar; nine functions in `plan.rs`
alone thread it as a parameter. Replace all of them with one `BarLines` value that answers `at`, `time_of`, and
`measures`, and that is built from a meter sequence even though — until prompt 64 — that sequence always has exactly
one element.

This prompt adds **no grammar, no feature, and no visible behaviour**. Every golden in the repository must stay
byte-identical. That is what makes it worth doing first: it is the only point at which this refactor can be proved
rather than argued.

## Read

- `crates/musa-compiler/src/score.rs` — `MeterMap` (a scalar named like a map) and `measure_len()`. Read what it
  actually is before designing on top of it; prompt 57 §"Irregular lengths are deferred" already wrote down why it
  blocks mid-piece meter.
- Every site that divides by it: `elaborate.rs::resolve_position` (:781), the bar-length check (:1680),
  `check_tuplets` (:2296), `resolve.rs::check_measure_sanity` (:927), `musa-project/src/facts.rs` (:290, :546), and
  `musa-render/src/plan.rs` — `plan_notation` (:516), `Fold::marks` (:660), `measure_of`/`last_measure_of` (:691,
  :700), `positioned` (:796), `plan_staff` (:820), `plan_lane` (:901), `assign_beams` (:1056).
- Prompt 58 and `plan.rs`'s `Fold` — repeats and endings **renumber measures**. This is the hazard the design section
  addresses; read `Fold::at`/`end_at` before writing anything.
- Prompt 49 and `elaborate.rs`'s `Share` — a shared body is elaborated once and referenced many times.
- `docs/course-correction.md` §4 (exact rationals) — no float enters this, at any point, for any reason.

## Design

### The one idea

`measure_of(at, measure_len)` is not a function of the piece; it is a function of *one number about the piece*, and
that number is the reason musa cannot change meter. Passing it as a parameter through nine call sites is what makes
the restriction invisible: nothing in `plan_lane`'s signature says "this piece has a single time signature", so
nothing objects. Replace the number with a value that can hold the general case, and the restriction becomes one line
in one constructor instead of an assumption spread across three crates.

```rust
// crates/musa-compiler/src/bars.rs

/// Where the barlines fall, and what each measure is numbered.
///
/// A function of the meters in force, not of a scalar: `at` and `time_of` are
/// inverses at every barline, and `measures` walks the piece in the order an
/// engraver draws it.
pub struct BarLines { /* ascending, contiguous, non-empty stretches */ }

/// A measure number and how far into it, as musicians count: 1-based, `1:1`
/// being the downbeat of the first measure.
pub struct BarBeat { pub measure: u32, pub beat: Ratio<i64> }

/// One measure: its number, its bounds, and the meter that gave it its length.
pub struct Measure { pub number: u32, pub start: MusicalTime, pub end: MusicalTime, pub meter: MeterMap }

impl BarLines {
    /// One meter for the whole piece — the only constructor until prompt 64.
    pub fn uniform(meter: MeterMap, extent: MusicalTime) -> Self;

    /// Which measure `at` falls in, and how far into it.
    pub fn at(&self, at: MusicalTime) -> BarBeat;

    /// The time of a written position, or `None` if the piece is not that long.
    pub fn time_of(&self, measure: u32, beat: Ratio<i64>) -> Option<MusicalTime>;

    /// The measure containing `at`, or the last one if `at` is past the end.
    pub fn measure_at(&self, at: MusicalTime) -> Measure;

    /// Every measure, in order.
    pub fn measures(&self) -> impl Iterator<Item = Measure> + '_;
}
```

`uniform` is the only constructor this prompt ships. A `from_changes` constructor with no caller would be exactly the
"public item for future use" the module-design rules forbid; prompt 64 adds it in the commit that needs it.

### Three things a naive version gets wrong

**Zero-length measures.** `measure_len` can be `0` — `plan.rs` guards it in five places with an early return of `1`
or `0`, and those guards are the only reason a degenerate meter does not divide by zero. Pull that guard into the
constructor: `BarLines::uniform` with a non-positive measure length produces a **single unbounded stretch**, so
everything is measure 1 and `at` never divides. The five scattered guards then delete, and the error is defined out
of existence rather than handled five times (PoSD ch. 10).

**`Fold` renumbers measures.** Prompt 58's `Fold` maps written time to sounding time so a repeat prints once and
plays twice. Notation therefore needs `BarLines` over **folded** time, and performance and `facts.rs` need it over
**unfolded** time. Two instances, constructed deliberately at the two call sites, with the reason in the module
documentation — not one instance with a flag, which would put the choice in the hands of whoever calls last.

**`Share` reuses a body.** A body elaborated once and referenced twice sits at two absolute times, so a bar-length
check that consults `BarLines` inside a shared body would get one of the two answers. Today this cannot bite,
because there is one meter. Prompt 64 makes it real; this prompt writes the invariant down in `bars.rs`'s module
documentation so 64 has something to break rather than something to discover.

### Who owns it

`BarLines` lives in `musa-compiler` and is reachable from `ScoreSnapshot`, because the meters that determine it are
the compiler's. `musa-render` and `musa-project` consume it and construct nothing except the folded instance, which
is notation's own business and belongs in `plan.rs`.

`positioned`, `measure_of` and `last_measure_of` all disappear into `BarLines::at`, which is the test of whether the
module is deep: three functions that each re-derived the same division become one call that hides the fold, the
guard, and the 1-based offset.

### The property that matters

```text
for every BarLines b and every measure m in b.measures():
    b.at(b.time_of(m.number, 1).unwrap()) == BarBeat { measure: m.number, beat: 1 }
```

as a `proptest` over generated meter sequences — generated, not fixed, because prompt 64's constructor must inherit a
test that already exercises the general case. Eight call sites depend on this being an inverse; none of them checks
it today.

## Target

- `crates/musa-compiler/src/bars.rs` (new): `BarLines`, `BarBeat`, `Measure`, `uniform`, `at`, `time_of`,
  `measure_at`, `measures`; the `Fold` and `Share` invariants in the module doc comment.
- `crates/musa-compiler/src/score.rs`: `ScoreSnapshot::bars()`. `MeterMap` and `measure_len()` stay for now — prompt
  63 is what deletes them.
- `crates/musa-compiler/src/{elaborate,resolve}.rs`, `crates/musa-project/src/facts.rs`,
  `crates/musa-render/src/plan.rs`: every `measure_len` parameter and every division removed;
  `facts.rs::position`, `plan.rs::measure_of`, `plan.rs::last_measure_of`, and `plan.rs::positioned`'s arithmetic
  deleted.
- `crates/musa-compiler/tests/bars.rs`: the inverse property, the degenerate-meter case, and a fixed-input test per
  deleted function so the replacements are compared against what they replace rather than against themselves.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm test
for f in examples/*.musa; do cargo run -q -p musa-cli -- check "$f"; done
git diff --stat -- examples/ crates/*/tests/snapshots apps/musa-desktop/ui/fixtures   # empty
grep -rn "measure_len" crates/musa-compiler crates/musa-project --include="*.rs" \
  | grep -v "score.rs\|bars.rs" | wc -l                                               # 0
cargo bench -p musa-compiler                                                          # P1-P5, no regression
```

The empty golden diff is the whole proof. A behaviour change here is a bug in this prompt, not an improvement.

## Repairs made while implementing

**`BarLines::uniform` takes no extent, because no caller had one to give.** The prompt's signature was
`uniform(meter, extent)`, and the extent had exactly one candidate use — bounding `measures()` — which the real
caller does not want: `plan_staff` computes its own per-staff span and asks for the measures covering *that*, not the
piece. So `measures()` became `measures_through(span)`, the extent parameter went, and `time_of` became total past
the end rather than returning `None` there. `resolve_position` already had its own past-the-end diagnostic with its
own wording, and moving that decision into `BarLines` would have replaced a good error with a silent `None`.

**`closing` was written off by one, and the existing suite caught it.** The deleted `last_measure_of` returned the
`ceil` index *as* the measure number, not the index plus one — so a first draft that reused `at`'s numbering made
every tuplet appear to cross a barline. Four tests failed, including `tuplet-fixture.musa` failing to compile at
all. Recorded because it is the answer to "did the goldens actually cover this": they did, and it is the reason the
prompt's decisive check is a diff rather than a review.

**`check_tuplets`'s epsilon is gone.** It compared bar indices with `(end - 1/1_000_000)` to stop a group ending
exactly on a barline from counting as crossing it. `closing` asks that question exactly, so the fudge factor
deleted rather than moved — the clearest evidence in this prompt that the missing abstraction was the *pair*
`at`/`closing`, not the division.

**Two `measure_len` sites survive on purpose, and the Check above is narrowed to say so.**

- `plan_lane` and `assign_beams` take the length of *the measure being planned*, which is now
  `Measure::length()` and is per-measure rather than per-piece. That parameter is correct in the general case; only
  its provenance changed.
- `ly.rs::measure_length` and `musicxml.rs::measure_length` derive a length from the **plan**, downstream of every
  compiler-side change, and rewriting them would have been an exporter change in a prompt that promised none. They
  are what prompt 64 has to reach when a plan stops having one measure length; noted here rather than discovered
  there.

**The non-empty invariant is structural, not defended.** `BarLines` holds `first: Stretch` and `rest: Vec<Stretch>`
rather than one `Vec`, so the two lookups have no fallback to get wrong. Clippy's `unwrap_or`-with-a-constructor
warning is what prompted the change; the warning was right for a better reason than it knew.

**The performance claim is measured against a baseline, not asserted.** `cargo bench -p musa-compiler` was run
against `HEAD` in a `git worktree` and against the change, on the same machine in the same session. Every P1–P5
median moved by under 2% and most moved down; allocation counts are identical. `BarLines::uniform` allocates
nothing — an empty `Vec` does not — and `stretch_at` iterates a list that is empty until mid-piece meter exists.

**`is_measured` replaced five scattered zero guards**, in `plan.rs` (three), `elaborate.rs`, and `resolve.rs`. The
degenerate meter is now one unbounded measure decided in one place, which is PoSD ch. 10 applied to a case the
codebase had already handled five times and could have handled inconsistently at any point.

Commit as `Make measure numbering a function of the meters in force`.

## Stop

- No grammar. `meter` remains a piece-level declaration until prompt 64.
- No `from_changes` constructor, no meter-change type, no `ContextTrack` — prompts 63 and 64.
- No mid-piece anything. If a golden moves, the refactor is wrong; do not update the golden.
- No pickup measures, no irregular bars, no `senza misura` (prompt 74). `BarLines` must be *able* to express them;
  this prompt does not let anyone write one.
- Do not merge the folded and unfolded instances "since they are equal today". They are equal today by accident.
