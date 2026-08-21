# Where the theory comes from

**Status: candidate — reference.** Where every theoretical construction Musa implements comes from.

Musa implements a bounded set of theoretical constructions. Every one of them either comes from a named source — a
chapter of [Open Music Theory](https://viva.pressbooks.pub/openmusictheory/) for the music, a chapter or paper for the
calculus — or is proved locally, and this chapter says which, for each. Sections 1–12 are the music. **Section 13 is the
calculus**, added when prompt 128's amendment made the core dependent and gave it constructions that are emphatically
not Musa's own.

The reason to keep the map is not scholarly manners. A convention that is built into a compiler stops looking like a
convention: a musician who disagrees with it has nothing to disagree *with* unless the software can say where it got the
idea. Naming the chapter makes the decision arguable again.

**How the citations are checked.** OMT is cited by filename — `` `017-triads.md` `` — and `scripts/check-docs.sh`
verifies that every filename cited anywhere in `docs/` exists in the local OMT checkout (`$OMT_ROOT`, default
`~/Code/papers/music-theory/open-music-theory`). A chapter that is renamed upstream breaks the build rather than
becoming a dead reference. The check skips with a notice when the checkout is absent, so the repository does not require
it to build.

## 1. Notation

| What Musa implements | Where it comes from |
| --- | --- |
| staff position, clefs, ledger lines | OMT `002-notation-of-notes-clefs-and-ledger-lines.md`, `003-reading-clefs.md` |
| octave numbering (`c4` is middle C) | OMT `006-american-standard-pitch-notation-aspn.md` |
| accidentals and half/whole steps | OMT `005-half-steps-whole-steps-and-accidentals.md` |
| note values, dots, ties, tuplets | OMT `009-notating-rhythm.md`, `012-other-rhythmic-essentials.md` |
| simple and compound time signatures | OMT `010-simple-meter-and-time-signatures.md`, `011-compound-meter-and-time-signatures.md` |
| dynamics, articulation, tempo as *notation* | OMT `007-other-aspects-of-notation.md` |
| voices and parts as texture, not as tracks | OMT `008-texture.md` |
| polymeter as two parts each in its own meter | OMT `098-twentieth-century-rhythmic-techniques.md` §Polymeter |
| repeats with first and second endings | OMT `057-binary-form.md` |

`meter none` — an unmeasured passage — is not from a chapter. It is a notational fact Musa needs because chant and
cadenzas exist, and it is specified in [`01-surface.md`](01-surface.md).

## 2. Pitch, interval, and pitch class

| What Musa implements | Where it comes from |
| --- | --- |
| `Pitch` and `Interval` as staff-and-chromatic pairs | OMT `005-half-steps-whole-steps-and-accidentals.md`, `016-intervals.md` |
| interval names (`M3`, `d5`, `P8`) and their qualities | OMT `016-intervals.md` |
| `NoteName`: a spelled pitch class, octave forgotten | Musa definition — the quotient lemma, [`03-musical-domains.md`](03-musical-domains.md) §1 |
| `Pc12`: `ℤ/12ℤ`, where `B♯ = C` | OMT `099-pitch-and-pitch-class.md`, `100-intervals-in-integer-notation.md` |

Musa proves two things here rather than citing them, because they are what make spelling survive arithmetic: that
interval addition is a **faithful action** on `ℤ²`, and that the action **descends** to the octave quotient. Both are in
[`03-musical-domains.md`](03-musical-domains.md) §1, and the consequence a composer sees is
[What musa refuses to blur §1](../../book/src/concepts/distinctions.md#1-written-pitch-is-not-sounding-pitch).

## 3. Scales, keys, modes, and collections

| What Musa implements | Where it comes from |
| --- | --- |
| major scales, degrees, key signatures | OMT `013-major-scales-scale-degrees-and-key-signatures.md` |
| the three minors as three collections under one key | OMT `014-minor-scales-scale-degrees-and-key-signatures.md` |
| diatonic modes | OMT `015-introduction-to-diatonic-modes-and-the-chromatic-scale.md`, `105-diatonic-modes.md` |
| octatonic, whole-tone, hexatonic, pentatonic | OMT `106-collections.md` |
| reading a passage against a collection | OMT `107-analyzing-with-modes-scales-and-collections.md` |
| `Degree` and `Frame`: an ordinal plus its register | Musa definition — the round-trip lemma, `../03-musical-domains.md` §2 |

The separation of `Key` from `Scale` is Musa's, forced by minor: OMT `014` describes three collections that share one
key signature, and no single collection can stand for the key. See
[What musa refuses to blur §3](../../book/src/concepts/distinctions.md#3-a-key-is-not-a-scale).

## 4. Harmony

| What Musa implements | Where it comes from |
| --- | --- |
| triads and their qualities | OMT `017-triads.md` |
| seventh chords | OMT `018-seventh-chords.md` |
| inversion and bass position | OMT `019-inversion.md` |
| Roman numerals | OMT `020-roman-numerals.md` |
| figured-bass figures on numerals | OMT `021-figured-bass-and-roman-numerals-with-figures.md` |
| chord symbols | OMT `075-chord-symbols.md` |
| applied chords and tonicization | OMT `050-tonicization.md` |
| modulation to closely related keys | OMT `051-extended-tonicization-and-modulation-to-closely-related-keys.md` |
| modal mixture | OMT `061-modal-mixture.md` |
| Neapolitan sixth | OMT `062-neapolitan-sixth-chords-ii6.md` |
| Italian, French, and German augmented sixths | OMT `063-augmented-sixth-chords.md` |
| altered dominants | OMT `071-altered-and-extended-chords.md` |
| embellishing tones, as a reason a slice may fit no chord | OMT `039-embellishing-tones.md` |

That a numeral carries **no quality** is a Musa result, not an OMT one: the quality-is-the-collection's lemma
(`../03-musical-domains.md` §3) shows that `ii` is minor in major and `II` is major in Dorian as a *consequence* of the
collection, so storing a quality on the numeral would let it contradict the notes.

## 5. Voicing

| What Musa implements | Where it comes from |
| --- | --- |
| SATB spacing, doubling, ranges, crossing | OMT `022-chords-in-satb-style.md` |
| close position, drop voicings, rootless voicings | OMT `076-jazz-voicings.md` |
| omitting the fifth of a dominant seventh | OMT `076-jazz-voicings.md` §Omitting Notes |
| a voicing forgets down to its chord class, not up | Musa definition — the forgetfulness lemma, `../03-musical-domains.md` §3 |

## 6. Sequences, schemas, and transformations

| What Musa implements | Where it comes from |
| --- | --- |
| diatonic sequences: falling fifths, thirds, seconds, parallel sixths | OMT `049-diatonic-sequences-in-middles.md` |
| galant schemas (Prinner, Fonte, Monte, Fenaroli, Quiescenza, and the rest) | OMT `033-galant-schemas.md`, `034-galant-schemas-summary.md` |
| the rule of the octave | OMT `035-galant-schemas-the-rule-of-the-octave-and-harmonizing-the-scale-with-sequences.md` |
| neo-Riemannian `P`, `L`, `R` and their cycles | OMT `072-neo-riemannian-triadic-progressions.md` |

`std::transformational` implements the three parsimonious transformations and says in the module what it does not
attempt: it is not a general `GIS`, and nothing in it decides whether a progression *is* neo-Riemannian.

## 7. Post-tonal

| What Musa implements | Where it comes from |
| --- | --- |
| pitch-class sets, normal order, `Tn`/`TnI` | OMT `101-pitch-class-sets-normal-order-and-transformations.md` |
| set class and prime form | OMT `102-set-class-and-prime-form.md` |
| interval-class vectors | OMT `103-interval-class-vectors.md` |
| twelve-tone rows and their `P`/`I`/`R`/`RI` forms | OMT `108-basics-of-twelve-tone-theory.md` |
| row naming conventions, stated rather than assumed | OMT `109-naming-conventions-for-rows.md` |
| symmetric rows with fewer than 48 distinct forms | OMT `110-row-properties.md` |

The finite-closure lemma (`../03-musical-domains.md` §4) is Musa's: composing a row with any index transformation yields
a bijection, which is why row operations are total once construction has succeeded — and why construction is the partial
step, returning what repeated and what never arrived.

## 8. Counterpoint and voice leading

Every style rule Musa checks is in the registry at `crates/musa-score/src/analysis/rules.rs`, and **every row of that
registry carries a citation** — a rule whose source is "everyone knows" is exactly what the registry exists to refuse.
The full table, with each rule's strength and the section it cites, is
[`07-analysis.md`](07-analysis.md#voice-leading-and-counterpoint).

| Profile | Where it comes from |
| --- | --- |
| `satb_common_practice` | OMT `022-chords-in-satb-style.md` |
| `species_1` … `species_4` | OMT `023-introduction-to-species-counterpoint.md` through `027-fourth-species-counterpoint.md` |
| `species_5` | OMT `028-fifth-species-counterpoint.md` — computed as the union of the first four rule lists, because that is what the chapter says florid counterpoint is |
| `jazz_voice_leading` | OMT `076-jazz-voicings.md` |

The profiles are named in the request precisely because they disagree. OMT `022` and OMT `076` take opposite views of
parallel fifths, and they are both right about the music they describe; a reading that averaged them would describe
none. See
[What musa refuses to blur §8](../../book/src/concepts/distinctions.md#8-a-failed-claim-is-not-a-style-violation).

## 9. Cadences and analysis

| What Musa implements | Where it comes from |
| --- | --- |
| cadence types and the criteria each one needs | OMT `036-introduction-to-harmony-cadences-and-phrase-endings.md` |
| tonicization read apart from modulation | OMT `050-tonicization.md`, `051-extended-tonicization-and-modulation-to-closely-related-keys.md` |
| a minor key read through natural *and* harmonic collections | OMT `014-minor-scales-scale-degrees-and-key-signatures.md` |
| the standing of a reading that has alternatives | Musa definition — the admission rule, [`07-analysis.md`](07-analysis.md) §2 |

Each cadence finding names the criteria it met and the ones it did not, taken from OMT `036` — which is what lets a
report say "authentic, but the soprano does not arrive on the tonic" instead of picking a label.

## 10. Performance and sound

| What Musa implements | Where it comes from |
| --- | --- |
| swing as a performed ratio over straight notation | OMT `074-swing-rhythms.md` |
| drum patterns as both performed and programmed practice | OMT `084-drumbeats.md` |
| dynamics and tempo as marks whose realization is a separate choice | OMT `007-other-aspects-of-notation.md` |

OMT `074` is explicit that the swing ratio varies and is not what the page says, which is the whole reason notated
duration and performed duration are different types
([What musa refuses to blur §10](../../book/src/concepts/distinctions.md#10-notated-time-is-not-performed-time)).

Instruments, the studio, and orchestration are specified in [`08-performance-and-sound.md`](08-performance-and-sound.md)
against OMT `114-core-principles-of-orchestration.md` and `116-transcription-from-piano.md`, and are built by prompts
156–168. Their citations belong to that specification until then; prompt 169 adds their half of this handbook.

## 11. What Musa proves for itself

Six results are not cited because they are not OMT's; they are stated and proved in
[`03-musical-domains.md`](03-musical-domains.md), and each exists to license an operation that would otherwise be a
convention.

| Result | What it licenses |
| --- | --- |
| faithful action | transposition preserves spelling by construction, not by a repair pass |
| the action descends | interval arithmetic on `NoteName` is well defined |
| round trip | a degree plus a frame determines exactly one pitch, and back |
| the quality is the collection's | a Roman numeral needs no quality field |
| voicing forgetfulness | a voicing has one chord class; a chord class has many voicings |
| finite closure | row transformations are total after construction |

Everything below the musical layer — the core calculus, the event-track, elaboration, and normalization — is specified
in [`02-core-calculus.md`](02-core-calculus.md) and `../events/`. No music theory is cited there, because none is used:
the event track knows about exact time and typed occurrences and nothing about notes. That layer is **not** all Musa's
own, and §13 and §14 below say which parts are borrowed and from where. What is Musa's own there is short: the
`Storable` constraint, the three-outcome budget law, the phase environment, the refusals, and derived identity as a
computed triple.

## 12. Cited, and deliberately not implemented

OMT covers a great deal that Musa does not, and the gap is a design decision rather than a backlog. Musa builds the
constructions that are **decidable from the notated score** and refuses the ones that require a reading it cannot
defend. It has no opinion on Schenkerian reduction, no formal-function analyzer, no style classifier, and no way to
decide whether a passage *is* in a style — `../07-analysis.md` §8 lists the limits this produces and the false positives
each one causes, stated up front rather than discovered.

The larger omission is the one worth saying plainly: the theory implemented here is one pedagogical tradition, mostly
Western and mostly common-practice. That a convention is built into this compiler is a fact about this software, not a
fact about music.

## 13. The calculus

Prompt 128's amendment replaced Musa's type discipline with a dependent core, and almost nothing in that core is Musa's
invention. This section says where each construction comes from, for the same reason the music sections exist: a
construction built into a compiler stops looking like a choice, and naming its source makes it arguable again. It also
does one thing the music sections do not — it records which sources are **on disk in this workspace** and which are
cited by name only, so a reader can tell what can be checked here and what has to be looked up.

Local paths below are relative to `~/Code/papers/logic-and-computation/`. Unlike the OMT citations, these are not
verified by `scripts/check-docs.sh`: the corpus is outside the repository and is not required to build it.

### 13.1 On disk

| What Musa implements | Where it comes from |
| --- | --- |
| the surface → core → evaluation architecture, and why Musa is *not* the enriched-calculus arrangement | Peyton Jones (1987), `software-engineering/implementation-of-functional-programming-languages/03-translating-a-high-level-functional-language-into-the-lambda-calculus.md` §3.1 |
| case-tree compilation of nested patterns: the variable, constructor, empty, and mixture rules — which is also what record and enum patterns, and the nested patterns method and operator sugar produce, compile through | Peyton Jones and Wadler, same book, `04-structured-types-and-the-semantics-of-pattern-matching.md` and `05-efficient-compilation-of-pattern-matching.md` |
| surface forms that add no term to the calculus — `with` update, `?`, `if`, method syntax, operator syntax — as a source-to-source translation rather than as new constructs | same book, `03-translating-a-high-level-functional-language-into-the-lambda-calculus.md` |
| list comprehensions as sugar over `map` and `filter` — cited to be **refused** for now (`10-traits.md` §9) | same book, `07-comprehensions.md` |
| the shape a type checker takes as a program: constraint generation separated from solving | Hancock, same book, `08-polymorphic-type-checking.md` and `09-a-type-checker.md` |
| `Y` and the fixed-point combinator — cited to be **refused** (§1.3) | same book, `02-the-lambda-calculus.md` §2.4 |
| dependent function and sum types, families over a base, and what a universe is | Jacobs, *Categorical Logic and Type Theory*, `type-theory/categorical-logic-and-type-theory/10-first-order-dependent-type-theory.md` and `11-higher-order-dependent-type-theory.md` |
| normalization by evaluation: evaluate into a semantic domain, quote back, compare normal forms | Abel and Sattler (2019), `type-theory/normalization-by-evaluation-for-call-by-push-value-and-polarized-lambda-calculus/text.md` |
| logical relations as the technique the NbE obligations are discharged with | `type-theory/logical-relations-as-types/text.md` |
| de Bruijn levels for quoting under binders, and the free-variable discipline | McBride and McKinna, *I am not a Number — I am a Free Variable*, `type-theory/functional-pearl-i-am-not-a-number-i-am-a-free-variable/text.md` |
| call-by-push-value — cited to be **refused** (§1.3, and note 42 §7.2) | Levy, `type-theory/call-by-push-value-decomposing-call-by-value-and-call-by-name/` |
| why non-termination is not the effect the no-go theorem is about | Pédrot and Tabareau, `type-theory/fire-triangle-how-to-mix-substitution-dependent-elimination-and-effects/text.md` |
| why Musa's phase environment is **not** two-level type theory | `type-theory/two-level-type-theory/`, read against note 39 §8.2 |

### 13.2 Cited by name

| What Musa implements | Where it comes from |
| --- | --- |
| the base case Musa's assignment rule is: a hole applied to nothing takes its solution directly, which is the pattern fragment's easiest instance and the only one still in force — the fragment itself is **not** implemented (note 51 §7) | Miller (1991), "A logic programming language with lambda-abstraction, function variables, and simple unification" |
| bidirectional type checking: check and infer modes and the two rules that switch | Pierce and Turner (2000), "Local type inference"; Coquand (1996), "An algorithm for type-checking dependent types" |
| coverage, and compiling `match` to eliminators. Dependent pattern matching and index unification are **not** implemented: §1.1 admits no indices, and §1.1's eliminator is non-dependent | Coquand (1992), "Pattern matching with dependent types"; Goguen, McBride, and McKinna (2006), "Eliminating dependent pattern matching" |
| the **stratified index**: a type refined by a value of a separate decidable arithmetic domain, erased before evaluation, with equality decided by a solver rather than by unification (`02-core-calculus.md` §1.5). Musa takes the stratification and the erasure; it takes neither existential indices nor the assertion-and-obligation discipline, because the fragment the corpus generates is equality of two linear forms | Xi and Pfenning (1999), "Dependent types in practical programming"; Xi (2007), "Dependent ML: an approach to practical programming with dependent types" |
| **conversion modulo a theory**: extending the conversion relation by a decidable first-order theory rather than adding a second equality beside it, with subject reduction and decidability of type checking preserved. This is the shape `02-core-calculus.md` §3 states — `≡` is the congruence generated by β, η, δ, ι together with §1.5's index theory | Strub (2010), "Coq Modulo Theory"; Blanqui (2005), "Definitions by rewriting in the calculus of constructions" |
| **refinement types**, cited to record what Musa's index stratum is *not*. A refinement type is a subset carved by a predicate over the inhabitant and its checking rests on subtyping; Musa's conversion is invariant — `Row(12)` is not a `Row` — and its index is a parameter beside the type rather than a proposition about the value. `Shape::Indexed` carries the accurate name | Freeman and Pfenning (1991), "Refinement types for ML"; Rondon, Kawaguchi, and Jhala (2008), "Liquid types" |
| strict positivity as the admission condition for an inductive family | Coquand and Paulin (1990), "Inductively defined types" |
| the predicative universe hierarchy and its consistency consequence | Martin-Löf (1984), *Intuitionistic Type Theory* |
| K, uniqueness of identity proofs, and what admitting it as an axiom would foreclose — cited to record the decision `02-core-calculus.md` §1.4 replaced, which is deleting the identity type rather than choosing a K for it | Streicher (1993), "Investigations into intensional type theory"; Hofmann and Streicher (1998), "The groupoid interpretation of type theory"; Hedberg (1998), "A coherence theorem for Martin-Löf's type theory" |
| well-founded recursion as the general form of a terminating definition, with structural decrease as its special case | Nordström (1988), "Terminating general recursion" |
| dictionary-passing elaboration of a class-like construct, which `10-traits.md` builds on | Wadler and Blott (1989), "How to make ad-hoc polymorphism less ad hoc" |
| coherence, and the design space that overlap, specialization, and defaulting sit in — Musa refuses all three and `10-traits.md` §9 says why | Peyton Jones, Jones, and Meijer (1997), "Type classes: an exploration of the design space" |
| the orphan rule as the module-level condition that makes coherence checkable rather than aspirational | the same paper's treatment of instance scoping, and the Haskell 98 Report's rule that instances are program-global regardless of import |
| termination of instance resolution — cited to say why Musa needs none of it: an `impl` carries no context, so `10-traits.md` §4 is a single table read with nothing to recurse into and nothing to measure | the Paterson conditions, as recorded in Sulzmann, Duck, Peyton Jones, and Stuckey (2007), "Understanding functional dependencies via constraint handling rules" |
| a closed value of a counting family represented as a literal count over an unchanged declaration, unfolded one constructor at a time by the eliminator (`02-core-calculus.md` §5.10) | Lean 4's kernel, `~/Code/lean4/src/kernel/`: `Expr.lit (Literal.natVal n)` over an unchanged `inductive Nat`, `type_checker.cpp`'s `reduce_nat` for the collapse, `inductive.cpp`'s `nat_lit_to_constructor` for the one-level unfold, and `is_def_eq_offset` for conversion without a walk. Musa keeps **one** canonical form where Lean tolerates two, derives the counting property from the declaration's shape where Lean hard-wires `Nat`, and adds no kernel arithmetic |

### 13.3 Musa's own, and priced

Four things in the calculus are not borrowed, and each is priced in
[`../../notes/research/language-design-closure/42-dependent-core-decision.md`](../../notes/research/language-design-closure/42-dependent-core-decision.md)
rather than asserted here:

| Construction | Where it is argued |
| --- | --- |
| the `Storable` constraint, with generated-only instances | [`02-core-calculus.md`](02-core-calculus.md) §1.2; the event track's side is `../events/12-payload-admission.md` |
| the three-outcome budget law, with exhaustion named as its own outcome | [`02-core-calculus.md`](02-core-calculus.md) §4 |
| the expansion phase environment and its law 11 | [`02-core-calculus.md`](02-core-calculus.md) §5.9 |
| the refusals — no cumulativity, no `partial`, no CBPV, no signed integer | [`02-core-calculus.md`](02-core-calculus.md) §1.3 and §1.4, note 42 §9 |

## 14. Quotation and the expansion phase

`11-quotation.md` adds a second borrowed body of work, and it is worth separating from §13 because it is about a *phase*
rather than about the calculus. None of these are on disk in this workspace.

| What Musa implements | Where it comes from |
| --- | --- |
| typed staged quotation: a quoted fragment is a value of a type that says what it is, and splicing is a checked operation rather than text substitution | Taha and Sheard (2000), "MetaML and multi-stage programming with explicit annotations" |
| the modal reading of a quoted term, which is why a quote must close and check before it becomes anything | Davies and Pfenning (2001), "A modal analysis of staged computation" |
| quotation and splicing indexed by **syntactic category** — expression, item, pattern, token tree — with a mismatch as a compile-time error | Sheard and Peyton Jones (2002), "Template meta-programming for Haskell", whose `Exp`/`Dec`/`Pat`/`Type` split is where the four categories come from |
| hygiene: an identifier written in a quote and one spliced into it are different names, and neither captures the other | Kohlbecker, Friedman, Felleisen, and Duba (1986), "Hygienic macro expansion"; Clinger and Rees (1991), "Macros that work" |
| quotation as a **pattern** — matching on a shape written in the source grammar rather than on an untyped tree | Culpepper and Felleisen (2010), "Fortifying macros" |
| that a quotation must share the real parser rather than get a template dialect | root `AGENTS.md`'s "no sublanguage by subtraction", after Peyton Jones (1987), `03-translating-a-high-level-functional-language-into-the-lambda-calculus.md` §3.1 |

Musa's own here is one construction: **derived identity as a computed triple**, `Derived { origin, quotation, path }`,
where every component is produced by the elaborator and none is allocated by the author. Template Haskell mints fresh
names with `newName` in a monad; Musa has no monad and no fresh-name effect, and the position inside the quote's own
tree is what makes uniqueness structural instead. It is argued in [`11-quotation.md`](11-quotation.md) §3 and priced by
the measurement that opens that document.
