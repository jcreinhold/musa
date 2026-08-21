# What the current source language contains

**Purpose:** identify what can stay, what must move, and what is missing before choosing a language extension.

## 1. The result in one page

Musa already has a useful small expression language. It has pure functions, products, finite lists and options,
exhaustive matching, finite folds, static structures, and an abstract `Music` value. Accepted expressions terminate. The
temporal kernel beneath it is also small and settled.

The main defect is ownership. Musical theories are compiler extensions today. A user can call the compiler's `Key`,
`Scale`, `ChordClass`, and `Row12` operations, but cannot define an equally well-typed alternative. That makes the
Western launch library easy to use and every other theory hard to state.

Four missing features account for most of the problem:

1. finite user-defined data;
2. exhaustive matching on that data;
3. abstract data members with private constructors; and
4. ordinary `Text` and `Result` values.

Two cases also expose a smaller omission: source programs can store exact rational numbers but cannot add, multiply,
order, or safely divide them. Flexible timing and acoustic tuning both need checked rational arithmetic.

No case so far needs general recursion, effects, dependent types, call-by-push-value, a `world` construct, or a stable
compiled type identity.

## 2. What was checked

This audit reads the governing language and cross-stage specifications and the implementation in
`crates/musa-compiler/src/phase/mod.rs`. The current implementation has these type forms:

```text
Unit, Bool, Nat, Ratio, Duration,
Pitch, NoteName, Interval, Scale, Key, Degree, Frame,
ChordClass, Triad, Roman, Voicing,
Pc12, PcSet12, Row12,
(A₁, ..., Aₙ), Option<A>, List<A>, A -> B, Music
```

It has seven structural operations: natural, list, and option folds; `map`; `filter`; `range`; and finite `repeat`.
Sixty-two first-order musical operations are registered as total compiler primitives. Music construction and controlled
music transforms form a third family.

The implementation does not have `Text`, `Result`, user data declarations, abstract data members, or private nominal
constructors. `Ratio` has literals and equality but no general source-level arithmetic.

## 3. General language features

These belong to the small language itself.

| Feature | Decision | Reason |
| --- | --- | --- |
| `Unit`, `Bool`, `Nat`, `Ratio` | keep | Ordinary finite data used across theories. |
| Products and arrows | keep | Enough for records by position, functions, and finite state transitions. |
| `Option`, `List`, and finite folds | keep | Express finite absence, collections, and bounded iteration without recursion. |
| `Text` | add | Packages need names, evidence, and loss descriptions without turning them into compiler enums. |
| `Result<A,E>` | add | A failed musical construction or arithmetic operation needs an error value, not a stuck term or compiler diagnostic. |
| Finite non-recursive `data` | add | Packages need their own notes, gestures, roles, claims, and protocol states. |
| Exhaustive `match` on user data | add | A constructor is useful only when a package can consume it safely. |
| Structures and signatures | keep | They group theory operations and hide members not named by a signature. |
| Abstract data members and private constructors | add | A theory must own equality and reject invalid values at its boundary. |
| Build-local nominal type identity | add | Two abstract types in one build must differ even when their private layouts match. No cross-build promise follows. |
| Checked rational arithmetic | add | Timing and tuning need exact arithmetic. Overflow and division by zero must return `Result`, not fail evaluation. |
| General recursion and `fix` | reject | No case needs them; excluding them keeps termination simple. |
| User polymorphism | defer | Built-in `List`, `Option`, and `Result` cover the repeated containers in the cases. No user package yet needs to define a generic one. |
| Anonymous functions and nested patterns | defer | Named functions and flat exhaustive patterns express all five cases. |
| Effects or hidden global context | reject | Live input belongs to a runtime boundary. Source evaluation remains pure. |
| Partial compiler operations | remove | Current examples can use named source functions. A special partly filled operation value adds a second calling convention without serving a case study. |
| Non-prefix partial calls | remove | Ordinary currying already captures a parameter prefix. Skipping an earlier parameter makes capture depend on argument names rather than function order. A wrapper states the intended order. |

`Duration` is not a theory of metre. It is a checked nonnegative rational used at the temporal boundary. It remains a
general stage type, while metre, groove, and tempo remain separate values and passes.

## 4. Stage adapters

Some types are intentionally built in because they cross into a compiler stage. That does not make them universal
musical concepts.

| Type or form | Owner | Meaning |
| --- | --- | --- |
| `Music` | notation adapter in `musa-compiler` | A finite recipe that closes to `Term<ScoreFact>`. It is not the type of all music. |
| `Duration` | temporal bridge | Exact nonnegative logical duration. It says nothing about metre or seconds. |
| `Pitch` | Western notation adapter | A written diatonic and chromatic coordinate with register. It is not frequency. |
| `NoteName` | Western notation adapter | A written pitch spelling with register forgotten. It is not a universal pitch class. |
| `kernel Timeline[ScoreFact] { ... }` | notation/kernel bridge | A checked quotation of finite score facts. It is not a general escape into compiler internals. |
| Meter, key signature, clef, bars, and notation marks | notation adapter | Facts required to print and read a score. A package need not use them to reach performance. |
| Performance profiles and `Gesture` | performance adapter | A finite interpretation of musical intent. They do not belong to source reduction. |
| Instrument bindings and process definitions | audio adapter | They choose physical realization and processing. They do not belong to the temporal kernel. |

The current `Music` recipe reads an elaboration context containing scale and key information. The long-term rule must
narrow that claim: `Music` is score material, and its notation context is explicit at its closing boundary. A theory
package passes its own key, rāga, tuning, or phrase values as ordinary arguments. Those values do not become hidden
fields of a universal context.

## 5. Theory-library types

These current compiler types are useful, but none is part of a general account of music.

| Current type | Long-term owner | Reason |
| --- | --- | --- |
| `Interval` | Western pitch package | Its diatonic size plus semitone quality assumes a written chromatic system. |
| `Scale` | named pitch or tonal package | A finite ordered collection is one theory of pitch organization. |
| `Key` | common-practice tonal package | It combines a tonic with a major/minor key convention; it is not merely a collection. |
| `Degree` and `Frame` | scale-based pitch package | They make sense only after a collection and registered tonic have been chosen. |
| `ChordClass` and `Triad` | chord-construction package | They describe harmonic content without voicing or duration. Other practices need other carriers. |
| `Voicing` | voicing package | Register, order, omission, doubling, and ensemble policy are choices beyond chord content. |
| `Roman` | tonal-label package | A Roman numeral is a key-relative label, not a chord or a harmonic function. |
| `Pc12`, `PcSet12`, and `Row12` | twelve-tone package | Their equalities deliberately assume twelve pitch classes and the relevant transformation group. |

The first implementation may wrap the existing Rust values behind these packages. Moving ownership does not require
discarding correct algorithms.

## 6. Analysis-only values

An analysis names a method, assumptions, observations, alternatives, and evidence. Harmonic function, cadence, motive,
form, inferred voice, and segmentation belong here unless an author writes them as an explicit claim.

`Roman` illustrates the boundary. A package may construct or print a Roman numeral, but deciding that a heard sonority
has that numeral is an analysis. Likewise, `HarmonicFunction` is not a field of `ChordClass`: the same chord may receive
different readings in different passages or theories.

## 7. What should be removed or deferred

The following are not admitted by this design closure:

- a universal `Pitch`, `PitchClass`, `Key`, `Chord`, `Function`, `Voice`, or `Motif` type;
- metrical plans as type indices;
- dependent extents or channel counts in source types;
- call-by-push-value as the source evaluation model;
- `world`, `link`, or profunctor syntax;
- a source term for an unbounded audio stream;
- compiler operations used as values or called with missing arguments;
- ordinary partial calls that skip an earlier parameter;
- first-class source syntax, declarations, packages, or structures;
- persistent private values or nominal type identities across builds; and
- a compiled-artifact cache theorem.

Each can be reconsidered after a concrete program fails without it. None is required to write the five cases below.
