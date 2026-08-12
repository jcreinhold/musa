# Presentations and stage judgments

## 1. Presentation kinds

The minimum project diagram uses these distinct kinds:

```text
Source
Core
ContextualMusic
Temporal A
GesturePresentation
NotationPlan
Analysis T
ProcessDefinition
PreparedExecution
AudioHistory
```

`A` is an admitted temporal payload schema and `T` is a named analysis owner. `AudioHistory` is not finite canonical
data; it is an observation of execution under an input/device history. A project need not contain every kind. A
gesture-primary practice may construct `GesturePresentation` without first producing staff notation, while an analysis
may branch from a temporal or source presentation without being rendered.

## 2. Source and total elaboration

The source stage uses the judgments governed by `docs/language/`:

```text
Σ ; Γ ⊢ e : A
ρ ⊢ e ⇓ v
Σ ⊢ declarations ok
```

Accepted core evaluation is pure, deterministic, and strongly normalizing under its deterministic resource-acceptance
premise. It produces finite values and contextual `Music`; it does not observe a temporal `Timeline` or execute DSP.

The compiler boundary is:

```text
Σ ; Γ ⊢ m : Music    ρ ⊨ Γ    κ context-valid
──────────────────────────────────────────────── Instantiate-Close
instantiate_close(m,ρ,κ) ⇓ t : Term[ScoreFact]
```

`t` is closed and well formed in the temporal term calculus. This is a compilation judgment, not a definitional equality
between the source core and the temporal kernel.

## 3. Finite temporal presentation

For an admitted payload `A`, a temporal value is

```text
M = (d,E)
d ∈ ℚ≥0
E a finite multiset of (s,e,a)
0 ≤ s ≤ e ≤ d, a:A.
```

The governing operations are:

```text
(d,E) ; (e,F) = (d+e, E ⊎ τ_d(F))
(d,E) ⊕ (e,F) = (max(d,e), E ⊎ F)
scale_r(d,E) = (rd, scale_r(E)), r>0
restrict_I(d,E) = the observation defined by docs/kernel/03.
```

No equality-of-extents premise appears in overlay. No padding or rest occurrence is inserted. Term evaluation is the
governing `docs/kernel/10-term-calculus.md` judgment `ρ⊢t⇓M`.

## 4. Interpretation and realization

An interpretation profile is a named, versioned owner of a partial total function:

```text
interpret :
  Sem_Score × Profile × RealizationSeed
  → Result GesturePresentation InterpretationError.
```

Partiality is in the result. A profile may preserve written pitch, symbolic technique, phrase grouping, and exact curves
without choosing a frequency or DSP address. A practice may instead define another typed pass into its gesture
presentation; the displayed score route is not mandatory ontology.

Notation and analysis are separate branches:

```text
engrave_T : Presentation(Temporal A) → Result NotationPlan EngravingError
analyze_T : Presentation(P) → Result (Analysis T) AnalysisError_T.
```

The subscript names an owner. Neither judgment is a kernel reduction.

## 5. Preparation and execution

Execution preparation is fixed in Chapter 4:

```text
prepare_execution :
  Sem_Gesture × Bindings × Seed × Options
  → Result PreparedExecution PrepareError.
```

A successful prepared execution contains one accepted finite process definition, initial state, resource allocation
description, and all fixed physical policies. It contains no source-editable authority.

Allocation and execution are separate judgments:

```text
allocate : PreparedExecution → Result RuntimeState AllocationError
RuntimeState ; InputTick → RuntimeState' ; OutputTick.
```

The second is Chapter 3's process transition. An audio history is the sequence of output ticks observed under a fixed
input history and runtime conformance contract.

## 6. Stage non-collapse laws

The following are ill typed rather than false equations:

```text
Source = Temporal ScoreFact
WrittenPitch = Frequency
Timeline Gesture = ProcessDefinition
PreparedExecution = AudioHistory
NotationPlan = Analysis T.
```

A named pass may relate each relevant pair. Sharing source anchors or canonical field layouts never supplies such a pass
automatically.
