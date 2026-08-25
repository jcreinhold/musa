# Compiler stages and crate ownership

**Status: descriptive.** This page maps the governing stages to current owners and public seams.

## Main flow

```text
.musa source
    │ musa-syntax: lossless parse
    ▼
CST
    │ musa-compiler: imports, names, CST → Raw
    ▼
RawProgram
    │ musa-calculus: bidirectional elaboration, NbE, independent recheck
    ▼
checked Term                         typed quotation stops here
    │ musa-compiler: registered realization and provenance
    ▼
EventTrack(WrittenTime, ScoreFact)
    ├──────────────► musa-notation ──► MEI / LilyPond / MusicXML / MIDI
    ├──────────────► analysis plus evidence
    │ performance profile and realization
    ▼
EventTrack(PerformedTime, Gesture)
    │ prompt 172: checked scheduling
    ▼
Schedule(Gesture)
    │ native primitive preparation and instrument binding
    ▼
PreparedAudio
    │ one exact sample-frame step
    ▼
audio history
```

This flow is implemented through live and offline audio. The native flattening graph and render plan are crate-private;
the project facade joins exact performed gestures with the compiler's generic checked studio artifact. `musa-dsp`
decodes a read-only exact preparation projection plus explicit options and returns opaque `PreparedAudio`; the compiler
has no DSP dependency and the projection is not a second Rust sound language.

No intermediate front-end representation crosses its owner's facade. Rowan nodes stay in `musa-syntax`; `Raw`, core
terms, values, environments, evaluator frames, and unification stay in `musa-calculus`; compiler resolution and
derivation records stay in `musa-compiler`. Callers receive checked results and caller-ready score facts.

## Ownership

| Work | Owner | Public seam |
| --- | --- | --- |
| Tokens, CST, formatting, and text edits | `musa-syntax` | parse/format/edit results, never Rowan types |
| Dependent terms, bidirectional elaboration, NbE, conversion, rechecking | `musa-calculus` | checked facade, never `Value` or evaluator internals |
| Name/import resolution, host registrations, typed quotation, adapter expansion, musical realization | `musa-compiler` | `compile`, snapshots, diagnostics, adapter edit/print operations |
| Exact finite event tracks and their laws | `musa-events` | coordinate-indexed tracks, terms, queries, exact encoding/hash |
| Score value types and projections | `musa-score` | pitch/time/fact/snapshot values, no parser or pass |
| Declarable sound/performance values and policy | `stdlib/src/{performance,sound}` | checked source declarations; no private runtime state |
| Engraving plan and export | `musa-notation` | `render_notation` and export results |
| Machine semantics, scheduling, preparation, offline DSP | `musa-dsp` | `prepare_machine`, `schedule`, `prepare_audio`, opaque prepared values |
| Device negotiation, transport, callback | `musa-playback` | `AudioEngine` and transport commands |
| Documents, revisions, commands, derived-result coordination | `musa-project` | `ProjectSession` |
| CLI, LSP, desktop, web | shell crates and apps | user-facing commands and results |

`musa-calculus` and `musa-events` are leaves. The compiler hosts the calculus and constructs event-track values, but
neither leaf depends on compiler or score policy. Audio crates do not depend on compiler score types.

## Stage contracts

Every conversion states its exact input and output, operation/data versions, options, deterministic diagnostics,
resource guarantees, provenance edges, and any loss. There is no public generic `Pass` trait: these stages hide
different work and deliberately have different useful interfaces.

The prompt-149 trusted boundary is load-bearing. Elaboration is not trusted to certify itself: accepted core terms are
independently rechecked before the compiler realizes them. Runtime preparation consumes those checked finite values; it
must not grow another source evaluator or checker.

The event track remains the semantic meeting point for finite music. A surface convenience elaborates to its fixed
operations; it does not add an operation to `musa-events`. A track meets a running machine only through the checked
schedule operation.

## Project coordination

`musa-project` owns source revisions and last-valid derived results. Compiler, notation, and audio stages return useful
artifacts and origin facts; the project attaches revisions and operation versions. The UI consumes facts rather than
compiler internals, no derived artifact becomes a second editable syntax tree, and cache hits compare exact arguments
after hashes select candidates.
