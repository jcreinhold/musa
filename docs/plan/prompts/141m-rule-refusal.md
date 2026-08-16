---
id: 141m
slug: rule-refusal
status: done
depends_on: [141b, 141e, 141j, 141k]
phase: 3
---

# Let a δ-Rule Refuse the Program

## Task

Give a δ-rule a way to say *the program is wrong*. Today its only "no" is `None`, which
[`eval.rs`](../../../crates/musa-core/src/eval.rs) reports as `Malformed::BuiltinStuck` — restated to the composer as "a
defect in the compiler rather than in the source" — so every registered operation with a real refusal had to answer
`Result τ Text` instead and hand the composer's own mistake back as a value. Add the third answer, move the refusals
that are program errors onto it, and take `Result` off the notation vocabulary, so that a notated block denotes
`EventTrack ⟨written⟩` and `01-surface.md` §2's signatures are true as written.

## Read

- [`../../rules/language/01-surface.md`](../../rules/language/01-surface.md) §2, which is the evidence and the
  acceptance test at once. Twenty signatures spell a notation value `EventTrack[WrittenTime, ScoreFact]` — the
  desugaring of `motif`, `fn figure()`, `fn transpose_answer(...)`, `template voice answer(subject: ...)` — and one
  worked line disagrees with the registry twice over:

  ```musa
  fn sound(chosen: Voicing) -> EventTrack[WrittenTime, ScoreFact] { play(chosen, 1/2) }
  ```

  `play` is registered at four arguments answering `Result (EventTrack ⟨written⟩) Text`, so §2's line has the wrong
  arity *and* the wrong answer type. §2 also fixes the reading of `use e;` — "checks that `e` is a written-time score
  track" — which is the sentence a fallible constructor makes unwritable, because every saved fragment is then a
  `Result` and `follow` demands a track.
- [`../../rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §4, which says elaboration has
  **three** outcomes and not two — a refusal is the program's fault, an exhaustion is no judgment at all, and a
  malformed term is the compiler's own defect. A rule that can only answer `None` collapses the first into the third,
  which is exactly the bug. §5.8 fixes when a rule fires; §5.7 is why `sounded` and `play` take an origin and a scope no
  source writes.
- `crates/musa-core/src/base.rs` — the `Rule` type, `Reduction::Delta`, and the long argument at `Builtin::constructor`
  for why "no reduction at all" is a *shape* rather than a rule that always answers `None`. This prompt adds the
  neighbouring distinction on the other side: a rule that answers nothing because the *term* is wrong, and a rule that
  answers nothing because the *program* is.
- `crates/musa-core/src/eval.rs`'s δ-reduction, the one site that turns a rule's answer into an outcome, and
  `crates/musa-core/src/error.rs`'s `Malformed::BuiltinStuck` and `MisfitAnswer` — the two caller-defect sentences that
  stay caller defects.
- `crates/musa-core/src/refuse.rs` — `Refusal`, which gains one variant, and which
  `crates/musa-compiler/src/lower/refusals.rs` files under a `Code` by an exhaustive match, so a variant added here
  fails to compile there until somebody says where it belongs.
- `crates/musa-compiler/src/registry/rules.rs`'s `answered` and `refused`, and every caller of `refused` in
  `registry/track.rs`, `registry/notation.rs`, and `rules.rs` itself. That list is the survey this prompt performs and
  the Target counts.
- [`141b`](141b-base-types-and-builtins.md), which introduced `Rule` and the `Datum` protocol, and
  [`141e`](141e-compiler-registry.md), which wrote `play` and the arithmetic rules against it. This prompt repairs what
  those two wrote, in the place they wrote it.
- [`141j`](141j-notation-vocabulary.md), whose Design fixes `sounded : … → Result (EventTrack ⟨written⟩) Text` and
  argues it from `play`'s shape: "Prompt 141e wrote `play` against §5.7's construction clause and prompt 142's Stop
  forbids re-translating what it wrote." The premise moves here, so the conclusion does too — 141j's file stays a record
  of why the fallible shape was reasonable when the core offered nothing else.
- [`141k`](141k-notation-lowering.md)'s "a block answers its own questions", which is the reading built on that shape:
  `Lowering::music` wraps the fold in `Result.Ok` and drains the `?`s the constructors asked. With total constructors
  there is nothing to drain, and the paragraph's consequence — "`music { … }?` is refused by the core" — stops being a
  limitation anybody has to live with.
- [`142`](142-surface-cutover.md)'s **Stop**: "No re-translation of a signature or a rule 141e already wrote. A
  disagreement between the two is a defect in one of them and is repaired where it is, not worked around here." This
  prompt is that repair, made where the defect is, so that 142's migration writes ordinary source instead of threading
  `Result` through every notation signature in `stdlib/` and `examples/`.
- The `module-design` skill's rule 5 — "Remove needless errors. If a normal result can safely cover an edge case, use
  it" — and its failure smell "error no caller can handle". Seven of the messages this prompt moves are things no
  program can branch on and every composer must fix in the source.
- Peyton Jones ch. 5, which draws the same line one level down: pattern-match `FAIL` and `ERROR` are different values on
  purpose, because a failure that the next equation may recover from and a failure that ends the program are not one
  thing. `Result τ Text` is being used for both here, and the chapter's argument is why that costs.

## Design

**A rule has three answers, because elaboration has three outcomes.** `Rule` becomes a function into a three-case
answer: the datum it computed, a refusal with the sentence to say, or nothing-at-all for arguments its own signature
does not admit. The third stays `Malformed::BuiltinStuck` and keeps its wording, because it is still what it always was
— the table and the rule disagreeing, which no source edit can fix. The second is new and becomes a `Refusal` carrying
the application's origin, so it arrives at `lower/refusals.rs` like every other refusal and lands on the composer's own
span. This is `02-core-calculus.md` §4's three outcomes reaching the one place that could not express them.

Two shapes were considered and one rejected. Widening the answer to `Result<Option<Datum>, Refusal>` would let a rule
build a `Refusal` directly, which means handing every host rule the core's refusal vocabulary and its origin discipline
— a wide interface for one narrow need. The narrow one wins: the rule says the sentence, the core says where. A rule has
no origin to name in any case, since it sees data and not terms.

**The criterion, stated once: `Option` is a musical answer; `Result τ Text` is a diagnostic wearing a value's clothes.**
`close_position(sonority, c4) : Option<Voicing>` is a real answer a program branches on — some chords have no
close-position voicing, and §2 writes the `match`. "A chord sounds for longer than no time at all" is not an answer; it
is a composer's mistake, and the only thing any caller can do with it is stop. Apply the criterion to every `refused`
call site, move the ones it classifies as refusals, and record in the prompt's own file any site it classifies the other
way, with the branch that reads it. A site nobody branches on and nobody moves is the drift this repo forbids.

**All of the notation and track refusals move, which is what takes `Result` off the vocabulary.** Four rules answer
`Result τ Text` in the track and notation families — `STRETCH`, `SHIFT`, `PLAY`, `SOUNDED` — across seven sites, and
every one of the seven is a program error: a stretch factor at or below zero, music shifted before the start or past the
end of its track, a chord or a fact given a length it cannot sound for or does not fit. None survives the criterion, so
the whole family goes total and the `Result` disappears from every track signature rather than from some of them.

**The arithmetic and duration rules are the same judgment and do not move yet.** Division by zero, a duration below
zero, and the overflow sentence "no result this language can represent" are composers' mistakes by the criterion, and
they matter for the same reason the track family does: 141l routed `x + y` through `Add.add`, so a fallible `Add` puts a
`?` between every two numbers a composer adds. They stay `Result` here for a constraint found in the doing rather than a
change of mind. `BUILTIN_OWNERSHIP` is one table read by **two** checkers — this registry and the one compiling
`stdlib/` today — so narrowing `ratio_add`'s declared answer rewrites the twenty `match … { Ok(v) -> … }` sites listed
below, in `stdlib/src/notation/staff.musa` and `stdlib/src/adapters/staff.musa`. Rewriting them is prompt 142's Task and
this prompt's own Stop forbids it. The track and notation families have no such tie: their signatures live in
`registry/track.rs` and `registry/notation.rs`, no `.musa` file names them, and that is why the move §2 needs can happen
here and the quieter one waits for its callers. Prompt 142 carries it.

### The sites the criterion leaves as values

Two are the criterion's own answer and stay values for good:

| Site | Why it stays | The branch that reads it |
| --- | --- | --- |
| `Builtin::Row12Of` | a `RowFault` names *which* positions repeat and which classes are missing — an analysis a program reads, not a sentence a composer is told | `stdlib/src/post_tonal/serial.musa:28`'s `row`, which hands the `Result<Row12, (List<Nat>, List<Pc12>)>` on |
| `SyntaxOp::Checked` | the gate exists so a transformer can *decide* what to say about a tree it built badly; refusing would take that decision away | `crates/musa-compiler`'s phase vocabulary, and prompt 145's rewritten adapter |

Eleven are refusals by the criterion and stay values only until their callers move, all under prompt 142:

| Rule | Sentence | Sites that branch on it |
| --- | --- | --- |
| `ratio_arithmetic` (`Add`, `Sub`, `Mul`, `Div`) | not divided by zero; no result this language can represent | `notation/staff.musa:153,164,176,177,213,353,354,398`; `adapters/staff.musa:599,753,1353,1354,1726` |
| `written_duration`, `DurationOf` | a duration is nonnegative | `notation/staff.musa:199,204,214` |
| `NatAdd`, `NatMul` | no result this language can represent | none in the corpus; reached through `Add.add` |
| `DurationAdd`, `DurationScale` | no sum, no product | `notation/staff.musa:286,380` |
| `PositionShift`, `PositionBetween` | no result; the second position is before the first | `notation/staff.musa:312,529` |

**The block stops being a `Result`, and the reading gets smaller.** With total constructors `Lowering::sounded` stops
asking a question, `Lowering::music` stops wrapping in `Result.Ok`, and a notated block denotes `EventTrack ⟨written⟩`.
`Lowering::used` then works as it was written, `01-surface.md` §2's twenty signatures are true as written, and the
questions machinery stays exactly where it belongs — serving a `?` the composer wrote. This is the payoff and the test:
if the reading does not get smaller, the refusal channel was not the missing piece.

**A source `play(v, d)` reads as `play(⟨origin⟩, ⟨scope⟩, v, d)`.** §5.7 requires every constructed fact to carry an
origin and a scope, `BUILTIN_OWNERSHIP` already calls both of them `play`'s hidden information, and a composer has
neither to give. The reading supplies them, exactly as `Lowering::sounded` already supplies them for a notation
statement, so §2's two-argument call elaborates and the ownership table's claim stays true. Nothing else about
application changes: this is one entry in a reading, not a general mechanism for hidden arguments.

**One diagnostic code, not seven.** The refusals moving here are not a new family a reader looks up individually; they
are the operations of the language saying no with their own sentences, which is what `lower/refusals.rs` already exists
to restate. Prompt 144 owns how good the sentences are.

### What §2 still says that is not true, recorded rather than repaired

Two words of §2's worked example, both outside this prompt's boundary and both prompt 142's:

- **`play(chosen, 1/2)`** does not check. A written `1/2` is a `Ratio` and `play` reads a `Duration ⟨written⟩`, which is
  a different type and the core says so. `stdlib/src/voicing.musa:57` writes the true version —
  `fn sound_for(chosen: Voicing, held: Duration<WrittenTime>) -> Music { play(chosen, held) }` — and it is what the law
  states. What a bare literal may mean at a coordinate type is the literal domains' question: `01-surface.md` fixes
  `Duration::of(n)` rather than return-type-directed overloading, so §2's own line wants `Duration::of(1/2)` or a
  coercion nobody has argued for.
- **`-> EventTrack[WrittenTime, ScoreFact]`** is not a spelling the grammar has. A track type is written `Music` today,
  and `lower/types.rs`'s table has no entry for either word: the return type in §2's signatures is exactly what prompt
  142 replaces when it deletes contextual `Music`.

## Target

- `musa_core::Rule` answering three cases rather than two, the third carrying the sentence to say; `eval.rs` turning it
  into a `Refusal` at the application's origin, and leaving `Malformed::BuiltinStuck` for the arguments-not-admitted
  case it already reports.
- The new `Refusal` variant filed under a `Code` in `crates/musa-compiler/src/lower/refusals.rs`, with the code declared
  in `crates/musa-compiler/src/diagnose.rs` and explained in `crates/musa-project/src/diagnostic.rs`.
- Every `refused` call site surveyed against the criterion, the ones it classifies as refusals moved, and each site it
  leaves as a value recorded in this file with the caller that branches on it.
- `sounded`, `play`, `stretch`, and `shift` registered total: `EventTrack ⟨written⟩` rather than
  `Result (EventTrack ⟨written⟩) Text`, with `registry/laws.rs`'s accounting unchanged — the same operations, at
  repaired signatures.
- `Lowering::sounded` no longer asking, `Lowering::music` no longer wrapping, and a notated block lowering to a term at
  `EventTrack ⟨written⟩`; `Lowering::used` unchanged and now meaningful.
- A source `play(v, d)` read as the four-argument application, with the origin and the scope supplied by the reading.
- Laws: a rule that refuses reports at the application's origin and not as a compiler defect; a rule that answers
  nothing still reports as a compiler defect; a notated block's type is `EventTrack ⟨written⟩`; a saved fragment folds
  into another block through `use` without a `?`; `stack c4 major/2` written where §2 writes it elaborates.
- `docs/plan/code-map/` rows for `musa-core` and `musa-compiler`.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-core -p musa-compiler
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Let a rule refuse the program`.

## Stop

- No migration of `stdlib/`, `examples/`, or any fixture corpus, and no oracle entry moved. Prompt 142 owns all of it,
  and this prompt is green with the corpus exactly as it stands.
- No deletion of `infer.rs` or of any superseded `core.rs` arm. Prompt 142.
- No `docs/rules/` amendment. This prompt exists precisely so that `01-surface.md` §2 needs no repair; if implementation
  finds a §2 sentence that is still false afterwards, that is a finding to record, not an edit to make.
- No change to which builtins are registered, and no signature change beyond dropping `Result τ Text` where the
  criterion says the failure is a refusal.
- No builtin-registry collapse. Prompt 143.
- No new surface syntax, and no general mechanism for compiler-supplied arguments — `play` gets one reading, written
  once.
- No rewrite of the `?` desugaring. Total constructors remove the reason `music { … }?` was reached for; the desugaring
  itself is 142's surface to own.
