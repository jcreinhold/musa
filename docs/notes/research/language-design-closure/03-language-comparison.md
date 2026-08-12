# Choose the source language

**Purpose:** compare three designs on the same five cases and choose the smallest one that works.

## Decision

Choose a plain, total, call-by-value language with finite user-defined data and abstract modules.

Do not add dependent indices or call-by-push-value. They are sound ideas for problems Musa does not currently have.
Neither makes any of the five musical cases shorter, clearer, or safer enough to offset its new rules.

This is a decision for the source language. It does not collapse the temporal kernel or the audio process rules into the
source evaluator.

## 1. What all candidates must express

The five cases require programs of these shapes:

```text
build a tonal value or return an error
turn phrase intent into notation or gestures, recording loss
compute exact timing and tuning values or return an arithmetic error
hide a theory's representation behind total operations
describe one total step of a live protocol without running the whole performance
```

They do not require a source expression that runs forever. They do not require the type checker to prove a whole musical
theory. A constructor may check a finite musical condition and return `Result`.

## 2. Candidate A: plain total call-by-value

Candidate A extends the current language with:

```text
Text
Result<A,E>
finite, non-recursive nominal data
exhaustive matching on nominal data
abstract type members and private constructors
build-local nominal identity
checked exact rational arithmetic
```

It keeps the current products, functions, `Option`, `List`, finite folds, structures, signatures, templates, abstract
`Music`, and kernel quotation. It keeps one left-to-right call-by-value evaluation order and has no general recursion.

### How it serves the cases

- The tonal package hides keys, chords, voicings, and function evidence behind its own types.
- The phrase package defines phrase and gesture intent without pretending they are Western pitches.
- The tuning package defines ensembles, paired roles, exact targets, and checked arithmetic.
- The timing case keeps logical, notated, performed, and physical time as separate ordinary types and passes.
- The interactive package defines a total `step` function. The host repeats it over live input.

Every invariant has one visible home: the constructor or function that checks it. Failure is a value. No index solver is
needed.

### Cost

Some mistakes are found when a constructor is evaluated rather than when its call is type-checked. For example, two
lists of equal static type may still fail a `make_phrase` check. That is acceptable: phrase membership is a theory
judgment over values, not a shape that all Musa programs must carry in their types.

## 3. Candidate B: restricted dependent indices

Dependent ML shows how to attach values from a chosen constraint domain to ML types. Its key condition is often missed:
type-checking a sufficiently annotated program reduces to constraint solving only after the language fixes a tractable
index domain. Pure inference is no longer available
([Xi and Pfenning 1999](https://www.cs.cmu.edu/~fp/papers/popl99.pdf)).

A safe Musa version could index a timeline by a rational extent using linear rational constraints. It could also index
an audio block by a fixed channel count. The design stops being the same simple system as soon as it asks the solver to
understand multiplication of unknown ratios, finite sets of pulse descriptions, package-defined pitch relations, or
arbitrary constructors. Those constraints need separate decision procedures, annotations, and equality rules.

### Test against the cases

- Equal or known extents do not help the flexible-time case. Unequal overlay is intentional, and an underdetermined
  fermata has no exact extent to put in a type.
- A phrase's admissibility depends on a named musical theory and context, not linear arithmetic.
- Ensemble tuning needs checked arithmetic and measured data, but the exact values are ordinary results. Indexing them
  does not remove a failure case.
- A live protocol needs typed inputs, state, and outputs. Its unbounded history must remain outside any finite index.
- Tonal chord and voicing validity is served by private constructors. Moving every rule into a refinement would make the
  theory part of type checking.

No two cases lose a real side condition. Candidate B adds annotations and a solver while leaving the musical work in
constructors. It fails the admission test.

### Decidability judgment

“Dependent ML style” is not by itself a decidability proof. Decidability follows only for the exact constraint domain
chosen and only when its entailment problem is decidable. Linear rational arithmetic would be manageable. The earlier
proposal's mixture of rational arithmetic, finite-set union, and plan concatenation had no such proof and did not earn
one from the examples.

## 4. Candidate C: call-by-push-value or a staged polarity language

Call-by-push-value separates values from computations and decomposes both call-by-value and call-by-name. It is useful
when evaluation order, thunks, effects, or monadic structure are central
([Levy 2006](https://pblevy.github.io/papers/hosc04.pdf)).

Musa's accepted source expressions are pure and call-by-value. A failed constructor returns `Result`; it is not an
effect. A `Music` value is a finite checked recipe consumed at a compiler boundary; it is not a suspended source
computation that the user can force. An audio process is a separate first-order state machine whose step rule is already
explicit.

Encoding these distinctions in CBPV would add value types, computation types, `return`, sequencing, thunk, and force.
The notation and audio stages would still need their present input, output, error, and loss rules. The new polarity
would restate the separation without removing a pass or a side condition.

No case becomes simpler. Candidate C fails the admission test.

## 5. Comparison

| Test | Plain total CBV | Restricted indices | CBPV or polarity |
| --- | --- | --- | --- |
| Expresses all five cases | yes | yes | yes |
| Adds a feature used by two cases | yes | no | no |
| Keeps checking decidable by syntax | yes | only after fixing and proving a solver | yes |
| Needs pervasive annotations | no | yes | no, but adds explicit staging forms |
| Keeps theory judgments in packages | yes | risks moving them into the checker | yes |
| Clarifies the source-to-audio path | yes | no change | restates, but does not replace, stage rules |
| Reuses the current safety proof | direct extension | new indexed metatheory | translation or new metatheory |

Candidate A wins.

## 6. Exact feature decision

The selected language has:

- `Unit`, `Bool`, `Nat`, `Ratio`, `Text`, and `Duration`;
- products, arrows, `Option`, `List`, and `Result`;
- monomorphic, finite, non-recursive nominal data;
- flat, exhaustive patterns whose constructor fields bind names or `_`;
- named total functions and finite folds;
- structures and signatures;
- abstract type members and sealing;
- private constructors hidden by an abstract signature;
- build-local nominal identities;
- checked rational addition, subtraction, multiplication, division, negation, and comparison;
- the existing abstract `Music` stage type and checked kernel quotation.

It does not have:

- general recursion, `fix`, effects, exceptions, or I/O;
- user-defined type parameters or polymorphic functions;
- recursive or mutually recursive user data;
- anonymous functions, nested patterns, guards, or repeated pattern variables;
- subtyping, coercions, dependent types, refinements, linear types, or graded modalities;
- first-class modules, declarations, syntax, timelines, processes, or streams.

`List`, `Option`, and `Result` remain compiler-owned type constructors with rank-1 operations. User code cannot define a
generic container in this version.

## 7. Data and module surface

An ordinary data declaration exposes its constructors inside its structure:

```text
data Direction {
    Ascending,
    Descending,
}
```

A top-level data declaration exposes its constructors to modules that import it. A signature uses an abstract type when
the representation must be hidden:

```text
signature PhraseTheory {
    type Phrase;
    let make_phrase: List<PhraseToken> -> Result<Phrase, PhraseError>;
}
```

A matching structure defines `Phrase` with `data Phrase { ... }`. Its constructors remain private because the signature
exports only `type Phrase`. Clients can create a `Phrase` only through exported functions. `PhraseError` and
`PhraseToken` are ordinary public top-level data declarations.

There are no type aliases or manifest data members in signatures in this extension. A structure provides exactly one
data definition for each abstract type member and no extra data members. Extra value members are private.

## 8. Ownership decision

| Value | Owner after migration |
| --- | --- |
| `Unit`, `Bool`, `Nat`, `Ratio`, `Text`, `Duration`, products, containers, arrows | source language |
| `Music` and `Term<ScoreFact>` quotation | notation-stage adapter |
| `Pitch` and `NoteName` | Western notation adapter |
| `Interval`, `Scale`, `Key`, `Degree`, `Frame`, `ChordClass`, `Triad`, `Voicing`, `Roman` | common-practice and related theory packages |
| `Pc12`, `PcSet12`, `Row12` | twelve-tone theory package |
| harmonic function, motive, form, inferred voice, cadence | named analysis packages |
| `Gesture` and performance controls | performance-stage adapter |
| exact frequency targets and tuning descriptions | tuning packages, converted by performance/audio adapters |
| processor nodes, ports, registers, and audio blocks | private audio process stage |

For compatibility, the first compiler version may implement package operations by calling the existing Rust algorithms.
The ownership change is visible in the source language and documentation, not a demand to rewrite correct code.

## 9. What would reopen this decision

- Reconsider dependent indices only after two unrelated programs need the same invariant at type-check time and
  value-level `Result` makes their composition materially worse.
- Reconsider CBPV only if source programs gain first-class effects, handlers, laziness, or computations that users
  explicitly suspend and force.
- Reconsider user polymorphism only after two real packages duplicate a generic data or algorithm pattern.
- Reconsider recursion only after two real finite algorithms cannot be expressed by the existing folds without an
  unreasonable expansion.
- Reconsider a `world` or `link` construct only after an ordinary typed pass with a loss record fails to compose two
  concrete stages.

Until one of those events occurs, the smaller language is the long-term design.
