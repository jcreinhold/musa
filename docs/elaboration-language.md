# The elaboration language — contextual music, total functions, and a typed kernel escape

**Status: design history. Non-governing, and no longer an input.** This document evaluates and replaces the earlier
proposal of the same name. Prompt 92 split its accepted decisions into the precise candidate specification at
[`docs/language/`](language/README.md), which is the implementation contract for prompts 93–145;
`docs/course-correction.md` and `docs/kernel/` continue to govern until prompt 146's conformance audit graduates it.
Where this essay and the candidate differ, **the candidate is right** — it was written to be implemented, this was
written to decide what to implement, and §18 below records what became of each decision.

Read this for *why*. Read [`docs/language/`](language/README.md) for what the language is, and
[`docs/language/handbook/`](language/handbook/README.md) for how to use it.

The proposal is grounded in the design conversation that produced the first draft, the current compiler, the kernel
specification, the interface's Origin view, and the *Open Music Theory* corpus at
`~/Code/papers/music-theory/open-music-theory/`. OMT chapter numbers below refer to that corpus.

## 1. Verdict

The direction is right and the first formalization is not.

Musa does need a typed, total elaboration language above the temporal kernel. General functions should replace the
present two-kind motif substitution mechanism; `motif` should survive as musician-facing syntax for one particular kind
of function. Users should be able to author kernel terms, both as standalone `.musa.kernel` files and through a typed
local escape. The ordinary notation language should remain the default and should not expose this machinery unless a
composer asks for it.

Seven claims in the first draft must not become design premises:

1. **`Timeline` is not the universal currency of elaboration.** The governing kernel explicitly has no canonical
   `Timeline[Timeline[A]] → Timeline[A]` operation (D12/X3). Music passed to a function needs a chosen temporal
   composition operation; it does not flatten “the way lists-of-lists” do.
2. **Open music is not a closed timeline.** A phrase containing `step 1` must still read the scale in force where it is
   used. Once reduced to `Timeline[ScoreFact]`, it is too late to re-elaborate it under another scale. The missing type
   is a contextual `music` value, distinct from a closed kernel term.
3. **Bare STLC is insufficient.** A simply typed lambda calculus terminates, but without finite data eliminators it
   cannot express “make `n` copies,” map a row, or fold a chord's members. Musa needs a total higher-order calculus with
   finite inductive data and structural folds—not general recursion, and not a first-order imitation of functions.
4. **A key is not a scale embedding into absolute pitches.** A key has no register, and minor keys admit natural,
   harmonic, and melodic-minor collections (OMT 014). `degree(c minor, 5) : pitch` is therefore underdetermined.
5. **Names and marks are not erased by kernel equality.** A marked kernel reference applies a payload map (T6), and
   differently provenanced `ScoreFact` values may be deliberately unequal. Names are non-sounding structure, but they
   also support resolution, stable realization choices, caching, diagnostics, and Origin view. Only an explicit
   provenance-erasing projection may forget them.
6. **A kernel escape cannot pass unknown payload text into `ScoreSnapshot`.** Kernel payloads are opaque to
   `musa-kernel`, but a `Timeline[ScoreFact]` consumer must parse every payload as `ScoreFact`. Unknown or malformed
   payloads are type/adapter errors, never warnings that flow downstream as imaginary score facts.
7. **A theory name is not grounds for a compiler primitive.** Neo-Riemannian operations, row matrices, applied chords,
   and common-chord searches are useful library operations. Their presence in a textbook neither makes them semantic
   primitives nor puts them in one compilation phase. The boundary is derivability and information ownership.

The corrected center is:

> **A Musa function computes typed values. A value of type `music` is an open, context-sensitive musical construction.
> Instantiating it at a score location contributes a well-scoped kernel fragment; closing the whole score produces a
> closed, well-formed `Term[ScoreFact]`, whose evaluation is the governing `Timeline[ScoreFact]`.**

That definition is small enough to state rigorously and rich enough for the motivating music.

## 2. Phenomena the design must make ordinary

These examples are requirements, not syntax commitments. Surface punctuation may change; the distinctions may not.

### 2.1 A motif whose notes actually depend on its root

The current `rise(root: pitch)` moves only the occurrence of `root`; its other notes are literals. Chromatic pitch
expressions make the dependency honest:

```musa
motif turn(root: pitch = c5) {
    root/8
    (root up M2)/8
    ((root up M2) down m2)/8
    root/8
}
```

This is interval arithmetic. It does not need a key. In the pitch algebra below, it is the action of the interval group
on written pitches.

A genuinely diatonic neighbour is different and must say which scale supplies the steps:

```musa
motif upper_neighbor(root: pitch) {
    root/8
    (root step 1)/8
    root/4
}

in scale d dorian {
    use upper_neighbor(f4)
}
```

The surface reads as music; the typed core distinguishes `up M2` from `step 1`.

### 2.2 The same open phrase under a different scale

```musa
fn phrase() -> music {
    c5/8
    (c5 step 1)/8
    (c5 step 2)/4
}

let subject: music = phrase();

in scale c major  { use subject }
in scale c dorian { use subject }
```

`subject` must remain open to its scale context after the `let`. If `let` captured an already elaborated timeline, both
uses would be identical and `in scale` would be a lie. This example decides the `music`/kernel-term distinction.

### 2.3 A canon as ordinary higher-order composition

```musa
fn canon(subject: music, answer: music -> music, gap: duration) -> music {
    overlay {
        subject
        shift(gap, answer(subject))
    }
}

use canon(theme(), transpose(P5), 1/2)
```

This is the named need that invalidates the first-order fence. Higher-order values do not threaten termination;
unrestricted recursion does. `transpose(P5)` is a total function value, not user-defined syntax.

Higher-order access need not expose the representation of `music`. A narrow traversal supports user-defined pitch work
while keeping annotations and provenance under compiler control:

```musa
fn harmonize(subject: music, answer_pitch: pitch -> pitch) -> music {
    overlay {
        subject
        map_note_pitches(answer_pitch, subject)
    }
}
```

`map_note_pitches` visits written pitches in note and sounded-chord events, preserves their times, and deliberately does
not rewrite key signatures or chord-symbol annotations. A diatonic `answer_pitch` handles `locate`'s `option` explicitly
or uses a checked standard-library helper. This is enough to make the traversal total and its ownership unambiguous.

### 2.4 Chord identity separated from voicing

```musa
let harmony = chord(c, major7);                 % a rooted spelled chord class
let voicing = close_position(harmony, bass: c4); % exact pitches and inversion
play(voicing, 1/2)
```

An optional musician-facing form such as `stack c4 maj7/2` may desugar to those operations with the absolute root `c4`
fixing the register. A `chord` whose root is only pitch class cannot directly become sounded notes: register, doubling,
spacing, and bass are independent choices (OMT 017–019, 075–076).

### 2.5 A local, typed kernel escape

```musa
fn delayed_double(subject: music) -> music {
    kernel Timeline[ScoreFact] {
        let s = ${subject} in
        overlay {
            s;
            shift by 1/2 s;
        }
    }
}
```

The text between antiquotations is the kernel interchange term language. `${subject}` is a typed host-language hole, not
a free kernel name. The completed term must be closed and well formed before it can enter a score.

## 3. The semantic layers

There are four semantic stages, not three source strata sharing one value type:

```text
musician-facing forms
motif, use, notes, blocks, repeat, named transforms
                    │ desugar and type-check
                    ▼
total elaboration language
functions, finite data, base musical values, contextual `music`
                    │ instantiate at (environment, score position)
                    ▼
closed kernel term
Term[ScoreFact]: sharing + exact temporal constructors
                    │ governing kernel evaluation
                    ▼
finite temporal value
Timeline[ScoreFact] = (extent, finite multiset of occurrences)
```

The first line and second line share one surface grammar. `motif`, `use`, and the block transforms are derived forms,
not a less powerful language embedded beside `fn`. The third and fourth lines are the already-governing kernel syntax
and denotation. No lambda, closure, scale, chord, or surface context crosses the kernel boundary.

Declaration templates (§12) run while traversing the first arrow: their arguments use the same total evaluator, but
their result is a finite bundle of ordinary declarations to elaborate. They add a static staging judgment, not a fifth
musical representation and not another temporal semantics.

The distinctions are load-bearing:

| Type of thing | May still read context? | Has source structure? | Kernel meaning? |
| --- | --- | --- | --- |
| elaboration value (`pitch`, `scale`, function, …) | no, unless it is `music` | yes | none by itself |
| `music` | yes | yes | only after instantiation |
| closed `Term[ScoreFact]` | no | kernel constructors, sharing, and marks | evaluates by kernel T1–T6 |
| `Timeline[ScoreFact]` | no | construction history is gone | the kernel denotation |

Calling all four `timeline` would make contextual rebinding, sharing, and equality impossible to state correctly.

## 4. The total elaboration core

### 4.1 Types

The core is a simply typed, call-by-value lambda calculus extended with products, options, finite lists, natural
numbers, and their structural eliminators:

```text
τ ::= unit | bool | nat | int | rational
    | duration | positive_ratio
    | pitch | pitchclass | pc12 | interval
    | key | scale | degree | scale_pitch
    | chord_class | triad | voicing | row12
    | music
    | option τ | list τ | τ × τ | τ → τ
```

Not every domain type must land in the first implementation. This is the semantic taxonomy that prevents one vague
`chord` or `number` type from absorbing independent meanings. In particular:

- `duration` is exact nonnegative musical time; `positive_ratio` is the domain of time scaling. Units/refinements make
  negative duration and zero scale unrepresentable after checking.
- `pitch` is written pitch with register; `pitchclass` is spelled but has no register; `pc12` is an unspelled member of
  `ℤ/12ℤ`; `interval` is directed and spelled.
- `key` is a tonal/key-signature declaration value; `scale` is an ordered periodic pitch collection. They are related,
  not identified.
- `chord_class` is symbolic/rooted pitch-class content; `voicing` is an ordered collection of absolute pitches.
- `music` is opaque. There is no pattern match exposing occurrences, source nodes, or the representation of a kernel
  term. Purpose-built transforms are the interface.

Function types are real types. Functions may be arguments and results. Multi-argument source functions desugar to
curried functions or an equivalent product-domain function; this is an implementation choice not observable by Musa
code.

Public function parameters and results should be annotated. Local result inference is useful; a broad implicit-generic
language is not required. The core remains monomorphic after elaboration: `option`, `list`, and their eliminators are
typed families instantiated at concrete types by the checker. The prelude may present those families as rank-1 schemes
and monomorphize each use. User-defined polymorphism should wait for a real library that needs it; adding it would
require an explicit surface inference/elaboration design, not an unmentioned extension of STLC.

### 4.2 Terms and finite iteration

Core terms include variables, `let`, functions, application, data construction, case analysis, and structural folds.
There is deliberately no `fix`, unrestricted recursion, mutation, I/O, exception effect, thread, or syntax reflection.

`repeat n { m }` is a musician-facing spelling of a natural-number fold whose result type is `music`. Row operations are
list maps/folds. A sequence generator can therefore accept a computed finite count without smuggling recursion into the
language.

This core is strongly normalizing: every well-typed closed term evaluates in finitely many steps. The theorem follows
from the ordinary normalization argument for STLC plus strictly positive finite data and structural eliminators. It does
**not** imply that every program is cheap: `repeat 10^12` terminates in the mathematical sense and is still an
unacceptable compilation. Implementations need deterministic size/work budgets and a resource diagnostic; the budget is
operational policy, not a fake termination proof.

### 4.3 Static and dynamic judgments

The core judgments are conventional:

```text
Γ ⊢ e : τ
e ⇓ v
```

The required metatheory is:

1. **Preservation.** If `Γ ⊢ e : τ` and `e → e'`, then `Γ ⊢ e' : τ`.
2. **Progress.** A closed well-typed term is a value or takes a step.
3. **Strong normalization.** A closed well-typed term has no infinite reduction path.
4. **Determinism.** Call-by-value evaluation chooses one result.

Musical operations with a genuinely partial domain do not become hidden evaluator failure. They use one of three honest
encodings:

- a refined input type (`positive_ratio`, `triad` rather than arbitrary `chord_class`);
- an `option` result (`locate : scale -> pitch -> option scale_pitch`);
- a checked surface constructor that emits a source diagnostic before a core value is built.

Compiler recovery may accumulate several diagnostics and substitute error values internally. That implementation
facility is not an effect available to Musa programs and must not be confused with the core's normalization theorem.

### 4.4 `fn`, `let`, `motif`, and `use`

The general forms are:

```musa
let fifth: interval = P5;

fn transpose_answer(subject: music, by: interval) -> music {
    transpose(by, subject)
}

fn third(root: pitch) -> pitch { root up M3 }
```

`motif` is retained but narrowed to what its name means:

```musa
motif turn(root: pitch = c5) { ... }
```

desugars to a named function returning `music`, plus a `Motif` role used by extraction, lints, Origin view, and editing.
A harmonizer or row operation is a `fn`, not a motif. A named non-parameterized passage remains `fragment` sugar for a
`music` binding.

`use e` is the musician-facing statement form that sequences a value `e : music` into the enclosing voice. Ordinary
expression position uses ordinary application. Existing `use name(args)` syntax remains source compatible.

Calling a named music-producing function records a provenance step. That step is metadata over musical facts, not part
of beta reduction and not a new kernel operation. Pitch-only helper calls need not clutter a note's Origin path; their
source expression is already the definition span. The exact provenance policy is syntax-directed and centralized, not an
effect users can intercept.

## 5. The missing concept: contextual `music`

### 5.1 Characterization

A `music` value represents a finite relative construction that has not yet been placed in a score. It is characterized
by one operation hidden inside the compiler:

```text
instantiate : Music × ElabEnv × Beat → Checked KernelFragment

KernelFragment = {
    term:     Term[ScoreFact],
    bindings: private acyclic binding environment,
    extent: Duration
}

close : KernelFragment → closed Term[ScoreFact]
```

`Beat` is the absolute notated position of the fragment's start. `ElabEnv` contains only information a reusable fragment
is allowed to read, including:

- the score scope in which its facts will live;
- the optional scale used by degree operations;
- prevailing key/meter tracks needed for placement-sensitive validation;
- the provenance/realization context owned by the compiler.

It does **not** expose mutable user state. Transposition and stretching are operations on `music`, not magic environment
fields. Key and meter declarations are score structure, discussed below.

`music` is context-reading but context-neutral. Statements whose meaning is “from here onward”—key, meter, tempo, and
clef changes—remain structural piece/voice items and are rejected inside reusable material, as they are today. The
enclosing structural pass places those facts and supplies their tracks to instantiation. A `music` value may contain
events, region-local annotations, and lexically scoped `in scale`, because none changes a sibling or its caller's
subsequent context. This restriction is what makes `sequence` and `overlay` below compositional without a hidden state
effect or order-dependent merging of parallel context changes.

This is a semantic characterization, not a public Rust representation. Internally `Music` may be an HIR closure, a typed
expression plus environment, or another structure chosen for incremental compilation. Callers may not know. The private
binding environment is what lets two calls share one elaborated body, as the current piece-level `Share` does.
Composition unions compatible binding environments; `close` emits the dominating kernel `let`s once. Requiring every
small fragment to carry its own closed copy would preserve meaning and accidentally destroy sharing.

### 5.2 Composition laws

Instantiation must respect the kernel algebra while accounting for placement. Suppressing `Checked` and the compatible
binding environments in the notation below, if `instantiate(m, ρ, p) = (t, d)` and `instantiate(n, ρ, p + d) = (u, e)`,
then:

```text
instantiate(sequence(m, n), ρ, p) = (seq t u, d + e)
```

If `instantiate(n, ρ, p) = (u, e)`, then:

```text
instantiate(overlay(m, n), ρ, p) = (over t u, max(d, e))
```

These are chosen higher-level compositions, not a generic `join` on nested timelines. They lower to D2 and D3 and
inherit the kernel's laws, including synchronized interchange and its hypotheses.

Absolute placement is an argument because some surface obligations genuinely depend on it: a tuplet may not cross a
barline, a named bar reads the meter at its use, and a context fact prevails at a particular time. The first draft's
`Env → Term` omitted this information and could not state those checks soundly.

### 5.3 Context rebinding

`in scale s { m }` is the Reader-style local operation:

```text
instantiate(in_scale(s, m), ρ, p)
    = instantiate(m, ρ[scale := s], p)
```

It obeys the familiar local laws under `≈music` (the provenance-erasing musical equality of §10.1):

```text
in_scale(s, in_scale(t, m)) = in_scale(t, m)
in_scale(s, sequence(m, n)) = sequence(in_scale(s, m), in_scale(s, n))
in_scale(s, overlay(m, n)) = overlay(in_scale(s, m), in_scale(s, n))
```

and rebinding independent fields commutes. A `music` binding remains contextual, so `let subject = phrase()` does not
freeze the scale read by `phrase`. Full score provenance may additionally record a `ScaleContext` step, so these laws
are not claims that differently written sources have identical Origin payloads.

An explicit parameter is always available instead:

```musa
fn phrase(s: scale) -> music { ... }
use phrase(scale g major)
```

Lexical rebinding is mercy for a musical region; explicit parameters are useful at library boundaries. They agree under
`≈music`; provenance may still distinguish the two spellings.

### 5.4 What may not be rebound this way

There is no `with meter` or `in meter` in this proposal. Meter determines actual barlines, measure coordinates, beaming
context, and validity of changes. Rebinding a Reader field without emitting and reconciling those score facts would
change checks without changing the score. Emitting facts would make it a structural score operation rather than local
context. The current `meter` statement remains the honest operation. A future `rebar` transform would need its own
specified effect on meter facts and notation; it is not generic context rebinding.

Likewise, `in scale g major` is **not** declared modulation or tonicization. It chooses coordinates for generative pitch
operations and emits no key signature. A `key g major` statement emits the score fact and changes the default scale for
subsequent material at that score location. Tonicization and harmonic analysis are annotations/analyses, not inferred
from a lexical compiler setting. This avoids turning OMT 050–051's interpretive continuum into a false binary language
semantics.

## 6. Pitch, scale, key, and register

### 6.1 Written pitch and interval

Let `natural(d)` be the chromatic height of the unaltered staff position `d`. A written pitch is most cleanly
represented by its diatonic and chromatic heights:

```text
Pitch = (diatonic_height : ℤ, chromatic_height : ℤ)
Interval = (diatonic_displacement : ℤ, chromatic_displacement : ℤ)

alteration(d, c) = c - natural(d)
(d, c) + (δd, δc) = (d + δd, c + δc)
```

Intervals form the abelian group `ℤ × ℤ` under componentwise addition and act on pitches by the displayed formula. This
makes transposition spelling preserving by definition: letters move by `δd`, sounding height by `δc`, and the resulting
accidental is derived from their difference (OMT 005, 016, 099–100).

The mathematical value permits arbitrary integer alteration, making the action total and its group laws true. The
ordinary notation surface may accept only natural through double accidentals, and a backend may reject an unengraveable
result with a diagnostic. A representability limit is not part of the pitch algebra.

`pitchclass` is the quotient of this representation by the subgroup generated by octave displacement `(7, 12)`. It
therefore retains spelling: it is not a MIDI number and does not identify D-sharp with E-flat merely because a tuning
later assigns the same frequency.

Serial and pitch-class set theory use a different quotient, `pc12 = ℤ/12ℤ`, which deliberately forgets spelling as well
as register. The projection `forget_spelling : pitchclass -> pc12` is total; its inverse requires an explicit spelling
policy and is not a function Musa may guess. A `row12` is a checked 12-element permutation of `pc12`; an arbitrary
pitch-class sequence remains `list pc12`. This keeps the serial group action exact without weakening written pitch
identity elsewhere.

### 6.2 Scale and key are distinct

A `scale` is a tonic pitch class plus a finite nonempty cycle of spelled interval offsets and a period. It determines a
periodic map into spelled pitch classes. Major, natural minor, harmonic minor, ascending melodic minor, descending
melodic minor, modes, pentatonic collections, and symmetrical collections are different values (OMT 013–015, 068,
105–107).

A `key` is a tonic, mode/tonal role, and key-signature spelling used as a score fact. `key a minor` has the
natural-minor key signature even though music in A minor may use G, G-sharp, F, and F-sharp (OMT 014). The function
`signature_scale : key -> scale` is well defined and may supply the default scale after a key statement. It does not
claim to encode every pitch legitimate in the key.

This resolves the old minor-mode open question by removing its false premise: there is one minor key signature and
several explicitly named minor scales.

### 6.3 Degree and register

A `degree` is a scale-independent integer coordinate plus an optional chromatic alteration; it is not already a pitch
and does not secretly carry an ambient scale. Interpreting it requires an explicit `scale`. The useful operations are:

```text
degree        : int -> degree
alter_degree  : int -> degree -> degree
pitchclass    : scale -> degree -> pitchclass
locate        : scale -> pitch -> option scale_pitch
step          : int -> scale_pitch -> scale_pitch
written_pitch : scale_pitch -> pitch
frame         : scale -> tonic:pitch -> option pitch_frame
at_degree     : pitch_frame -> degree -> pitch
```

`frame` supplies the missing register by choosing an absolute tonic whose pitch class agrees with the scale. It lifts
the periodic pitch-class map to an embedding of integer degrees into absolute written pitches. `locate`/`step` is the
convenient route when a motif already has a root pitch. The surface expression `root step 1` elaborates to “locate
`root` in the ambient scale, move one coordinate, return its written pitch,” with a precise diagnostic if no scale is in
force or the root is not a member.

Chromatic motion remains `pitch up interval`. Diatonic and chromatic motion intentionally do not share an operator and
need not commute.

## 7. Harmony without a universal chord blob

The current law remains: harmony annotations are recorded, not treated as the ontology of the notes beneath them. A
generative harmony library is compatible with that law only if it keeps four things separate:

| Type | Meaning |
| --- | --- |
| `chord_symbol` | text/structured annotation written over a score |
| `chord_class` | rooted, spelled pitch-class content, with optional bass member |
| `triad` | refinement of `chord_class` to a major/minor triad where PLR is total |
| `voicing` | finite ordered absolute pitches, including register, spacing, doubling, and bass |

Chord spelling builds a `chord_class`; a voicing policy turns it into `voicing`; `play` turns a voicing and duration
into `music`. The compiler must not silently choose register or doubling in the first step.

This makes theoretical domains precise:

- `P`, `L`, and `R` have type `triad -> triad` and are involutions; `S`, `N`, and `H` may share that domain (OMT 072).
  They are not partial `chord -> chord` functions over suspended, quartal, or altered chords.
- `invert_chord` changes the designated bass member of a chord class. A voicing operation realizes that inversion; it
  does not confuse root with the lowest pitch (OMT 019).
- `stack(c4, major7)` may be standard-library sugar for a specified close-position voicing above the absolute root `c4`.
  `stack(c, major7)` is incomplete and must not pretend otherwise.
- Applied chords, mixture, common-chord searches, and rule-of-the-octave harmonizations consume explicit key/scale and
  chord values. They are library algorithms, not facts every note must satisfy.

## 8. Transforms and the library boundary

### 8.1 Compiler-owned primitives

A compiler primitive is justified only when it needs information intentionally hidden by `music`, needs source-aware
provenance, or corresponds directly to a kernel operation unavailable to user code. The small expected set is:

```text
sequence, overlay, note/rest/chord-event construction
transpose, invert, retrograde
map_note_pitches
stretch, shift, restrict/observe where a musical caller exists
in_scale, instantiate/kernel quote
finite fold/repeat
```

Even here, laws need their real hypotheses. `transpose` is a group action on pitch-bearing facts only if the mapping
policy for every pitch-bearing `ScoreFact` is specified. `retrograde` is an anti-homomorphism for sequence. `stretch`
acts by positive rationals. `restrict` is observation and must preserve whole-span provenance. No law may be stated over
a broader type than the operation actually handles.

Opacity is not a fixed catalog of canned transforms. A controlled eliminator such as `map_note_pitches` belongs here
because it must traverse hidden music, preserve time and Origin paths, and specify exactly which facts it visits. New
eliminators should expose the smallest musically coherent view—pitches rather than raw `ScoreFact`, for example—and
otherwise remain ordinary higher-order functions.

### 8.2 Source standard libraries

Operations expressible through functions, finite data, and the primitive interface belong in `.musa` libraries. A useful
organization follows mathematical domain, without turning that organization into compiler ontology:

- `pitch` / `scale`: interval actions, degree operations, modes and collections;
- `harmony`: chord spelling, inversions, voicing policies, applied chords;
- `transformational`: PLR/SNH over the `triad` refinement;
- `serial`: rows and T/I/R/RI;
- `schemas`: diatonic sequences, rule of the octave, and other named constructions.

OMT is a grounding and regression corpus: it supplies terminology, examples, and expected laws. It is not a closed world
and not an admission rule. An unnamed but orthogonal and useful operation may belong; a named textbook operation still
stays derived when the core can express it.

The serial algebra must be stated accurately. Transpositions and inversions of `pc12` generate a dihedral action of
order 24 (under the convention where `D12` has 24 elements). Row reversal commutes with that pitch-class action,
yielding up to 48 P/I/R/RI forms for a generic `row12`. “The row operations generate D12” and “there are 48 forms”
cannot both describe the same group without this extra reversal factor.

### 8.3 Analysis and verification

Row matrices, Roman-numeral analysis, common-chord searches, cadence classification, and theory checks observe closed
musical facts. They do not construct `music` merely because they use the same domain values. They belong in an analysis
service/library over `Timeline[ScoreFact]` or the score projection, returning data and diagnostics.

This keeps construction, annotation, and verification independent. A composer may generate a chord without annotating
it, annotate a chord without deriving the notes from it, and ask an analysis to compare the two.

The language should distinguish three strengths of guarantee:

- **Type/constructor invariant, before a core value exists:** positive stretch, exact duration, `row12` uniqueness, and
  PLR's `triad` domain.
- **Explicit musical assertion, after each contextual instantiation:** a named bar fills its meter, selected notes
  realize this chord symbol, or pitches lie in this scale.
- **Interpretive analysis, on request over closed facts:** inferred key, Roman numeral, tonicization/modulation,
  cadence, and counterpoint style.

An assertion is a checked identity transform on `music`: if it succeeds, it returns the same musical facts and records
the assertion in provenance; if it fails, instantiation returns a diagnostic. Its predicate is a compiler-owned or
typed-library interface over a coherent musical view, not arbitrary access to `ScoreFact`. This permits a composer to
ask for strict diatonic membership or chord/notes agreement exactly where intended.

Musa must not globally reject a note because it is outside the prevailing key, infer that every simultaneity realizes
the nearest chord label, or call a key change a modulation by syntax alone. Chromatic tones, incomplete voicings,
non-chord tones, mixture, and tonicization are ordinary music. The compiler proves declared constructions and explicit
claims; analysis proposes interpretations and reports evidence without rewriting the score.

## 9. Kernel files and the local splice

### 9.1 The subset claim

The exact claim has a syntactic part and a typed semantic part:

> Every syntactically valid `.musa.kernel` file is accepted unchanged as a Musa source document. For every payload type
> with a registered adapter, the unified route produces the same checked term and kernel denotation as direct parsing
> with that adapter.

The current interchange grammar has a required `% musa-kernel-1` header and exactly one `composition` declaration. The
unified source-document grammar therefore has a `kernel-file` top-level alternative owned by the kernel grammar; it must
not reimplement a drifting copy. The extension chooses tooling defaults, not meaning. A kernel document exposes its one
composition as the document's typed result. It is not silently rewritten into a surface `library` containing plural
compositions the kernel grammar does not have. If Musa does not own the named payload type, it may preserve and edit the
syntactically valid document but must refuse evaluation rather than invent a payload meaning.

This inclusion does **not** mean every kernel payload type is score material. A standalone `Timeline[A]` is valid when
the kernel consumer owns `A`. Only a `Term[ScoreFact]` whose facts pass the context-neutral material check can be
inserted where surface `music` is expected; a whole-score kernel document may also contain structural context facts.

### 9.2 Quote and antiquotation

A local splice is a host-language quotation:

```musa
kernel Timeline[ScoreFact] { <kernel composition-expression> }
```

The quoted language is the `composition-expression` grammar of `docs/kernel/01-grammar.md`, extended only with holes.
`${e}` is antiquotation and requires `e : music`. At quote instantiation, each hole is instantiated once under the host
environment at a deterministic *quotation locus*: the quotation's score position, adjusted structurally by enclosing
`sequence`, `overlay`, `shift`, and positive time `scale`; a `let` value starts at its enclosing locus. Prefix extents
are exact, so this calculation does not guess. The compiler binds the resulting term to a fresh kernel name and closes
the quotation around those bindings. Fresh names are compiler-generated, so capture is impossible. The resulting
fragment is closed relative to the compiler's private binding environment. The completed quotation must pass
`Term::check` after `close`, and every raw payload must pass the `ScoreFact` adapter and the context-neutral material
check before the quotation exists as `music`.

A raw reference or temporal operator may subsequently reuse, move, stretch, or observe the already-instantiated facts.
It does not reinstantiate the hole or reread surface context at each resulting occurrence. This is the precise point
where the author chose the assembly-level semantics, and it explains the reduced surface guarantees in §9.3.

A standalone `.musa.kernel` file has no antiquotation: it is the pure interchange sublanguage. This is the same
quote/antiquote distinction used by typed assembly embeddings, not an accidental second reference resolver.

### 9.3 Guarantees

Inside a raw splice Musa provides:

- kernel well-formedness, exact time, finiteness, payload uniformity, and closed references;
- total parsing of every payload as the declared payload type;
- for a local `music` quote, rejection of key/meter/tempo/clef facts that would mutate the caller's context tracks;
- exact extent, which lets surrounding bar arithmetic continue;
- a `KernelSplice` provenance step on every score fact leaving the splice.

It does not infer or enforce surface-only relationships inside the raw term: bar assertions, degree membership,
music-theory lints, motif editing structure, or source-level transform laws. An antiquoted value retains its typed facts
and its own derivation provenance, but a raw `scale`, `shift`, or `restrict` around it may invalidate
placement-sensitive surface checks it passed before the raw operation. The entire splice therefore carries a
`KernelSplice` step and makes no stronger surface guarantee than the completed raw term. Exact outer extent and payload
typing still hold.

Malformed `ScoreFact` text is an error at the boundary. It cannot become an opaque event that notation and performance
quietly drop. The escape is from the surface theory, not from type safety or the backend contract.

## 10. Provenance, names, equality, and caching

### 10.1 Payload equality and its contextual lifting

The kernel keeps its existing equality:

```text
T ≡kernel U  iff their canonical Timeline[ScoreFact] forms agree
```

Because provenance is carried in payloads and marked references may map payloads, two differently marked instantiations
need not be `≡kernel`. For laws about sounding/notated content independent of source history, define an explicit payload
projection:

```text
erase_origin : ScoreFact -> MusicalFact
T ≈facts U    iff map(erase_origin, eval(T)) = map(erase_origin, eval(U))
```

This equality lifts observationally to open values. Let `observe(m, ρ, p)` instantiate `m`, close and evaluate its
fragment, erase Origin data, and return either the resulting `Timeline[MusicalFact]` or a stable domain-error kind
without presentation spans. Then:

```text
m ≈music n  iff  for every compatible ρ and p, observe(m, ρ, p) = observe(n, ρ, p)
```

Thus `≈music` includes equal extent, musical facts, and domain behavior in every context while intentionally ignoring
source-history presentation. Derived-form correctness is normally stated with `≈music` plus a separate provenance
theorem. For example, `motif` and its `fn -> music` desugaring produce the same musical facts, while the motif spelling
intentionally adds a role the Origin view can show.

### 10.2 What a name does

A name has no pitch or temporal extent, but it is not “only a mark.” It may supply:

- lexical binding and type identity;
- a stable path for realization decisions;
- a provenance label and editing unit;
- a cache identity tied to a definition revision;
- a reference-index entry for lints and navigation.

None of those changes what sounds after the appropriate erasure. All remain observable to compiler/editor clients and
must not be justified by pretending kernel normalization forgets them automatically.

### 10.3 Cache soundness

The cache key for instantiating `music` must include everything its result or diagnostics can observe:

```text
definition revision / closure identity
typed arguments
relevant environment fields
absolute placement when obligations are placement-sensitive
realization decisions
```

An implementation may dependency-track which fields were actually read and omit the rest. “Function, arguments, and
environment” is not yet an interchangeability theorem until `music` and its placement-dependent checks are defined as
above.

## 11. Surface desugarings

Every musician-facing construct has one checked elaboration into the core:

| Surface form | Core account |
| --- | --- |
| `motif name(params) { body }` | named `params -> music` function + `Motif` role/provenance |
| `use e` | require `e : music`, instantiate and sequence at this voice position |
| `fragment name { body }` | named `music` value + `Fragment` role |
| `transpose up i { body }` | `transpose(i, music { body })` |
| `retrograde`, `stretch`, `invert` blocks | application of the corresponding `music -> music` function |
| `repeat n { body }` | finite natural-number fold using kernel sharing and iteration provenance |
| `[c4 e4 g4]/2` | explicit `voicing` played for a duration |
| a named bar | `music` binding plus an extent assertion; it still sounds where declared |
| `use ... with { note n = p }` | occurrence specialization after instantiation, not context rebinding |
| kernel quotation | checked construction of contextual `music` from a closed `Term[ScoreFact]` template |

`with` is already the language's occurrence-specialization word. The context operation should therefore be spelled
`in scale ...` (or another musician-reviewed phrase), not overloaded as `with key`.

## 12. Pieces, voices, structural templates, and actual module functors

An OCaml functor is a function from modules to modules. A lexical scale override is a Reader `local`; a transposition is
a function on `music`; neither should be sold as an OCaml functor.

`piece`, `part`, and `voice` should not become ordinary values in the value calculus merely to reuse that analogy:

- a piece combines score, performance, and studio declarations that must remain independent;
- a voice establishes identity, absolute placement, and context-change authority;
- turning either into `music` would leak those structural policies into every transform.

The request for a family of pieces or voices parameterized by key, scale, subject, or transform is nevertheless real. It
needs a small *declaration-level template* judgment beside, not inside, the value calculus:

```text
Γ ⊢ e : τ
Γ ⊢ D : declaration κ                 κ ∈ {library, piece, voice}
Γ ⊢ template (x₁ : τ₁, …, xₙ : τₙ) D : (τ₁ × … × τₙ) ⇒ declaration κ
```

A template is not a first-class value and `declaration κ` is not a Musa value type. Instantiation evaluates its
arguments in the total core, substitutes the resulting values into the declaration template, assigns stable fresh
identities derived from the instance site, and produces ordinary declarations before context tracks and `music` are
instantiated. The template dependency graph is finite and acyclic. This gives the structural stage deterministic,
terminating semantics without permitting a function to inspect or rewrite arbitrary source syntax.

For example, musician-facing syntax could support this shape:

```musa
piece study(k: key, s: scale, subject: music) "Study" {
    key k;
    score {
        part piano {
            voice right {
                in scale s { use subject }
            }
        }
    }
}

make study(g major, g mixolydian, theme());
```

The punctuation is open, but the staging is not. A parameterized `voice` works the same way and may contain the
key/meter/tempo/clef declarations forbidden inside `music`, because each template instance has one structural identity
and one placement. Parameters may themselves be functions or `music`, so a template can accept a user-defined
`music -> music` transformation without making voices first class.

If a parameter must be a bundle of types and declarations from another library, that is the point for a small ML-like
module signature and a true module-to-module functor. Module matching and instantiation happen at this same structural
stage. They do not add `piece -> piece`, `voice -> voice`, or `module` to the value types, and they do not justify a
public Rust `Functor` trait. Parameterized declarations cover the stated key/voice/piece use case first; module functors
extend the parameter domain only when a multi-declaration bundle is actually needed.

## 13. Rust and crate boundaries

Two materially different implementation boundaries were considered.

### Alternative A — a public `musa-elaboration` crate

This would expose `Type`, `Value`, `Closure`, `Music`, environments, and evaluation errors. Today it has one caller,
`musa-compiler`; its types are precisely the volatile choices this design is still settling. A new crate would turn pass
boundaries into public architecture and force callers to know how elaboration is staged. Reject it.

### Alternative B — one deep private compiler subsystem

Keep the public facades unchanged:

```rust
musa_language::parse(&str) -> ParsedDocument
musa_compiler::compile(&SourceDocument, &CompileOptions) -> Compilation
musa_kernel::{Term, Timeline, ...}
```

Inside `musa-compiler`, a private elaboration subsystem owns type checking, closures, contextual `Music`, primitive
registration, provenance instantiation, and kernel quotation. Its narrow internal operation is conceptually:

```text
elaborate(typed source, compile parameters) -> checked closed Term[ScoreFact]
```

The exact Rust structs stay private. `musa-language` owns lossless surface syntax; it does not assign musical types.
`musa-kernel` remains musically opaque and gains no lambda/application/map form. The kernel-file grammar remains owned
by `musa-kernel`; the source-document dispatcher and tree-sitter integration recognize it without inventing another
semantics.

Choose Alternative B. It hides the most volatile decisions, adds no pass-through facade, preserves the one-way crate
graph, and leaves `compile` as the operation ordinary callers need.

Do not create a `musa-theory` crate merely because the vocabulary is large. Begin with source libraries and private
compiler helpers. Extract a crate only when an analysis or another independent consumer needs a stable typed API.

## 14. Acceptance rules

Every future construct or library addition must answer these questions in order:

1. **Meaning.** Does it compute a base value, contextual `music`, a kernel term, a structural declaration/template, or
   an analysis result? One construct may not straddle these categories silently.
2. **Derivability.** Can it be written as an ordinary total function using existing operations? If yes, it is a library
   function, not a compiler primitive or keyword.
3. **Context honesty.** Which environment fields and score position facts may it read? Does a lexical override change
   actual score facts or only generative coordinates? The name and documentation must say which.
4. **Domain honesty.** Is the operation total on its declared type? PLR takes `triad`, not arbitrary `chord_class`;
   degree-to-pitch requires a register anchor.
5. **Termination and size.** Is computation structural over finite data? What deterministic resource budget stops a
   finite but explosive result?
6. **Kernel soundness.** Does closing a successful instantiation produce a closed, well-formed term using meanings
   already defined by the kernel?
7. **Provenance.** What Origin step and editing unit does it create? What equality is its law stated under?
8. **Guarantees.** Which surface checks remain, which are intentionally bypassed, and can the boundary still report
   exact extent?
9. **Surface value.** Is musician-facing syntax clearer than the ordinary function call? If not, do not add syntax.
10. **Evidence.** Which materially different examples and counterexamples test it? A theory citation informs behavior
    but never substitutes for these answers.

User-defined syntax macros are not part of this proposal. Hygienic token-tree or syntax-object macros do not
*necessarily* make parsing impossible, so that was not a sound rejection argument in the first draft. They are still the
wrong first tool: all named motivating transformations are value-level computations; syntax reflection would add a
second staging/provenance system before one is needed. If a future construct truly cannot be expressed as a function,
macros require a separate typed quotation, hygiene, phase, formatting, and Origin design.

## 15. Laws and verification obligations

The design is accepted only if the following become executable checks or formal arguments at the prompt that introduces
them:

- **Type soundness and normalization** for the elaboration core.
- **Context laws** for `in scale`, including shadowing and distribution over sequence/overlay.
- **Open-binding law:** binding a `music` value does not capture the scale at the binding site.
- **Context-neutrality law:** reusable `music` cannot emit a context-track change; declaration templates retain the
  structural authority needed to parameterize such changes.
- **Kernel soundness:** closing every successful instantiation yields a closed, `Term::check`-passing `Term[ScoreFact]`.
- **Desugaring laws:** each surface form agrees with its core expansion under `≈music`.
- **Provenance law:** desugaring/instantiation adds the specified Origin path without changing temporal support except
  where the transform itself does.
- **Pitch-action laws:** interval identity/composition over the unbounded written-pitch domain; explicit negative tests
  for surface/backend representability.
- **Scale laws:** stepping composes additively within one located scale; chromatic transposition and scale stepping have
  tested non-commutation examples.
- **Transform laws:** identity/composition/homomorphism claims only over the facts and domains actually transformed.
- **Assertion law:** a successful explicit assertion preserves facts and extent while adding its Origin step; failure
  produces the declared stable domain error instead of changing or guessing the music.
- **Kernel-subset law:** every committed `.musa.kernel` fixture parses unchanged through the unified source entry and
  has the same term/denotation as direct kernel parsing.
- **Splice law:** quotation followed by antiquotation is hygienic, closed, and preserves each hole's exact extent and
  provenance.
- **Template law:** static instantiation is acyclic and deterministic, creates stable distinct identities per instance,
  and agrees with the corresponding handwritten declarations under the appropriate musical and provenance relations.
- **Differential compatibility:** every existing `.musa` example without new syntax produces the same score facts,
  context tracks, decisions, and Origin paths as before.

The grounding corpus should include at least: the root-dependent turn, a major/dorian re-instantiation, a multi-voice
canon, a chord class realized in two voicings, a minor-key passage choosing harmonic versus natural minor explicitly, a
generic twelve-tone row with 48 distinct forms and a symmetric counterexample, and a kernel splice containing both raw
and antiquoted material.

## 16. Migration shape

This remains a design, not a work order. If adopted, dependency order is:

1. Repair this proposal into the governing surface and kernel-elaboration documents, including the `music`/term
   distinction and exact kernel-file inclusion claim.
2. Add expression/type syntax to the lossless CST, formatter, LSP, and tree-sitter together; keep existing music forms
   valid.
3. Implement the private total evaluator for base values and functions, with its normalization/resource checks.
4. Implement contextual `Music` and migrate existing motif/fragment/repeat/transform lowering to desugar through it,
   preserving every fixture and Origin path.
5. Add pitch expressions, explicit scale values, degree/register operations, and `in scale`; remove no existing pitch
   spelling.
6. Add higher-order function values and finite folds as one capability, then express the canon/sequence corpus without
   new keywords.
7. Add declaration-level piece/voice templates, proving the key-parameterized piece and transformed-voice cases without
   introducing first-class structural values.
8. Integrate kernel files at the source-document boundary, then add typed quote/antiquote and the `KernelSplice` Origin
   step.
9. Grow source standard libraries by domain; keep analyses/checkers on the observation side of the kernel.

Each step becomes one or more prompts with its own falsification examples and Check. The governing docs are repaired
before code, and no prompt is allowed to introduce a second public compiler facade or expose the private HIR.

## 17. Deliberately open, non-load-bearing choices

- Exact expression punctuation and whether anonymous lambdas are initially surfaced. Named higher-order functions are
  semantically supported either way.
- Whether standard-library polymorphic folds are displayed with type variables or through specialized musician-facing
  wrappers.
- The notation for altered scale degrees and for choosing a register (`near`, an explicit tonic frame, or another
  musician-tested spelling).
- The first set of voicing policies beyond explicit pitch lists and close position.
- The surface spelling of declaration-template definition and instantiation, and the first module signature worth adding
  beyond direct typed parameters.

None of these changes the semantic center. The load-bearing decisions are settled here: total higher-order functions,
finite folds, contextual `music`, explicit scale/register, typed kernel quotation, provenance-aware equality, and a
separate structural-template stage behind a private deep compiler boundary.

## 18. What became of this document

Written after prompts 93–125 implemented it. Each row names where the decision now lives and where to read the code that
keeps it. Nothing above has been edited to match: an essay that quietly agrees with its own outcome is worth nothing as
a record.

| Section | Decision | Now specified in | Built by |
| --- | --- | --- | --- |
| §1, §4 | total higher-order calculus with finite folds, no general recursion | `docs/language/02-core-calculus.md` | 93–99 |
| §3 | the semantic layers and what each may know | `docs/language/00-semantics.md` | 93 |
| §5 | contextual `music`, distinct from a closed kernel term | `docs/language/00-semantics.md` §3 | 96, 104 |
| §6 | pitch, key, scale, degree, and register as separate types | `docs/language/03-musical-domains.md` §§1–2 | 100–101 |
| §7 | chord class and voicing separated, with no universal chord blob | `docs/language/03-musical-domains.md` §3 | 102 |
| §8 | theory operations are library code, not compiler primitives | `stdlib/src/`, `docs/language/06-standard-library` boundary | 105–113 |
| §9 | typed kernel quotation with holes, and the `splice` provenance step | `docs/language/01-surface.md` | 114 |
| §10 | provenance-aware equality, names retained, one erasing projection | `docs/kernel/05-normalization.md`, `docs/interface/04-provenance.md` | 97, 124 |
| §11 | the surface desugarings | `docs/language/01-surface.md` | 103 |
| §12 | declaration templates and real module functors | `docs/language/04-templates-and-modules.md` | 108–113 |
| §13 | crate boundaries and what stays private | root `AGENTS.md`, `docs/language/handbook/07-implementor.md` §1 | throughout |
| §14, §15 | acceptance rules, laws, and verification obligations | `docs/language/05-verification.md`, `07-analysis.md` | 116–121 |
| §16 | the migration shape and its baseline measurements | `docs/language/06-performance.md` | 94 |

### The open choices, resolved

§17 left six things open on purpose. All six have been decided by implementation, and the decision is the code:

- **Expression punctuation and anonymous lambdas.** Named functions only. A transformation passed to another function is
  written as a named `let` of function type (`examples/canon-functions.musa`), which keeps every function a hover target
  with a record of its own.
- **Polymorphic folds.** Displayed with their type variables. `std::list` publishes `fold` as it is, and the
  musician-facing wrappers are the domain modules that call it rather than a second spelling of it.
- **Altered degrees and register.** An explicit `Frame` — a collection plus the absolute pitch that registers it. `near`
  was not adopted; the round-trip lemma is stated over exactly the frame pair, which is what made it provable.
- **Voicing policies.** Close position, drop-*n*, explicit pitch lists, omission, and rootless
  (`examples/chord-voicings.musa`). Each is a policy that may decline, returning `Option<Voicing>`.
- **Template spelling.** `template piece`/`template voice`/`template structure`, instantiated with `make … as …`
  (`examples/template-study.musa`, `examples/module-functor-study.musa`).
- **The first signature worth adding.** `TonalContext` in `std::context`, which bundles the key, the collection, the
  spelling policy, and the voicing policy — the four facts that go wrong when they travel separately.

### What was rejected, and stayed rejected

The seven claims §1 refuses are still refused, and each is now load-bearing rather than rhetorical: `Timeline` did not
become the currency of elaboration, open music did not become a closed timeline, the calculus is not bare STLC, a key
still does not embed into absolute pitches, kernel equality still does not erase provenance, an unparseable kernel
payload is still an error rather than a warning, and no theory name has become a compiler primitive. The falsifier for
each is in `docs/language/03-musical-domains.md` §6 and in
[`docs/language/handbook/06-distinctions.md`](language/handbook/06-distinctions.md).
