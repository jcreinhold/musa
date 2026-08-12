# The motive question

## 1. The question

Musa computes several different things from one source. Each is a *realization*: a functor from pieces of music to some
category of artifacts.

| Realization | Lands in | Crate |
| --- | --- | --- |
| Engraving | MEI, LilyPond, MusicXML — notated symbols with spelling, stems, beams, barlines | `musa-render` |
| Performance | gestures and control curves on exact rational time | `musa-compiler` |
| Sound | a sample stream | `musa-audio`, `musa-engine` |
| Analysis | harmonic function, set class, voice-leading verdicts, form | `musa-compiler::analysis` |
| MIDI | note numbers, velocities, physical onsets | `musa-render` |
| Identity | the semantic hash — when two pieces are the same piece | `musa-kernel` |

> **The question.** Is there an object `M(p)` such that every realization factors through it — `p ↦ M(p) ↦ R(p)` for
> each `R` — and which carries *exactly* what the realizations jointly need and nothing else?

Today's answer is "`Timeline[ScoreFact]`, and we hope." That is a candidate, not a derivation. Nobody has checked that
every realization factors through it, and §1 of `docs/core-boundary.md` shows at least one that does not: sound does not
factor through a finite timeline, which is why prompt 126 had to erect a boundary rather than exhibit a factorization.

## 2. Why this is not an analogy

The temptation is to say "the kernel is like a motive" and stop. That is decoration, and
`~/Code/proofs/.claude/skills/rethink-math/references/working-rules/core-rules.md` names the failure mode: *conviction
is not clearance*. What makes the motive question answerable is not the analogy but the method, which
`~/Code/papers/category-theory/50-examples-for-the-motive/text.md` states operationally:

> After completing B, write the smallest description that all six analyses agree on. This is the information that
> survives every projection. State it as concretely as possible — not "a computation" but "a linear chain of two reads
> on `b` followed by a consuming freeze."

So the method is empirical and bottom-up: take musical examples, run each realization over each, write down what each
one extracts, and read the residue off the table. [01-realizations-and-residue.md](01-realizations-and-residue.md) is
that table. Every candidate in this directory is answerable to it.

## 3. The three tests

Any proposed motive must pass all three. Each is stated so that a specific example can fail it.

### T-1. Factorization

For every realization `R` in the table above, exhibit the factorization `R = R' ∘ M`, or name the realization that does
not factor and say why.

*Falsified by:* a realization that needs something `M` does not carry. Sound currently falsifies
`M = Timeline[ScoreFact]` in exactly this way, and [02](02-candidate-polarity.md) is the attempt to fix it without
moving signals into the kernel.

### T-2. No side conditions

Grothendieck's own test, as `~/Code/papers/category-theory/grothendieck-method/process.md` states it:

> If your proof requires a side condition that does not appear in the theorem statement, your definitions are at the
> wrong level. If two theorems in different domains have the same shape but no common generalization, you have not yet
> found the right setting.

*Falsified by:* any law of the motive that holds only under a proviso. `docs/kernel/04-algebraic-laws.md` L18 is exactly
such a law today, and [04](04-candidate-fibred.md) argues that this is not a wart to be tolerated but the single most
informative fact in the law list.

### T-3. Tight fit, in both directions

The motive must be neither too coarse nor too fine.

- **Too coarse** — it forgets something some realization needs. *Falsified by:* two pieces that `M` identifies and some
  realization distinguishes. `docs/scratch/kernel-hypothesis/01-atoms.md` Proposition A is an attempt at exactly this
  falsification (parallel motion versus voice exchange), and `06-evidence-log.md` G0.2 records that it fails against the
  real pipeline because the voice tag rides in the payload.
- **Too fine** — it carries something no realization reads. *Falsified by:* a field of `M` that no realization consumes.
  This direction is much less often checked and is the cheaper of the two audits; `audit-module.sh`'s "no public item
  without a caller" is the same discipline one level down.

## 4. What an answer looks like

Not a slogan. A four-part statement:

1. The carrier — what `M(p)` is, as a mathematical object, in enough detail that two people would build the same Rust
   type from it.
2. The morphisms — what a map between two pieces is. This is the part everyone skips, and it is where the motive analogy
   earns its keep or fails: Grothendieck's motives are built by replacing *functions* between varieties with
   *correspondences*, and musical relationships (theme and variation, subject and answer, model and sequence) are
   correspondences rather than functions. If the morphisms of the candidate are just functions, the candidate is
   probably a data structure and not a calculus.
3. The realizations — each factorization, exhibited.
4. The invariants — what is preserved, which is what the semantic hash should hash.

## 5. Hottest iron, deferred deliberately

Naming what is being put off, so that deferral does not pose as closure:

- **The morphisms are barely treated in this directory.** Every candidate here specifies a carrier well and morphisms
  poorly. That is the next `yin` target and it is hotter than any remaining work on carriers.
- **Form** (AABA, sonata, verse–chorus; OMT `086`–`088`) resists every candidate here. All five candidates model *what
  sounds when*; none models *what a passage is a repetition of*, which is the whole content of form. Correspondences are
  the obvious tool and it is not obvious they are enough.
- **Timbre** is not modelled by anything here and is not obviously part of the motive at all (OMT `114`–`115` treat
  orchestration as its own discourse).
