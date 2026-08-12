# The kernel hypothesis

> **Read [06-evidence-log.md](06-evidence-log.md) first.** Two gates have been run. Gate 0 refuted Atom 3 and materially
> weakened Atom 2 — the two atoms `01`–`02` lead with. Gate 2 then withdrew Atom 2 outright: prompt 119 built seven
> voice-leading profiles on the existing per-note voice tag without adding a line relation, which closes
> `docs/rules/kernel/08-open-questions.md` Q3. Those documents are left as written, with corrections marked in place,
> because the log is only evidence if the claims it corrects are still visible.
> [07-adoption-plan.md](07-adoption-plan.md) is the resulting plan; what remains live there is Track A (Atoms 5 and 6)
> and Track B (Atom 4).

**Status: research. Governs nothing.** `docs/rules/kernel/` remains the governing temporal-kernel specification and
`docs/course-correction.md` remains the governing ontology. Nothing here changes what the compiler must do. This
directory exists to state a hypothesis precisely enough that it can be *refuted*, and to record the evidence for and
against it while that is still cheap.

## The hypothesis in one sentence

> Musa's kernel forgot three things the theory it serves treats as primary — **succession**, **alternative**, and
> **metrical layering** — and every open question in `docs/rules/kernel/08-open-questions.md` is a symptom of one of
> those three omissions.

The proposed repair is that a piece of music denotes a **labelled event structure over exact rational time**, of which
the current kernel's `(d, E)` — an extent and a multiset of occurrences — is one *configuration*: a single maximal
consistent run with its causal order forgotten.

## What this is not

It is not a proposal to make the kernel bigger because bigger is more expressive. `docs/rules/kernel/00-purpose.md`
quotes course correction §34 as the acceptance test, and this directory accepts that test unchanged:

> A construct belongs in the kernel only if removing it makes an important class of musical meanings impossible or
> unnatural to represent faithfully across multiple independent consumers.

Every atom proposed in `01-atoms.md` is argued against that rule, with a named falsifying example from the repertoire
and a named citation in Open Music Theory. An atom with no falsifier is not proposed.

It is also not a proposal to encode musical taste in a type system. `03-claims-and-styles.md` argues at length that
"prevent bad music by construction" is the wrong target and states the target that replaces it.

## Reading order

| File | What it settles |
| --- | --- |
| [00-constitution.md](00-constitution.md) | The amendments: what the music kernel *is*. Everything else is downstream. |
| [01-atoms.md](01-atoms.md) | Which primitives exist, the OMT evidence for each, and the falsifier that would remove it. |
| [02-denotational-semantics.md](02-denotational-semantics.md) | What a kernel object denotes, and the theorem relating it to today's kernel. |
| [03-claims-and-styles.md](03-claims-and-styles.md) | Why "prevent bad music" is wrong, and what dependent types are actually for here. |
| [04-operational-semantics.md](04-operational-semantics.md) | The term calculus, its reduction, and why it stays total. |
| [05-open-questions.md](05-open-questions.md) | What is deliberately undecided, and what evidence would settle it. |
| [06-evidence-log.md](06-evidence-log.md) | What has been run and what it did to the claims above. Append-only. |
| [07-adoption-plan.md](07-adoption-plan.md) | The path to implementation: gates, kill criteria, and where prompts go. |

Read `00` and `01` first. If `01`'s falsifiers do not convince you, nothing downstream will.

## The standard of evidence

Three kinds of claim appear here, and they are marked differently on purpose.

- **Cited.** A claim about music theory, with an Open Music Theory chapter given by filename. OMT is the authority this
  repo already cites; a claim about what musicians mean is checked against it, not asserted.
- **Derived.** A mathematical consequence of the definitions, stated as a numbered proposition. These are the claims
  that would be formalized first.
- **Judged.** A design choice among options the evidence leaves open. Every judged claim says what it is choosing
  against and what would change the choice.

A paragraph that is none of the three is a paragraph that should be deleted.
