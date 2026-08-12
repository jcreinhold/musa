# Claims, styles, and what type theory is actually for here

The prompt behind this directory was: *what if you could prevent bad music by construction?* This document answers that
the target is wrong in a specific, fixable way, states the target that replaces it, and then draws the line between what
type theory should do in a music language and what it should not — because the second list is longer than the first and
pretending otherwise is how the project would waste a year.

---

## 1. The textbook refutes the goal, twice, on its own pages

OMT `024-first-species-counterpoint.md` states the rule flatly:

> Parallel perfect consonances are forbidden.

OMT `069-chromatic-sequences.md`, discussing a sequence of German augmented-sixth chords, observes that it produces
parallel fifths between each successive chord — and notes that Chopin, in the Mazurka Op. 30 No. 4, did not seem to
mind. Same textbook. Same interval. One chapter forbids it; another finds it in the canon and moves on.

This is not an inconsistency in OMT. It is the structure of the subject. `030-16th-century-contrapuntal-style.md` makes
the relativization explicit — avoid parallels "as in later idioms," but with different exceptions than later idioms have
— which is a rule *indexed by style*, written as such.

> **Consequence.** A type system that makes parallel fifths unrepresentable rejects Chopin. A type system that makes
> them representable prevents nothing. There is no third option at the level of types, because the rule is not a
> property of the objects; it is a property of a practice.

The project already knows this. `docs/rules/language/03-musical-domains.md` §5 says counterpoint rules are style-indexed
assertions rather than universal laws. Amendment VI of `00-constitution.md` promotes that from a local decision to a
constitutional one, so that no future prompt can quietly re-litigate it by arguing that some particular rule is
universal enough.

---

## 2. The target that replaces it

> **Prevent false claims about music, not bad music.**

The failure mode a composition system can actually eliminate is not "you wrote something ugly." It is "the system told
you something untrue, or told you something true while hiding what made it true." Three concrete failures, all of which
the current architecture can still produce:

- A verdict with no derivation. "Error: parallel fifths" as though it were a fact about the notes rather than a
  consequence of a style the user did not choose.
- A claim carried across an equivalence that does not preserve it. Respelling C♯ as D♭ and keeping a voice-leading
  verdict that depended on the spelling.
- A structural claim recovered from a convention rather than read from the object. Beaming that depends on payload
  metadata that some other consumer set by a different rule (`01-atoms.md` §8.3).

Each of these is preventable by construction, and each of them is preventable by *type*-level construction. That is the
honest scope.

---

## 3. Three tiers, and what belongs in each

### Tier 1 — Type-level, universal, unconditional

Statements that are true in every style because they are true of the objects. These are made unrepresentable.

| Prevented | Why it is universal |
| --- | --- |
| Adding two pitches | `Pitch` is a torsor over `Interval`; a torsor has no origin (`01-atoms.md` §5) |
| Adding a point in time to a point in time | Same law, on the time coordinate |
| A "row" that is not a bijection on the twelve classes | It is the definition (OMT `108`) |
| An event with negative extent | `s ≤ t` in E0 |
| One event in two lines at once, or a line that is not a chain | It is what a line *is* (`02-denotational-semantics.md` §1) |
| A claim about a quotient that is not invariant under the quotient's group | Amendment V |

The last row is the only one that needs more than ordinary static typing, and it is the subject of §4.

### Tier 2 — Style-indexed propositions, checked with evidence

Everything a musician would call a rule. A style is a named theory; a claim is a proposition in it; a check produces a
derivation, not a verdict. `Analysis<S>` is the type of a claim in style `S` together with what justifies it.

The user-visible consequence, which is where "impeccable user experience" is decided: the system never says *error:
parallel fifths*. It says *in strict first-species counterpoint, bars 3–4 move in parallel fifths* — with the style
named, the rule cited, the two events highlighted, and the derivation readable in the Origin view that
`docs/rules/desktop/` already specifies. Switching to the sixteenth-century style changes which exceptions apply.
Switching to no style at all silences it, without switching off anything else.

This is also the answer to why the tiers must be visibly different in the UI. A Tier 1 violation is a red squiggle that
cannot be dismissed. A Tier 2 finding is an annotation attributed to a style, and it must never look like the first,
because looking like the first is the false claim.

### Tier 3 — Quantitative and corpus-relative

"Most Bach chorales cadence this way." Genuinely useful, genuinely not a proposition of the same kind. None of the
machinery in this directory holds it, and `05-open-questions.md` Q-G records that as a gap rather than pretending it is
covered.

---

## 4. Where dependent types earn their place, precisely

Four uses, in decreasing order of confidence. Anything past the fourth should be treated as unmotivated until a
falsifier shows up for it.

**4.1 Quotients with invariant elimination (high confidence).** Amendment V. Octave equivalence, enharmonic equivalence,
set class, row class are one construction — `A/G` — and a function out of `A/G` must be `G`-invariant. This is the
single strongest case, it collapses four hand-written forgetful maps into one, and it makes the respelling failure in §2
a type error. The lightweight form (a quotient former whose eliminator demands an invariance proof) is very likely
sufficient; full univalence is not obviously needed and `05-open-questions.md` Q-F keeps that open.

**4.2 Indexed claims (high confidence).** `Analysis<S>` indexed by a style, and a proof term that is the derivation.
Tier 2 above. This needs indexing over a data domain — a style is data — and it is exactly the shape that
`~/Code/kan/docs/rules/constitution.md` Amendment 1 handles by making dependency fibrational over an *admissible index
domain*. Styles are admissible; musical objects are not, and the distinction matters: a type may depend on *which
style*, and must not depend on *which piece*.

**4.3 Structural refinements (medium confidence).** A row as a Σ-type of a sequence and a bijectivity proof; a voicing
as a chord together with an ordering proof; a configuration as a subset together with conflict-freedom and
downward-closure. Cheap, obviously correct, but a refinement-type system would do all of it, and reaching for dependent
types when refinements suffice is overreach.

**4.4 Directed structure (speculative, and probably more relevant than cubical).** Voice leading, resolution, and
transformation are *directed*: a leading tone resolves upward and the reverse is not the same relation. Lewin's
transformational networks are diagrams — objects are musical entities, arrows are transformations, and the analysis is
the commuting of the diagram. Amendment IV's group action is the invertible special case; the general case is a
category. If a type theory is wanted for that, a directed one fits the shape and a cubical one does not — cubical
machinery is built for symmetric identification, and the interesting musical relations are asymmetric. This is recorded
as a direction, not a commitment.

---

## 5. Where dependent types do not belong, and why the temptation is strong

**Encoding style rules in types.** §1. Non-negotiable under Amendment VI.

**Making the kernel dependently typed.** The kernel calculus is a six-form language with no abstraction and a totality
theorem (`docs/rules/kernel/10-term-calculus.md` T4). Making it dependent buys nothing, because it has no functions to
index and no computation to normalize at the type level, and costs T4's simplicity. The dependency belongs one layer up,
in the elaboration language and the library, where functions exist. This is Amendment IV's `act`-not-`map` decision seen
from the type side.

**Proof obligations on the composer.** The most important negative. Every proof obligation that reaches the surface
syntax is a moment where a musician is asked to satisfy a type checker instead of writing music. Tier 1 obligations are
acceptable because they are discharged by construction — you cannot accidentally add two pitches, so you never prove
anything. Tier 2 obligations must be discharged by the *checker*, producing a derivation the musician reads, never one
they write. A design in which composing requires writing proofs has failed the "impeccable user experience" requirement
no matter how elegant its metatheory.

---

## 6. The honest summary

Of the six atoms in `01-atoms.md`, five need no type theory at all: they are ordinary algebra — a partial order, a
conflict relation, a periodic set, a group action — and would be implemented in Rust with ordinary data structures.
Exactly one, orbit identification, needs proof-relevant machinery, and it needs the weakest form of it.

That ratio is the finding. "Express music theory in type theory" is a good idea in one place and a distraction
everywhere else, and the value of this document is knowing which place.
