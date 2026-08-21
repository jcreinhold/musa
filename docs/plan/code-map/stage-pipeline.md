# Compiler stages and crate ownership

This page answers two engineering questions: which crate owns each stage, and how much of that stage may cross a public
API.

The stage names below are the governing ones installed by prompt 127a. The Rust identifiers in the workspace still carry
their pre-127a spellings until prompts 127b–127d, 142, and 170–173 land; the pairs are listed in
[`../clean-break-ledger.md`](../clean-break-ledger.md), and this page is stale wherever it uses one as if it were the
other.

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
EventTrack<WrittenTime, ScoreFact>
    |--------------------> NotationPlan ------> MEI / LilyPond / MusicXML
    |--------------------> Analysis result + supporting evidence
    |
    | apply performance profile and realization choices
    v
EventTrack<PerformedTime, Gesture>
    |
    | schedule(format, policy, time map, track)
    v
Schedule<Gesture> = machine + decisions
    |
    | bind instruments and studio; fix sample rate, channels, seed, and options
    v
PreparedMachine
    |
    | allocate state and step one sample frame at a time
    v
audio history
```

These stages use different data because they answer different questions. The compiler does not force them into one large
intermediate representation. A practice may add another explicit route—for example, phrase instructions directly to
gestures—without pretending those instructions are Western score facts.

The first arrow above covers four front-end stages, which are worth drawing separately because each one is a place a
later tool needs to stop at:

```text
.musa source
    |
    | lex and parse, keeping every byte
    v
lossless CST                      <- the formatter, text edits, and syntax highlighting read this
    |
    | resolve names, apply units, build music-oriented declarations
    v
music-oriented HIR                <- diagnostics that talk about voices and motifs read this
    |
    | evaluate total expressions
    v
closed Term<ScoreFact>            <- events documents and typed quotation read this
    |
    | evaluate exact musical time
    v
EventTrack<WrittenTime, ScoreFact>
```

None of these intermediate types crosses a crate boundary. The CST is Rowan-backed and stays inside `musa-syntax`; the
HIR and the evaluator's values stay inside `musa-compiler`. What crosses is the closed term and the event track.

The event-track core in particular is not spread through the compiler. Its public interface is roughly: construct and
check a track, `follow`, `together`, restrict, normalize, compare, `map_payloads`, and scale time. Its internal
representation choices stay hidden behind that. The machine is the second core value and lives on the sound side; a
track and a machine meet only at `schedule`.

## 2. Which crate owns what

| Work | Owning crate | Public API should expose |
| --- | --- | --- |
| Tokens, concrete syntax tree, formatting, and text edits | `musa-syntax` | parsing and edit operations |
| Name resolution, type checking, total evaluation, score and gesture compilation | `musa-compiler` | `compile` and caller-ready snapshot facts |
| Exact finite event tracks and their laws | `musa-events` | `Term`, the track type, construction, queries, equality, and hash |
| Engraving plan and file export | `musa-notation` | `render_notation` and export results |
| Studio checking, machine construction and scheduling, audio preparation, and offline rendering | `musa-dsp` | `prepare_execution` and an opaque prepared machine |
| Audio-device negotiation, transport, and callback | `musa-playback` | `AudioEngine` and transport commands |
| Source documents, revisions, commands, and derived-result coordination | `musa-project` | `ProjectSession` |
| CLI, LSP, desktop, and web entry points | shell crates and apps | user-facing commands and results |

`musa-dsp` keeps registered primitives, buffers, state layout, and step orders private. `musa-playback` receives a
prepared machine it can step; it does not inspect the machine. Audio crates do not depend on compiler score types.

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
