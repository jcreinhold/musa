# The core calculus

**Status: research. These notes do not set Musa's rules.**

This line of work asked one question:

> What is the smallest typed language that can build written music, connect it to instruments and effects, and account
> for the sound that runs from the result?

It answered it in two parts — `EventTrack<C, A>` for finitely many occurrences placed in a finite duration, and
`Machine<K, A, B>` for a finite deterministic machine that may keep taking steps, with `schedule` as the checked
connection between them. That answer is what `../../../rules/constitution.md` §4–§5 and `../../../rules/events/` now
state, so the specification is the place to read it. These pages are kept only for what the specification cites.

| Page | Why it is kept |
| --- | --- |
| [The selected calculus](05-selected-calculus.md) | The frozen typing and execution rules the two pages below are about |
| [Proof outline](06-proof-outline.md) | §2 is the standing proof of termination and of resource failure not changing an accepted value, cited by `rules/across-stages/05-metatheory.md` §1 |
| [Final review](17-final-review.md) | The verdict `rules/across-stages/05-metatheory.md` names: no fatal, high, or medium error in the frozen definitions or conditional theorems |
| [The vocabulary amendment](18-vocabulary-amendment.md) | Position split from duration, primitive from builtin. Cited by `rules/README.md` as an amendment record |
| [The events vocabulary amendment](23-events-vocabulary.md) | Retiring *kernel* as a second name for the event track. Cited by `rules/events/00-purpose.md` |

The drafts, the five design audits, and their repairs are in the history of this directory. They were deleted once the
calculus they converged on became the specification; a design that has been replaced is not kept beside it.
