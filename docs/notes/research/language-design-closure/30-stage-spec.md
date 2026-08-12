# From source values to notation and sound

## Purpose

This document defines the boundaries after source evaluation. Its main claim is simple:

> Musa connects distinct representations with checked conversions. It does not force source, notation, analysis,
> performance, and sound into one value.

The temporal kernel and audio process semantics stay as governed today. This note only states how the new source
language reaches them.

## 1. Start with two valid paths

A notation-led piece may follow:

```text
staff source
-> StaffDocument
-> explicit staff realization choices
-> Music
-> closed Term<ScoreFact>
-> Timeline<ScoreFact>
-> NotationPlan
```

A phrase-led piece may follow:

```text
phrase source
-> Phrase
-> explicit phrase and instrument context
-> finite GestureIntent
-> GestureTimeline
-> PreparedExecution
-> audio steps
```

The phrase may also have a lossy staff transcription. That branch does not become the cause of the performance branch.
Both branches retain origin paths to the phrase value that produced them.

## 2. What `Music` means

### 2.1 `Music` is a finite closed recipe

`Music` is an abstract compiler-owned value. Source code cannot inspect or forge its representation. Its private value
is a finite `ScoreRecipe`:

```text
ScoreRecipe = Empty
            | Fact(relative span, FactTemplate, FactOrigin)
            | Sequence(ScoreRecipe, ScoreRecipe)
            | Overlay(ScoreRecipe, ScoreRecipe)
            | Share(RecipeName, ScoreRecipe, ScoreRecipe)
            | Use(RecipeName, generation site)
```

A relative span has nonnegative exact rational start and end. `FactOrigin` contains the original source anchor and an
ordered finite list of `(ScoreMapId, map anchor)` steps. A `FactTemplate` contains every musical field of one
`ScoreFact`, including written facts, named context changes, and presentation detail. It lacks only final score scope,
absolute placement, and the complete origin path. Closing supplies those three fields.

`ScoreMapId` names one admitted total first-order map on `FactTemplate`. Transposition, for example, is an explicit map
id plus its already supplied interval; no source closure is stored in `Music`. `Share` binds one finite subrecipe, and
`Use` names that binding at a generation site. Names are lexical, acyclic, and unique within the recipe.

The compiler operations are:

```text
music_empty: Unit ⇒ Music
music_fact: RelativeSpan × FactTemplate × Anchor ⇒ Music
music_sequence: Music × Music ⇒ Music
music_overlay: Music × Music ⇒ Music
music_map: ScoreMapId × Music × Anchor ⇒ Result<Music, MusicBuildError>
music_share: RecipeName × Music × Music ⇒ Result<Music, MusicBuildError>
music_use: RecipeName × Anchor ⇒ Music
```

They construct this finite tree. `music_map` first validates and unfolds every `Share` and `Use` in its finite argument,
then applies the admitted map to each resulting fact and appends `(map id, map anchor)` to its `FactOrigin`. Its result
contains no `Share` or `Use`. It does not call a nonexistent temporal-term map, and mapping one use cannot change an
unmapped use. The operation charges for the fully expanded result, so it may return a resource error before committing a
large rewrite. `music_share` rejects a duplicate name; `ValidMusic` rejects a free or forward `Use` and any binding
cycle. The typed body normally calls package wrappers, not these operations directly.

Nested maps run inside out. `music_map(g, music_map(f, m))` applies `f` and then `g`, and the fact origin lists those
steps in that order.

All theory- or adapter-specific choices must be supplied before a package returns `Music`. Examples include:

- a tonal package choosing chord spelling and voicing;
- a staff package choosing durations for an unmeasured passage;
- a phrase package choosing a tonic and performance reading; and
- a tuning package choosing an ensemble and acoustic target.

This rule avoids a universal bag of package-specific context. A package takes its own ordinary context record, then
returns either `Music` or a package error.

### 2.2 The remaining musical context is small

Closing a `Music` value still needs the ambient placement at which it is used:

```text
MusicalContext = {
  placement: nonnegative Ratio,
  score_scope: ScoreScope,
  source_root: source Anchor,
}
```

`score_scope` names the part, staff, and voice destination. It does not contain a key, tuning, metre, tempo, notation
policy, or performance profile. Those are explicit facts or earlier package arguments.

The closing operation is:

```text
close_music:
  Music × MusicalContext
  -> Result<Term<ScoreFact>, MusicError>
```

It places the finite recipe, checks scope and payload invariants, and returns a closed temporal term. A `MusicError`
names an invalid placement, scope conflict, missing source anchor, or rejected payload. It never means “the compiler
looked up an unstated global key.”

### 2.3 Validity contract

Write `FiniteMusic(m)` when `m` is a finite recipe whose local fields have the right representation. Every compiler
operation returns `FiniteMusic` or a stated `MusicBuildError`.

Write `ValidMusic(m)` when, in addition, every stored payload satisfies the `ScoreFact` schema, every share name is
unique in its lexical region, and every `Use` names an earlier enclosing `Share`. Validity is a decidable whole-recipe
check. A bare `music_use("missing", anchor)` is finite but invalid.

The compiler-owned constructors satisfy:

```text
well-typed Music arguments
──────────────────────────
operation result is FiniteMusic or a stated MusicBuildError
```

The close contract is:

```text
ValidMusic(m)       ValidContext(κ)
────────────────────────────────────────────────────────────────
close_music(m, κ) = Err(error)
or
close_music(m, κ) = Ok(t) and ∅ ⊢ t : Term<ScoreFact> and t is finite
```

Because both the recipe and context are finite and closing is structural, the operation terminates.

### 2.4 Under-specified notation stays before `Music`

A staff page may contain a fermata, feathered beam, or unmeasured passage whose performed duration is not fixed. That
page is a `StaffDocument`, not yet `Music`.

The staff package exposes an operation like:

```text
realize_staff:
  StaffDocument × StaffRealization
  -> Result<Music, StaffRealizationError>
```

`StaffRealization` names every required timing choice. Engraving can branch directly from `StaffDocument` without those
choices. This is clearer than hiding an open-ended context dictionary inside `Music`.

## 3. What every conversion returns

For stored representations `S` and `T`, a successful conversion returns:

```text
PassResult<S, T> = {
  output: T,
  paths: finite ordered set of OriginPath<S, T>,
  losses: finite ordered List<Loss>,
}
```

It may instead return a finite diagnostic. Each pass has one descriptor containing its source kind, target kind,
version, input and output schemas, evidence schema, and loss schema.

An origin path uses the governed steps:

```text
Preserved(source anchor, target anchor, evidence)
Generated(source root, generation site, target anchor, evidence)
Combined(source anchors, target anchor, evidence)
```

Every anchor must exist in its exact representation version.

A `PassResult<S, T>` is valid only when every addressable target anchor in `output` has at least one path ending there.
Each path must start at a declared source root, source anchor, or generation site accepted by the pass descriptor. Extra
paths that end outside the output are forbidden. Combined and generated steps may have several incoming source anchors,
but they still cover one exact target anchor.

To compose `P : A -> B` with `Q : B -> C`, retain the intermediate `B` presentation and form this path set: for each
path `q` covering a target anchor of `C`, concatenate `q` with every `P` path that ends at each exact `B` source anchor
used by the first step of `q`. If any required `B` anchor has no exact match, composition returns a diagnostic.
Deduplicate only exact duplicate complete paths, in the fixed path order. This construction covers every target anchor
because `Q` does, and it never selects an empty subset by choice.

Expansion records from note 29 are source maps, not these musical conversion steps. The first musical pass starts from
the adapter use-site anchor and may retain the adapter definition as evidence.

## 4. The front-end boundaries

### 4.1 Source to grouped syntax

| Item | Rule |
| --- | --- |
| Input | exact source bytes and source version |
| Output | lossless tokens and groups with byte ranges |
| Errors | invalid byte encoding, string escape, delimiter, or indentation |
| Equality | exact bytes plus lexer and grouper version |
| Record | token and group source ranges; no musical derivation yet |

### 4.2 Grouped syntax to expanded syntax

| Item | Rule |
| --- | --- |
| Input | grouped syntax, resolved syntax imports, exact adapter definitions |
| Output | ordinary expression syntax plus `ExpansionRecord`s |
| Errors | adapter syntax error, adapter resource limit, forbidden output form |
| Equality | exact syntax tree, opaque scopes, source info, and adapter versions |
| Record | original/generated source paths; no musical derivation yet |

### 4.3 Expanded syntax to resolved and typed body

| Item | Rule |
| --- | --- |
| Input | ordinary expanded expressions and one finite build graph |
| Output | resolved ids, inferred monotypes, principal schemes, coverage and privacy evidence |
| Errors | unresolved or ambiguous name, private access, invalid data, non-exhaustive match, type error, phase limit |
| Equality | exact structure with the same build-local ids and inferred types |
| Record | every node retains its complete source or expansion path |

### 4.4 Typed body to source values

| Item | Rule |
| --- | --- |
| Input | well-typed evaluation core and compiler-operation registry |
| Output | one typed finite value |
| Errors | deterministic evaluation limit or a compiler-operation diagnostic stated by its contract |
| Equality | structural base/data equality; function values have no public decidable equality |
| Record | value construction sites point to typed-body anchors |

Two evaluations of the same core under the same operation registry and limit either return equal values and charges or
the same diagnostic. This is evaluation determinism, not a cache theorem.

## 5. The musical and temporal boundaries

### 5.1 Package value to `Music` or gesture intent

A package conversion has ordinary source types, for example:

```text
realize_staff: StaffDocument × StaffRealization -> Result<Music, StaffError>
perform_phrase: Phrase × PhraseContext -> Result<List<GestureIntent>, PhraseError>
realize_tuning: Ensemble × TuneRequest -> AcousticTarget
```

The package defines value equality and errors. The pass records added choices and losses. A package cannot claim that a
staff transcription is lossless merely because its function returned successfully.

### 5.2 `Music` to a closed temporal term

| Item | Rule |
| --- | --- |
| Input | `ValidMusic` and explicit `MusicalContext` |
| Output | closed finite `Term<ScoreFact>` |
| Errors | `MusicError` from the finite validity checks in §2.2 |
| Equality | no required decidable equality on `Music`; terms compare by the governed temporal semantics |
| Record | recipe anchors map to term anchors; generated reuse records root and use site |

`close_music` first translates the private recipe at logical origin zero, then delays the whole term by
`MusicalContext.placement`. It traverses the recipe structurally:

- `Fact` fills the scope, keeps its relative span, starts the origin at `source_root` and the fact's source anchor, then
  appends its ordered map steps;
- `Sequence` closes both children and builds temporal sequence;
- `Overlay` closes both children and builds temporal overlay;
- `Share` closes its definition once as a marked temporal binding, then closes its body; and
- `Use` emits a marked reference whose derivation records the bound root and generation site; and
- the final outer delay supplies the one ambient placement without changing internal sequence offsets.

Lexical name checking happens during construction and closing checks it again as defense in depth. Structural traversal
of a finite acyclic recipe terminates. `close_music` is deterministic on the same recipe value and context. Musa does
not persist or cache arbitrary source closures as `Music` identity.

### 5.3 Temporal term to timeline

The governed kernel judgment is:

```text
⊢ t : Term<A>
```

Evaluation returns `Timeline<A> = (d, E)`, where `d` is a nonnegative exact ratio and `E` is a finite multiset of typed
occurrences inside `[0, d]`.

Sequence adds lengths. Overlay takes the greater length and combines occurrences. Overlay does not require equal lengths
and does not insert rests. Normalization and payload equality remain those in `docs/rules/kernel/`.

| Item | Rule |
| --- | --- |
| Input | closed well-typed finite temporal term |
| Output | normalized finite timeline |
| Errors | resource limit or violated admitted-payload contract |
| Equality | exact extent plus normalized multiset under the payload's versioned equality |
| Record | term references and payload origins remain available outside kernel semantic equality |

Both source evaluation and temporal evaluation normalize. They are two different normalization theorems.

## 6. Notation, analysis, and performance branch

### 6.1 Notation

```text
engrave:
  Timeline<ScoreFact> × NotationOptions
  -> Result<PassResult<Timeline<ScoreFact>, NotationPlan>, NotationError>
```

`NotationOptions` names backend, page settings, spelling policy for exact durations, and every choice that changes the
plan. `NotationPlan` equality is exact structural equality under its versioned schema. Engraving records choices such as
spelling `Exact(3/8)` as a dotted quarter.

A notation package may also engrave a richer `StaffDocument` directly. That pass has a different id and source schema;
it must not be confused with timeline engraving.

### 6.2 Analysis

```text
analyze_T:
  Input_T × AnalysisOptions_T
  -> Result<PassResult<Input_T, Analysis<T>>, AnalysisError_T>
```

Package `T` owns the result, evidence, equality, and error. The compiler does not infer harmonic function, scale degree,
metre, form, or motive from a shared field name. An analysis may combine several source anchors into one claim.

### 6.3 Performance intent to gestures

```text
interpret_T:
  Intent_T × PerformanceContext_T × RealizationSeed
  -> Result<PassResult<Intent_T, GestureTimeline>, PerformanceError_T>
```

The context includes performer, instrument capability, tempo and timing choices, and every reading that changes the
gesture. The result is finite for one prepared score or finite response. It may contain exact control curves and named
technique, but no processor address or sample buffer.

Gesture equality is the exact versioned equality of the gesture schema. A seed is an explicit argument even when one
interpreter ignores it.

## 7. Preparing and running sound

### 7.1 Preparation

```text
prepare_execution:
  GestureTimeline
  × CheckedStudio
  × InstrumentBindings
  × Seed
  × PrepareOptions
  -> Result<PreparedExecution, PrepareError>
```

`PrepareOptions` contains sample rate, channels, fixed semantic step size, numeric rules, render bounds, quality policy,
and resource limits. A successful result contains the checked process definition, fixed options, initial state, and
resource plan.

Origin data is prepared separately:

```text
prepare_lineage:
  GesturePresentation × PreparedExecution
  -> PassResult<GesturePresentation, ProcessPresentation>
```

Lineage cannot change processor selection, graph shape, state, or samples.

### 7.2 One process step

The process judgment remains:

```text
G ⊢ (σ, ι) -> (σ', o)
```

The graph is finite. Ordinary-wire dependencies form an acyclic whole-node graph. Registers supply prior-step values, so
every feedback loop crosses at least one register. A fixed topological order and deterministic first-order processor
steps give one next state and output.

### 7.3 Audio history

For input history `ι₀, ι₁, ...`, repeated process steps produce `o₀, o₁, ...`. A run may have no final step. An audio
history therefore does not normalize to one finite source value.

Equal prepared executions produce equal histories only when initial state, external inputs, processor contracts, and
numeric/device conditions also match. This is the governed `R1-frames` claim, not a stronger promise.

## 8. Equality and identity by representation

| Representation | Equality used by this work |
| --- | --- |
| Source | exact versioned bytes |
| Grouped or expanded syntax | exact tree, source info, scopes, and phase versions |
| Resolved or typed body | exact structure and build-local ids within one build |
| Ordinary data value | structural equality of its type, when that type declares equality |
| Function value | no public decidable equality |
| `Music` | no required decidable equality; same value and context give one close result |
| Temporal timeline | governed normalized semantic equality for its payload schema |
| Staff or theory value | package-declared versioned equality |
| Analysis | analysis-package equality and evidence schema |
| Gesture | versioned gesture equality |
| Prepared execution | complete versioned structural equality |
| Audio history | stepwise equality under the process and device premises |

A hash may locate candidates for an exact value. It never establishes equality.

## 9. Composition of conversion records

Suppose pass `P` ends at exact target anchor `b`, and pass `Q` begins at that same representation version and anchor
`b`. Their composite:

1. keeps both pass descriptors;
2. concatenates each matching origin path at `b`;
3. keeps `b` as the intermediate anchor; and
4. concatenates loss lists in pass order.

If the anchors or representation versions differ, composition is undefined and the compiler reports the mismatch. A
semantic resemblance, equal payload, or equal hash is not enough.

Path composition is associative because it is concatenation of finite typed paths at exact equal endpoints. Empty paths
at an anchor are identities. These are the existing governed derivation laws.

## 10. What normalizes and what runs

The final distinction is:

```text
source expression      -> one finite value
Music close            -> error or one finite temporal term
temporal term           -> one finite timeline
notation or analysis   -> error or one finite result
gesture interpretation -> error or one finite gesture timeline
audio preparation      -> error or one finite prepared execution
process step            -> one next state and output block
audio run               -> possibly unbounded history
```

No polarity calculus is needed to say this. The type and operation at each boundary already state whether Musa computes
a finite value or advances a running process.
