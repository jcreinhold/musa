---
id: 144
slug: diagnostics-and-performance
status: pending
depends_on: [143]
phase: 3
---

# Make the New Failures Legible and the New Checker Fast Enough

## Task

A dependent checker fails in ways Musa has never failed before — a conversion mismatch between two normal forms, an
incomplete match, a recursion the termination checker cannot see, an unsolved metavariable, an ambiguous instance — and
it does work Musa has never done before, because conversion evaluates. Bring the diagnostics up to the standard the rest
of the compiler holds, and bring P1 and P2 back inside `06-performance.md`'s 10% gate.

And make one more failure legible, because today it is not a failure at all: the checker exhausts the host stack and
aborts the process before any budget refuses it. `02-core-calculus.md` §4.1 already forbids that outcome and already
says what the implementation owes instead. This prompt discharges it.

## Read

- `docs/rules/language/06-performance.md` §"Measurements and honest seams", the P1–P5 table, the recorded baselines, and
  §"Gates and budgets" — the 10% relative gate, and the end-to-end authority in `docs/rules/desktop/06-performance.md`:
  B1 is ≤120 ms after debounce and B2 is ≤400 ms with the previous engraving visible. Those two are what a musician
  actually experiences, so they are the numbers that decide whether this prompt is done.
- `crates/musa-compiler/src/diagnose.rs` and `crates/musa/src/main.rs`'s `cmd_explain` — the existing diagnostic
  vocabulary and the rule that every code has an explainable rule behind it.
- Every `Code` variant added by prompts 134, 135, 136a, 137, 137a, 139, and 140, and the message each currently
  produces. Those were written to be correct; this prompt makes them good.
- The `rust-performance` skill's workflow in full — name the workload, reuse the nearest credible bench, find what
  actually costs, make the smallest matching change, re-measure, and report the numbers with the command.
- `crates/musa-compiler/src/bench.rs` and `core_budget.rs` — where measurement already happens and where the budget is
  charged.
- `docs/rules/language/02-core-calculus.md` §4.1 in full, **both halves**: the nesting metric, and the non-normative
  obligation that the compiler run checking and evaluation with at least `nesting limit × frame ceiling` bytes of stack.
  The first half is implemented and does not bound what it claims to; the second is not implemented on the new path at
  all. §4.1 also fixes what may move without a version bump — shrinking the frame ceiling is free, raising the limit is
  a cost-table version bump — which is the constraint every option below is scored against.
- `crates/musa-compiler/src/core.rs`'s `with_room` and `core_budget.rs`'s `FRAME_CEILING` — the *old* evaluator's
  discharge of that obligation: a scoped thread of `NESTING × FRAME_CEILING`, derived rather than picked, with the wasm
  fallback beside it. `musa-core` has no equivalent, and the shape of the answer is probably this one moved.
- `crates/musa-core/src/budget.rs` (`Budget::NESTING`, `Meter::nested`) and the frame-splitting note above `eval.rs`'s
  `fn pi` — the ~2 KiB per level that note records, and, more to the point, **what it is a measurement of**: `eval`
  recursing into itself. The chain a real program drives is `infer → check → eval → apply → infer`, and it costs five
  times that.
- `crates/musa-compiler/src/expand.rs`'s `nested_region` helper and the two laws beside it,
  `a_region_deeper_than_the_budget_allows_is_refused_rather_than_fatal` and
  `a_region_nested_deeper_than_anyone_writes_still_expands` — the second's doc comment records a measurement taken
  during 142 and explicitly declines to decide the number, because the number is this prompt's. Read it against
  `Budget::NESTING`'s own doc claim, "past anything a person writes and short of anything a host cannot hold", which was
  written about an evaluator that no longer exists.
- `crates/musa-core/src/elab.rs`'s `check` and `infer` — mutually recursive over the raw term, and charged nothing for
  nesting. `zonk` is the only thing in that file that calls `Meter::nested`.
- `crates/musa-compiler/src/lower/notation.rs`'s `notated`, at the line that writes
  `built = applied(origin, follow, [built, next])` — the voice fold's left-nested `follow` spine. One statement, one
  level of function position, one frame of elaboration; the depth of a voice's term is the number of notes in it.
- `apps/musa-desktop/src-tauri/src/session.rs`'s `spawn` — the session thread, created with a name and no `stack_size`,
  which is Rust's 2 MiB default. That thread is the smallest host the compiler runs on, and `cargo nextest`'s test
  threads are the same size.
- `docs/rules/desktop/` on states and voice: a diagnostic is interface text, and the desktop specification already says
  what Musa sounds like when it refuses.
- [`docs/notes/research/language-design-closure/44-audit-against-smalltt-and-peyton-jones.md`](../../notes/research/language-design-closure/44-audit-against-smalltt-and-peyton-jones.md)
  in full, and its closing section. Prompt 136b removed the audit's ad hoc divergences and left this prompt six items it
  could not decide without real code going through the checker. They are named in **Design** below and each is cited
  there; do not rediscover them by reading the code.
- `~/Code/smalltt`'s README on glued evaluation, approximate conversion, the three quotation modes, and approximate
  occurs checking — the techniques the items below are, stated by the implementation this core was audited against.
- Peyton Jones ch. 5 §5.4.1 and ch. 6's let-bound right-hand side, for Finding A, plus
  [`crates/musa-core/src/case.rs`](../../../crates/musa-core/src/case.rs) and the law
  `a_pattern_binder_is_a_definition_at_every_leaf_it_reaches` in `coverage_laws.rs`, which is the program that falsified
  the hoisting condition 136b was going to ship.

## Design

**The conversion error is the one that decides whether this language is usable.** Two normal forms printed in full is a
correct message nobody can read. The policy prompt 134 stated — normalize far enough to be honest, unfold no further
than necessary, show the surface node each side came from — gets implemented properly here: find the first point of
disagreement, print the two sides around it, name the definition that was unfolded to get there, and offer the fully
unfolded forms behind `musa explain` rather than in the message. Test it on a deliberately deep mismatch, because a
message that is fine on a two-line example and useless on a real one has not been tested.

**Coverage and termination failures should name the program, not the theory.** An incomplete match names the missing
constructor pattern and offers it. A termination failure names the recursive call and the argument that did not
decrease, and — since there is no `partial` to suggest — says what a measure would look like for that shape. A
non-positive occurrence names the constructor and the position. None of these should require the author to know what a
recursor is.

**An unsolved metavariable is a question, not a failure.** It says what could not be determined, where the choice was
left open, and what annotation would settle it. This is the message that will appear most often while people learn the
new language, so it is worth more care than its frequency in the test suite suggests.

**Ambiguity names both candidates.** Prompt 137a made ambiguity an error rather than a default; the error is only better
than a default if it says what the two possibilities were and how to pick one.

**Then measure, in that order.** Diagnostics first because a fast compiler with unreadable errors is worse than a slow
one with good errors, and because improving messages sometimes changes what code runs. The workloads are
`06-performance.md`'s existing four — open-shape, higher-order-shape, declaration-heavy, audio-bridge — so the numbers
are comparable to the recorded baseline rather than to a benchmark invented to make this prompt look good.

**Where a dependent checker actually costs, and where it does not.** Conversion evaluates, so the suspects are
normalization during checking, metavariable solving, and instance resolution — not parsing and not term construction.
Profile before changing anything. The likely interventions, in order of how much they cost to get wrong: caching normal
forms of definitions, resolving known-concrete instances at elaboration time (prompt 143 may already have done this),
and avoiding re-normalization when checking against a type already in normal form. Each is measured independently, and a
change that does not move the number is reverted rather than kept because it seemed principled.

**The nesting budget does not bound the stack, and the room obligation is unhonoured on the new path.** §4.1 says a
limit is only a refusal if the machine survives long enough to print it. It does not. Elaborating `examples/*.musa`
through `musa-core` on `cargo nextest`'s 2 MiB test thread aborts with `has overflowed its stack`; under
`RUST_MIN_STACK=268435456` the same run completes and reports `nested evaluation levels at 257 of 256` and
`reduction steps at 200001 of 200000`, so the budgets do fire — long after the process would already be dead. Three
things are wrong and they are not the same thing, which is why the obvious single fix is the wrong one.

_First, the metric is charged in the wrong places._ §4.1 says the limit is charged "wherever an evaluation can stand
inside another one". `Elaborator::check` and `Elaborator::infer` stand inside one another all day and are charged
nothing. Measured on a single voice of 256 notes with no library in scope: at the stack low-water mark, `elab::infer`
stands 516 frames deep and `elab::check` 258, while the nesting counter — which sees only `eval`, `quote`, and `unify` —
reads **2**. The counter is not a loose bound on what the machine spends stack on; on this path it is uncorrelated with
it.

_Second, the depth is linear in the music, not in anything an author nested._ The fold appends left, so a voice of `N`
statements is `follow(follow(…, n₁), n₂)` and its elaboration descends `N` function positions. Measured, one voice, no
library, arm64:

| notes in one voice | native stack, debug | native stack, release | `eval` | `elab::check` | `elab::infer` |
| --- | --- | --- | --- | --- | --- |
| 16 | 401 KiB | 76 KiB | 40 | 19 | 38 |
| 32 | 733 KiB | 136 KiB | 72 | 35 | 70 |
| 64 | 1,398 KiB | 256 KiB | 136 | 67 | 134 |
| 128 | 2,720 KiB | 494 KiB | 188 | 131 | 262 |
| 256 | 5,380 KiB | 974 KiB | 188 | 259 | 518 |

That is 20.7 KiB per note in a debug build and 3.7 KiB in a release one, straight-line across the range. A 2 MiB thread
is exhausted by a voice of about 96 notes in debug and about 540 in release — a phrase, not a pathology. (`eval` stops
climbing at 188 because the step budget fires there; the stack it already spent is not given back by that refusal.) The
real corpus is past the cliff already: `examples/in-c.musa` reaches 3,505 KiB with `elab::check` 388 deep.

_Third, nothing arranges the room._ `musa-compiler` honours §4.1 for the *old* evaluator and only there: `core.rs`'s
`with_room` runs a transformer on a scoped thread of `NESTING × FRAME_CEILING`. Every `musa-core` entry point — `check`,
`infer`, `normalize`, `convertible`, `declare` — runs on whatever stack the caller happened to have.

> **Step 1 landed early, out of stack order.** Prompt 141s found the same defect from the other end — elaborating a
> hand-built `Nat` tower of 220 aborted at about 215 levels while `Budget::NESTING` promised a refusal at 256 — and the
> room was arranged then rather than left waiting for this prompt, because §4 does not admit an abort and the fix
> touches no acceptance. `crates/musa-core/src/room.rs` is `with_room` at the `musa-core` seam, over `check`, `infer`,
> `normalize`, `normalize_type`, `convertible`, `convertible_types`, `declare`, `declare_program`, `declare_trait`,
> `declare_impl`, `well_typed`, and `Cx::assume`/`define`, with `FRAME_CEILING` at 32 KiB against a measured 10 KiB a
> level for the `infer → check → eval → quote` chain in a debug build on arm64 and under 3 KiB in a release one. The law
> is `budget_laws.rs`'s `elaborating_a_term_nested_past_the_limit_is_refused`, stated on a 2 MiB thread.
>
> **What that leaves this prompt** is steps 2 and 3 below, undiminished, and one measurement it should not have to
> rediscover: the room bounds the stack only where the charge tracks the descent. The elaborator reaches the bottom of a
> raw term before any value on the way back up is charged, so the counter hits the limit `NESTING` levels from the
> *bottom* rather than from the top, and a term far enough past the limit still aborts — measured on a raw `let` chain
> in a debug build, 756 levels are refused and 1,256 abort. No ceiling repairs that; only the charge does.

_Fourth, the same defect seen from the adapter's side: the headroom for nested regions shrank fourfold and nobody
decided that._ §4.1's metric was written for §5.9's recursor, which descends *through* the transformer's own branches,
so one level of a region's nesting costs a chain of frames rather than one. Under the replaced Algorithm-W evaluator
that chain was about four levels long and the first refusal came at 64 groups deep. Under normalization by evaluation it
is about fourteen — `quote` walks a value the way `eval` walks a term — and the first refusal is at **19**: measured
during 142 by bisection against `expand.rs`'s `nested_region` helper and the `EACH_ONCE` recursor, where depth 18
expands and depth 19 stops. `a_region_nested_deeper_than_anyone_writes_still_expands` was re-calibrated from 48 to 16 in
that prompt, with a comment saying exactly this and deferring the number here, because a test that asserts a limit it
did not choose is a test pinned to whatever the compiler happened to do. Two things follow. `Budget::NESTING`'s doc
claim is now unsupported for the adapter path in the direction that matters — 19 groups is not "past anything a person
writes" by the margin 64 was. And the 19 is measured **before** step 3: any charge added to `check` and `infer` charges
an adapter's region levels too, so 19 is a ceiling on what a region may be after this prompt rather than a floor.

**What follows, and what does not.** Lowering `Budget::NESTING` to something a 2 MiB thread survives is the one option
to reject outright: the counter that would be lowered is not the counter that grows, so it would refuse programs without
saving the ones that crash. Charging `check` and `infer` for nesting is right by §4.1 and insufficient alone — at 256 it
would refuse a 128-note voice, turning a crash into a rejection of ordinary music, and refusing a program that compiles
today is a cost-table version bump and a bad one. So:

1. **Arrange the room first**, because it is owed already, it is cheap, and it is the only half that turns an abort into
   a diagnostic without touching acceptance. Move `with_room`'s shape to the `musa-core` seam and give `musa-core` a
   measured `FRAME_CEILING` of its own — measured on the `infer → check → eval` chain that real programs drive, not on
   `eval` recursing into itself, which is what the existing ~2 KiB note measured and why it reads five times too low.
   Every host gets it, including the desktop session thread, which today asks for none. **Done at prompt 141s** — see
   the note above; what is left here is to confirm the ceiling still holds once (2) has changed what a level costs, and
   to decide whether the session thread wants room of its own now that the seam has some.
2. **Then take the depth out of the spine**, so that the room needed stops being a function of how long a voice is.
   Either the fold builds a shape whose elaboration is not `N` deep, or the elaborator walks an application spine
   iteratively with an explicit work stack. Both are behaviour-preserving; measure both against P1/P2 before choosing,
   and record the number for the one not chosen.
3. **Then charge the elaborator's own recursion**, which is what makes §4.1's sentence true rather than aspirational. Do
   it last, because only after (2) is a limit of 256 a limit on nesting an author wrote rather than on the length of a
   phrase. If the charge still refuses a program that compiles today, that is a cost-table version bump with a stated
   reason, argued in `02-core-calculus.md` §4 — never a threshold quietly raised to make the suite pass.
4. **Then restate the depth law**, once (2) and (3) have settled what a level costs, at whatever the re-measurement
   supports rather than at whatever the suite tolerates. Two ways out, and they are priced differently. Spending fewer
   frames a level in the NbE descent is free by §4.1's own sentence — shrinking the frame ceiling changes nothing
   normative — so it is the one to try first, and it is the same work as (2) pointed at the recursor instead of at the
   voice spine. Raising `NESTING` is a cost-table version bump and is not free in a second way either: both seams size
   their stack as `NESTING × FRAME_CEILING`, `musa-core`'s `room.rs` at 8 MiB today and `core.rs`'s `with_room` for the
   expansion phase, so the room every host allocates scales with the limit. Whichever way it goes,
   `a_region_nested_deeper_than_anyone_writes_still_expands` ends this prompt carrying a depth the measurement supports
   and a comment that states the measurement, and `Budget::NESTING`'s doc claim ends it re-earned or corrected.

A stack that is merely *larger* is step 1 and is not steps 2 and 3. `RUST_MIN_STACK` in a test command, a `.cargo`
config, or CI is not any of them: it hides the defect from the suite while leaving every host that is not the suite
exactly as it was.

**The six items prompt 136b left here, and what each one waits on.** Note 44 audited `musa-core` against smalltt and
Peyton Jones and found seven divergences. 136b removed the ad hoc ones; these are the ones it could not, and the reason
is the same for all six: none of them can be _priced_ until prompt 142 points the standard library at this checker, and
five of them cannot be _built_ until there is a top-level definition scope to hold folded.

- **Glued evaluation (Finding C).** `musa-core` evaluates one way and unfolds everything it meets. smalltt keeps a
  definition's folded and unfolded forms side by side so that conversion can try the cheap comparison first and unfold
  only where it must. Nothing is foldable in the core today — there is no definition scope — so this becomes a real
  divergence exactly at 142 and not before. It is the prerequisite for the next three.
- **`Head::Def` and the `G` pair (Finding C's representation).** The head that carries both forms, which is what makes
  everything below expressible.
- **Approximate conversion (Finding D's remainder).** 136b gave conversion structural and η arms with early exit;
  smalltt's rigid/flex/full distinction is a further refinement that only pays where heads can stay folded.
- **The flexible quotation mode (Finding E's remainder).** `rigidQuote`/`flexQuote`/`fullCheck` keep folded heads folded
  during read-back, for the same reason.
- **The per-metavariable occurs cache (Finding E's remainder).** 136b fused the occurs check into the quotation that
  writes a solution, and a cache saves nothing against a walk that must visit every node anyway. It saves work at
  exactly the moment quotation may stop at a folded definition, which is glued evaluation's moment.
- **Hoisting a `match` arm's body (Finding A).** An arm whose pattern in a split column is a variable survives into
  every branch of that split, and its body is type-checked once per leaf it reaches. Peyton Jones §5.4.1 is that failure
  and ch. 6's let-bound right-hand side is the answer: `let armᵢ = λ (x⃗ : T⃗). body in <tree whose leaves are armᵢ v⃗>`.
  **The condition note 44 proposed for when this is safe is wrong**, and 136b proved it: a leaf binds its pattern
  variables with `define`, so an arm body may rely on the binder _reducing_, while a λ binder is an assumption. The
  witness has every binder at `Nat` and a goal of `Nat` — nothing dependent anywhere — and its abstraction does not
  typecheck. The sound shape is speculative: split `case.rs`'s `solve` into a plan pass and an emit pass, establish that
  the arm's binder telescope is the same at every leaf (splitting refines `Vec A n`, so it often is not), build the
  abstraction, elaborate the body against it once, and fall back to per-leaf elaboration where that is refused —
  reporting the per-leaf refusal, never the hoisted one. 136b's first-row column-selection rule already removed the
  exponential this was urgent for, so what is left is a linear constant factor on the leaves a non-first-row arm
  reaches: measure it on 142's output before building any of the above, and record the number either way.

**Update the recorded baseline, honestly.** `06-performance.md` records both the pre-migration baseline and the current
numbers. If something is genuinely slower and the 10% gate is exceeded, the prompt's output is either a mitigation or an
argued amendment to the budget — with the musician-facing B1/B2 numbers as the check that the argument is acceptable —
and never a quietly raised threshold.

## Target

- Every diagnostic from prompts 134, 135, 137, 137a, 139, and 140 rewritten to the standard above, each with
  `musa explain` text and a test that pins the message on a realistic program rather than a minimal one.
- Profiles of the new checker on the four recorded workloads, the interventions chosen, and the re-measured numbers.
- `docs/rules/language/06-performance.md` updated with post-migration P1/P2 rows and, if the gate was exceeded, the
  argument and its resolution.
- `docs/rules/desktop/06-performance.md`'s B1/B2 confirmed still met, measured rather than assumed.
- A verdict on each of note 44's six remaining items, measured on 142's output rather than argued: glued evaluation and
  `Head::Def`, approximate conversion, the flexible quotation mode, the per-metavariable occurs cache, and the hoisting
  of `match` arm bodies. Building one is an outcome; declining one with a number attached is equally an outcome, and
  leaving one unmeasured is not.
- A closing line in note 44 for each item this prompt settles, so the audit ends rather than being inherited again.
- The room obligation discharged at the `musa-core` seam, with a `FRAME_CEILING` constant in `musa-core` carrying the
  measurement that justifies it — of the `infer → check → eval` chain, in a debug build, with the command that produced
  the number in the doc comment beside it, as `core_budget.rs`'s already does. **Delivered at prompt 141s**; what
  remains is re-measuring the ceiling after the spine walk below changes what a level costs.
- The elaborator's recursion charged for nesting, and the spine walk that lets 256 stay 256. If either forces a
  cost-table version bump, the bump lands in `02-core-calculus.md` §4 with its reason, and `06-performance.md` records
  what the change cost.
- The region depth law restated: `expand.rs`'s `a_region_nested_deeper_than_anyone_writes_still_expands` at a depth the
  finished checker's measured frames-per-level supports rather than at 142's provisional 16, the first-refusal depth
  recorded beside it with what measured it, and `Budget::NESTING`'s doc comment either re-earned for the adapter path or
  corrected. If the answer is a raised limit, one version bump lands in `02-core-calculus.md` §4 carrying both
  measurements — the voice spine and the adapter region — as its reason, and `room.rs`'s allocation is re-stated at the
  size the new limit implies.
- A law that a voice long enough to reach the nesting limit is **refused and not fatal**, run on a thread no larger than
  the desktop session thread, so the guarantee is stated at the size the smallest host actually has. The law
  `a_term_nested_past_the_limit_is_refused` in `musa-core`'s `budget_laws.rs` is the one to extend: it descends through
  `eval` alone today, which is exactly the measurement that read five times too low.
- `apps/musa-desktop/src-tauri/src/session.rs`'s `spawn` giving its thread the room §4.1 requires, or an argued note
  saying why the seam alone is enough.
- `docs/plan/code-map/` rows.

## Check

```sh
cargo build --workspace
env -u RUST_MIN_STACK cargo nextest run --workspace   # the default 2 MiB thread is the point of this one
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-compiler -- p1_compile p2_elaborate
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Make the new failures legible and the new checker fast enough`.

## Stop

- No behaviour change: same programs accepted, same programs rejected, same values, same rendered output. A diagnostic
  prompt that changes what compiles has changed the language.
- No new language feature offered as a fix for a bad message.
- No optimization without a measurement, and no `#[inline(always)]`, SIMD, parallelism, or custom allocator on
  intuition. The `rust-performance` skill's rule is the rule.
- No raised budget threshold in place of a fix, and no benchmark chosen to flatter the result.
- No `RUST_MIN_STACK`, in a test command, a `.cargo/config.toml`, a CI file, or a doc. It is the one change that makes
  the symptom disappear from the suite and leaves every host exactly as broken as it was.
- No lowering of `Budget::NESTING` to a stack-safe number. The measurement says the counter it would lower is not the
  counter that grows, so it would refuse programs and still crash.
- No adapter rewrite. Prompts 145 and 146, which is also where the real-program evidence for these diagnostics comes
  from.
