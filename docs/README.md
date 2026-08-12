# The musa documentation

Every directory here has one job and one status. This page says which is which, and which document wins when two
disagree.

## The map

| Path | Job | Status |
| --- | --- | --- |
| [`governance/`](governance/README.md) | The few decisions every part of musa must follow, and the rules that fall out of them | **Governing.** Amendable only by the six-step procedure in its README |
| [`spec/`](spec/README.md) | Formal rules at the boundaries *between* stages: what data exists, when it is valid, how one stage produces the next, what equality means | **Governing** |
| [`kernel/`](kernel/00-purpose.md) | The finite temporal kernel — exact time, typed occurrences, `timeline`/`sequence`/`overlay`, normalization, the backend contract | **Governing** |
| [`interface/`](interface/README.md) | The desktop interface: visual language, engraving quality, interaction, states, performance budgets | **Governing** since prompt 26 |
| [`language/`](language/README.md) | The source language above the kernel and the sound pipeline after it | **Candidate.** Not governing until prompt 146's conformance audit is green |
| [`roadmap.md`](roadmap.md) | The broad crate and product plan: layers, crate ownership, DSP rules, what is deliberately rejected | **Governing** where the documents above are silent; refined by them where they are not |
| [`style-guide.md`](style-guide.md) | `.musa` naming and spelling — what the formatter cannot say. Its machine-checkable subset is the lint pass | **Governing.** `crates/musa-compiler/src/lint.rs` cites its sections by number in diagnostics |
| [`architecture/`](architecture/README.md) | Which crate implements which stage, and what is implemented, partial, or absent | **Descriptive.** Reports on code; never decides semantics |
| [`prompts/`](prompts/README.md) | The numbered work plan, executed in dependency order, one prompt per commit | **The work plan.** A prompt that contradicts a governing document is repaired before it is implemented |
| [`book/`](book/src/introduction.md) | How to *use* musa: tutorials, how-to guides, explanation, reference. Built with `make docs` | **User-facing.** Teaches; does not decide |
| [`scratch/`](scratch/README.md) | The research notebook: candidates, metatheory, proof reviews, and the arguments that failed | **Governs nothing.** Kept so a refuted claim stays visible next to its refutation |

## Which document wins

Read top to bottom. A document is bound by everything above it and binds everything below it.

1. **`governance/`** — the core decisions. Nothing overrides these; they change only through their own amendment
   procedure, which requires a musical or engineering reason, the examples that break, and a record in `scratch/`.
2. **`spec/`** — the cross-stage rules. Refines `governance/` with judgments, derivations, process semantics, and
   identity.
3. **`kernel/`**, **`interface/`**, **`style-guide.md`** — the governing per-stage specifications. Each owns its own
   stage and defers to `spec/` at the boundaries.
4. **`language/`** — the source language. Still a *candidate*: where it and anything above it differ, the thing above
   wins, and the difference is a defect in the candidate to repair.
5. **`roadmap.md`** — the broad plan. Everything above refines it. Where it is silent, it is silent; where it and a
   specification above disagree, the specification wins.
6. **`architecture/`** and **`prompts/`** — descriptions and work. If either disagrees with anything above, either the
   code is wrong or the document needs a deliberate repair; neither may drift silently.
7. **`book/`** and **`scratch/`** — teaching and research. Neither governs anything. Where the book and a specification
   disagree, the book has a bug.

## How to tell what is current

- **Every index page above leads with a `Status:` line.** That is the page this table links to — usually the directory's
  `README.md`, `kernel/00-purpose.md` for the kernel. If you are holding a document and cannot tell whether it governs,
  its index page's status line is the answer, and a missing one is a bug in the document.
- **Nothing superseded is kept.** A design that has been replaced is deleted, and the argument that replaced it goes to
  `scratch/` as a decision record. There is no archive directory and no "historical, non-governing" tier — if a document
  is here and is not in `scratch/`, it is live.
- **Dead links are a build failure.** `scripts/check-docs.sh` checks every relative link and anchor across `docs/`,
  every `musa` code block against the fixtures in `examples/` and `stdlib/src/`, and every theory citation against the
  Open Music Theory corpus.

## Working on the documents

One prompt is one commit. If a prompt's design proves wrong, repair the prompt file first, commit the repair, then
implement. If code and a governing document disagree, one of them gets repaired rather than left to drift — see
[`../AGENTS.md`](../AGENTS.md) for the working conventions in full.
