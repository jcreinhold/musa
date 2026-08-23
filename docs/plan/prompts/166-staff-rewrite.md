---
id: 166
slug: staff-rewrite
status: pending
depends_on: [142f, 162]
phase: 3
---

> **Reinstated and repaired by notes [`51`](../../notes/research/language-design-closure/51-the-terseness-audit.md) and
> [`52`](../../notes/research/language-design-closure/52-the-musical-algebra.md).** This is still the acceptance gate,
> and it is now the gate for the correction *and* its correction: the constitution's amendment record says the pass is
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
  deletes. The rewrite therefore lands before the registry collapse and the cost table, and it is what turns prompt
  142's staff budget failures green.
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
- **The open question this prompt closes**: `syntax_anchor`'s third argument. `crates/musa-compiler/src/phase/mod.rs`'s
  `SyntaxOp::Anchor` still takes `(subject, path, here)`, and `stdlib/src/adapters/staff.musa`'s `anchored` supplies the
  place as `syntax_built(here, 33, 0)`. `docs/rules/language/11-quotation.md` §5 and
  `docs/notes/research/language-design-closure/45-phase-registry-survey.md`'s row both say the argument goes and both
  name prompt 139 — but 139 deferred every phase-operation deletion to the prompt that removes the last caller, which
  for the anchor is this one. Meanwhile prompt 132's trial (`43-dependent-language-trial.md` §1.1) already wrote
  `syntax_anchor(region, here)` with two arguments, and `tests/fixtures/staff-construction.musa`'s header (lines 28–41)
  records the workaround it had to run under instead: pass `here` for the place and put the answer through
  `as_expression`, which is sound only for one call per node — two `anchored` calls at one anchor would build two tokens
  at one path and `check_expression`'s duplicate-path gate would refuse the expansion. An author writing quotes has no
  operation that mints a fresh place, because `syntax_built` is exactly what quotation removed. The Design section
  decides the shape; it is not a change to make quietly inside the rewrite.
- `crates/musa-compiler/src/quote/mod.rs`'s `Derived` and `check_expression`, and
  `crates/musa-compiler/src/phase/mod.rs`'s `syntax_quote` — how a quote mints provenance without an author supplying a
  number, which is the mechanism the anchor either adopts or argues against.

## Design

**This runs before the registry collapse, and — as it turned out — after 143 and 144.** The reordering the Read section
records was granted and then only half kept: 143 and 144 ran anyway, so the diagnostics this rewrite is written against
are 144's improved ones rather than the migrated checker's. What did survive is the half that matters here. The registry
the anchor change lands in is the one prompt 141e translated, uncollapsed, because prompt 164 waits on this one: its
Check asserts a green staff budget class, and the thirty staff budget tests 142's Check names are red until this prompt
turns them green. Neither reordering changes the gate below: the rewrite is measured against 2,404 lines and prompt
132's prediction, whenever it runs.

**Five things must be gone, and each is checkable.** Zero `callN` helpers. Zero hand-allocated role integers. Zero
string dispatch on token kinds or delimiters. `Pending` as a record with named fields rather than an eight-field
destructure. And the reading algorithm running forwards, because prompt 141 gave it a way to accumulate. Write each as a
test or a grep in the Check, not as a claim in the commit message.

**`syntax_anchor` loses its place argument, and the evaluator derives it the way a quote does.** Of the two shapes the
Read section names, only one survives its own consequences. Keeping the argument means giving authors an operation that
mints a fresh place — and that operation is `syntax_built` under a new name, which `11-quotation.md` §5 deletes and
`43-dependent-language-trial.md` §13 records as a falsifier the pass did *not* fire ("a hand-written provenance path, or
a role integer by another name — no"). Re-introducing one to serve a single builtin would reverse the pass's own result
in the last prompt that could still be said to have measured it. So the shape becomes `syntax_anchor(subject, path)`,
answering the anchor of the input node at `path` as before, and the place its answer stands at is
`Derived { origin: path, quotation: <a reserved site>, path: [0] }` — the origin being the node the anchor is *about*,
and `[0]` because a site's outermost node is `[0]` and this site builds exactly one node.

    *Repaired during implementation.* The sentence this replaces drew the site index from `Resolver::next_quotation`,
    "the same counter `syntax_quote` draws from". The code said no: the anchor executes as a δ rule,
    `musa_calculus::Rule` is `fn(&[Datum]) -> Option<Answer>` with no compiler state by design (D3 is a property of the
    type), and since prompt 142 the adapter's calls never pass through the lowering that owns the counter. The
    mechanism is the one `48-the-anchors-place-without-a-name-supply.md` argues from Peyton Jones ch. 9 and Idris2's
    `UST.nextName`: the place derives from the arguments alone — `origin` is the anchored node's path, already unique
    per node, and `quotation` is `u32::MAX`, a reservation the upward-counting quote counters never draw. Disjointness
    by construction rather than by allocation; `PhaseFamily::Builder`'s "no counter, no clock, no compiler state"
    stays true of the anchor, and that Target item's doc repair falls out.

**That does not remove the one-call-per-anchor obligation; it makes it the obligation every quote already carries.** Two
calls of one anchor site at one `here` still mint one path, exactly as calling a helper whose body is
`quote at here { … }` twice with one `here` does. What changes is that this stops being a fact an author has to know
about one builtin and becomes the derived-identity law of `11-quotation.md` §3, reported by the same duplicate-path gate
in the same words. Two *different* call sites at one `here` no longer collide at all, which the hand-allocated role
could only achieve by the author keeping twenty-seven numbers apart by hand.

**The answer stays at `Syntax<TokenTree>` and the call site keeps its `as_expression`.** The trial's §1.1 assumed both
changes at once and wrote `anchored` as if the anchor arrived at `Syntax<Expr>`; the fixture's note is right against it.
An anchor is a token, an emitted call needs an expression, and the operation that turns one into the other is the
checked parse — the same route the composer's own spliced pitch takes, which is §1.2's general finding rather than a
special case. Making the builtin answer at `Expr` would put a parse inside a builtin to save one `match` in one helper.
This prompt changes the arity and nothing else about the signature.

**What moves with it, and each of these is a Target item, not a side effect.** The registry entry, declared type, doc
comment, and evaluator arm in `crates/musa-compiler/src/phase/mod.rs`; `PhaseFamily::Builder`'s doc comment, which
currently says every builder is "a function of its displayed arguments and nothing else — no counter, no clock, no
compiler state", and is no longer true of the anchor — repair it the way `CheckedSyntaxQuote` is already argued, since
which site wrote a node is a fact about the *program* and two runs of one program still agree exactly; the two
`syntax_anchor` unit tests in `crates/musa-compiler/src/expand/mod.rs` around lines 1628–1652, which hold the law that a
derived path anchors to nothing; `stdlib/src/adapters/doubled.musa`'s one call site, which no other prompt owns and
which is a mechanical two-line edit rather than a rewrite — `doubled` keeps its `syntax_built` calls, because it is the
flat fixture and is not being moved onto quotation; `tests/fixtures/staff-construction.musa`'s `anchored` and its header
note, whose recorded workaround is discharged here; and the two documents that name the wrong prompt,
`docs/rules/language/11-quotation.md` §5 and `45-phase-registry-survey.md`'s `syntax_anchor` row.

**The compatibility oracle: check it, and expect it not to move.** An anchor *number* is a node's position in the
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
- `syntax_anchor` at two arguments: the registry entry, declared type, doc comment, and evaluator arm in
  `crates/musa-compiler/src/phase/mod.rs`, with the site index drawn from `Resolver::next_quotation`;
  `PhaseFamily::Builder`'s doc comment repaired; and `crates/musa-compiler/src/expand/mod.rs`'s two anchor unit tests
  rewritten at the new arity with the law they hold unchanged.
- `stdlib/src/adapters/doubled.musa`'s call site moved to the new arity, and nothing else in that file touched.
- `tests/fixtures/staff-construction.musa`'s `anchored` written the way prompt 132's trial wrote it, and its header note
  (lines 28–41) reduced to whatever workaround genuinely remains after prompt 142 and this prompt.
- `docs/rules/language/11-quotation.md` §5 and `docs/notes/research/language-design-closure/45-phase-registry-survey.md`
  repaired to name the prompt that actually removes the argument.
- The oracle check recorded: manifest, rendered corpus, and snapshots compared, and the result stated rather than
  assumed.
- **Every test in the staff budget class prompt 142's Check names, green.** That class is the reason this prompt moved;
  the two tonal examples it names beside the staff class stay red here and stay 144's.

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

| Tests | What they read |
| --- | --- |
| `staff_expansion_laws::*` (25) | a staff region through `std::adapters::staff` |
| `staff_writing_laws::a_printed_page_says_what_the_value_said` | the same, printed back |
| `large_score_generators::*` (2), `session_laws::the_staff_page_example_compiles_and_renders` | `examples/staff-page.musa` |
| `musa::cli wav_export_is_deterministic_for_all_examples` | the same example, exported |
| `elaboration_fixture_generators::the_pressure_workloads_compile_and_denote_what_they_claim` | `core-pressure.musa`, no adapter involved |

**There are two walls, and the near one is nesting, not steps.** The failure reported today is `evaluation exceeded the
budget for nested evaluation levels at 257 of 256` — `Budget::NESTING`, not `Budget::LANGUAGE`'s step count. Raising
only the nesting limit does not make the class green: the compile then runs past 2×10¹⁰ reduction steps without
finishing, which is `budget.rs`'s own recorded measurement ("the staff adapter's expansion run exceeds 4×10⁹ steps")
seen from the other side. So this prompt owes *both*: an adapter whose expansion is shallow enough not to stand 257
evaluator frames deep, and one whose step count is inside 200,000. A rewrite that clears only the first will report the
second and still be red.

**The budget does not move.** `crates/musa-calculus/src/kernel/budget.rs` states that in as many words and this Check
does not re-open it.

The tonal class this Check used to name — `diatonic-sequences` and `rule-of-the-octave` exhausting the step budget with
no adapter involved — **closed itself**; neither appears in the measured failure list, and prompt 165's note records
that. What stands in its place is `core-pressure`, which is prompt 165's own pressure fixture and hits the *nesting*
wall at the same 257 of 256. It is listed above because it is the one member of the class the staff adapter does not
explain: if the rewrite makes `core-pressure` green too, the wall was the checker's notion of a nesting level and 144
should say so; if it does not, `core-pressure` is 144's to carry and every other row here must be green. Any other red
means this prompt is not done.

The oracle stays fixed: a rewrite of a library file has no business changing a semantic hash or a rendered corpus file,
and the Design section's analysis says the anchor's derived path is not observable in any of them. If one moves, stop
and report it rather than re-baselining.

Commit as `Rewrite the staff adapter on the new language`.

## Stop

- No new notation coverage, no new diagnostic, no changed anchor, no changed conformance level. The adapter does the
  same job.
- No change to `crates/` beyond `syntax_anchor`'s arity, which the Design section argues in advance and the Target
  lists. Anything else a library rewrite turns out to force is a finding worth reporting rather than a quiet commit —
  the whole claim is that this file is ordinary unprivileged Musa.
- No operation that mints a `NodePath` — not a renamed `syntax_built`, not a `here` that an author derives, not a "fresh
  place" helper in `stdlib/`. If the rewrite needs one, the argument in the Design section is wrong and this prompt says
  so rather than adding it.
- No other phase-operation deletion. The builders `11-quotation.md` §5 retires still have callers in
  `stdlib/src/adapters/doubled.musa`; removing them belongs to the prompt that audits the registry, not to this one.
- No weakening or rewriting of an existing staff test to accommodate the rewrite.
- No adjustment of prompt 132's prediction. It is a fixed gate.
- No studio work. Prompt 167.
