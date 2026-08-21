# Decision records

**Status: governs nothing.** `docs/rules/` holds the decisions; these pages hold the arguments behind them. A page is
here because something binding cites it, and for no other reason.

## What belongs here

One page per decision that has already landed somewhere binding, written after it landed. Not a working notebook: a
candidate still being weighed, a draft under review, and a sieve of alternatives belong in a branch or a scratch
directory until the decision they support is made, and then one record of it arrives here.

That rule is a repair. This directory previously kept every candidate beside its refutation, on the argument that an
argument is worth more next to the thing that killed it. Over three design lines it reached 159 files and 44,000 lines,
almost all of it reachable only from itself, and a reader could no longer tell a live decision from a dead draft. The
refutations are in git history, where a superseded design belongs.

**A page leaves when its last citation does.** If nothing under `../../rules/` or `../../plan/` names a page any more,
the decision it records has been absorbed or reversed, and the page goes.

## The language

| Page | What it decided |
| --- | --- |
| [26](language-design-closure/26-language-design-decision.md) | The elaboration-language candidate: what the source language is, and the stage boundaries it elaborates across |
| [27](language-design-closure/27-adapter-trials.md) | That one adapter interface serves both notation and sound |
| [33](language-design-closure/33-metatheory.md) | The metatheory of the inferred language — prompt 169's conformance list is drawn from it |
| [41](language-design-closure/41-staff-on-the-repaired-interface.md) | The staff adapter measured per repair, which is the baseline prompt 166 is answerable to |
| [42](language-design-closure/42-dependent-core-decision.md) | The dependent core: the evidence, the cost, and the refusals prompt 128's amendment overturned |
| [43](language-design-closure/43-dependent-language-trial.md) | The ten-program trial of that core, run before any code implemented it |
| [44](language-design-closure/44-audit-against-smalltt-and-peyton-jones.md) | The audit against smalltt and Peyton Jones, and the diagnostics work prompt 165 owes |
| [45](language-design-closure/45-phase-registry-survey.md) | The phase registry entry by entry, after the syntax index |
| [46](language-design-closure/46-collections-and-the-vec-answer.md) | That `Vec A n` does not ship, and the condition that re-opens it |
| [47](language-design-closure/47-diagnostics-about-another-document.md) | Carrying a diagnostic about a document the composer did not write |
| [50](language-design-closure/50-the-course-correction-audit.md) | The course correction: the audit of what the dependent core cost, and the plan that deleted it |
| [51](language-design-closure/51-the-terseness-audit.md) | What the correction got right, and its three missteps — the stratified index, bounded argument reordering, and written sections |
| [52](language-design-closure/52-the-musical-algebra.md) | The algebra the language has to be able to say: torsors, group actions, orbits, and laws decidable by enumeration |
| [60](60-language-decision-record.md) | How the elaboration language was decided, and then corrected |
| [61](61-core-boundary-decision-record.md) | How the core boundary was decided |
| [62](62-course-correction-decision-record.md) | The course correction, and where each of its sections went |

## The core calculus

[`core-calculus/`](core-calculus/README.md) — the event track and the machine, the two core values, with the proof
outline and final review that `../../rules/across-stages/05-metatheory.md` §1 cites, and the two vocabulary amendments
`../../rules/README.md` and `../../rules/events/00-purpose.md` cite.

## Earlier evidence

[`kernel-hypothesis/06-evidence-log.md`](kernel-hypothesis/06-evidence-log.md) — the repertoire evidence behind the
event-track atoms, cited by `../../rules/events/08-open-questions.md` on when demand precedes design. The rest of that
line, and the K₁/K₂/K₃ and T₂ lines that followed it, are in git history.

## Standard of evidence

A claim in one of these pages is one of three things, and says which:

- **Cited** — a claim about music theory, with an Open Music Theory chapter given by filename.
- **Derived** — a mathematical consequence of stated definitions, as a numbered proposition.
- **Judged** — a design choice the evidence leaves open, saying what it chooses against.

And two rules the design lines were held to, worth keeping because they are what made the records usable:

- **Precise enough to be wrong.** A claim no example could falsify does not get written down.
- **Report what fell out, and what required side conditions.** A side condition is the sign that a definition is at the
  wrong level.
