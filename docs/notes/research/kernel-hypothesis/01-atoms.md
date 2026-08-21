# Atoms and the evidence for them

Six atoms are proposed. Two are retained from the existing kernel unchanged; four are new. Each entry has the same four
parts, and an atom missing any of them is not an atom:

- **What it is** — the mathematical object.
- **Evidence** — a citation into `~/Code/papers/music-theory/open-music-theory/`, by filename. Claims about what
  musicians mean are checked against the theory this repo already treats as authoritative, not asserted.
- **Falsifier** — a concrete musical object that cannot be represented faithfully without this atom. This is the §34
  test from `docs/rules/kernel/00-purpose.md`: *a construct belongs in the kernel only if removing it makes an important
  class of musical meanings impossible or unnatural across multiple independent consumers.* An atom whose falsifier is
  hypothetical is an atom that should be cut.
- **Cost** — what the atom makes harder, because every one of them makes something harder.

Section 7 lists what was considered and **rejected**, with the reason each fails the test. Section 8 audits which atoms
are genuinely intertwined and must not be separated, and which are separable and must not be merged.

---

## 1. Ambient exact time (retained)

**What it is.** An ordered field of rational values, ambient: no term names it, reads it, or holds it.

**Evidence.** Every notated rhythm is a rational subdivision (OMT `009-notating-rhythm.md`); tuplets are exact ratios,
not approximations (OMT `012-other-rhythmic-essentials.md`).

**Falsifier.** Any triplet against a duple division. Floating-point time makes `1/3 + 1/3 + 1/3 = 1` false, and every
downstream alignment inherits the error.

**Cost.** No continuous time, so no direct account of rubato or glissando trajectory inside the kernel. These live at
the performance edge today; whether that is right is `05-open-questions.md` Q-A.

---

## 2. Occurrence (retained) and **succession** (new)

> **Withdrawn by Gate 2 (`06-evidence-log.md`).** Gate 0 already weakened it: Proposition A below is true of
> `musa-kernel` in isolation and was verified against it, but it does not hold of the pipeline, because
> `elaborate.rs:885` puts a `VoiceId` in the payload and the two readings differ downstream. What survived was narrower
> — partial ordering and divisi remain inexpressible, and the tag is a convention every consumer must share — and it was
> gated on prompt 119. Prompt 119 landed on the tag with no shadow line structure, so the narrower case has no consumer
> either, and the atom is withdrawn. The section is left as written; §2.3's partial-order argument is the one part still
> live, and it is live as a *question awaiting a consumer* (`07-adoption-plan.md` Track C), not as a proposal.

**What it is.** An occurrence is an event `e` labelled with a start and an extent in ambient time and a payload. The new
part is a partial order `≤` on events: `e ≤ e′` means `e′` continues `e` — same line, same thread, same voice.

**Evidence.** Polyphony is defined as "multiple voices with separate melodic lines and rhythms" (OMT `008-texture.md`).
Species counterpoint is entirely about what a *line* does — motion between successive notes of one voice, and the
relation of one line's motion to another's (OMT `023-introduction-to-species-counterpoint.md` through
`028-fifth-species-counterpoint.md`). SATB part-writing treats voice membership as the primary datum (OMT
`022-chords-in-satb-style.md`). None of these speak of "the set of notes sounding"; all of them speak of lines.

### 2.1 Succession is not determined by time and payload

This is the load-bearing claim of the whole document, so it gets a proof rather than a paragraph.

> **Proposition A.** There exist two musical objects that differ in a way musicians routinely name, and whose
> denotations under `docs/rules/kernel/03-denotational-semantics.md` are equal.

*Proof.* Take four occurrences: C4 and A3 on `[0,1)`, D4 and B3 on `[1,2)`. Reading one is two voices in parallel motion
— the upper line C4→D4, the lower A3→B3. Reading two is a voice exchange — C4→B3 descending, A3→D4 ascending. The two
readings have identical multisets of `(start, extent, payload)` triples and identical extent, so `⟦·⟧` assigns them the
same value. They are not the same object: the first has no voice crossing and two melodic steps, the second has a
crossing, a descending semitone, and an ascending fourth. First-species analysis returns different verdicts for them,
and an engraver beams and stems them differently. ∎

The existing kernel does distinguish these — through HIR structure and payload metadata, that is, *outside* the
denotation. `docs/rules/kernel/08-open-questions.md` Q3 records this as an open question and gives that arrangement as
the working stance. Proposition A is the argument that it cannot be closed any other way: whatever carries voice
identity is carrying semantic content, so either it is in the denotation or the denotation is incomplete.

### 2.2 Time constrains succession

> **Law (monotonicity).** If `e < e′` then `start(e) ≤ start(e′)`.

A line proceeds forward. Two events at the same onset may be comparable (one legato note beginning exactly as its
predecessor releases) but a continuation never starts earlier than what it continues. Together with 2.1 this is the
precise sense in which succession *refines* the temporal order without being *derived* from it — and it is the reason
succession belongs in the same kernel as time rather than in a layer above it. Separating them would require a coherence
theorem stating monotonicity across the boundary, which is exactly the cost that
`~/Code/kan/docs/rules/across-stages/foundations/index-layering.md` says to avoid paying.

### 2.3 It is a partial order, not a total one

Serial practice supplies the citation directly: OMT `110-row-properties.md` §"Partially ordered sets" observes that much
music called serial is not based on a fixed total succession at all, and borrows the mathematical term for what it is
instead. A row whose tetrachords are fixed but whose internal orderings are free is a partial order on twelve events. A
kernel with only sequences cannot hold it; a kernel with a partial order holds it exactly.

### 2.4 Prime, not general

General event structures allow *disjunctive* enabling — an event with two alternative causes. The clear repertoire case
would be a resolution reachable from either of two preparations, where the analysis genuinely does not commit to which.
This is rarer than the partial-order case and much more expensive: general event structures lose the
lattice-of-configurations property that makes the recovery theorem in `02-denotational-semantics.md` short. Judged: take
prime event structures, and record in Q-C what would force the general form.

**Falsifier.** The voice exchange above. Also any fugue subject entry: "this is the answer to that subject" is a
statement about which events continue which, across an overlap where the sounding set is ambiguous.

**Cost.** Every constructor must now say what it does to `≤`, and the algebraic laws must be re-proved with it.
`02-denotational-semantics.md` §5 does that work; two laws weaken, and the document says which.

---

## 3. Conflict

> **Corrected by Gate 0 (`06-evidence-log.md` G0.3). This atom is refuted as proposed.** The falsifier below is wrong:
> first and second endings are *deterministic* — one hearing, both endings sounding at different times — and
> `musa-compiler` already implements them by pass expansion, which is correct there.
> `docs/rules/kernel/11-realization.md` is governing and independently refuses this constructor for reasons that still
> stand. The section is left in place as the record of what was claimed.

**What it is.** An irreflexive symmetric relation `#` on events, hereditary along succession: if `e # e′` and `e′ ≤ e″`
then `e # e″`. `e # e′` means no single hearing contains both.

**Evidence.** First and second endings are two mutually exclusive continuations of one passage (OMT `057-binary-form.md`
treats repeat structure as constitutive of the form, not as an abbreviation). Aleatory and open form make the
exclusivity the point rather than a notational convenience.

**Falsifier.** A first/second-ending pair. Without conflict there are exactly two representations available and both are
wrong: assert both endings, and the piece contains music that is never heard together; expand into one linear
realization, and the *piece* has been replaced by *a performance of it* — which is precisely the layering violation
`docs/course-correction.md` exists to prevent. `docs/rules/kernel/11-realization.md` pushed aleatory above the kernel
for this reason; the push was forced by a missing atom, not by a judgment that aleatory is peripheral.

### 3.1 Why conflict and succession are one amendment

The hereditary axiom is a statement about both relations at once, and it is not optional: without it, a configuration
could contain an event whose own prerequisite is excluded, which is not a hearing of anything. Any design that adds `≤`
now and `#` later must re-prove every law of the first in the presence of the second. Adding both together costs one
proof; adding them in sequence costs two, and the second is the same proof again. This is the merge criterion applied,
not a preference for large definitions.

### 3.2 What it subsumes

Repeats, first and second endings, ossia staves, *ad libitum* passages, cue-sized optional parts, mobile form, and
score-level alternative versions are today four or five special cases across `musa-compiler` and `musa-notation`, each
with its own representation. All are the same construct: alternative configurations of one object. APOSD names this red
flag *special-general mixture*, and merging removes it rather than adding to it.

**Cost.** A piece no longer has *an* extent; it has an extent per configuration. Consumers that want a single number — a
timeline width, a MIDI file — must name which hearing they mean. That is a real burden and it is also correct: a
mobile-form piece does not have a duration, and today's kernel only appears to give it one.

---

## 4. Pulse layers

**What it is.** A metrical layer is a pair `(phase, period)` in ambient time, denoting the pulse set
`{ phase + k·period | k ∈ ℤ }`. A piece carries a set of layers. Layers are ambient in the same sense time is: nothing
holds one, and they do not own intervals.

**Evidence.** Three independent chapters, which is what the §34 phrase "multiple independent consumers" asks for:

- **Hypermeter** (OMT `117-hypermeter.md`): metrical grouping continues recursively above the notated bar — in twos,
  fours, eights, and also threes, fives, and sevens. The notated meter is one layer among several that are all
  simultaneously in force.
- **Metrical dissonance** (OMT `118-metrical-dissonance.md`): two or more *unaligned* metrical layers coexist in a
  single passage.
- **Polymeter** (OMT `098-twentieth-century-rhythmic-techniques.md` §Polymeter): "two or more meters are performed
  simultaneously," and the chapter distinguishes explicit polymeter, where both are actually written, from implicit.

**Falsifier.** Any explicitly polymetric passage — the chapter's own examples. A region carries one signature; the
passage has two, written, at once. `crates/musa-compiler/src/elaborate.rs` currently comments that "Key and meter are
*regions*," and the polymeter section of OMT is a counterexample to the meter half of that sentence.

### 4.1 The model predicts the existing taxonomy

Krebs's two categories of metrical dissonance fall out of the pair `(phase, period)` without being stipulated:

| Krebs | In this model |
| --- | --- |
| Displacement dissonance | Two layers, equal period, unequal phase |
| Grouping dissonance | Two layers, incommensurable periods |

OMT `098` independently notes that implicit polymeter is "a type of grouping dissonance," which the model also predicts:
an implicit second meter *is* a second layer, so the two phenomena are one phenomenon seen from two chapters. A
representation that recovers an existing classification it was not built to recover is doing better than a
representation that has to be told the classification.

### 4.2 What becomes derived

The barline. Which layer is engraved as the bar is a decision `musa-notation` makes; re-barring a passage changes no
denotation. Beaming, which OMT `009` ties to metrical grouping, is then a function of the layers rather than of a stored
bar.

**Cost.** Layers must come from somewhere. Notated meter supplies one; hypermeter and implicit polymeter are *heard*,
and a piece does not carry them unless someone asserts them. This is the sharpest unresolved question in the directory —
whether layers are denotation or analysis — and `05-open-questions.md` Q-B states both readings.

---

## 5. Payload torsors and group actions

**What it is.** The kernel is parametric in a payload domain `A`, and knows exactly one thing about it: a group `G` acts
on `A`. `act g M` is a kernel term. Where the action is simply transitive, `A` is a *torsor* over `G`: differences of
elements of `A` are elements of `G`, but elements of `A` cannot be added.

**Evidence.**

- Intervals are differences of pitches, and pitches do not add (OMT `016-intervals.md`). Torsor.
- Transposition and inversion are operations on pitch-class sets (OMT
  `101-pitch-class-sets-normal-order-and-transformations.md`). Group.
- The `P`, `L`, and `R` operations compose, and the Tonnetz is the picture of their composition — infinite as drawn, a
  torus once enharmonic equivalence is applied (OMT `072-neo-riemannian-triadic-progressions.md`). The Tonnetz is the
  Cayley graph of the group generated by `P`, `L`, `R` acting on the twenty-four consonant triads; the chapter gives the
  picture and the operations, and the group-theoretic reading of it is *derived*, not cited.

**Falsifier.** Twelve-tone practice. A row has forty-eight forms and the analysis is entirely about which form is which
(OMT `108-basics-of-twelve-tone-theory.md`, `109-naming-conventions-for-rows.md`). Without an action, either the
forty-eight forms are forty-eight unrelated objects, or the kernel needs general functions to relate them.

### 5.1 Why an action and not a function

`docs/rules/kernel/10-term-calculus.md` T4 proves every closed kernel term evaluates, and it holds because the kernel
has no abstraction and no `fix`. `map f M` would require a function space and reopen that proof. A group action is total
by construction, so `act g M` extends the calculus without touching T4. That is the entire reason for the phrasing, and
it is worth stating plainly: this atom was chosen for what it *refuses* as much as for what it provides.

**Cost.** Non-invertible operations are excluded. Filtering a chord to its upper voices is a function, not a group
element. A monoid action would cover both and would give up the torsor laws that make the `Pitch`/`Interval` distinction
provable rather than conventional. Q-E.

---

## 6. Orbit identification

**What it is.** For a declared group `G` acting on `A`, the quotient `A/G`, together with the rule that a proof that two
elements share an orbit transports every `G`-invariant claim between them.

**Evidence.** OMT states the equivalences as equivalences, separately, four times:

| Equivalence | OMT | The group |
| --- | --- | --- |
| Octave equivalence | `099-pitch-and-pitch-class.md` | Translation by the octave |
| Enharmonic equivalence | `099-pitch-and-pitch-class.md` | Reduction of the line of fifths mod 12 |
| Set class | `102-set-class-and-prime-form.md` — "a group of pitch-class sets related by transposition or inversion" | `T/I` |
| Row class | `109-naming-conventions-for-rows.md` | `T/I/R` (and `M` where used) |

The set-class definition is an orbit stated in words. The theory has been describing quotients by group actions in four
chapters without a single name for the construction.

**Falsifier.** Enharmonic respelling. A musician respells C♯ as D♭ and expects the harmonic analysis to survive but the
voice-leading spelling not to. Without orbit identification, either the two are one object — and the spelling
distinction that `docs/rules/style-guide.md` is careful to preserve is lost — or they are two objects with no way to
move a claim across, and every enharmonically invariant theorem must be proved twice.

### 6.1 This is where dependent types earn their keep, and it is the only place

Everything else in this document is ordinary algebra and could be implemented in Rust with no type theory at all. This
atom is different: "transports exactly the `G`-invariant claims" is the Structure Identity Principle, and it is the one
place where a proof-relevant equality does work no first-order encoding does as briefly.
`~/Code/kan/docs/rules/constitution.md` Amendment 3 — equality is transport, the proof *is* the movement — is imported
here for this and for nothing else. The honest scope of "express music theory in type theory" is this section, and
overclaiming it beyond this section is how the project would waste a year.

**Cost.** The quotient's eliminator must be restricted to invariant motives, which is a real proof obligation on every
library function that looks at a `Pc12` and returns something. Whether the full univalent machinery is needed or a
quotient former with a restricted eliminator suffices is Q-F; the second is much cheaper and probably enough.

---

## 7. Considered and rejected

Discipline is visible here or nowhere. Each of these was attractive and each fails the §34 test.

**Bars.** Derived from layers (§4.2). Removing bars from the kernel makes no meaning unrepresentable.

**Tempo.** A map from score time to sounding time. Removing it costs nothing at the kernel level because no *notated*
meaning depends on it; it is the performance layer's job, exactly as `docs/course-correction.md` places it. Reopened
only if score time itself must be tempo-relative — Q-A.

**Dynamics, articulation, timbre.** Payload. No consumer needs them structurally, and every consumer needs them
*present*, which parametricity already gives.

**Prolongation and hierarchy.** The most tempting rejection. Phrase-model and prolongational analysis run through a
dozen OMT chapters (`042`, `044`, `045`, `052`) and are unquestionably central to how tonal music is understood. They
are rejected as kernel atoms because they fail the falsifier test in a specific way: no piece becomes unrepresentable
without them. What becomes unstatable is an *analysis*, and Amendment VI puts analyses in styles, with evidence, above
the kernel. A prolongation is a claim about a piece, and the kernel's job is to be the thing claims are about.

**Probability and weight.** A weighted conflict would let aleatory express preference. Rejected: conflict says "may,"
and how often is a performance directive carried in payload. Revisit only if a consumer needs to *compute* with the
weights — Q-C.

**Infinite and live streams.** `docs/rules/kernel/08-open-questions.md` Q1. Rejected for now, because T4 totality is
worth more than the class of pieces it excludes, and because an event structure with infinite events still has finite
configurations, which softens the loss.

**A simultaneity or chord constructor.** Already derivable from overlay. Adding it would be a special case of a general
mechanism — the APOSD red flag in its textbook form.

---

## 8. What must be together, and what must be apart

The user's constraint was to avoid complecting without artificially splitting things that are genuinely intertwined.
That cuts both ways, and both directions are argued here rather than asserted.

### 8.1 Intertwined — separating these costs theorems

| Pair | Why they cannot be layered |
| --- | --- |
| `≤` and `#` | The hereditary axiom is a statement about both. Adding one then the other re-proves the first's laws. |
| `≤` and time | Monotonicity (§2.2) is a law across the pair. Layered, it becomes a coherence theorem to maintain. |
| Layers and time | A layer *is* a periodic subset of the ambient coordinate. There is nothing to separate. |

### 8.2 Separate — merging these leaks information

| Pair | Why the boundary holds |
| --- | --- |
| Event structure and payload algebra | The kernel knows only that a group acts. Nothing about pitch, spelling, or timbre crosses. This is the deep-module boundary and it is narrow: one interface, `act`. |
| Kernel and styles | Amendment VI. A kernel that knows what a parallel fifth is has taken a side. |
| Kernel and layer *provenance* | Whether a layer was notated or inferred is provenance, and provenance is `musa-project`'s existing job. |

### 8.3 Red-flag audit

Checked against the APOSD list, in the direction that matters — does the proposal *remove* red flags or add them?

- **Information leakage — removed.** Voice identity as payload metadata is a leak today: `musa-notation` must know the
  metadata convention to beam correctly, `musa-compiler` must know it to analyze counterpoint, and neither is told by a
  type. As a chain in `≤` it is structure both read from one place.
- **Special-general mixture — removed.** §3.2: five special cases become one construct.
- **Pass-through — removed.** The current voice metadata is threaded through layers that only forward it.
- **Temporal decomposition — avoided.** Amendment III forbids the "set the current meter, walk, clear it" traversal
  shape that a region model invites.
- **Shallow module — the risk this proposal adds.** An event structure has more moving parts than a multiset, and if its
  interface is as wide as its implementation, the atom is not paying for itself. The mitigation is that consumers see
  *configurations* and *chains*, not the raw relations; `02-denotational-semantics.md` §6 fixes that interface and it is
  deliberately small.
