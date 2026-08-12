# Compiler stages and crate ownership

This page answers two engineering questions: which crate owns each stage, and how much of that stage may cross a public
API.

## 1. Main flow

```text
.musa source
    |
    | parse, resolve names, check types, evaluate total expressions
    v
closed Term<ScoreFact>
    |
    | evaluate exact musical time
    v
Timeline<ScoreFact>
    |--------------------> NotationPlan ------> MEI / LilyPond / MusicXML
    |--------------------> Analysis result + supporting evidence
    |
    | apply performance profile and realization choices
    v
Timeline<Gesture>
    |
    | bind instruments and studio; fix sample rate, channels, seed, and options
    v
PreparedExecution
    |
    | allocate state and run fixed audio steps
    v
audio samples
```

These stages use different data because they answer different questions. The compiler does not force them into one large
intermediate representation. A practice may add another explicit route—for example, phrase instructions directly to
gestures—without pretending those instructions are Western score facts.

## 2. Which crate owns what

| Work | Owning crate | Public API should expose |
| --- | --- | --- |
| Tokens, concrete syntax tree, formatting, and text edits | `musa-language` | parsing and edit operations |
| Name resolution, type checking, total evaluation, score and gesture compilation | `musa-compiler` | `compile` and caller-ready snapshot facts |
| Exact finite timelines and their laws | `musa-kernel` | `Term`, `Timeline`, construction, queries, equality, and hash |
| Engraving plan and file export | `musa-render` | `render_notation` and export results |
| Studio checking, audio preparation, processor graph, and offline rendering | `musa-audio` | `prepare_execution` and an opaque prepared plan |
| Audio-device negotiation, transport, and callback | `musa-engine` | `AudioEngine` and transport commands |
| Source documents, revisions, commands, and derived-result coordination | `musa-project` | `ProjectSession` |
| CLI, LSP, desktop, and web entry points | shell crates and apps | user-facing commands and results |

`musa-audio` keeps processor nodes, buffers, state layout, and schedules private. `musa-engine` receives a plan it can
run; it does not inspect the graph. Audio crates do not depend on compiler score types.

## 3. What each conversion must provide

The crate that converts one representation to another defines:

- the exact input and output types;
- which part of the input it reads;
- a version for the operation and its data formats;
- every option that can change the result;
- deterministic diagnostics;
- origin links between input and output anchors;
- a list of information lost or approximated; and
- resource and determinism guarantees.

There is no public generic `Pass` trait today. The passes are concrete and have different useful interfaces. Origin-path
storage should remain private until at least two crate-level callers need one stable public API.

## 4. Project-level coordination

`musa-project` already owns source revisions and the latest valid derived results, so it should also coordinate the
versioned registry of those results. Compiler, render, and audio crates return useful artifacts and origin facts. The
project layer attaches source and target versions and rejects conflicting registry records.

The public boundary stays small:

- the UI asks for selections, source locations, and diagnostics; it does not read origin-path storage structs;
- render and audio receive only the input representation they need;
- no derived result becomes a second editable syntax tree; and
- cache invalidation uses exact input and operation identity, not widget state.

## 5. Future music-theory packages

If the proposed nominal type and module design passes review, its parser and checker belong in `musa-compiler` beside
the existing total core. Nominal ids, constructor tables, package-version selection, and sealed implementations remain
private.

Static structures disappear before ordinary evaluation. Their exported theory values are finite runtime values.
Translation between two theory packages is an explicit source function or compiler pass, not a registry of Rust trait
objects.
