# Constitution of the music kernel

Six amendments. Each says what the kernel *is*, what follows from that, and what it deliberately leaves open. An
amendment is not a feature: it is a commitment that later documents may elaborate but may not contradict. When a later
document and an amendment disagree, the amendment wins or the amendment gets repaired — never silent drift.

The form is borrowed from `~/Code/kan/docs/rules/constitution.md`, which earns it: an amendment with no "what this
leaves open" section is usually an amendment that has quietly decided more than its evidence supports.

---

## Amendment I — Time is ambient, exact, and not a thing you can hold

Musical time is an ambient exact-rational coordinate. It is not a value the language passes around, not a field on an
event, and not something a term can read.

**This amendment is retained, not proposed.** It is `docs/course-correction.md`'s central claim and
`docs/rules/kernel/01-time.md`'s law, and everything in this directory is built on top of it unchanged. It is listed
here because the later amendments only make sense against it, and because it is the one amendment that has already
survived a full implementation.

### What follows

- Composition operators *place* things in time; they do not compute with time. `M ; N` shifts `N` by the extent of `M`
  without either term naming the shift.
- Floating point is a rendering artifact. It appears at the performance and DSP edge and nowhere earlier.
- Two pieces that place the same material at the same rational offsets are the same piece, whatever route the surface
  syntax took to say so. This is what makes `docs/rules/kernel/04-algebraic-laws.md` provable rather than aspirational.

### What this leaves open

- Whether the ambient coordinate is ℚ or a larger ordered field. Rubato, `accelerando`, and tempo curves are currently
  handled by the performance layer mapping ℚ to seconds. Nothing here decides whether that mapping belongs in the
  kernel; see `05-open-questions.md` Q-A.
- Whether "ambient" ever admits a second time coordinate — score time and sounding time as two ambients rather than one
  ambient plus a map. See Q-A.

---

## Amendment II — A piece is an event structure, not a set of events

A kernel object denotes a set of event occurrences carrying **two relations**: *succession* (`≤`, which occurrence
continues which) and *conflict* (`#`, which occurrences cannot both happen in one hearing). The current kernel's
multiset of `(start, extent, payload)` triples is what remains after both relations are forgotten.

The two relations are one amendment, not two, because they are not separable: conflict is *hereditary* along succession
— if `e # e′` and `e′ ≤ e″` then `e # e″`. A world that has one relation and adds the other later has to re-prove every
law it had. This follows the merge criterion in `~/Code/kan/docs/rules/across-stages/foundations/index-layering.md`:
keep domains separate unless merging eliminates coherence theorems. Here it eliminates them.

### What follows

- **A piece has runs.** A *configuration* is a conflict-free, succession-closed subset of the events: one coherent
  hearing. A piece with no conflict has exactly one maximal configuration and behaves exactly as today's kernel does.
- **Voice is not payload metadata.** A voice is a chain in `≤`. `docs/rules/kernel/08-open-questions.md` Q3 asks how
  voice identity is represented and currently answers "payload metadata plus HIR structure," which is an admission that
  the kernel cannot say it. Under this amendment the kernel says it directly, and two elaborations that disagree about
  which notes are in the same line are distinguishable objects rather than the same object with different comments.
- **Repeats, endings, ossias, and open form become kernel objects.** `docs/rules/kernel/11-realization.md` had to push
  aleatory above the kernel because the kernel had no way to hold two alternatives without asserting both. Conflict is
  precisely that way.
- **Succession refines time without being determined by it, and time constrains succession.** Both directions are laws,
  not conventions; `01-atoms.md` §2 proves the first and states the second.

### What this leaves open

- Whether `≤` is a partial order (prime event structures) or a general enabling relation (general event structures,
  which allow *disjunctive* causes — "this resolution is enabled by either of these two preparations"). `01-atoms.md`
  §2.4 argues for the partial order and names the repertoire that would force the general form.
- Whether conflict is binary or `n`-ary. Binary conflict cannot express "at most two of these three may sound," which
  some texture constraints want. Deferred to Q-C.
- Whether an event may be *shared* between two configurations that reach it by different histories. Under prime event
  structures it may; whether that is the right reading of a repeated section is Q-D.

---

## Amendment III — Structure is ambient, in the same sense time is

Meter, key, and the other pervasive frames are **ambient layered structure**, not spans painted on a region of the
timeline. A metrical layer is a periodic pulse, given by a phase and a period in the ambient rational coordinate.
Several layers coexist, they need not agree, and none of them owns an interval of time.

### What follows

- **Meter is not an interval with a signature attached.** `crates/musa-compiler/src/elaborate.rs` currently comments
  that "Key and meter are *regions*." That is falsified by two independent chapters of the theory this project cites:
  hypermeter, where grouping continues recursively above the notated bar (OMT `117-hypermeter.md`), and metrical
  dissonance, where two or more unaligned layers coexist in a single passage (OMT `118-metrical-dissonance.md`). A
  region has one signature; the repertoire has several at once.
- **The theory's own taxonomy is derived rather than stipulated.** With a layer as a pair `(phase, period)`, Krebs's two
  categories fall out: *displacement dissonance* is two layers with equal period and unequal phase, *grouping
  dissonance* is two layers with incommensurable periods. A model that predicts an existing classification is worth more
  than one that has to be told it.
- **Barlines become a projection, not a fact.** Which layer is notated as the bar is an engraving decision made by
  `musa-render`, and re-barring a passage does not change what it denotes.

### What this leaves open

- Whether key is the same kind of thing as meter. Both are pervasive and both admit conflict (polytonality, pivot
  regions), but pitch frames lack the periodicity that makes a metrical layer cheap to represent. `01-atoms.md` §4
  proposes layers for meter and leaves key at "a labelled frame that pieces of the event structure may be indexed by,"
  pending Q-B.
- Whether layers are part of the denotation or part of an *analysis* over it — that is, whether two pieces that sound
  identically but are heard in different metrical layers are equal. This is the sharpest open question in the directory
  and `05-open-questions.md` Q-B gives both readings and the experiment that separates them.

---

## Amendment IV — Transformation is a group acting, not a function applied

The kernel is parametric in a payload domain, and the kernel knows one thing about that domain: a group acts on it.
Transposition, inversion, retrograde, augmentation, and the neo-Riemannian operations are elements of groups acting on
payloads and on time. They are not user-written functions that the kernel calls.

### What follows

- **Totality survives.** `docs/rules/kernel/10-term-calculus.md` T4 says every closed kernel term evaluates to a value.
  It holds today because the kernel has no `fix` and no abstraction. A group action is total by definition, so `act g M`
  can be a kernel term without reopening T4. This is the reason the amendment is phrased in terms of actions rather than
  the obvious `map f`: `map` needs functions, functions need a function space, and a function space in the kernel is the
  end of the totality proof.
- **The torsor laws explain the type distinctions the language already has.** Spelled pitch is a torsor over a rank-2
  free abelian group (fifths and octaves); `Interval` is that group. Pitch minus pitch is an interval, pitch plus
  interval is a pitch, and pitch plus pitch is nonsense — not by taste but because a torsor has no origin. Musa already
  keeps `Pitch` and `Interval` apart; this says why, and says the same thing about `Duration` versus a point in time.
- **The Tonnetz is a Cayley graph, not a picture.** The `PLR` group acts simply transitively on the twenty-four
  consonant triads, so the triads are a torsor over it and the Tonnetz is that torsor drawn. "Geometry" in this domain
  means the geometry of a group action; it is not a metaphor and it does not need to be built separately.
- **Transformational analysis becomes a kernel-level claim.** "This passage is `L` then `P` applied to that one" is a
  statement about group elements, checkable, and preserved by every law in `docs/rules/kernel/04-algebraic-laws.md` that
  commutes with the action.

### What this leaves open

- Which groups are *built in* versus *declared*. Time's affine action (shift, scale) is already kernel. Whether the
  twelve-tone `T/I` group, the `PLR` group, and the multiplicative operators `M5/M7` are kernel constants or values in a
  declared payload signature is Q-E.
- Whether non-invertible operations get the same treatment. Rotation and inversion are invertible; *filtering* a chord
  to its upper voices is not. A monoid action covers both and loses the torsor laws; the choice is Q-E.

---

## Amendment V — Identification is by declared orbit, and only where it is declared

Two musical objects are identified exactly when a declared group carries one to the other, and a proof that it does is a
*path*: it transports every claim invariant under that group and no others.

### What follows

- **One mechanism replaces a pile of ad-hoc quotients.** Enharmonic equivalence, octave equivalence, pitch-class set
  class, and twelve-tone row class are today four different forgetful maps with four different implementations. Each is
  `A / G` for a different `G`: reduce the line of fifths modulo twelve; quotient by the octave; quotient by `T/I`;
  quotient by `T/I/R/M`. The kernel gains one construct and the library loses four special cases.
- **It explains why `NoteName` and `Pc12` must be two types and yet share their theorems.** They are `A` and `A/G`. A
  claim proved about `Pc12` transports to `NoteName` precisely when it is enharmonically invariant, and the type checker
  can say which claims those are instead of a comment saying it.
- **This is where dependent type theory actually earns its place in a music language**, and it is worth being blunt that
  it is the *only* place in this constitution where it does. Everything else here is ordinary algebra. The transport
  story is not: it is the Structure Identity Principle, it is genuinely what musicians do when they respell a note and
  expect the harmonic analysis to survive, and no first-order encoding of it is as short.

### What this leaves open

- Whether identification is *definitional* (the quotient type has no observations distinguishing orbit members) or
  *propositional with transport* (members remain distinct and equality is a proof that moves claims). The second is
  strictly more expressive and strictly more expensive. Q-F.
- Whether the kernel needs univalence proper or only a quotient former with an eliminator restricted to invariant
  motives. The second is much weaker and probably sufficient. Q-F.

---

## Amendment VI — A style is a theory, and a claim is a proposition inside it

The kernel does not prevent bad music. It prevents *false claims about* music. Every rule that a musician would call a
rule — parallel fifths, resolve the leading tone, prepare the dissonance — is a proposition relative to a named style,
and the kernel's job is to make the relativization explicit and the checking honest.

### What follows

- **"Prevent by construction" is scoped to what is actually universal.** Type-level prevention is for statements that
  are true in every style because they are true of the objects: an interval cannot be added to an interval to get a
  pitch, a voice cannot be in two places at once, a row is not a row unless it is a bijection. Style rules do not get
  this treatment, because a type that rejects parallel fifths rejects Debussy.
- **This is already the repo's position, and this amendment is only making it a constitutional one.**
  `docs/rules/language/03-musical-domains.md` §5 says counterpoint rules are style-indexed assertions rather than
  universal laws. The amendment says the same thing one level up: nothing may be promoted to a kernel law by arguing
  that it is usually true.
- **Analyses are evidence-carrying.** An analysis is not a verdict but a derivation in a named theory, and the Origin
  view in `docs/rules/desktop/` is the place a musician reads that derivation back. A claim the system cannot justify is
  a claim it must not display.

### What this leaves open

- Whether styles form a lattice with inheritance (strict species extending free counterpoint) or an unordered set of
  independent theories. Inheritance is convenient and is a strong claim about the theory being cumulative, which it
  frequently is not. Q-G.
- Whether a style may be *quantitative* — a claim that holds with a frequency rather than absolutely. Most corpus-based
  claims are of this shape and none of the machinery here holds them. Q-G.

---

## What the amendments jointly forbid

Read together, these six rule out a class of designs that would otherwise look attractive:

1. **No global mutable frame.** Amendment III makes structure ambient and layered; a "current meter" that a traversal
   sets and clears is a temporal decomposition of exactly the kind APOSD names as a red flag, and it cannot represent
   two layers at once anyway.
2. **No user functions in the kernel.** Amendment IV supplies the one thing `map` was wanted for. Adding abstraction
   later must pay for T4 again, in full.
3. **No unlabelled equality.** Amendment V forbids "these two chords are the same" without naming the group. Every
   equivalence in the system is someone's declared invariance.
4. **No universal musical laws.** Amendment VI forbids promoting a style rule to a kernel law, however well attested.
5. **No second event identity.** Amendments II and V together mean that "which note is this" is answered by the event
   structure and by nothing else — not by an index, not by a name in payload metadata, not by source position.
