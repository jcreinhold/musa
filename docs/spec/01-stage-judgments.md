# What exists at each compiler stage

This chapter lists Musa’s main representations and the operations that connect them. The list is not one chain that
every project must follow. An analysis can branch from a score, and a gesture-based practice may reach performance
without first producing Western notation.

## 1. The main representations

| Name used here | What it contains | Finite? |
| --- | --- | --- |
| `Source` | `.musa` text and locked package inputs | yes |
| `Core` | checked total expressions after parsing and name resolution | yes |
| `ContextualMusic` | music expressions that still need key, metre, voice, or other local context | yes |
| `Timeline<A>` | exact rational positions carrying values of type `A` | yes |
| `Gesture` | instrument-independent performance instructions | yes |
| `NotationPlan` | the information an engraver needs | yes |
| `Analysis<T>` | a result defined by analysis package `T`, with supporting evidence | yes |
| `ProcessDefinition` | a checked graph of audio processors and stored state | yes |
| `PreparedExecution` | a process definition plus fixed options, initial state, and allocated resources | yes |
| `AudioHistory` | output samples observed while a prepared execution runs | not necessarily |

`A` names the type carried by a timeline. `T` names the package that defines an analysis. An audio history may keep
growing, so it is not stored inside the finite timeline kernel.

## 2. Source expressions are total

The source language uses the typing and evaluation rules in `docs/language/`. In the usual notation,

```text
Σ ; Γ ⊢ e : A
```

means that expression `e` has type `A` when `Σ` supplies declarations and `Γ` supplies local variables. Evaluation is
pure, deterministic, and terminating for accepted programs, subject to the stated limits on foreign operations and
resources.

Music expressions first produce `ContextualMusic`. The compiler then supplies the local context and closes the result:

```text
m has type Music    ρ supplies its free values    κ is a valid musical context
───────────────────────────────────────────────────────────────────────────
instantiate_close(m,ρ,κ) returns a closed Term<ScoreFact>
```

This is a compilation step. It does not claim that source syntax and a score timeline are the same data.

## 3. Finite timelines

A timeline is a pair `M = (d, E)` where:

- `d` is a nonnegative rational length; and
- `E` is a finite multiset of occurrences `(s, e, a)` with `0 ≤ s ≤ e ≤ d` and payload `a : A`.

A multiset keeps duplicates. Two performers may therefore contribute identical events without one being deleted.

The main operations are:

```text
(d,E) ; (e,F) = (d + e, E together with F moved forward by d)

(d,E) ⊕ (e,F) = (max(d,e), E together with F)
```

The first operation plays values in sequence. The second overlays them. Overlay does not require equal lengths and does
not insert rests. Scaling and restriction are defined in `docs/kernel/03-denotational-semantics.md`.

## 4. Notation, analysis, and performance are separate conversions

A performance profile converts score meaning into gestures:

```text
interpret(score, profile, realization_seed)
    -> gesture representation or interpretation error
```

The result may still contain written pitch, technique names, phrases, and exact control curves. Frequency and processor
addresses need not be chosen yet.

Notation and analysis branch separately:

```text
engrave(timeline) -> notation plan or engraving error

analyze_T(input) -> Analysis<T> or analysis error
```

The subscript `T` says which package defines the analysis and its evidence. These operations are compiler passes, not
timeline reduction rules.

## 5. Preparing and running audio

Audio preparation receives every choice that can affect the result:

```text
prepare_execution(gestures, bindings, seed, options)
    -> prepared execution or preparation error
```

A successful result contains a checked process graph, initial state, resource plan, and fixed physical settings. The
engine then allocates the plan and runs one audio step at a time:

```text
allocate(prepared execution) -> runtime state or allocation error

(runtime state, input tick) -> (new runtime state, output tick)
```

An audio history is the sequence of output ticks produced for a chosen input history.

## 6. Values from different stages are not interchangeable

The following pairs are different types:

```text
source text                    score timeline
written pitch                 frequency
gesture timeline              audio process graph
prepared audio plan           audio sample history
notation plan                 analysis result
```

A named conversion may connect a pair. Shared fields, identifiers, or hashes do not create an automatic conversion.
