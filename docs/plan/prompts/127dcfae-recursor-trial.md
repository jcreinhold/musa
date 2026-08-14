---
id: 127dcfae
slug: recursor-trial
status: pending
depends_on: [127dcfad]
phase: 3
---

# Paper-Trial the Sealed-Step Recursor Before Any Code Implements It

## Task

Prompt 127da made `fold_syntax` the only way into a syntax value, and the staff trial found the cost: a bottom-up
catamorphism cannot inspect a node before deciding whether, in what order, and under what context to read its children,
so the adapter encodes context, lookahead, order, and failure in a seven-slot `Pending` record and a closure chain.
Research note 39 §5 proposes replacing the primitive with an inherited-context recursor over **sealed steps**.

That proposal has never been written against a program. This prompt writes it against five, on paper, and freezes the
exact surface only if all five pass. Nothing here touches code. Its purpose is to make the following prompt's
implementation a transcription rather than a design.

## Read

- `docs/notes/research/language-design-closure/39-totality-and-structural-abstraction.md` §5 in full — the phenomenon
  (§5.1), the sealed-step interface and its intrinsic equations (§5.2), the eleven laws (§5.3), the five representative
  programs (§5.4), and §12.1's falsifier. §5.2's "Correction to the first draft" is the reason a step seals its child
  *and* its runner together, and this trial must not quietly unseal them.
- `docs/notes/research/language-design-closure/27-adapter-trials.md` — the precedent for a paper trial that decides an
  interface: complete programs, no ellipses, and findings that are permitted to fail the interface.
- `docs/notes/research/language-design-closure/26-language-design-decision.md` §§9, 10 — the admission conditions and
  the proof obligations this trial's output must be able to feed.
- Prompt [127da](127da-path-aware-syntax.md), especially the paragraph beginning "The fold is the only way into a syntax
  value" — the design law this trial tests, and the path, `SourceInfo`, and builder rules that must survive intact.
- Prompt [127dcfa](127dcfa-staff-expansion.md) and `stdlib/src/adapters/staff.musa` — the implemented trial and the
  machinery at issue: `data Pending` (line 274), `from_the_end` (1871), `read_body` (1941), and `document_read` (1988).
- Prompt [127dcg](127dcg-studio-trial.md) — the studio adapter this trial must write on paper before that prompt builds
  it, so the interface is not fitted to one adapter.
- `docs/rules/language/02-core-calculus.md` §5, §5.5, and §5.8 — the closed type grammar, strong normalization, and the
  four builtin families. A sealed step is a new phase-local type, and the trial must state exactly where it sits in
  each.
- Prompts [127dcfab](127dcfab-expression-if.md), [127dcfac](127dcfac-record-update.md), and
  [127dcfad](127dcfad-result-question.md) — landed before this trial deliberately, so their measurements are already in
  hand and this trial cannot credit the recursor for what they did.

## Design

**Write five complete programs, with no ellipses.** Note 39 §5.4 names four; hostile nested traversal is the fifth and
is the one that killed the first draft.

1. **Staff, simplified.** A representative slice of the current expansion — enough to cover right-to-left tie reading
   and right-nested output — with `C` the meter/open-form state and `A` the reader's result. The question it answers:
   does inherited context remove the `Pending`-plus-closure encoding, or merely relocate it?
2. **Studio.** A node declaration whose header selects the parameter grammar for its body. The question: does selective
   descent help, or is a derived bottom-up fold clearer here? An honest "the fold was better for studio" is a finding,
   not a failure, and it must be reported as staff/studio asymmetry rather than smoothed over.
3. **Source-preserving edit.** Stop after the anchored child, with the path law still identifying the edit locus.
4. **Degenerate leaf.** `Missing`, token, and identifier. Their termination argument is immediate and the trial states
   it, because a degenerate case that is merely obvious in prose is where a law hides.
5. **Hostile nested traversal.** An inner `recurse_syntax` invoked from inside an outer group branch, choosing the same
   `C` and `A`, capturing an outer step, and attempting to run it from a descendant callback. Under the first draft's
   separated child-and-descender this was self-descent with agreeing types. The trial must show that sealing makes the
   attempt either ill-typed or harmlessly equal to running the outer step, with no dynamic owner check and no failure
   result — and if it cannot, say so.

**Check every law against every program.** Note 39 §5.3's eleven laws are the frozen list. For each, the trial says
which program exercises it and what executable evidence prompt 127dcfaf will owe. A law no program exercises is either
unnecessary or the program set is incomplete, and the trial must say which.

**Discharge structural decrease explicitly, including capture.** Law 5 claims a step application enters a strict subtree
"even when the step is captured, invoked later, or invoked from a nested traversal." That is the whole termination
argument and it is the claim most likely to be wrong. Write it as a measure argument over `Syntax` with the capture and
nesting cases discharged, in the shape §5.5's strong-normalization measure will need.

**Settle what §12.1 leaves open.** Exact names, argument order, whether a child's path arrives beside its step or only
at the resumed branch, whether `run_syntax_step` is a builtin or callable syntax, and whether the derived `fold_syntax`
stays public. Each is settled by what the five programs read best as, with the losing option recorded.

**The stop condition is real.** If sealed steps still force staff or studio into the same higher-order state machine, or
if structural decrease under nesting cannot be discharged without a dynamic failure result, this prompt does not freeze
an interface. It records the finding, repairs note 39 §5, and stops — implementation does not begin on a design the
paper trial rejected.

## Target

- `docs/notes/research/language-design-closure/40-<slug>.md` and its entry in that directory's `README.md`: the five
  complete programs, the law-to-program table with the evidence each will owe, the structural-decrease argument with its
  capture and nesting cases, the settled surface with the options it beat, and any staff/studio asymmetry.
- The frozen interface, exactly: the recursor's branch signatures, `SyntaxStep<C,A>`'s kinding and its exclusion from
  `d`, `run_syntax_step`, the intrinsic equations, and the derived `fold_syntax` at `C = Unit`.
- A repair to note 39 §5 where the trial contradicts it, marked as a correction rather than folded in silently.
- If the trial fails: the finding, the smallest program that exhibits it, and no frozen interface. Say so in the report
  and stop.

## Check

```sh
python3 scripts/renumber-prompts.py audit
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

This prompt writes no Rust, so the four crate commands have nothing to run; that is the honest Check for a paper trial,
not a weakened one. Prompt 127dcfaf owes the executable evidence.

Commit as `Paper-trial the sealed-step recursor before any code implements it`.

## Stop

- No implementation. No change under `crates/`, `stdlib/`, `editors/`, or `examples/`.
- No amendment to `docs/rules/`. Prompt 127dcfaf amends the candidate language pages as its own Task, in
  rules-before-code order; a paper trial that amends a rule has skipped that step.
- No public `Syntax` type, no parent/sibling cursor, no quotation or antiquotation, no text-to-syntax, no fresh-name
  operation. §5.4's closing paragraph is the boundary.
- No general recursion, no `fix`, no higher-rank polymorphism, and no higher-kinded type variable. If the interface
  needs one, that is a finding and a stop, not a design freedom.
- No container abstraction, no `Listing`/`Building` generation, and no dependent typing. Note 39 §§6 and 8 decided those
  and this trial does not reopen them.
- No new adapter privilege. An adapter that needs one is evidence against the boundary and is reported as such.
