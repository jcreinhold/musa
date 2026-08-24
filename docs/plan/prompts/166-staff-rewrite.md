---
id: 166
slug: staff-rewrite
status: pending
depends_on: [142f, 162, 165a, 165b]
phase: 3
---

> **Reinstated and repaired by notes [`51`](../../notes/research/language-design-closure/51-the-terseness-audit.md) and
> [`52`](../../notes/research/language-design-closure/52-the-musical-algebra.md).** This is still the acceptance gate,
> and it is now the gate for the correction _and_ its correction: the constitution's amendment record says the pass is
> answerable to this number, and the number has moved the wrong way — 2,404 lines when the amendment was granted, 2,515
> today. The rewrite runs on the language of 142a–142e, and if it is not dramatically shorter, note 51's diagnosis is
> wrong.

# Rewrite the Staff Adapter, and Find Out Whether Any of This Worked

## Task

Rewrite `stdlib/src/adapters/staff.musa` on the new language. This is the acceptance gate for the entire pass: the
adapter is the recorded failing program that prompt 128's amendment was granted on, and if the rewrite is not
dramatically shorter and more obviously correct than 2,404 lines, the design failed and this prompt is a repair of Phase
A rather than an implementation.

## Read

- **The ordering, repaired.** This prompt sat behind 143 and 144 until the measurement they were waiting for arrived and
  said not to wait. `examples/staff-page.musa`, 77 lines, instrumented on the migrated checker: 1,605,182,361 reduction
  steps, of which 1,108,756,085 are `eval` entries over 2,719 distinct source origins; 18,922,391 δ-unfoldings over 84
  distinct definitions; and a per-term closedness tally — every distinct term shape walked once for a `Var` anywhere
  under it, the sufficient condition under which a pointer-keyed cache could fire — answering 198,753,114 closed
  entries, **17.9%**, with **zero** at the five hottest sites, all of them closed-looking type annotations in
  `stdlib/src/notation/staff.musa` whose elaborated terms carry the context elaboration glued on. So caching caps below
  a fifth of the spend and never fires where the spend is, a specified cache would be a cost-table version bump besides
  (`02-core-calculus.md` §4: it changes what the meter charges), and the spend itself is the call count this rewrite
  deletes. The rewrite therefore lands before the registry collapse and the cost table. What that bullet got wrong is
  the last clause of its own conclusion — that the rewrite is _what_ turns prompt 142's staff budget failures green. Two
  of the three orders of magnitude were the evaluator's, and
  [note 59](../../notes/research/language-design-closure/59-the-staff-wall-is-the-evaluators.md) measures them;
  [165b](165b-graph-update-and-data-descent.md) takes them and runs first.
- `docs/notes/research/language-design-closure/42-dependent-core-decision.md` — the failing program and the measurements
  it recorded, which are the "before" side of this comparison.
- Prompt 132's trial and its **predicted** line and byte count. That number was written before any code existed and is
  the gate; discovering it was optimistic is a finding about the design, not a reason to restate the gate.
- `stdlib/src/adapters/staff.musa` as prompt 142 migrated it — the actual starting point, which is the old structure on
  the new checker.
- `docs/rules/language/11-quotation.md`, `10-traits.md`, and `01-surface.md` — the language as specified, since a
  rewrite that reaches for something unspecified is evidence, not a shortcut.
- Prompts [127dcd](127dcd-adapter-edit.md) and [127dce](127dce-adapter-print.md), and
  `crates/musa-compiler/tests/suite/staff_writing_laws.rs` — the edit and print laws, the three conformance levels, and
  the tests that already hold this adapter to them. **Those tests survive the rewrite unchanged.** They were written
  against behaviour, not against implementation, and if the rewrite needs them changed then either the rewrite changed
  the adapter's meaning or the tests were coupled to internals — and which one it is has to be established, not assumed.
- `docs/notes/research/language-design-closure/41-staff-on-the-repaired-interface.md` — the last time this file was
  rewritten and measured, and the method that measurement used.
- **The open question this prompt was written to close is already closed, and the Read section says so rather than
  sending a reader looking.** `syntax_anchor`'s third argument — the place, supplied as `syntax_built(here, 33, 0)` —
  went at prompt **145**, not here: `crates/musa-compiler/src/phase/ownership.rs` declares
  `syntax_anchor(subject, path)`, `crates/musa-compiler/src/registry/rules.rs`'s `SyntaxOp::Anchor` derives the place as
  `crate::quote::anchor_place(&wanted)` at the reserved site `crate::quote::path::DELTA_QUOTATION` (`u32::MAX`), and
  `stdlib/src/adapters/staff.musa`, `stdlib/src/adapters/doubled.musa`, and `tests/fixtures/staff-construction.musa` all
  call it at two arguments already. `45-phase-registry-survey.md`'s row records the prompt correctly and
  `11-quotation.md` §5 names this one, which is the remaining half-sentence to repair. What was this prompt's about the
  anchor is therefore done, and the Design paragraph below is kept as the record of _why_ the shape is the one it is —
  the argument, not an instruction.
- `crates/musa-compiler/src/quote/mod.rs`'s `Derived` and `check_expression`, and
  `crates/musa-compiler/src/phase/mod.rs`'s `syntax_quote` — how a quote mints provenance without an author supplying a
  number, which is the mechanism the anchor either adopts or argues against.

## Design

**This runs before the registry collapse, and — as it turned out — after 143 and 144.** The reordering the Read section
records was granted and then only half kept: 143 and 144 ran anyway, so the diagnostics this rewrite is written against
are 144's improved ones rather than the migrated checker's. What did survive is the half that matters here. The registry
this rewrite calls is the one prompt 141e translated, uncollapsed, because prompt 164 waits on this one: its Check
asserts a green staff budget class, and the staff budget tests 142's Check names are red until 165b and then this prompt
close them — 165b takes the two walls that were the evaluator's, and this prompt takes the factor of 1.9 that is left.
Neither reordering changes the gate below: the rewrite is measured against 2,404 lines and prompt 132's prediction,
whenever it runs.

**Five things must be gone, and each is checkable.** Zero `callN` helpers. Zero hand-allocated role integers. Zero
string dispatch on token kinds or delimiters. `Pending` as a record with named fields rather than an eight-field
destructure. And the reading algorithm running forwards, because prompt 141 gave it a way to accumulate. Write each as a
test or a grep in the Check, not as a claim in the commit message.

**`syntax_anchor` lost its place argument, and the evaluator derives it the way a quote does — at prompt 145, not
here.** The argument is kept because it is the record of why the shape is the one the code now has, and because prompt
139's deferral pointed at this number. Of the two shapes, only one survives its own consequences. Keeping the argument
means giving authors an operation that mints a fresh place — and that operation is `syntax_built` under a new name,
which `11-quotation.md` §5 deletes and `43-dependent-language-trial.md` §13 records as a falsifier the pass did _not_
fire ("a hand-written provenance path, or a role integer by another name — no"). So the shape is
`syntax_anchor(subject, path)`, answering the anchor of the input node at `path` as before, and the place its answer
stands at derives from the arguments alone: `origin` is the anchored node's path, already unique per node, and
`quotation` is `u32::MAX`, a reservation the upward-counting quote counters never draw. Disjointness by construction
rather than by allocation, which is what keeps `PhaseFamily::Builder`'s "no counter, no clock, no compiler state" true
of the anchor — a δ rule is `fn(&[Datum]) -> Option<Answer>` and cannot reach a counter, which is why an earlier version
of this paragraph, drawing the site index from `Resolver::next_quotation`, was wrong.

**That does not remove the one-call-per-anchor obligation; it makes it the obligation every quote already carries.** Two
calls of one anchor site at one `here` still mint one path, exactly as calling a helper whose body is
`quote at here { … }` twice with one `here` does. What changed is that this stopped being a fact an author has to know
about one builtin and became the derived-identity law of `11-quotation.md` §3, reported by the same duplicate-path gate
in the same words.

**The answer stays at `Syntax<TokenTree>` and the call site keeps its `as_expression`.** The trial's §1.1 assumed both
changes at once and wrote `anchored` as if the anchor arrived at `Syntax<Expr>`; the fixture's note is right against it.
An anchor is a token, an emitted call needs an expression, and the operation that turns one into the other is the
checked parse — the same route the composer's own spliced pitch takes, which is §1.2's general finding rather than a
special case. Making the builtin answer at `Expr` would put a parse inside a builtin to save one `match` in one helper.
This prompt changes the arity and nothing else about the signature.

**What is left of that, and it is one line of prose.** The registry entry, the declared type, the doc comment, the
evaluator arm, `PhaseFamily::Builder`'s doc, the two anchor unit tests in `crates/musa-compiler/src/expand/tests.rs`,
`stdlib/src/adapters/doubled.musa`'s call site, and `tests/fixtures/staff-construction.musa`'s `anchored` and header
note are all already at the new arity. What is not is `docs/rules/language/11-quotation.md` §5, which still says "prompt
166 owns the anchor's arity". Repair it to name 145, and check `45-phase-registry-survey.md`'s row against it.

**The compatibility oracle: check it, and expect it not to move.** An anchor _number_ is a node's position in the
region's reading order and this changes nothing about reading order, so every anchor the staff adapter emits is the
number it emitted before — which is what `staff_writing_laws.rs` asserts and why those tests survive unchanged. What
changes is the derived path of the token carrying that number, from `built(here, 33, 0)` to `built(here, <site>, 0)`,
and reallocating the shared counter shifts every other quote's site index with it. No `origin=` line in
`tests/fixtures/elaboration-compatibility.txt` carries a derived path, no oracle fixture expands a syntax region at all,
and no `.snap` prints one — so the prediction is that the manifest, the rendered corpus, and every snapshot are
byte-identical. State that it was checked rather than assumed: if any of them does move, the derived path is observable
somewhere this analysis missed, and that is a finding about the identity law rather than an oracle entry to update.

**Measure the same way twice.** Lines and bytes for the whole file, and separately for the four sections prompt 132
predicted: construction, dispatch, `Pending`, and `document_read`. A whole-file number can hide a section that got worse
under a section that got much better, and the section that got worse is the interesting one.

**"More obviously correct" is a claim that needs evidence.** The available evidence is: the same behavioural tests
passing unchanged, the anchors landing on the same nodes, the printer round-trip holding at the same conformance level,
and the rendered corpus for every staff example staying byte-identical. State each one. A rewrite that is shorter and
changes what a musician sees has not passed.

**Report honestly if the gate is missed.** If the rewrite lands materially above prompt 132's prediction, this prompt
does not quietly accept the number. It records what the language failed to remove, identifies which of prompts 129–131
made the wrong promise, and becomes a repair of that specification — which is the whole reason the prediction was
written down in advance. That is stop condition 5 territory for the prompt stack, and it hands the decision back rather
than absorbing it.

**Do not improve the notation coverage.** The rewrite reads the same music it read before, produces the same values, and
carries the same anchors. Adding a feature during the rewrite makes the measurement meaningless and would be the easiest
possible way to make the number look worse for a good reason and better for a bad one.

## Target

- `stdlib/src/adapters/staff.musa`, rewritten on the new language, with the five eliminations above.
- The measurement: whole file and per-section, before and after, against prompt 132's prediction, recorded in
  `docs/notes/research/language-design-closure/` and linked from the code map.
- `crates/musa-compiler/tests/suite/staff_writing_laws.rs` and every existing staff test passing **unchanged**.
- Mechanical checks in the Check section for the five eliminations.
- The rendered corpus for every staff example byte-identical.
- `docs/rules/language/11-quotation.md` §5 repaired to name prompt 145 as the prompt that removed `syntax_anchor`'s
  place argument, and `docs/notes/research/language-design-closure/45-phase-registry-survey.md`'s row checked against
  it. Everything else the anchor change touched is already at the new arity, and this prompt confirms that rather than
  redoing it.
- The oracle check recorded: manifest, rendered corpus, and snapshots compared, and the result stated rather than
  assumed.
- **Every test in the staff budget class prompt 142's Check names, green.** That class is the reason this prompt moved;
  the two tonal examples it names beside the staff class stay red here and stay 144's.
- **The class's `#[ignore]` markers removed.** Twenty-five of them, and their stated reason is the wall this prompt
  closes. `staff_expansion_laws.rs`'s module header says in as many words that these laws are "the measurement it will
  be greeted by, unignored one green run at a time", and AGENTS.md requires an `#[ignore]` to carry an argument for
  itself — a marker whose argument the commit has just falsified is drift, not caution. The laws run in seconds, so
  they belong in the fast suite.

## Check

```sh
cargo build --workspace
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- format --check stdlib/src/adapters/staff.musa
! grep -nE '\bcall[1-9]\b' stdlib/src/adapters/staff.musa
! grep -n 'syntax_built' stdlib/src/adapters/staff.musa
! grep -nE 'text_equal[(][a-z_]*kind' stdlib/src/adapters/staff.musa
! grep -rnE 'syntax_anchor[^)]*syntax_built' stdlib/ tests/fixtures/
wc -lc stdlib/src/adapters/staff.musa
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

The four `grep` lines must find nothing; `wc` records the number the gate is judged on. The second is stronger than it
was — it asked for a literal role integer and now asks for the operation at all, because after the anchor loses its
place argument there is nothing left in this file for `syntax_built` to serve. The fourth says the same thing about
every remaining caller. Arity itself needs no grep: a three-argument `syntax_anchor` is a `WrongArity` diagnostic, so
`cargo nextest run --workspace` is the check that every call site moved.

The two `nextest` lines close the staff budget class. **The class is thirty tests, measured, and the wall is not the one
this prompt was written against.** `--run-ignored all` on a clean tree at `7cf258e0` reports 31 failures; 30 are this
class and one was an unrelated orphan since deleted:

| Tests                                                                                       | What they read                                                   |
| ------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `staff_expansion_laws::*` (25)                                                              | a staff region through `std::adapters::staff`                    |
| `staff_writing_laws::a_printed_page_says_what_the_value_said`                               | the same, printed back                                           |
| `session_laws::the_staff_page_example_compiles_and_renders`                                 | `examples/staff-page.musa`                                       |
| `large_score_generators::*` (2)                                                             | `tests/fixtures/large-score.musa`, no adapter involved           |
| `musa::cli wav_export_is_deterministic_for_all_examples`                                    | every example, and it dies on `examples/diatonic-sequences.musa` |
| `elaboration_fixture_generators::the_pressure_workloads_compile_and_denote_what_they_claim` | `core-pressure.musa`, no adapter involved                        |

**Three of those rows are not this prompt's, and the original table said otherwise.** It attributed the two
`large_score_generators` tests and the `wav` export to `examples/staff-page.musa` on the strength of their being in the
same failure list. They are not: `tests/fixtures/large-score.musa` is a generated 100-bar score with no `syntax` region
and no adapter import at all, and the `wav` export walks every example and panics on
`examples/diatonic-sequences.musa`, which likewise imports no adapter. Both are the whole-file 200,000-step
_elaboration_ budget, and both are already named in prompt [165](165-diagnostics-and-performance.md)'s Target — the
1500-event fixture by that description, the tonal class by name. They belong to 165 and cannot be closed here without
doing 165's work.

One test goes red _because_ this prompt succeeds, and closing it is part of finishing:
`document::laws::the_kernel_rechecks_every_example` asserts `` `staff-page` no longer meets the budget wall — strike it
from BUDGET_WALL ``. `BUDGET_WALL` in `crates/musa-compiler/src/document/laws.rs` names this prompt as `staff-page`'s
owner and says "Striking an entry is the commit that fixes it". So this commit strikes it, and the Stop section below
carves out exactly that one line of `crates/`.

**There were two walls and neither of them was this prompt's**, which is
[note 59](../../notes/research/language-design-closure/59-the-staff-wall-is-the-evaluators.md) and the reason
[165b](165b-graph-update-and-data-descent.md) now runs first. Measured before any rewrite: a staff region with _nothing
in it_ cost 455,942 steps against a budget of 200,000, because δ had no graph update and every consumer of a folded
value re-ran the whole unfold; and `examples/staff-page.musa` peaked at 672 of 320 nesting levels inside
`canonical`/`realize`, which charge one level per level of _data_. There was no algorithm in an empty region for a
rewrite to remove. With those two fixed and the adapter untouched, 23 of the 24 `staff_expansion_laws` pass at the
unchanged 200,000-step budget.

**What is left is this prompt's, and it is a factor of 1.9.** After 165b, `examples/staff-page.musa`'s expansion costs
381,055 steps against 200,000 — about 2,800 of fixed cost and 9,000 an item, straight-line. That is the number the
rewrite closes, and it is the honest form of "the class goes green here": every remaining member of the class reports
`reduction steps`, 165b recorded the count each one reports, and this prompt is measured against those numbers rather
than against a wall it did not build.

**The budget does not move.** `crates/musa-calculus/src/kernel/budget.rs` states that in as many words and this Check
does not re-open it.

The tonal class this Check used to name — `diatonic-sequences` and `rule-of-the-octave` exhausting the step budget with
no adapter involved — **closed halfway, and this Check overstated it**. Measured on the tree this prompt runs on:
`examples/rule-of-the-octave.musa` compiles, and `examples/diatonic-sequences.musa` still reports `evaluation exceeded
the budget for reduction steps at 200001 of 200000` — identically with the pristine pre-rewrite adapter, which it does
not import. It is absent from the `nextest` list only because `BUDGET_WALL` excuses it there; a whole-file check still
fails, which is what the `wav` export row above is. Prompt 165 owns it. What stands in its place is `core-pressure`, which is prompt 165's own pressure fixture and hit the _nesting_ wall
at 257 of 256. Prompt 165's note asked this prompt to settle it: "if the staff rewrite makes `core-pressure` green as a
side effect, the wall was the adapter's shape after all; if it does not, the notion of a nesting level is what needs
repairing." Note 59 answered it before the rewrite — the wall was not the adapter's shape, and 165b is the repair. So
`core-pressure` is not this prompt's, and every other row above must be green. Any other red means this prompt is not
done.

The oracle stays fixed: a rewrite of a library file has no business changing a semantic hash or a rendered corpus file,
and the Design section's analysis says the anchor's derived path is not observable in any of them. If one moves, stop
and report it rather than re-baselining.

Commit as `Rewrite the staff adapter on the new language`.

## Stop

- No new notation coverage, no new diagnostic, no changed anchor, no changed conformance level. The adapter does the
  same job.
- **One line of `crates/`, and it is the bookkeeping this prompt is named in.** The one change this prompt used to
  carry — `syntax_anchor`'s arity — landed at prompt 145, and the two evaluator repairs the budget class turned out to
  need are [165b](165b-graph-update-and-data-descent.md)'s. What is left is striking `"staff-page"` from `BUDGET_WALL`
  in `crates/musa-compiler/src/document/laws.rs`, and its accompanying half of that constant's doc comment: the list
  names this prompt as the entry's owner, says in as many words that "striking an entry is the commit that fixes it",
  and asserts against a wall that is no longer met. That is a record of which prompt owes which fixture, not compiler
  behaviour. The same goes for the twenty-five `#[ignore]` markers the Target names: deleting a marker whose reason the
  commit just falsified is bookkeeping too, and it strengthens the default suite rather than touching what any law
  asserts. No other change to `crates/` is in scope. Anything else a library rewrite turns out to force is a finding
  worth reporting rather than a quiet commit; the whole claim is that this file is ordinary unprivileged Musa.
- No operation that mints a `NodePath` — not a renamed `syntax_built`, not a `here` that an author derives, not a "fresh
  place" helper in `stdlib/`. If the rewrite needs one, the argument in the Design section is wrong and this prompt says
  so rather than adding it.
- No other phase-operation deletion. The builders `11-quotation.md` §5 retires still have callers in
  `stdlib/src/adapters/doubled.musa`; removing them belongs to the prompt that audits the registry, not to this one.
- No weakening or rewriting of an existing staff test to accommodate the rewrite. Removing an `#[ignore]` is the
  opposite of that and is in scope; changing what a law asserts is not.
- No adjustment of prompt 132's prediction. It is a fixed gate.
- No studio work. Prompt 167.
