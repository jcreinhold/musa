# The musa documentation

Four directories, and which one a document is in tells you what force it has.

| Directory | What it is | Force |
| --- | --- | --- |
| [`rules/`](rules/README.md) | What musa must be: the constitution, the rules that cross stages, and one specification per stage | **Governing.** Code that disagrees is wrong |
| [`plan/`](plan/README.md) | What to build, in what order, and where the code has actually got to | **Directive.** A plan that contradicts a rule is repaired before it is worked |
| [`book/`](book/src/introduction.md) | How to use musa: tutorials, guides, explanation, reference | **Teaches.** Decides nothing |
| [`notes/`](notes/README.md) | Research, decision records, and toolchain traps | **Governs nothing.** Kept so a refuted claim stays next to its refutation |

## Which document wins

Read top to bottom. A document is bound by everything above it and binds everything below it.

1. [`rules/constitution.md`](rules/constitution.md) and [`rules/obligations.md`](rules/obligations.md) — the few
   decisions every part of musa follows, and what falls out of them. Nothing overrides these; they change only through
   the amendment procedure in [`rules/README.md`](rules/README.md).
2. [`rules/across-stages/`](rules/across-stages/README.md) — the rules no single stage owns: what data exists, when it
   is valid, how one stage produces the next, what equality means.
3. [`rules/events/`](rules/events/00-purpose.md), [`rules/desktop/`](rules/desktop/README.md), and
   [`rules/style-guide.md`](rules/style-guide.md) — the per-stage specifications. Each owns its stage and defers to
   `across-stages/` at the boundaries.
4. [`rules/language/`](rules/language/README.md) — the source language. Still a **candidate**: where it and anything
   above it differ, the thing above wins, and the difference is a defect in the candidate to repair. Prompt 172's
   conformance audit is what graduates it.
5. [`plan/roadmap.md`](plan/roadmap.md) — the broad crate and product plan. Everything above refines it; where it and a
   specification above disagree, the specification wins.
6. [`plan/code-map/`](plan/code-map/README.md) and [`plan/prompts/`](plan/prompts/README.md) — what the code currently
   does, and the work queued to change it. If either disagrees with anything above, either the code is wrong or the
   document needs a deliberate repair; neither may drift silently.
7. [`book/`](book/src/introduction.md) and [`notes/`](notes/README.md) — teaching and research. Neither governs. Where
   the book and a specification disagree, the book has a bug.

## How to tell what is current

- **The directory is the status.** `rules/` governs, `plan/` directs, `book/` teaches, `notes/` records. You do not have
  to open a document to learn which of those it is.
- **Every index page leads with a `Status:` line** — the page this table links to, usually a `README.md`. If you are
  holding a document and cannot tell whether it governs, its index page's status line is the answer, and a missing one
  is a bug in the document.
- **Nothing superseded is kept.** A design that has been replaced is deleted, and the argument that replaced it goes to
  [`notes/research/`](notes/research/README.md) as a decision record. There is no archive directory and no "historical,
  non-governing" tier — if a document is here and is not under `notes/`, it is live.
- **Dead links are a build failure.** `scripts/check-docs.sh` checks every relative link and anchor across `docs/`,
  every `musa` code block against the fixtures in `examples/` and `stdlib/src/`, and every theory citation against the
  Open Music Theory corpus.

## Working on the documents

One prompt is one commit. If a prompt's design proves wrong, repair the prompt file first, commit the repair, then
implement. If code and a governing document disagree, one of them gets repaired rather than left to drift — see
[`../AGENTS.md`](../AGENTS.md) for the working conventions in full.
