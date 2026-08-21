# Open questions, and how to find out cheaply

Every question below is referenced from an amendment or an atom. Each states the question, both live answers, and — the
part that matters — **what evidence would settle it**. A question with no settling experiment is a question that will be
answered by whoever writes the code first, which is how a design drifts.

§10 is a staged plan for testing the hypothesis at increasing cost, ordered so that the cheapest experiment is also the
one most likely to refute it.

---

## Q-A — Is ambient time ℚ, and does tempo belong below or above the kernel?

**Question.** Score time is rational and the performance layer maps it to seconds. Is that map outside the kernel, or is
score time itself tempo-relative — so that a *ritardando* is a fact about the piece rather than about a performance of
it?

**Answers.** (i) Outside, as today: tempo is performance, the kernel is notation. (ii) Inside, as a second ambient
coordinate with a monotone map between them, making the kernel a two-clock system.

**What settles it.** Find a notated meaning that requires sounding time and is not a performance directive. Candidates:
metric modulation (OMT `098` treats it as a notated relation between tempi, which is suggestive), and electronic parts
fixed in seconds against a live part in beats. If a click-track-against-free-part score cannot be represented with one
clock, answer (ii) wins.

**Cost of getting it wrong.** Low in the (i) direction, high in the (ii) direction: adding a second ambient later
touches every law.

---

## Q-B — Are metrical layers denotation or analysis?

**The sharpest question in the directory.** Amendment III says layers are ambient structure. But hypermeter is *heard*,
and two analysts hear different hypermeters in the same passage — OMT `117-hypermeter.md` presents hypermetric reading
as interpretive, with Beethoven's *ritmo di tre battute* as a case where the composer intervenes because the default
reading would be wrong.

**Question.** Do two pieces with identical events and different metrical layers denote differently?

**Answers.** (i) Yes: layers are denotation, a piece carries its layers, and re-hearing is a different piece. (ii) No:
layers are an analysis, a claim in the sense of Amendment VI, and the kernel carries only the notated meter — which is
itself just the layer the engraver chose to bar.

**What settles it.** Whether any *non-analytic* consumer needs a layer. `musa-notation` needs one to bar and beam; that
is satisfied by the notated meter alone. `musa-engine` needs one for a click; same. If no consumer outside analysis
needs the non-notated layers, answer (ii) wins and Amendment III narrows to "the notated meter is a layer, not a region"
— which is still enough to refute the region model and still enough to represent explicit polymeter.

**Current lean.** (ii), with the amendment narrowed. Explicit polymeter (OMT `098`) is notated and so is carried;
implicit polymeter and hypermeter are heard and so are analyses. This preserves the falsifier that motivated the atom
while giving up the more ambitious reading.

---

## Q-C — Binary conflict, or more?

**Question.** `#` is binary and unweighted. Should it be `n`-ary ("at most two of these three"), or weighted (a
probability over branches), or should enabling become disjunctive (general rather than prime event structures)?

**What settles it, per part.**

- *`n`-ary*: a real texture constraint from the repertoire that binary conflict cannot express. None found yet.
- *Weighted*: a consumer that must **compute** with the weights rather than display them. A player that samples is such
  a consumer; whether the kernel or the engine should sample is the actual question.
- *Disjunctive*: this one has pressure already. `02-denotational-semantics.md` E2 must duplicate the continuation after
  a branch precisely because prime event structures cannot share an event above two conflicting causes. If the
  branch-indexed representation in §4.1 of that document does not work out, disjunctive enabling is the alternative, and
  it costs the lattice properties that make Theorem R short.

---

## Q-D — What is a repeat?

**Question.** A repeated section: one event heard twice, or two events? Amendment II makes events the identity of notes,
so this is not a free choice.

**Answers.** (i) Two events. Then `repeat` is `seq x x`, sharing is a *term-level* saving only (which is exactly what
`docs/rules/kernel/10-term-calculus.md`'s mark mechanism already assumes, since the third iteration must carry
`RepeatIteration(2)`), and T2 holds as stated. (ii) One event in two configurations. Then a repeat is closer to a cycle
than a sequence, and the finiteness of `E` stops matching the finiteness of the performance.

**Current lean.** (i), strongly — it is consistent with the existing mark design, with X1, and with the fact that a
repeat has different provenance each time. Recorded as a question only because Amendment II makes it a semantic
commitment rather than an implementation detail, and it should be written down as one.

---

## Q-E — Which groups are built in, and group or monoid?

**Question.** The time group is built in. Are `T/I`, `PLR`, and `M5/M7` kernel constants or values in the payload
signature `Σ`? And should `Σ` supply a *monoid* action, which would cover non-invertible operations like filtering a
chord to its upper voices?

**What settles it.** For the first: whether any kernel law needs to know which group it is. None found — every law in
`04-operational-semantics.md` T7 is parametric — so the signature should supply them, and the twelve-tone and
neo-Riemannian groups become library values. That answer looks stable.

For the second: whether the torsor laws are load-bearing. They are what make `Pitch + Pitch` a type error
(`03-claims-and-styles.md` Tier 1), and a monoid action gives that up. **Lean: keep the group**, and let non-invertible
operations be `Payload(f)` applied outside the kernel, which is where they already live.

---

## Q-F — Quotient former, or univalence?

**Question.** Amendment V needs `A/G` and transport of `G`-invariant claims. Does that require a univalent identity
type, or a quotient type former whose eliminator demands an invariance proof?

**What settles it.** Find a musical claim that needs to transport along a *path between paths* — a proof that two
respellings are the same respelling. Enharmonic reinterpretation in modulation (OMT `066-chromatic-modulation.md`,
`069-chromatic-sequences.md`) is where to look, because a German sixth reinterpreted as a dominant seventh is precisely
a claim about *which* identification is being used. If that needs to be first-class, higher structure is doing work; if
the identification can always be named as data, the cheap former suffices.

**Lean.** The cheap former. But this is the one place the answer might genuinely be the expensive one, and it is worth
an afternoon's investigation rather than a guess.

---

## Q-G — What is a style, formally?

**Question.** Amendment VI makes styles named theories. Do they form a lattice with inheritance (strict species
extending free counterpoint), or an unordered set? And can a claim be quantitative?

**What settles it.** For inheritance: whether any two styles in the repertoire stand in a genuine extension relation
where *every* claim of the weaker holds in the stronger. OMT `030-16th-century-contrapuntal-style.md` is the test case
and it looks like a counterexample — it shares most of species counterpoint's rules but has *different* exceptions,
which is not extension. If inheritance fails there, styles are an unordered set with explicit sharing, which is more
code and less wrong.

For quantitative claims: Tier 3 of `03-claims-and-styles.md` is currently unhoused. This is a real gap and the honest
position is that the hypothesis does not cover corpus statistics at all.

---

## Q-H — Does retrograde extend the time group?

**Question.** `02-denotational-semantics.md` E5 observes that retrograde must reverse `≤` to preserve monotonicity. Is
retrograde an element of an extended time group (allowing negative scaling), or a separate structural automorphism?

**What settles it.** Whether `act (-1) (act (-1) t) = t` should hold definitionally and whether every law in T7 survives
negative scaling. It probably does and probably they do, but the monotonicity law is stated with `≤` and flipping it is
exactly the kind of detail that breaks a proof quietly. Prove it or find the counterexample before writing the code.

---

## Q-I — Does canonical form stay cheap enough?

**Question.** `04-operational-semantics.md` T3 notes that semantic equality is now isomorphism of labelled event
structures rather than equality of sorted multisets, which is graph-isomorphism-shaped in the worst case.

**What settles it.** Measurement, against the budgets in `docs/rules/kernel/09-performance.md`, on the largest fixture
in `examples/`. The refinement algorithm should terminate immediately on real music; if it does not, the hypothesis has
an engineering problem serious enough to reconsider.

---

## 10. How to test this cheaply, in order

Ordered so that the earliest step is the most likely to refute the hypothesis, which is the only sensible order.

1. **Write the two counterexamples as tests, against the current kernel.** Proposition A's voice exchange and a
   first/second-ending pair. Both should demonstrably lose information today. If they do not — if the existing
   architecture already distinguishes them somewhere that counts — the hypothesis is answering a question that is not
   open, and this directory should be closed. *Cost: an hour.*

   **The voice-exchange half is already run and confirmed.** Against `musa-kernel` at this commit,
   `overlay(seq(60, 62), seq(57, 59))` and `overlay(seq(60, 59), seq(57, 62))` satisfy `semantic_eq`. The current
   denotation identifies parallel motion with a voice exchange, exactly as Proposition B predicts. What remains of
   step 1 is the first/second-ending case and a check of whether any consumer recovers the distinction downstream —
   `musa-notation`'s beaming is the place to look, since if it beams both identically the information is gone from the
   whole pipeline and not just from the kernel.
2. **Prototype the branch-indexed value representation** (`04-operational-semantics.md` §4.1) and check the linear size
   obligation on a piece with nested alternatives. This is the one non-obvious implementation requirement, and if it
   fails, `alt` gets much more expensive and Q-C's disjunctive answer comes back into play. *Cost: a day.*
3. **Measure canonical form** (Q-I) on the largest existing fixture. *Cost: a day.*
4. **Re-prove L1–L24 against E1–E6 by hand**, and confirm that L18 is the only casualty. If a second law falls, the
   claim that this is a refinement rather than a rewrite is weaker than advertised and the trade needs restating. *Cost:
   a few days.*
5. **Narrow Amendment III per Q-B** before writing any layer code, since the narrow reading is much cheaper and probably
   right.
6. **Only then** consider prompts. Adopting the hypothesis would mean a repair to `docs/course-correction.md` and
   `docs/rules/kernel/`, and a block of new prompts inserted into `docs/plan/prompts/` with the rest renumbered — which
   is a large, irreversible-feeling change and should not happen on the strength of a document.

---

## 11. What would refute the whole thing

Stated plainly, so it is possible to lose:

- **Step 1 fails.** If voice identity and alternatives are already faithfully represented where it matters, the
  motivating deletions do not exist.
- **Nobody wants `lines`.** If `musa-notation` and the counterpoint style are both happier with payload metadata after
  seeing the alternative, the atom is not paying for itself and Amendment II should be withdrawn to a note.
- **Step 2 or 3 fails badly.** An exponential value representation or an expensive canonical form would make the kernel
  violate its own performance budgets, and a semantics that cannot be computed is not a semantics for this project.
- **L18's failure turns out to be load-bearing somewhere real.** §5.3 of `02-denotational-semantics.md` claims the
  migration is small. If a consumer depends on interchange in a way that cannot be re-expressed through `U`, the
  refinement is more disruptive than argued.

If it is refuted, what survives is still worth having: the atoms are independent, and Amendments III (meter is not a
region), V (identification by declared orbit), and VI (styles are theories) stand or fall separately from II. That
independence was the point of writing them as separate amendments rather than as one design.
