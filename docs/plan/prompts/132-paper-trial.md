---
id: 132
slug: paper-trial
status: done
depends_on: [129, 130, 131]
phase: 3
---

# Paper-Trial the Whole Language Before Any Code Implements It

## Task

Prompts 129–131 specify a dependent core, records, enums, traits, operators, methods, quotation, and syntax patterns.
None of it has been written against a program. Write it against nine, on paper, and record the predicted size of the
staff rewrite — that number becomes prompt 165's gate. If any program needs something the specification refuses, this
prompt repairs 129–131 and stops; implementation does not begin on a design the trial rejected.

## Read

- Prompt [127dcfae](127dcfae-recursor-trial.md) and its output,
  `docs/notes/research/language-design-closure/40-sealed-step-recursor-trial.md` — the precedent for what a paper trial
  is: complete programs, no ellipses, a law-to-program table, and findings that are permitted to fail the interface.
  That trial's discipline is the standard here; the only thing that changes is the scope.
- `docs/notes/research/language-design-closure/41-staff-on-the-repaired-interface.md`, especially §7's finding that a
  list cannot be constructed. That note is the last time an interface was measured against this program, so its numbers
  are the baseline this trial predicts against.
- `docs/rules/language/02-core-calculus.md`, `01-surface.md`, `10-traits.md`, and `11-quotation.md` as prompts 129–131
  leave them, plus `42-dependent-core-decision.md` from prompt 128. This trial reads the specification as written, not
  as intended; a program that only works under a charitable reading is a finding.
- `stdlib/src/adapters/staff.musa` in full — the construction section, the dispatch table, `data Pending`, and
  `document_read`'s `call7`, which are the four pieces the trial rewrites.
- `docs/notes/research/language-design-closure/27-adapter-trials.md` and note 28's five programs — the existing trial
  corpus, so the new language is measured on programs chosen before it existed rather than on programs chosen to suit
  it.
- `stdlib/src/adapters/studio.musa` (or the superseded prompt [127dcg](127dcg-studio-trial.md)'s design if the file does
  not exist yet), specifically `validate` — the second adapter, which is what keeps the design from being fitted to
  staff.
- Peyton Jones ch. 3 and ch. 5, for the shape a translation from surface to core is expected to take. Every rewritten
  program in this trial is one, and a program whose desugaring cannot be written down is a specification defect.

## Design

**Write nine complete programs, with no ellipses.** Four rewrite the pieces of `staff.musa` that the failing-program
record in prompt 128 names, four come from the existing trial corpus, and one is the second adapter.

1. **Staff construction.** The whole `// ---- writing` section, on `quote at here { … }`. The question it answers: do
   `call1`–`call7` and every hand-written `syntax_group`/`syntax_token` assembly actually disappear, or do they come
   back as a quotation the author still has to nest by hand?
2. **Staff dispatch.** The token-kind table, as syntax patterns over `TokenTree`. The question: does a pattern quote
   decide the shape, and does anything still need `text_equal` on a token kind?
3. **`Pending` as a record.** Eight named fields, with the reads that currently destructure all eight written as
   projections and the updates as nested `with`. Count the lines both ways.
4. **`document_read`'s `call7`.** The single worst call site, rewritten. If it is not obviously shorter *and* obviously
   more correct, that is the whole design failing at the place it was designed for.
5–8. **Note 28's five programs**, minus any the corpus has since retired, each rewritten on the new surface. These
   were chosen against a different language, which is exactly what makes them evidence.
9. **The studio adapter's `validate`.** A materially different adapter, and the only thing in the trial that supports a
   generality claim rather than a staff claim.

**Check every mechanism against a program.** Build the table the recursor trial built: for each specified mechanism —
universes, Π, dependent records, inductive families, dependent match with coverage, `Id`/K, NbE conversion,
metavariables and pattern unification, well-founded termination, records, namespaced enums, traits and coherence,
operators, inherent methods, `Syntax<Cat>`, quotation, sequence splicing, derived provenance, syntax patterns — name the
program that exercises it and the executable evidence a later prompt will owe. **A mechanism no program exercises is a
mechanism to delete**, and the trial says which prompt deletes it. This is the cheapest moment in the whole pass to
remove something, and the specification is nine documents long because nothing has yet had to justify itself against
code.

**Record the falsifiers, and mean them.** The design is wrong, and this prompt repairs 129–131 rather than proceeding,
if any program needs:

- rank-2 or higher polymorphism, or a higher-kinded type variable that the specification does not give;
- a `partial` definition, general recursion, or a termination measure the checker cannot see;
- search during trait resolution — an overlapping instance, a default, or a resolution that depends on elaboration
  order;
- a hand-written provenance path, or a role integer by another name;
- an escape from `Syntax<Cat>`'s index, such as a cast between categories or an untyped `Syntax`;
- a string round-trip to say something the typed API should have said.

Each falsifier gets a yes or no with the program that decided it. "Probably not" is not an answer a later prompt can act
on.

**Predict the staff rewrite's size, in public.** State a predicted line count and byte count for prompt 165's
`staff.musa`, derived from the four rewritten pieces and stated with the extrapolation shown, not asserted. Record the
current 2,404 lines / 93,252 bytes beside it — the file as prompt 127dcfb left it, not the smaller figure note 41
measured before the `// ---- writing` section existed. Prompt 165 measures against this number, so a prediction that is
generous here is a gate that means nothing there. Say what would count as the design failing — a rewrite that is not
*dramatically* shorter — and put a figure on "dramatically".

**Two findings the trial should expect to make, and must not smooth over.** First, the K decision in 129: index
unification for `Vec` and for `Syntax<Cat>` is where K either pays for itself or turns out to be unnecessary, and the
trial is the last chance to drop it cheaply. Second, staff/studio asymmetry: if quotation is a large win for staff and
neutral for studio, that is a real result about adapters, and it belongs in the note rather than being averaged away.

**The stop condition is real.** If the trial rejects the design, this prompt's output is the finding, the smallest
program that exhibits it, and a repair to 129–131 — committed as the repair it is, under the prompt README's §6
procedure. Implementation does not start.

## Target

- `docs/notes/research/language-design-closure/43-<slug>.md` and its row in that directory's `README.md`: the nine
  complete programs, the mechanism-to-program table with the evidence each will owe, the falsifier list answered one by
  one, the predicted staff size with its derivation, the mechanisms marked for deletion, and any staff/studio asymmetry.
- Repairs to `docs/rules/language/02-core-calculus.md`, `01-surface.md`, `10-traits.md`, and `11-quotation.md` wherever
  the trial contradicts them, each marked as a correction with the program that forced it rather than folded in
  silently.
- A repair to `42-dependent-core-decision.md` if the trial changes an answer that record gave to note 39 §11.2.
- If the trial fails: the finding, the smallest failing program, no frozen design, and a stop. Say so in the report.

## Check

```sh
python3 scripts/renumber-prompts.py audit
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
/Users/jcreinhold/.cargo/bin/mdwright check docs/rules docs/plan
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
git diff --check
```

This prompt writes no Rust, so the four crate commands have nothing to run; that is the honest Check for a paper trial,
not a weakened one. Prompts 133–137 owe the executable evidence.

Commit as `Paper-trial the dependent language before any code implements it`.

## Stop

- No implementation. Nothing under `crates/`, `stdlib/`, `editors/`, `examples/`, `apps/`, or `packages/` changes — the
  rewritten programs live in the note, not in the file they rewrite.
- No amendment to `docs/rules/constitution.md` or `docs/rules/obligations.md`. Prompt 128 owns those, and a trial that
  amends the constitution to make its own programs pass has stopped being a trial.
- No new mechanism. This trial may delete from the specification and repair it; adding a feature because a program
  wanted one is how the specification got long enough to need a trial.
- No new prompt. If the trial finds work that 133–169 do not cover, it says so and the next prompt to run is a repair,
  written under the prompt README's §6 and §7 with that finding as its evidence.
