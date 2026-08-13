# What exists at each compiler stage

This chapter lists Musa’s main representations and the operations that connect them. The list is not one chain that
every project must follow. An analysis can branch from a score, a gesture-based practice may reach performance without
first producing Western notation, and a project may begin at a microphone and never produce a score at all.

## 1. The main representations

| Name used here | What it contains | Finite? |
| --- | --- | --- |
| `Source` | `.musa` text and locked package inputs | yes |
| `Core` | checked total expressions after parsing, adapter expansion, name resolution, and inference | yes |
| `EventTrack<C,A>` | a length and a finite multiset of occurrences carrying `A`, in coordinate `C` | yes |
| `Gesture` | instrument-independent performance instructions, carried as `EventTrack<PerformedTime,Gesture>` | yes |
| `NotationPlan` | the information an engraver needs | yes |
| `Analysis<T>` | a result defined by analysis package `T`, with supporting evidence | yes |
| `Primitive<K,A,B>` | one registered unit: id, version, and storable configuration | yes |
| `Machine<K,A,B>` | a finite description of a stepping unit built from primitives and fixed wiring | yes |
| `Schedule<A>` | a source machine emitting event batches, plus the record of every time decision | yes |
| `PreparedMachine` | a machine plus fixed format, checked resource contracts, and allocated state | yes |
| `AudioHistory` | the frames observed while a prepared machine runs | not necessarily |

`A` names the payload type carried by an event track and must be storable data. `C` names the time coordinate. `K` names
the kind of step. `T` names the package that defines an analysis. An audio history may keep growing, so it is not a
source value and is not stored inside an occurrence.

There is no `ContextualMusic` row. Music expressions produce ordinary values, event tracks, and machines; there is no
representation that means one thing here and another thing there, and no compiler stage whose job is to close one later.

## 2. Source expressions are total

The source language uses the typing and evaluation rules in `docs/rules/language/`. In the usual notation,

```text
Σ ; Γ ⊢ e : A
```

means that expression `e` has type `A` when `Σ` supplies declarations and `Γ` supplies local variables. Types are
inferred: `A` is the principal type of `e`, and an annotation is required only where a public signature or separate
checking needs one.

Evaluation is pure, strict, deterministic, and terminating for accepted programs, subject to the stated limits on
foreign operations. A typed evaluation of result `A` is one of

```text
run(remaining budget, e)     where e : A
done(v)                      where v : A
failed(ResourceError)        still an evaluation of A
```

A budget can stop an evaluation. It cannot change the value an accepted program produces.

Two static classes of type run through every rule below. Every type is a **value type**; a type with no source function
at any depth and a versioned exact encoding is additionally **storable data**. Only storable data may cross into an
occurrence payload, a machine port, a feedback value, a primitive configuration, or a foreign call.

## 3. Finite event tracks

An event track is a pair `M = (d, E)` where:

- `d` is a nonnegative exact rational length in coordinate `C`; and
- `E` is a finite multiset of occurrences `(s, e, a)` with `0 ≤ s ≤ e ≤ d` and payload `a : A`.

A multiset keeps duplicates. Two performers may therefore contribute identical occurrences without one being deleted. A
positive span is half open, `[s, e)`; an occurrence with `s = e` is a point.

The core operations are `empty`, `event`, `follow`, `together`, `map_payloads`, and `length`:

```text
follow((d,E), (q,F))   = (d + q, E together with F moved forward by d)
together((d,E), (q,F)) = (max(d,q), E together with F)
```

`follow` places one passage after another. `together` places both in one region: it does not require equal lengths and
does not insert rests. Scaling, restriction, and the coverage queries are defined in
`docs/rules/kernel/03-denotational-semantics.md`; they are retained operations with named callers rather than part of
the six-operation basis.

## 4. Notation, analysis, and performance are separate conversions

A performance profile converts score meaning into gestures:

```text
interpret(score track, profile, realization seed)
    -> EventTrack<PerformedTime,Gesture> or interpretation error
```

The result may still contain written pitch, technique names, phrases, and exact control curves. Frequency and primitive
configurations need not be chosen yet.

Notation and analysis branch separately:

```text
engrave(track) -> notation plan or engraving error

analyze_T(input) -> Analysis<T> or analysis error
```

The subscript `T` says which package defines the analysis and its evidence. These operations are compiler passes, not
event-track reduction rules.

## 5. Scheduling, preparation, and running

Scheduling is the checked connection from a finite track to a running source:

```text
schedule(format, policy, time map, track)
    -> Schedule<A> or schedule error
```

It is defined in `03-machine-calculus.md` §6. Preparation receives every choice that can affect the result:

```text
prepare_execution(gestures, bindings, seed, options)
    -> prepared machine or preparation error
```

A successful result contains a checked machine, its start state, a resource plan, and the fixed physical settings. The
engine then allocates the plan and advances the machine one step at a time:

```text
allocate(prepared machine) -> runtime state or allocation error

(runtime state, input frame) -> (next runtime state, output frame)
```

An audio history is the sequence of output frames produced for a chosen input history. It is observed, never stored as a
source value.

## 6. Values from different stages are not interchangeable

The following pairs are different types:

```text
source text                        score event track
written pitch                      frequency
gesture track                      machine
prepared machine                   audio history
notation plan                      analysis result
length in written beats            length in seconds
```

A named conversion may connect a pair. Shared fields, identifiers, or hashes do not create an automatic conversion.
