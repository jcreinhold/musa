# Musical domains

**Status: candidate.** Where musical practice preserves a distinction, the language gives it a separate type.

Musa uses separate types where musical practice preserves separate choices. The cited Open Music Theory files live in
`~/Code/papers/music-theory/open-music-theory/`; citations name exact files so the definitions remain auditable. Where
OMT supplies practice rather than a mathematical object, the definition is explicitly Musa's and its elementary laws are
proved here.

## 1. Written pitch and interval

Define `WrittenPitch = ℤ × ℤ`. A pitch `(d,c)` records absolute diatonic height and absolute chromatic height. Define
`Interval = ℤ × ℤ`; intervals form an abelian group componentwise and act on pitches by `(d,c) + (i,j) = (d+i,c+j)`.
Named intervals parse to pairs: for example `M2=(1,2)`, `m2=(1,1)`, and `P8=(7,12)`. This represents OMT's joint generic
and specific interval naming (`016-intervals.md`) and retains the spellings that accidentals distinguish
(`005-half-steps-whole-steps-and-accidentals.md`).

**Lemma (faithful action).** `p + 0 = p` and `(p+i)+j = p+(i+j)` by componentwise integer identities. If `p+i=p+j`,
integer cancellation gives `i=j`. Thus transposition is total, associative, and spelling-preserving.

The Rust facade stores these coordinates in checked fixed-width integers. That is an implementation representation, not
a musical accidental bound: triple and more deeply altered spellings are ordinary values, while the unreachable
machine-overflow fringe is rejected explicitly rather than clamped or respelled.

Define spelled pitch class as the octave quotient

```text
SpelledPC = WrittenPitch / ((d,c) ~ (d+7k,c+12k), k∈ℤ).
```

It forgets register but not enharmonic spelling: `C♯` and `D♭` have different diatonic coordinates and are unequal.
**Lemma (the action descends).** The interval action passes to `SpelledPC`: if `(d,c) ~ (d+7k,c+12k)` then
`(d,c)+(i,j) ~ (d+7k,c+12k)+(i,j)`, since both sides differ by the same `(7k,12k)`. The quotient action is therefore
well-defined, and total, associative, and spelling-preserving for the same reasons the action on pitches is. This is
what the surface writes as `root up M3` when `root` is a `NoteName`: the same `up`/`down` operator as on a pitch, with
the octave simply absent rather than chosen. A written interval acting on a spelled pitch class is what lets a
transformation name the root of its image — `L` on a minor triad is the major triad a major third below it — without
first inventing a register the transformation does not have.

Define `pc12 = ℤ/12ℤ` and the forgetful map `χ([(d,c)]) = c mod 12`. This is well-defined because changing the
representative adds `12k`; it is not injective because enharmonic spellings can share `c mod 12`. OMT
`099-pitch-and-pitch-class.md` motivates octave equivalence for pitch-class work, while OMT
`100-intervals-in-integer-notation.md` and `101-pitch-class-sets-normal-order-and-transformations.md` use mod-12
intervals and transformations. Musa never uses `pc12` where notation must retain spelling.

### 1.1 The algebra these lemmas name

The two lemmas above are not facts about pitch alone. They are the definitions of three structures that recur across
every domain in this document, and `std::algebra` declares them so that a reader meets each one once:

| Trait | Method | What it asserts |
| --- | --- | --- |
| `Group<G>` | `unit`, `compose`, `inverse` | the movers compose associatively, and every move can be undone |
| `Action<X, G>` | `act` | the movers move a carrier, and composing then moving agrees with moving twice |
| `Torsor<P, V>` | `difference` | *exactly one* mover joins any ordered pair of points |

`Interval` is the group; `Pitch` and `SpelledPC` are two carriers of it; `Pitch` alone is a torsor over it, because the
faithful-action lemma's cancellation step is precisely the uniqueness a torsor asserts. `SpelledPC` is **not** one, and
the reason is the quotient lemma directly above: `P8` fixes every spelled class, so two classes are joined by infinitely
many intervals rather than by one. A specification that gave both carriers a torsor would be claiming the octave
quotient does not exist.

**The carrier is the first parameter.** This began as a resolution key — instance lookup chose on the first parameter,
and one mover moving several carriers would have put `Action<Interval, Pitch>` and `Action<Interval, SpelledPC>` at one
head. Prompt 146 deletes that mechanism and the ordering survives it as a reading convention, stated at
[`../style-guide.md`](../style-guide.md) §6. It also puts the head where method syntax looks: `p up M3` is `p.act(M3)`,
resolved on `p`.

**Time is an action and not a torsor**, for a reason that is about time rather than about the structures. A `Duration`
is a length and not a displacement — it is nonnegative, and the operation answering the length between two positions
refuses a second position standing before the first — so the movers have no inverses and `Position` carries
`Action<Position, Duration>` alone. §4's row and §5's indexed domains carry the same three structures at the moduli they
are stated over.

**The laws are prose here and law suites in `05-verification.md` §4.** A structure declaration in this language carries
operations and never obligations — a checked law is a proof obligation, a judgment, and a budget, and the language
refuses all three — so §4's law 7 is where the pitch action's identity, composition, and cancellation are actually
checked.

## 2. Scale, key, degree, and register

A `Scale` is a named root plus a nonempty cyclic ordered vector of distinct `SpelledPC` offsets within its period. Its
order supplies step traversal. Major's step pattern is W–W–H–W–W–W–H
(`013-major-scales-scale-degrees-and-key-signatures.md`); minor has distinct natural, harmonic, and melodic collections
(`014-minor-scales-scale-degrees-and-key-signatures.md`); diatonic modes rotate the ordered diatonic collection
(`105-diatonic-modes.md`). Other collections are explicit scale values rather than mislabeled modes
(`106-collections.md`).

A `Key` is a tonal/notational context: spelled tonic, mode family, key signature, and its named collection policies. It
has no register and is not a single scale. A minor key may select different scale collections in different
constructions. `in scale` changes only generative coordinates; `key` emits a contextual score fact. This prevents a
local Dorian phrase from falsely declaring modulation, an interpretive continuum discussed in `050-tonicization.md` and
`051-extended-tonicization-and-modulation-to-closely-related-keys.md`.

A `Degree` is a signed ordinal relative to a scale. `locate(s,p) : Option<(Degree, Register)>` is partial because a
chromatic pitch may not belong to `s`. `realize(s,Degree,Register) : Pitch` is total. Register is an integer lift
through the scale period; it is mandatory whenever a pitch rather than a pitch class is requested.

**Lemma (round trip).** Because scale members are distinct within a period, Euclidean division of a member's ordered
index into quotient/register and remainder/degree is unique. Therefore `locate(s,realize(s,n,r))=some(n,r)` under the
canonical degree representative. A request to realize a key degree without choosing a scale policy and register is
underdetermined and rejected.

## 3. Harmony

A `ChordClass` is `(root : SpelledPC, members : finite nonempty set Interval, labels : metadata)`, normalized so unison
is a member. It describes rooted harmonic membership without register, spacing, doubling, omission, or bass choice. A
`Triad` is the checked subtype whose core has three spelled members arranged as two thirds; quality follows the member
intervals described in `017-triads.md`. Seventh-chord membership follows `018-seventh-chords.md`.

A `Voicing` is a nonempty ordered collection of exact `WrittenPitch` values plus an explicit association to a chord
class. Its bass is its lowest sounding pitch; inversion is classified from which chord member occupies that bass, as in
`019-inversion.md`. Spacing, doubling, and omission are voicing choices (`075-chord-symbols.md`,
`076-jazz-voicings.md`). Consequently a chord symbol or Roman numeral cannot directly sound.

A `ChordSymbol` is a notation annotation encoding a conventional root/quality/extensions/bass description. A
`RomanNumeral` and figured bass are context-dependent analysis results, following `020-roman-numerals.md` and
`021-figured-bass-and-roman-numerals-with-figures.md`. As a *constructive* value the surface writes `roman`, which is
`(ordinal : 1..7, members : 3..7, inversion : < members)` and carries no quality and no collection.

**Lemma (the quality is the collection's).** Let `S` be a scale with offsets `o_0, ..., o_{n-1}` and period `p`, and let
`deg(k)` be the interval from the tonic to the `k`-th degree, counting periods. The diatonic stack of `m` members on
ordinal `k` is `{ deg(k + 2j) - deg(k) : j < m }`. Nothing in that expression mentions a quality: two scales that agree
on `deg` agree on every stack, and two that differ at one offset differ at every stack that crosses it. Hence `ii` is
minor in major and `II` is major in Dorian as a consequence rather than a stipulation, and a `roman` that stored a
quality could contradict the scale it is realized against. The map is partial in exactly one place — the resulting
member set need not be a named chord type, and C harmonic minor's `III7` is the witness. Chromatic chord vocabularies in
OMT 061–071 and Neo-Riemannian operations in `072-neo-riemannian-triadic-progressions.md` are library
constructors/analyses over these types, not compiler ontology.

**Lemma (voicing forgetfulness).** Projecting every pitch of a valid voicing to its interval class above the declared
root yields members of the chord class. The converse is not unique: octave-displacing a member, doubling one, or
choosing another bass produces a different voicing with the same chord class. Hence `voice : ChordClass × VoicingPolicy
→ Voicing` needs a policy and cannot be a coercion.

## 4. Pitch-class sets and rows

A `PCSet` is a finite subset of `pc12`; transposition and inversion are the mod-12 actions described in
`101-pitch-class-sets-normal-order-and-transformations.md`. A `Row12` is a bijection `Fin 12 → pc12`. Its constructor
checks exactly 12 entries and no duplicate. `P`, `I`, `R`, and `RI` are finite derived permutations as in
`108-basics-of-twelve-tone-theory.md`.

Normal order is the rotation of the ascending members packed most tightly to the left; ties break inward — first to
last, then first to the one before last, and so on — and finally by the lowest starting pitch class. Prime form
transposes the normal orders of the set and of its inversion each to begin on zero and takes whichever reads lower. The
interval-class vector has six entries and not twelve, because interval class seven is interval class five heard the
other way round (`103-interval-class-vectors.md`).

A row's matrix is twelve rows: row zero is the row as written, and row *i* is the transposition beginning on the *i*th
pitch class of the inversion about the row's own head, so every column read downward is an inversion. The construction
names no label, because published naming conventions vary (`109-naming-conventions-for-rows.md`). Which transposition is
called P0 is asked separately and by name — `fixed_zero_index` for the convention where P0 begins on pitch class zero,
`moveable_zero_index` for the one where P0 is a stated reference row — rather than by a convention value the operations
carry, because the operations produce rows and not labels. A generic row may have 48 distinct forms; invariant
transformations of a symmetric row can identify forms, so cardinality is computed rather than asserted
(`110-row-properties.md`).

There are 24 affine pitch-class operations: the 12 transpositions `Tₙ(x)=x+n` and the 12 inversions `Iₙ(x)=-x+n`. They
are distinct functions on `pc12`, closed under composition, and every composition again has coefficient `+1` or `-1`, so
this group has exactly 24 elements. Row-form vocabulary additionally chooses whether to reverse index order, giving 48
labelled `P/I/R/RI` operations before stabilizers are considered. Calling the latter “the 48-element pitch-class T/I
group” is false; a symmetric row may also identify several labelled results.

**Lemma (finite closure).** Composition of a row with any `P/I/R/RI` index transformation remains a bijection because
each transformation is a permutation of `Fin 12` followed, where applicable, by a bijection on `pc12`. Thus every row
operation is total and finite.

## 5. Indexed domains: the modulus is a parameter, not a constant

Everything above §4 is written for one division of the octave, and the corpus is not. OMT
`068-equal-divisions-of-the-octave.md` is *about* dividing the octave into n parts and asking which n admit which
symmetries; quarter-tone practice needs 24, Turkish and Arabic theory are usually modelled at 24 or 53, and a 19- or
31-tone meantone keyboard is an ordinary object. Bohlen–Pierce divides the *tritave* (3:1) into 13, so the period is a
parameter too. `02-core-calculus.md` §1.5's index is what lets one type say all of that, and this section fixes what
each indexed domain means.

Define, for a period `P` (a frequency ratio, by default 2:1) and a division `n ≥ 1`:

```text
Cyclic(n)         % the cyclic group ℤ/nℤ — an element, not a set
Pc(n)  =  Cyclic(n)   in a pitch reading
Ic(n)              % an interval class: the vector type acting on Pc(n)
Row(n)             % a bijection Fin n → Pc(n)
Icv(n)             % the interval-class vector: ⌊n/2⌋ entries
Voicing(k)         % exactly k voices, in fixed order low to high
```

`pc12` of §1 is `Pc(12)` and `Row12` of §4 is `Row(12)`; the definitions there are unchanged and are the `n = 12`
instances of these. Every count that was a literal in §4 is now derived: a row's matrix is `n` rows and not twelve, and
the T/I group on `Pc(n)` has exactly `2n` elements and not 24. The interval-class vector's `⌊n/2⌋` entries generalize
§4's six for the reason §4 already gives — interval class `n − k` is interval class `k` heard the other way round — and
the entry at `n/2` for even `n` is its own inverse, which is why the tritone counts once at `n = 12`.

**`Cyclic(n)` serves pitch class and rhythmic cycle alike, and that is the point of naming it.** A pitch-class set in
ℤ/12 and a bell pattern in a 12-pulse cycle are the same mathematical object: transposition is rotation, inversion is
reflection, and the interval-class vector is the pattern's autocorrelation. So `102-set-class-and-prime-form.md`'s prime
form and a rhythm's canonical rotation are one algorithm, and a Carnatic tala, a West African bell pattern, and
`098-twentieth-century-rhythmic-techniques.md`'s additive meters are cycles of n pulses with an onset set. A language
that names the structure once serves all of them; one that names `pc12` twelve times serves none of them well. The pitch
reading and the rhythmic reading are separate *types* over the shared carrier, because a listener does not hear them as
the same thing and Musa never identifies what practice distinguishes — but they share their operations, their laws, and
their algorithm.

**The period is a parameter and equal division is a special case.** A scale is an interval pattern that sums to the
period; equal division is the pattern where every step is the same. Gamelan slendro and pelog are unequal and
instrument-specific, so `Scale` (§2) keeps its ordered pattern of offsets and gains the period it sums to; `Pc(n)`
describes a division only where one has been chosen, and a scale in an unequal tuning has no `n` at all. Naming the
division is therefore something an author does, not something that happens to them — the same commitment §4 and note 52
§2.4 make about forgetting a spelling.

**What the index buys is counted, not claimed.** `Pc(n)` gives the checker the carrier's size, so a finite carrier's
laws are decidable by enumeration inside the existing budget: orbit, stabilizer, and prime form become ordinary
functions of a finite action rather than one compiler builtin per question. Messiaen's modes of limited transposition
(`106-collections.md`) are exactly the collections with a nontrivial stabilizer under T, and a row's symmetries
(`110-row-properties.md`) are its stabilizer — two chapters that are this and nothing else. The seventeen `pc12_*` and
`row12_*` builtins §4 is implemented by exist because the modulus could not be said; prompt 164 removes them, and if it
does not, the argument in
[`../../notes/research/language-design-closure/51-the-terseness-audit.md`](../../notes/research/language-design-closure/51-the-terseness-audit.md)
was wrong.

**What the index does not buy.** `Bar(m)` — a bar whose contents sum to the meter `m` — is checkable exactly when the
durations are statically known, which is the common case in written notation and in every expansion an adapter produces.
A bar folded out of a list whose length the checker cannot see keeps the constructor and the runtime refusal it has
today. `02-core-calculus.md` §1.5 states which case each type is in; a governing document that implied the strong one
everywhere would be promising a length-indexed list, and that is a dependent type this language does not have.

## 6. Construction, assertion, and analysis

- A constructor returns a value only after enforcing its representation invariant (`row12`, `scale`, `voice`).
- An assertion is a decidable proposition over constructed finite values or music and either returns its input with
  evidence or emits a diagnostic. Counterpoint rules are style-indexed assertions, not universal laws: OMT
  `023-introduction-to-species-counterpoint.md`, `024-first-species-counterpoint.md`,
  `025-second-species-counterpoint.md`, `026-third-species-counterpoint.md`, `027-fourth-species-counterpoint.md`, and
  `028-fifth-species-counterpoint.md` describe a particular pedagogical practice.
- An analysis returns `AnalysisResult[A] = { method, assumptions, observations, alternatives, evidence }`. It may be
  ambiguous. Tonalization/modulation (`050`, `051`), Roman numerals, and orchestration readings do not become facts
  merely because an algorithm selected one.

This split prevents a theory algorithm from silently changing construction and prevents one analytic convention from
becoming type checking.

## 7. Source map and falsifiers

Every base type this document adds owes a row here. That is `02-core-calculus.md` §5.8's standing budget: a new base
type is admitted by a registry entry rather than a new induction, and the price of the cheap admission is that the type
says here what it means, where the meaning comes from, and what it refuses to identify. A row without a falsifying
example is a type that has not yet said what it is *for*, since a domain that rules nothing out could have been a `nat`.

| Concept | Musa definition | Source or local theorem | Falsifying example | Prompt |
| --- | --- | --- | --- | --- |
| written pitch/interval | `ℤ²` and its faithful action | OMT `005-half-steps-whole-steps-and-accidentals.md`, `016-intervals.md`; faithful-action lemma above | `C♯4 + d2` must not spell `E♭4` | 100 |
| spelled pitch class | octave quotient by `(7,12)` | Musa definition; well-defined quotient above | `C♯ = D♭` is false here | 100 |
| `pc12` | chromatic quotient `ℤ/12ℤ` | OMT `099-pitch-and-pitch-class.md`, `100-intervals-in-integer-notation.md` | `B♯ ≠ C` is false here | 105 |
| scale | ordered distinct cyclic spelled collection | OMT `013`, `014`, `105`, `106` exact filenames above | treating melodic minor as one immutable ascending/descending fact | 101 |
| key | tonic/signature/mode-family context | OMT `013`, `014`, `050`, `051` | `key c minor` cannot determine one registered pitch for degree 6 | 101 |
| degree/register | ordinal plus explicit periodic lift | Musa definition and round-trip lemma | `degree(scale c major, 1)` cannot have type `pitch` without register | 101 |
| frame | scale plus the absolute tonic pitch that registers it | Musa definition; the round-trip lemma is stated over exactly this pair | the same `scale c major` framed at `c4` and at `c5` are different frames, so degree 1 realizes to one pitch only once one is chosen | 101 |
| chord class | rooted spelled membership, no register | OMT `017-triads.md`, `018-seventh-chords.md`, `075-chord-symbols.md` | a Cmaj7 symbol does not choose C3 or C4 bass | 102 |
| triad | checked three-member tertian subtype | OMT `017-triads.md` | `{C,D,G}` is not a triad merely because it has three notes | 102 |
| roman numeral | ordinal, member count, and bass position, against no collection | OMT `020-roman-numerals.md`, `021-figured-bass-and-roman-numerals-with-figures.md` | `V` alone names no pitch class, and a triad has no third inversion | 107 |
| inversion/voicing | exact pitches plus bass/spacing/doubling choices | OMT `019-inversion.md`, `076-jazz-voicings.md`; forgetfulness lemma | drop-2 and close position cannot compare equal as voicings | 102 |
| pcset12 | finite subset of `pc12`, with normal order, prime form, and interval-class vector | OMT `101-pitch-class-sets-normal-order-and-transformations.md`, `103-interval-class-vectors.md` | a set is unordered and duplicate-free, so `[0,4,7]` and `[7,4,0,0]` are the same `pcset12` and neither is a `row12` | 105 |
| row12 | bijection `Fin 12 → pc12` | OMT `108-basics-of-twelve-tone-theory.md`; finite-closure lemma | a repeated pc rejects construction | 105 |
| row convention | explicit naming policy | OMT `109-naming-conventions-for-rows.md` | bare `P7` in a convention-free API is ambiguous | 105 |
| symmetric row | row with nontrivial stabilizer | OMT `110-row-properties.md` | requiring exactly 48 distinct forms rejects valid rows | 105 |
| `Cyclic(n)` / `Pc(n)` | ℤ/nℤ as an indexed type, with the pitch and pulse readings separate over one carrier | OMT `068-equal-divisions-of-the-octave.md`, `099-pitch-and-pitch-class.md`, `098-twentieth-century-rhythmic-techniques.md`; note 52 §1 | a 12-pulse bell pattern and a pcset in ℤ/12 must share `transposed` and `prime form`, and must still be two types | 142c–142d |
| `Row(n)` | bijection `Fin n → Pc(n)`, with `2n` T/I forms and an `n`-row matrix | OMT `108-basics-of-twelve-tone-theory.md`, `110-row-properties.md`; finite-closure lemma, read at `n` | a 24-EDO row cannot be typed `Row12`, and asserting 48 forms is false at every `n` | 142c–142d |
| `Icv(n)` | interval-class vector of `⌊n/2⌋` entries | OMT `103-interval-class-vectors.md`, generalized | six entries is the `n = 12` case, not the definition | 142c–142d |
| period and division | a scale is an interval pattern summing to its period; equal division is the special case | note 52 §1, after OMT `068-equal-divisions-of-the-octave.md`, `106-collections.md` | Bohlen–Pierce divides 3:1 and slendro divides nothing equally, so neither has an `n` by default | 142c–142d |
| harmonic transform | library operation over typed harmony | OMT `061`–`073`, especially `072-neo-riemannian-triadic-progressions.md` | calling every common-tone move an event track primitive | 106–113 |
| assertion | decidable, evidence-producing check | Musa definition; style evidence OMT `023`–`028` | species rules applied as universal well-formedness | 109, 112 |
| analysis result | named method, assumptions, alternatives, evidence | OMT `020`, `021`, `050`, `051` | one Roman-numeral reading mutates the chord facts | 115–116 |

Abbreviated OMT numbers in the table refer to the exact filenames already written in the corresponding definition row;
implementing prompts must cite the filename in user-facing reference documentation.
