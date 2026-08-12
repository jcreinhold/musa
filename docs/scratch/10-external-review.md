# External review: the proposal has found useful seams, not the motive

- **Review target:** `main` at `ff9dbcc`
- **Decision:** do not execute step 1 of [09](09-the-proposal.md).

The notebook is valuable research, and its habit of retaining failed probes is exactly right. Its final synthesis is
not. It combines several independently useful distinctions, calls the product a universal object without supplying a
universal property, and then proposes kernel changes that the governing documents explicitly forbid, including a padding
operation the code's history already removed for lack of a caller and an overlay restriction for which ordinary musical
practice supplies counterexamples.

The immediate recommendation is therefore unambiguous:

> Keep overlay total with extent `max(d, e)`. Do not index the kernel by extent, do not make overlay reject unequal
> extents, and do not reintroduce ambient extension as `pad` now. If a real consumer needs synchronized interchange,
> validate or construct that synchronization at the consumer boundary.

This is not a verdict against every ingredient in the proposal. Exact rational ambient time survives. The finite
occurrence kernel survives. The score/signal boundary survives. R1 survives as the right backend law. Local indexed
types may be useful. A metrical-layer analysis may be useful. What does not survive is the claim that these are axes of
one discovered motive, or that they justify changing the primitive temporal algebra.

## 1. Evidence protocol

I use three labels throughout.

- **VERIFIED** means I checked the cited document, primary reference, repository history, implementation, or executable
  test. It does not mean that every interpretation subsequently placed on that fact is verified.
- **JUDGED** means a conclusion from programming-language design, mathematics, software architecture, composition,
  engraving, or music theory. These are the review's arguments, not facts mechanically read from a source.
- **UNCHECKED** means I did not establish the claim and do not want the absence of a finding mistaken for validation.

I read `docs/scratch/README.md` and 00–09 in order; the governing course correction, core boundary, complete kernel
specification, kernel hypothesis, requested core-calculus sections, and `AGENTS.md`; every requested Open Music Theory
chapter; the requested Peyton Jones, Grothendieck-method, 50-examples, and graded-modal-types texts; Levy's CBPV paper;
and Xi and Pfenning's Dependent ML paper. I inspected the requested implementation files at `ff9dbcc`, the relevant
callers and repository history, and the frontmatter of prompts 130–132.

I also ran these targeted tests on the review worktree:

```text
cargo test -p musa-kernel interchange -- --nocapture
cargo test -p musa-compiler a_classs_degree_is_its_framed_degree_with_the_register_forgotten -- --nocapture
cargo test -p musa-audio mismatch -- --nocapture
```

All passed: three kernel interchange tests, the named scale test, and the two audio mismatch tests. There were unrelated
working-tree edits in `voice_leading.rs` and `voice_leading_validation.rs`; I did not modify them and used committed
source for claims about `ff9dbcc`.

## 2. Findings at a glance

| ID | Claim under review | Disposition |
| --- | --- | --- |
| F1 | Step 1 touches none of the governing forbids | **Wrong.** It directly violates forbid 5, may violate forbid 1 depending on `pad`'s status, and changes governed restriction semantics. |
| F2 | Equal-extent overlay plus explicit `pad` is musically preferable | **Wrong as a primitive rule.** Unequal active extents are ordinary; ambient extension is not a notated rest. |
| F3 | L18's proviso reveals an extent fibre that should become a type | **Not derived.** The proposal makes the law unconditional by rejecting valid terms; it does not show the restriction is intrinsic. |
| F4 | The finite pulse-layer metrical plan covers metre generally | **Too narrow.** It handles a useful regular-meter fragment and fails or merely approximates several named cases. |
| F5 | Pitch abstractions form one quotient tower of a pitch torsor | **Wrong.** Several maps have different carriers, are contextual or partial, and assume 12-TET/octave equivalence. |
| F6 | Harmonic function is the orbit under translation by the tonic | **Wrong.** Key normalization is not harmonic function. |
| F7 | A motif is a split idempotent | **Unsupported and presently a pun.** No musically defined projector or correspondence category has been supplied. |
| F8 | CBPV is the load-bearing score/signal framework | **Unsupported.** The desired boundary follows from the payload grammar or an opaque staged API, not from CBPV. |
| F9 | The proposed index language gives LRA/LIA typechecking | **Wrong as written.** Metrical-plan constraints are not merely LRA/LIA, and general inference is not established. |
| F10 | Indices erase at zero runtime cost, including `pad` | **Misleading.** Evidence can erase; `pad` changes a runtime extent and cannot. |
| F11 | Structural recursion on normal forms makes R1 a theorem | **Missing essential hypotheses.** Recursion proves termination, not invariance under a quotient. |
| F12 | The audio domain gets a real type system from `Sig[r;n]` | **Wrong as a codebase claim.** Audio already validates typed ports and channel mismatches. |
| F13 | `StudioGraphSpec` crosses no crate boundary | **Wrong.** It crosses through public functions even where callers rely on inferred types. |
| F14 | `StudioSpec` and `StudioGraphSpec` are a suspect duplicate seam | **Wrong diagnosis.** They are source intent and a lowered processor DAG, with a real pass between them. |
| F15 | P-2 found a commuting quotient square and the missing analysis abstraction | **Only the narrow test survives.** The test is a commuting triangle; `locate` is partial and does not solve the cited leading-tone case. |
| F16 | The synthesis is the object through which realizations factor | **Not shown.** No category, cone, factorization maps, or universal property is defined. |

## 3. The immediate decision: reject step 1

### 3.1 It contradicts the documents it claims to sit under

**VERIFIED.** The core boundary's list of what the decision forbids (now `docs/governance/01-constitution.md` §7) says
that reopening any forbid requires a joint amendment of that document and the corrective memo's §36. Its forbid 5 is “No
dependent indices in the kernel.” Its forbid 1 fixes the kernel forms and says a new convenience must elaborate from
them. §36 independently said that the amendment “does not admit dependent types” and that no kernel form, operation, or
law changes; both statements now live in `docs/governance/01-constitution.md` §7 and §4, and
[`62-course-correction-decision-record.md`](62-course-correction-decision-record.md) records where the rest of the memo
went.

Step 1 does two unambiguously prohibited things and proposes a third unless `pad` is only derived syntax:

1. `Timeline[M; d](A)` adds dependent indices to the object-language kernel.
2. T-Over changes total overlay into a partial, equal-extent operation.
3. `pad_e t` is displayed as a new term former/coercion with runtime denotation. It avoids forbid 1 only if it
   elaborates to an existing sequence with an empty timeline; in that case there is no reason to reintroduce it as a
   kernel operation.

It changes more than the work-order description admits. The existing term calculus says restriction is an observation
and keeps the body's ambient extent. T-Restr in 09 returns extent `j - i`. That is a different restriction semantics,
not a static annotation on the existing dynamic semantics. The statement in 09 §5 that the dynamic semantics are
unchanged is therefore false.

The eight forbids audit as follows:

| Core-boundary §6 forbid | Result for 09 |
| --- | --- |
| 1. No fourth combinator | **At risk in step 1.** `pad` violates it if primitive; a derived spelling does not. T-Restr still changes an existing form's meaning. |
| 2. No bespoke temporal structure above the kernel | **No direct violation shown.** The proposal intends to index the existing temporal object. |
| 3. No payload without an admission record | **No direct violation shown.** Any new `G`-set payload would still require the existing admission process. |
| 4. No signal/coinductive/absolute-time payload | **Survives.** `Sig` is excluded from `A` by the proposed grammar. |
| 5. No dependent indices in the kernel | **Direct violation in steps 1 and 5.** This is exactly what `Timeline[M;d](A)` adds. |
| 6. No `join` | **Survives.** No timeline flattening is proposed. |
| 7. No calculus under the studio | **Direct violation in step 2 as described.** `Sig[r;n]`, T-Force, and audio-graph typing are a calculus under the studio. |
| 8. No second kernel crate or upward dependency | **No direct violation shown.** 09 explicitly rejects Rust const-generic implementation. |

Thus step 1 already requires the joint amendment procedure because of forbid 5, regardless of whether `pad` is derived.
The claim that steps 1–4 touch no forbids is also false beyond step 1: an object-language audio calculus beneath
`StudioSpec` conflicts with forbid 7 unless the boundary is deliberately amended. Whether such amendments would be good
is separate; the notebook cannot claim conformance while proposing them.

**JUDGED.** These forbids are not sacred because they are written down. They are relevant because the notebook presents
itself as subordinate research and because the boundary document already considered and costed the dependently typed
alternative. A contrary result is allowed, but the burden is to reopen that decision explicitly, answer its old
objections, and repair the documents before code. Calling the change “indices, not forms” evades rather than answers the
governing decision.

### 3.2 The proposal's own caller falsifier has already fired

**VERIFIED.** `git log -S 'pub fn extend' -- crates/musa-kernel` finds commit `1b15995`, titled “Make kernel observation
composable and drop ambient extension.” [Kernel static semantics §3](../kernel/02-static-semantics.md) records the
reason: `extend` was removed at prompt 37 because nothing called it. The current implementation has no `extend`, though
the crate-level documentation still contains a stale link to `Timeline::extend`.

09 §9 says the proposal is falsified if `pad` acquires no callers in real scores. Step 1 has no new real-score caller;
the proposed type system itself is declared to be the caller. That is circular. It changes the typing rules so that an
operation becomes compulsory and then cites the compulsory calls as demand for the operation.

**JUDGED.** The historical removal is unusually strong negative evidence because it is the exact operation under a new
name. A new external requirement could overturn it. No such requirement has been demonstrated. L18 is a conditional law
already handled by the law suite, not an external consumer.

### 3.3 Equal extent is not the natural domain of overlay

**VERIFIED.** The governing ontology defines overlay as simultaneous presence in a common ambient timeline, with extent
`max(d, e)`. The implementation does exactly that and “nothing pads”: it takes the maximum and combines occurrences
without adding facts. Its fixed-duration commutative monoid is explicitly a statement about each fixed duration, not a
claim that overlay is undefined between durations.

The executable counterexample `interchange_fails_without_synchronization` is real, and the synchronized law passes. But
the counterexample contains no metre, pulse, accent, or grouping. It combines generic timelines of different extents.
Calling it “hemiola-shaped” is vocabulary-driven pattern matching, not a fact about the test.

**JUDGED.** Voices, parts, and layers enter and leave throughout ordinary scores. Pickups, dovetailing, staggered
entries, divisi, cues, ossias, accompanimental figures that stop under a held note, and instruments tacet for long spans
are not exceptional counterexamples. OMT's orchestration material explicitly discusses changing instrumental
participation, dovetailing, and very long rests. A composer can mean “overlay these gestures from the same onset”
without also meaning “assert that both have the same formal lifetime.”

The proposed repair conflates three distinct things:

- **ambient extension:** the region continues and nothing occurs;
- **silence as absence:** there is no occurrence in that region;
- **a notated rest:** a `ScoreFact::Rest` that an engraver may print and a notation consumer may reason about.

`pad` is the first, not the third. Requiring it does not make rests explicit. It makes an ambient extent annotation
explicit. In notation, rests are often inferred, consolidated, suppressed, split at barlines, or represented as
multimeasure rests according to voice and engraving context. Treating every unequal overlay as a demand for a term-level
rest is neither how composers consistently think nor how engraving consistently represents silence.

### 3.4 The claimed fibre is not even the one in the rule

**VERIFIED.** 09 calls the base `(M, d)` and calls overlay fibre-local. T-Over accepts `Timeline[M; d]` and
`Timeline[N; d]` and returns `Timeline[M ∪ N; d]`. Unless `M = N`, those operands are in different fibres over the
stated base. The rule is local only in the extent coordinate, not in `(M, d)`.

T-Pad also leaves an unresolved semantic question: `M ⌢_d ∅` appears to say that metre ceases during the padded tail.
Usually a silent voice remains under the passage's ambient metre. If the empty plan is meant to inherit ambient metre,
then it is not empty. If it means no metre, the coercion changes metrical meaning. Similarly, T-Restr must specify how
phases and change points are translated when `[i,j)` becomes a new extent `j-i`; the displayed restriction merely uses
`M|[i,j)` and leaves that issue open.

**JUDGED.** These are not cosmetic omissions. They show that “extent indexing” and “metrical base” are not orthogonal
axes in the proposed rules, even though the work order depends on implementing them separately.

### 3.5 Interchange does not force a partial primitive

**VERIFIED.** L18 already states its synchronization conditions. Those conditions are in the theorem statement, not a
hidden implementation side condition. The test suite proves the conditional law and preserves the counterexample to an
unrestricted version.

The pairwise equalities chosen by T-Over are sufficient for equality of the two extent expressions, but they are not the
only arithmetic situation in which

```text
max(a, b) + max(c, d) = max(a + c, b + d)
```

holds. For example, the same side can dominate both maxima without pairwise equality. Occurrence translations impose
additional constraints, so the exact largest lawful domain needs a semantic derivation; it is not established merely by
choosing equal pairs.

**JUDGED.** A type system may deliberately choose a simple sufficient condition. But “this condition is convenient to
check” is not evidence that the primitive operation belongs only in that fibre. The proposal makes L18 unconditional by
shrinking the language. That can be good when rejected programs are meaningless or erroneous. Unequal-duration overlay
is neither.

### 3.6 Concrete decision

**JUDGED.** Step 1 is not worth doing. Keep `Timeline<A>` opaque and runtime-extented; keep primitive overlay total.
When a demonstrated compiler, renderer, or analysis consumer needs interchange or barwise alignment, add a narrow
consumer-side operation such as:

```text
require_equal_extent(parts) -> Result<AlignedTimelines<A>, AlignmentError>
```

or build an `AlignedSection` in the compiler from actual score structure. Let that witness enable a derived
synchronized-interchange transformation. Keep notated rests as `ScoreFact::Rest`, and keep occurrence-free ambient time
as absence. Do not publish even this witness until a caller identifies which notion of alignment it needs: equal ambient
extent, equal measure grid, equal sounding support, or equal notated voice span are different contracts.

## 4. Music-theoretic findings

### 4.1 The metrical plan is one analysis representation, not the base of music

**VERIFIED.** OMT's meter material distinguishes notated meter, perceived meter, grouping, pulse, hypermeter,
non-isochronous/asymmetrical meters, polymeter, ameter, metric modulation, time measured in seconds, and feathered or
gradually changing rhythmic processes. The kernel hypothesis leaves Q-B—whether metrical layers are denotation or
analysis—explicitly open. The governing course correction already demonstrates polymeter and polytempo using scoped
facts without a kernel change.

**JUDGED.** A finite, piecewise-constant set of `(phase, period)` pulse layers is a useful model for a restricted class:
regular notated meter, many stable polymeters, some hypermetrical readings, and finite sequences of discrete meter
changes. It is not a defensible universal base. Against the requested cases:

- **Rubato:** fails as metre. Rubato is a relation between notated musical time and performed physical time. It belongs
  in a timing/performance map, not in a periodic pulse set.
- **Swing and groove:** fails unless reduced to the fact of subdivision. The characteristic unequal subdivision and
  microtiming are not represented by a uniform `(phase, period)` lattice.
- **Senza misura and Gregorian chant:** `∅` can record “no periodic metre,” but it does not represent accentual rhythm,
  text-driven grouping, proportional duration, or phrase structure. Recording absence is not modeling the phenomenon.
- **Fermatas:** fails. A fermata suspends or opens duration locally and has performance semantics; it is not a
  piecewise-constant periodic layer.
- **Accelerando and feathered beaming:** fails exactly, except by finite approximation. Their point is continuous or
  directed change in tempo/note density, not a finite sequence of constant periods.
- **Carter-style metric modulation:** degrades. A new constant period can record the result, but the defining equality
  between an old note value and a new one—the relational modulation—is lost.
- **Free/ametrical contemporary notation:** `∅` again says only that the proposed base has no data. Spatial,
  proportional, cue-based, bracketed-duration, and performer-coordinated systems require other structures.
- **Additive and non-isochronous meters:** degrades unless pulse layers gain grouping and accent structure. A set of
  periods does not distinguish, for example, alternative organizations of the same total span.

Union of finite sets also loses provenance and scope: which part asserts a layer, whether it is notated or inferred,
whether two coincident layers are distinct, and what their accent/grouping hierarchy is. Calling union “polymeter” can
therefore collapse the very multiplicity being analyzed.

The two advertised consequences do not “fall out.” Sequencing concatenates plans because T-Seq stipulates the
concatenation. Overlay unions plans because T-Over stipulates union. Those rules may be reasonable definitions for one
analysis domain; they are not consequences of finiteness or indexing.

**Concrete alternative.** Keep meter declarations as scoped payload/resolver facts. If analysis needs pulse layers,
define an analysis-owned `MeterReading` with provenance and an explicit interpretation kind such as `Notated`,
`Perceived`, or `Performance`. Validate that model against a corpus containing every hard case above before considering
any promotion. Do not put an unresolved analytical choice into every timeline type.

### 4.2 The pitch “tower” is not a tower

**VERIFIED.** The code contains useful maps, but their exact meanings matter:

- `Pitch::diatonic_height`, `Pitch::chromatic_height`, and `Pitch::pitch_class` exist.
- Musa's `PitchClass` retains letter spelling and accidental while forgetting register. It is not OMT's enharmonic pitch
  class, which identifies spellings such as G-sharp and A-flat in 12-TET.
- `Scale::class(Degree)` and `Frame::pitch(Degree)` realize a degree under a chosen collection/frame.
- `Frame::locate(Pitch)` is partial: it locates pitches belonging to the selected unaltered collection and does not
  serve as a total quotient for arbitrary altered degrees.
- Set class acts on unordered pitch-class sets; row class acts on ordered twelve-tone rows. Retrograde acts on a
  sequence or row, not on a single pitch.

**JUDGED.** Therefore the proposed chain

```text
spelled pitch -> chromatic pitch -> pitch class -> set class -> row class -> scale degree
```

is not one quotient tower, and there is no single group acting on one pitch torsor that generates it. The carriers
change. Some maps forget, some choose a contextual coordinate, some are partial analyses, and some operations act on
collections or sequences rather than pitches. Set class and row class are not consecutive quotients of one object; scale
degree depends on a key/scale/frame and is not below either one.

The local chromatic and octave-forgetting maps are useful under their stated assumptions. The universal claim is
common-practice and 12-TET-heavy. Modal systems need a mode/final/reciting-tone context rather than necessarily a tonic
quotient. Microtonal and just-intonation systems need tuning-dependent pitch relations that may distinguish intervals
collapsed by chromatic coordinates. Spectral practice may organize partials relative to a fundamental without equal
division. Maqam, raga, and gamelan repertoires involve culturally specific intonation, melodic grammar, hierarchy, and
performance inflection that are not recovered from a 12-class torsor. Non-octave-repeating scales defeat the assumed
octave action itself.

**Concrete alternative.** Model a diagram of named representations and partial/contextual maps parameterized by tuning,
temperament, collection, and analytical framework. Ask which diagrams commute under explicit hypotheses. Do not force
those objects into a linear quotient tower merely because several operations are called “forgetting.”

There is also a typing failure in 09's premise that every payload `A` is a `G`-set. The proposed `G` does not act on one
carrier: retrograde acts on an ordered row or a timeline, not a pitch; set-class operations act on sets; row operations
act on ordered twelve-tone aggregates; time translation and scaling act on occurrence support; and a `ScoreFact` may be
a rest, meter, key, dynamic, or other fact on which those operations have no uniform musical action. Declaring trivial
actions on unrelated variants would satisfy an algebraic interface while erasing the claimed musical content. The
payload grammar therefore presupposes the very common action that the tower fails to define.

### 4.3 Harmonic function is not translation by the tonic

**VERIFIED.** OMT's scale-degree material distinguishes scale-degree position from Roman-numeral chord labeling, and its
harmony material does not identify either with harmonic function. Translation can normalize a passage to a common tonic
or transpose a key; that is a coordinate change.

**JUDGED.** 05 §5's “surprise survivor” is wrong. Harmonic function concerns contextual role, expectation, syntax, voice
leading, phrase location, and style. I and vi can participate in tonic function; V and vii° can participate in dominant
function; ii and IV commonly share predominant function; a Neapolitan chord is not explained by naming its flat
supertonic scale degree. Secondary dominants and tonicizations make the failure still clearer.

Algebraically, the orbit under translation by a tonic either normalizes transposed copies or, under a sufficiently large
cyclic action, collapses distinctions far more aggressively than functional theory does. It cannot group I with vi and V
with vii° while keeping tonic and dominant roles distinct merely by translating the tonic.

**Concrete alternative.** Treat function as a theory-relative analysis relation over a passage, key context,
voice-leading evidence, and formal location. If an algebraic account is wanted, first fit the equivalence classes and
transitions of an explicit functional theory; do not infer “function” from the availability of a transposition action.

### 4.4 “A motif is a split idempotent” has not acquired musical content

**VERIFIED.** 09 correctly labels the claim conjectural and lists serious missing premises: a category of
correspondences, pullbacks for composition, a restriction to musically meaningful correspondences, and an account of
form. The surface-language motif currently has ordinary definition/reference behavior; no implemented endomorphism
`e : P -> P`, section, or retraction gives it split-idempotent semantics.

**JUDGED.** A crop followed by reinsertion can be made idempotent only after choosing a host, window, and embedding.
That identifies an excerpt, not a motif. Motifs remain recognizable under transposition, rhythmic alteration,
fragmentation, reorchestration, embellishment, and contested analytical segmentation; those relations do not determine a
canonical projector. The Karoubi envelope freely splits all idempotents, most of which the notebook itself expects to be
musically meaningless.

Until the work supplies:

1. a category whose objects and morphisms have musical interpretations,
2. an actual musically motivated idempotent,
3. a section/retraction whose composite is that idempotent, and
4. evidence that the split corresponds to motif recognition or reuse,

the phrase is compatible with the mathematics but not derived from the music. It is currently a sophisticated pun on
“motive/motif.” Keep it as a research question, not as part of the proposal's evidence.

## 5. Programming-language and semantic findings

### 5.1 CBPV does not derive the score/signal boundary

**VERIFIED.** Levy's CBPV decomposes call-by-value and call-by-name and separates value types from computation types
using `F` and `U`; computation types support effectful semantics. It does not state that finite data are values,
coinductive streams are computations, or signals are negative merely because they “do.” A thunk `U B` is itself a value
and can be bound or duplicated unless another type discipline restricts it.

In 09, “`U(Sig)` is the only value type mentioning `Sig`” follows directly from the displayed grammar. It would remain
true if `U` were renamed `PreparedPlanHandle` and CBPV were removed. Likewise, keeping signals out of payloads follows
from the payload admission rule. Neither property is a CBPV theorem.

The identification of `U(Sig)` with the prepared render plan also merges two objects the boundary document separates. In
CBPV, `U(Sig)` is a suspended signal computation. Musa's `RenderPlan` is finite prepared data/state consumed by the
engine to produce signal behavior. If `U(Sig)` really contains the coinductive behavior, it is not the finite plan the
boundary says crosses. If it denotes only that plan, then `Sig` is the wrong type name and thunk polarity is doing no
demonstrated work.

The operational identification is also too quick. `force` in CBPV initiates a computation. A Musa prepared render plan
is finite data/state that an audio engine repeatedly consumes in blocks. The callback is not thereby the operational
semantics of CBPV `force`, and copying or storing a plan handle is not prohibited by polarity.

T-Prep itself is underspecified: the conclusion contains `r` and `n`, but no premise determines them from gestures,
bindings, graph options, or an expected type. `F(P)` is introduced but does no visible work in the proposed pipeline.

**JUDGED.** CBPV would be load-bearing if the Musa source or intermediate language exposed a meaningful distinction
between returned values and effectful computations, needed thunk/force equations, or embedded evaluation strategies
whose CBV/CBN decompositions mattered. The proposal shows none of those. An opaque, staged
`prepare : GestureTimeline × Bindings × Seed -> RenderPlan` API enforces the existing boundary with much less apparatus.
A monad or effect system would be appropriate only if Musa exposes audio computations/effects; plain staging is
sufficient for the architecture now described.

CBPV is good explanatory vocabulary here. It is not a demonstrated common calculus.

### 5.2 The Dependent ML decidability argument does not survive the proposed index language

**VERIFIED.** Xi and Pfenning's result is parameterized by a chosen constraint domain: DML(C) typechecking is decidable
modulo decidable constraint satisfaction in C. It is not a license to add arbitrary first-order index operations. Their
development also makes the annotation tradeoff explicit: pure inference is no longer generally possible, and sufficient
type annotations are required. The cited graded-modal-types implementation similarly relies on signatures and solver
support; it explicitly says typechecking is undecidable in general for its full setting and leaves stronger inference as
future work.

09's metrical index language contains more than linear rational or integer arithmetic:

- symbolic finite sets of rational `(phase, period)` pairs;
- piecewise-constant objects with symbolic change points;
- union and set equality or inclusion;
- concatenation that translates the second plan by a symbolic extent;
- restriction with phase/change-point rebasing;
- scaling of phases, periods, change points, and extent.

No syntax, equality theory, canonical form, unification procedure, constraint language, or annotation discipline is
given for these objects. If both a scaling factor and a period/extent are variables, terms such as `r * d` are not
linear arithmetic. The claim “So type checking is LRA/LIA constraint solving” is false as written.

**JUDGED.** A deliberately restricted plan language can certainly be made decidable—for example, closed canonical plan
literals with no variables, or hash-consed symbolic atoms compared structurally. That restriction would give up much of
the claimed inference and compositional equality. A richer domain might remain decidable with a custom decision
procedure, but practical inference is an unperformed design task, not a consequence of DML. Expect explicit signatures
at abstraction boundaries and potentially noisy equality failures.

Before proposing implementation, specify C completely and prove or implement its normalization and constraint decision
procedure. Test it on higher-order definitions, imported motifs, polymetric overlays, scaling, and restriction. “Most
extents are literal” is not evidence about metrical-plan variables.

### 5.3 Erasure does not make `pad` free

**VERIFIED.** The Rust `Timeline<A>` retains a runtime `extent` field. T-Pad changes a value of extent `d` into one of
extent `e`. Even if all static index expressions and proofs are erased, the operation must construct or return a value
whose runtime extent is `e`; later sequence, overlay, observation, normalization, rendering, and equality can inspect
that extent.

**JUDGED.** There is no contradiction in erasing *index evidence* while retaining the runtime operation, but then the
claim must be stated accurately: checking evidence is zero-cost; `pad` is not. The proposal currently uses DML erasure
to imply that its static additions leave evaluation unchanged while simultaneously adding an operation with observable
dynamic effect. It also duplicates duration statically and dynamically without explaining how the two are kept in
agreement across parsing, elaboration, imports, and errors.

### 5.4 Structural recursion is not R1

**VERIFIED.** R1 is a quotient-invariance claim:

```text
M ≡ N  =>  prepare(M, B, s) = prepare(N, B, s).
```

Structural recursion establishes that a definition follows the structure of an input and, for an inductive input,
terminates. It does not show that the result ignores distinctions erased by semantic equality.

**JUDGED.** R1 follows by construction only under stronger conditions:

1. `prepare` receives the canonical normal form, not the original term or provenance.
2. Equal terms produce literally identical canonical inputs, including payload canonicalization.
3. `prepare` is pure and deterministic relative to explicit `B` and `s`.
4. Bindings, assets, options, traversal order, floating-point conversions, and platform behavior are canonical or have
   an equality compatible with the law.
5. The required output equality is defined—graph equality, plan equality, frame identity, or observational audio
   equality are not interchangeable.
6. `prepare` consults nothing normalization deliberately forgets.

If these hold, R1 is the trivial factorization `prepare = f ∘ normalize`. The important work is defining the private
normalized input and auditing every dependency, not making timelines dependently indexed. Step 3 therefore does not
depend on step 1, contrary to 09's work-order table.

**Concrete alternative.** Give `prepare` a private normalized/canonical input type that can only be obtained through the
kernel normalizer, make every nondeterministic input explicit, and property-test R1 by generating equivalent term forms.
Add deterministic golden tests at the graph/plan equality actually promised. This can be done on the current kernel.

### 5.5 The SPJ “Proposition 7” is a useful heuristic, not the cited theorem

**VERIFIED.** Peyton Jones §6.5 argues locally for retaining the simple `let`/`letrec` distinction through polymorphic
typechecking and efficient implementation, then erasing or transforming it afterward. The book does not state a general
“Proposition 7” saying every distinction must survive to its last independent consumer.

**JUDGED.** “Keep a distinction until its last consumer” is a sound engineering heuristic, already aligned with Musa's
layering. It should be presented as the notebook's extrapolation, not a cited proposition. More importantly, the
heuristic does not decide that two types represent the same thing. That requires inspecting the translation and
consumers—as the studio probe demonstrates.

## 6. Empirical code audit

### 6.1 P-2 found a real commuting equality, then overread it

**VERIFIED.** The named test asserts, in substance,

```text
scale.class(degree) == frame.pitch(degree).map(Pitch::pitch_class)
```

and it passes. This is a legitimate commuting triangle: realize a degree directly without register, or realize it in a
frame and then forget register. The test does not call `Frame::locate`.

`Frame::locate` is a partial lookup in a selected scale collection. It rejects pitches that are not unaltered members of
that collection. The leading-tone predicate in the reviewed `voice_leading.rs` checks the contextual raised seventh in
minor. A natural-minor frame's generic `locate` is not the missing abstraction for that question. A harmonic-minor
`Scale::class(Degree::new(7))` comparison can reuse the scale model, but that is not P-2's claimed quotient map.

**JUDGED.** P-2 successfully corrected P-1's too-narrow search. It then repeated the notebook's characteristic error at
a higher level: a suggestive vocabulary match became “the quotient tower exists” without checking the map's domain,
codomain, partiality, or the test's actual calls. The surviving result is ordinary representation consistency, not
evidence for the proposed motive.

The three pitch coordinates likewise deserve exact names. `chromatic_height` is an integer coordinate that forgets
spelling; `pitch_class` in this code forgets register while preserving spelling. Calling all three “named quotients” is
acceptable informally only if those different equivalence relations remain explicit.

`analysis/motion.rs` does contain local reduction: `simple` applies `rem_euclid(7)` and `rem_euclid(12)` to an
`Interval` for counterpoint consonance and motion rules. That is not evidence that the analysis layer is hand-rolling a
missing pitch quotient. It is an intentionally local view of an interval, with its bass-context exception supplied by
the caller. The committed leading-tone predicate similarly asks a key-relative two-coordinate question that is not
equivalent to membership in the key signature's natural-minor collection. These two files therefore do not rescue P-1;
they show why following the exact consumer matters.

### 6.2 `StudioGraphSpec` does cross a crate boundary

**VERIFIED.** The syntactic observation in P-3 is true: downstream code seldom spells the type name. The semantic
conclusion is false. `musa_audio::lower_studio` publicly returns `(StudioGraphSpec, StudioLowering)`.
`musa-project/src/playback.rs` calls it, binds the inferred `spec`, and passes that value to `compile_graph`. Rust type
inference does not make a public type stop crossing a crate boundary.

**JUDGED.** Searching only for the type name repeated P-1's scoping mistake in a different form. Boundary audits must
follow values through public signatures and callers, not only textual type mentions.

### 6.3 The two studio specs are different justified representations

**VERIFIED.** Compiler `StudioSpec` retains editable source intent: named patches, buses, signals, assignments, routes,
sends, source locations, and exact/source-level values. Audio `StudioGraphSpec` is a lowered processor DAG: `NodeId`s,
processor kinds, typed ports, edges/modulations, graph options, and an output. `lower_studio` is the translation: it
resolves names and routing and materializes processor/source/envelope structure for `compile_graph`.

`StudioGraphSpec` derives `Clone`, `Debug`, and `Default`, not `PartialEq` or `Hash`, exactly as P-3's correction says.
Prompts 130, 131, and 132 are all still `pending`, so instrument bindings B are not yet the object whose computable key
can be audited.

**JUDGED.** This is not duplicate representation of one graph. It is a normal deep-module boundary between source
semantics and executable graph specification. Step 4's instruction to “settle” the seam starts from the wrong
presumption. The seam is already substantially settled; future work should verify that every field has a last consumer,
not collapse the two structures.

The editor-side result survives: `StudioFacts` and `StudioEdit` support a source-canonical projection and text edits,
not a second mutable studio AST. That is a good boundary.

### 6.4 The audio layer already checks the advertised bug class

**VERIFIED.** `musa-audio` defines `PortKind` variants for audio channel counts, control, gate, and note events.
`compile_graph` validates port compatibility and graph output. The tests `port_mismatch_is_rejected` and
`channel_mismatch_requires_adapter` pass. Rate is currently a graph option rather than a per-node symbolic index;
channel shape is attached to ports.

**JUDGED.** D-4's claim that `Sig[r;n]` gives the audio domain a “real type system” and catches what the positional DAG
cannot is stale against the implementation. There may be value in moving some checks earlier or expressing source-level
polymorphism, but that requires a demonstrated caller and error-quality study. A generic index mechanism shared with
musical extent is not justified merely because both domains contain numbers. Their operations, equality theories,
failure modes, and runtime relevance differ. A single public abstraction over them would be shallow and leaky.

## 7. The Grothendieck framing

### 7.1 The notebook alternates between intersection and factorization

**VERIFIED.** Early residue tests repeatedly ask what all realizations retain and reject information when several
realizations ignore it. The proposal later constructs a product of axes rich enough for different realizations to read
different components. Those are different constructions.

**JUDGED.** “The smallest data every realization agrees on” is an intersection/common quotient. “An object through which
every realization factors” is a jointly sufficient carrier and may contain coordinates ignored by any particular
realization. There is no universal reason to keep only data read by two consumers, as 01's heuristic suggests. A field
needed by exactly one legitimate realization may belong in a factorization carrier; a field shared by several may still
belong in a derived view.

The proposal defines no category of pieces, no category of realizations, no functor or cone, no factorization maps, and
no initial/final or representing property. “All are forgetful” is asserted, not typed. Engraving does not simply keep
the finest pitch payload and present the base; it introduces layout choices and can omit or infer musical information.
Performance adds interpretation. Analysis can add a theoretical reading rather than only forget. Sound depends on
instrument, acoustics, and rendering choices. A universal factorization may still exist, but none is exhibited.

### 7.2 Where the framing did earn its keep

**JUDGED.** The Grothendieck method was useful in one real way: it forced the notebook to look for maps, quotients, base
change, and representational compatibility instead of merely accumulating fields. That led to good questions about pitch
forgetfulness, fixed-duration overlay, and the score/signal boundary. The discipline of preserving refutations also
improved the inquiry.

It did not discover the final type system. In particular, L18's synchronization assumptions are already written in the
law, so the method's diagnostic about an “external side condition not in the theorem” does not apply. The final product
of polarity, indices, and payload actions is ordinary indexed-language engineering until a universal property derives
it. Category-theoretic terminology currently decorates the synthesis more than it constrains it.

The right next Grothendieck-style move is not to implement the product. It is to name a candidate category and attempt
five explicit factorization diagrams on hostile examples. If the morphisms cannot even express an engraving that
revoices, a performance with rubato, a tuning-dependent realization, an analysis with competing readings, and a live
electronic response, the object is not universal enough.

## 8. Additional errors and omissions

### 8.1 Smaller mathematical overclaims

**JUDGED.** Several claims are plausible only after weakening:

- Duration is a homomorphism from sequence to addition and overlay to maximum. It does not forget “exactly
  multiplicity”; it also forgets payloads, positions, spans, support, and construction history.
- The nonnegative `max/+` structure lacks the absorbing additive zero required of an ordinary semiring. Tropical
  max-plus uses negative infinity as the additive identity. The notebook notices that sequence/overlay are not the
  desired semiring, but “music has no absorbing zero” is an interpretation, not a discovered categorical obstruction.
- Levy's conservativity result about complex values and computation equations does not prove that adding the proposed
  indices is conservative over Musa's kernel. A Musa-specific preservation/conservativity theorem is still required.
- Roadmap §2's layer distinctions do not “fall out” as instances of one quotient fact. A dynamic marking is not a point
  in a group action whose quotient is decibels; its realization depends on instrument, context, and interpretation.
  Voice versus mixer track and part versus synthesizer are ownership/routing distinctions, often many-to-many, not
  coarser orbits of one payload. Non-injectivity is too weak to identify a quotient construction.

### 8.2 Musical framings largely absent

**JUDGED.** Form, timbre, orchestration, and correspondence morphisms are correctly named gaps. Several other omissions
are load-bearing rather than peripheral:

- tuning, temperament, intonation, and pitch-continuous gesture;
- articulation, dynamics, phrasing, breath, bowing, fingering, and embodied instrument technique;
- groove, microtiming, and performance timing independent of metre;
- text, lyrics, phonetics, and text-setting;
- improvisation, open form, chance, performer choice, and incomplete/constraint-based scores;
- spatialization, live electronics, reactivity, and multimedia synchronization;
- texture, hierarchy, prolongation, and multiple incompatible analyses;
- notation systems whose spatial or graphical organization is not a projection of pulse layers;
- cultural ontologies in which “piece,” fixed identity, octave equivalence, score primacy, or tonic-centered analysis is
  the wrong starting point;
- version, arrangement, transcription, quotation, and work identity over history.

No single core must internalize all of these. But a claimed motive must at least show how their realizations factor or
why they are outside the intended category. The current notebook silently takes a Western notation-first, largely
fixed-work perspective and then universalizes it.

## 9. Claims that survived the attack

These findings are worth retaining.

1. **VERIFIED + JUDGED:** Exact rational ambient time plus a finite multiset of typed occurrences is a strong,
   backend-independent temporal kernel for Musa's stated finite-score scope.
2. **VERIFIED + JUDGED:** Current total overlay, sequence, scale, and observation form a coherent small algebra. The
   conditional synchronization law is not a defect that demands a smaller term language.
3. **VERIFIED + JUDGED:** Signals and signal graphs should remain outside the finite kernel. Crossing through a
   prepared, opaque render plan is the right architectural boundary.
4. **JUDGED:** R1 is the right law to demand of preparation, even though 09 does not derive it. Defining preparation on
   canonical normal forms is a promising implementation strategy once all side inputs and equality notions are made
   explicit.
5. **VERIFIED + JUDGED:** Local forgetful maps in pitch handling are real and useful. The scale test checks a genuine
   commuting equality under a chosen scale/frame.
6. **JUDGED:** Indexed or graded types may be useful locally—for audio port contracts, closed extent assertions, or a
   synchronized-section witness—without becoming the universal core or one cross-domain abstraction.
7. **JUDGED:** Pulse-layer sets are a plausible analysis representation for a bounded family of metric phenomena. They
   should remain an analysis candidate until Q-B is answered empirically.
8. **VERIFIED + JUDGED:** `StudioFacts`/`StudioEdit` respect source canonicality, and the compiler/audio studio seam is
   a legitimate lowering boundary.
9. **JUDGED:** The SPJ-inspired last-consumer rule is a useful design heuristic when treated as a heuristic and applied
   after following actual consumers.
10. **VERIFIED + JUDGED:** Keeping P-1 and D-1 beside their refutations is excellent research practice. The notebook is
    more trustworthy for recording them; it now needs to apply the same standard to the remaining overclaims.
11. **VERIFIED:** 09 correctly preserves the language specification's two-stage evaluation boundary: the higher-order
    elaboration calculus evaluates to a first-order kernel term, which evaluates separately to a timeline. The two
    strong-normalization results remain separate. This staging fact survives even though T-Pad and T-Restr contradict
    09's further claim that all proposed dynamic semantics are unchanged.

## 10. What I would do instead

### 10.1 Repair the research claims before opening implementation prompts

Append or place refutations beside the claims they overturn:

- mark 09's “steps 1–4 touch no forbids” as false;
- mark equal-extent overlay as falsified by normal unequal-duration musical layers and the absence/rest distinction;
- replace the “pitch quotient tower” with a typed diagram of heterogeneous contextual maps;
- retract harmonic-function-as-tonic-orbit;
- demote split-idempotent motif to an uninstantiated analogy;
- correct the DML/LRA claim and the index-erasure claim;
- correct the `StudioGraphSpec` boundary and audio-type-system claims;
- restate R1 with its actual hypotheses.

The notebook's own discipline argues for repairs in place rather than quietly letting 10 supersede the false text.

### 10.2 Keep the kernel deep and total

Do not expose the metrical plan or extent expression through every kernel caller. Preserve the narrow facade and the
opaque implementation. If a consumer needs alignment, give that consumer a validated witness as described in §3.6. If
repeated callers later agree on one operation, consider a derived combinator that elaborates to the existing kernel
forms. Do not make the primitive algebra less expressive to simplify one theorem.

### 10.3 Separate the three research programs

The proposal's “axes” should become separate investigations because they have different consumers and proof burdens:

1. **Temporal alignment:** Which compiler/render/analysis transformations need equal extents or grids? Define their
   exact preconditions and diagnostics.
2. **Meter analysis:** Build a provenance-preserving analysis model and test it against regular, polymetric,
   non-isochronous, ametrical, gradual, and performance-timed examples.
3. **Audio contracts:** Extend the existing audio-owned `PortKind`/graph validator only when source programs require
   earlier or polymorphic checking. Keep rate/channel logic inside the audio deep module.

Shared implementation machinery can be extracted later if their callers reveal genuinely common operations. Similar
surface notation for indices is not enough.

### 10.4 Prove R1 at the real boundary

After prompts 130–132 define gestures and bindings, specify:

- the exact domain and equality of normalized gestures;
- the canonical representation/equality of bindings and assets;
- whether equality is of graph specs, render plans, sample frames, or observations;
- every source of nondeterminism and floating-point/platform variance.

Then make `prepare` consume only a canonical private representation and test quotient invariance. This work is valuable
independently of dependent types, CBPV, and meter.

### 10.5 Restart the motive question with hard factorization cases

Define the category before naming its motive. At minimum, give objects, morphisms, composition, identities, the target
of each realization, and the claimed universal property. Then construct explicit factorization maps for:

1. a conventionally engraved tonal score with an anacrusis and staggered voice entries;
2. an unmeasured chant or text-led piece;
3. a microtonal or just-intonation piece whose tuning is realization-dependent;
4. a feathered/accelerating or rubato passage separating notation from performed time;
5. an open-form or live-electronic work with performer/environmental input.

Require the same candidate to explain competing analyses rather than pretending analysis only forgets. Only after those
maps exist should correspondences or idempotent completion be evaluated.

## 11. What I could not check

- **UNCHECKED:** I did not survey a representative corpus of actual Musa scores to count how often a hypothetical
  alignment witness or ambient `pad` would be requested. The repository history and ordinary counterexamples are enough
  to reject compulsory padding now, but corpus evidence would help design a future derived operation.
- **UNCHECKED:** I did not implement a decision procedure for any restricted metrical-plan language. My finding is that
  09 has not specified one and its LRA/LIA reduction is false, not that no decidable restriction can exist.
- **UNCHECKED:** I did not prove that no useful category of musical correspondences or idempotent completion exists. The
  finding is that the proposed one lacks definitions and musical witnesses.
- **UNCHECKED:** I did not validate the notebook's broad historical claims about every tradition or every composer's
  working practice. The named counterdomains are sufficient to defeat universality; they are not a substitute for
  specialists in those repertoires.
- **UNCHECKED:** Prompts 130–132 are pending, so the eventual instrument-binding representation and full R1 preparation
  function do not yet exist to audit. Any claim that their final design will or will not satisfy R1 is premature.

## 12. Final verdict

The notebook has not found “the motive for music.” It has found a good finite temporal kernel, a good architectural
boundary to audio, several useful local representation maps, and some candidate static checks. Those do not assemble
into a universal object merely by being placed in an indexed CBPV grammar.

Step 1 should not proceed. It is contrary to the governing decision, revives an operation already removed for lack of a
caller, changes more semantics than advertised, and makes a normal musical operation ill-typed in order to remove a
correctly stated condition from one algebraic law. The practical route is to preserve total overlay and introduce
narrow, consumer-owned alignment evidence only when a real transformation requires it.

The most important repair is methodological: stop treating an evocative identification as established when the maps have
not been typed. That was the cause of P-1 and D-1, and it remains the cause of the tonic orbit, quotient tower, split
idempotent, CBPV boundary, DML decidability, and R1 claims. The next phase should be smaller and harsher: define the
maps, state their domains and side conditions, and make them survive the musical cases the current proposal leaves out.
