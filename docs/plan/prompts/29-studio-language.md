---
id: 29
slug: studio-language
status: done
depends_on: [28]
phase: 2
---

# Studio Language

## Task

Implement the `studio` block of the language: patch declarations with signal-chain syntax, buses,
`assign`/`route`/`send` bindings, and `modulate` connections — compiled into a `StudioSpec` that `musa-dsp` can turn
into a `StudioGraphSpec`. The bridge stays narrow: parts are assigned to patches; the studio never sees notes (roadmap
§6.5).

## Read

- Roadmap §6.5 (the narrow bridge: `PartId → PatchId`, `PatchOutput → BusId`), §7.1 (the full `studio` example:
  patch/oscillator/envelope/lowpass/output, `|>` chains, `lfo |> scale |> bias`, `modulate`, `bus hall { reverb(...) }`,
  assign/route/ send with `at -18 dB`), §7.2 (units are syntax; no raw backend escapes), §13.7 (modulation is a typed
  connection).
- Prompt 16's `StudioGraphSpec`/`PortKind`, prompt 05's unit-checking skeleton.

## Design

- Language additions: `studio { ... }` with `patch`, `bus`, `assign`, `route`, `send`, `modulate`; signal expressions as
  `name = construction |> stage |> output;` chains; processor constructions `oscillator(sine, ratio: 2)`,
  `envelope(adsr(attack: 30 ms, ...))`, `lowpass(cutoff: 1400 Hz, q: 0.7)`, `reverb(room: 0.82, damping: 0.55)`,
  `gain(-15 dB)`, `scale(250 Hz)`, `bias(1400 Hz)`, `mix(a, b)`. This is the Pratt-parser prompt (§10.2): a small
  expression parser for signal chains and arguments.
- Unit enforcement (§7.2) activates fully here: `1400 Hz`, `30 ms`, `-18 dB`, `0.08 Hz` are required with units; bare
  numbers for unit-bearing parameters are diagnostics. The prompt-05 unit skeleton becomes the real table: each
  processor parameter has a declared `Unit` — shared with prompt 16's `ParameterDescriptor`, so language units and DSP
  parameters check against **one** declaration, not two.
- Compiler produces `StudioSpec` (§6.3/§10.6 pipeline: `ScoreSnapshot + StudioSpec`): patches (node trees from the `|>`
  chains — desugared to spec nodes + connections), buses, bindings (`assign violin -> glass_pad`), routing, sends,
  modulation connections. All names resolved; unknown part/patch/parameter → diagnostic with spans.
- `musa-dsp` gains `From<StudioSpec>`-style lowering to `StudioGraphSpec` (a function `studio_spec_to_graph`, direction:
  audio depends on compiler per §15.1). Desugaring rules: `a |> gain(x) |> output` = node chain; `mix(...)` = mixer
  node; `modulate lfo -> glass_pad.lowpass.cutoff` = control connection to a parameter port. Processors that don't exist
  yet in DSP (envelope, lowpass, reverb, lfo, scale, bias — prompts 30–26) are represented in the graph as typed
  placeholders that render as pass-through/silence **with a compile warning**, so language work isn't blocked on DSP
  work. Remove placeholders as 20–21 land.
- Default studio (no `studio` block) is unchanged: every part → default sine polysynth → master. Once a `studio` block
  exists, only `assign`-ed parts leave the default (document the rule; §14.8's zero-setup guarantee must survive partial
  studio blocks).

## Target

- `musa-syntax`: studio grammar, expression parser, typed wrappers, formatter.
- `musa-compiler`: `StudioSpec`, resolution, unit table wired to audio descriptors, diagnostics.
- `musa-dsp`: spec→graph lowering, placeholder processors with warnings.
- Restore the §7.1 `studio` block in `examples/glass-mountain.musa` (it compiles and renders audio end-to-end,
  placeholders and all).
- Tests: snapshot StudioSpec debug rendering; unit diagnostics (bare `1400` rejected); unknown-name diagnostics;
  graph-lowering structure tests.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-compiler -p musa-dsp
cargo clippy --all-targets -p musa-syntax -p musa-compiler -p musa-dsp -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/glass-mountain.musa
cargo run -p musa -- render examples/glass-mountain.musa --to wav -o /tmp/gm5.wav
```

Commit as `Add studio language and StudioSpec`.

## Repairs made while implementing

- **`StudioSpec` rides on `Compilation`, not `PerformanceOptions` or `ScoreSnapshot`.** The pipeline produces two
  documents (§10.6), and putting the studio inside the score would conflate the two layers §2 keeps apart. `Compilation`
  gained `studio()` and `into_parts()`; `ProjectSession` keeps the pair in `ValidArtifacts`, so a render can never use
  one piece's sound with another's notes.
- **The one `Unit` declaration lives in `musa-compiler`** and `musa-dsp` re-exports it. The prompt asked for one
  declaration rather than two; since the dependency runs compiler → audio, the language side is where it has to be. The
  per-processor *descriptors* still differ — the language names `lowpass`, the DSP names `Sine` — and unify when prompts
  30–31 give those processors real implementations.
- **The expression parser is a loop, not a Pratt table.** `|>` is the only operator in the language and it is
  left-associative, so a precedence table would have exactly one entry. What the studio grammar actually needed was
  *lookahead*: `name =`, `name(`, and a bare `name` share a first token. `Parser::nth_significant` is that, and it is
  the only lookahead in the parser.
- **`.` is a new token.** The lexer had every studio keyword reserved but no path separator, so
  `glass_pad.lowpass.cutoff` could not be written.
- **The formatter stacks long chains.** A chain of three or more stages is written one stage per line, which is how §7.1
  writes them and how a signal path is read. Two stages stay inline. `-` also became word-spaced, so `at -18 dB` no
  longer formats as `at-18 dB`.
- **A modulation path addresses a stage by its binding name or its processor name**, and an ambiguous path (two unnamed
  `lowpass` stages) is a diagnostic rather than a silent pick.
- **`adsr(...)` flattens rather than nesting.** It is an argument group for readability, not a second node: its
  arguments are the envelope's.
- **A patch's source section is the polyphonic synth.** Until prompt 30's parameter system and envelope exist, the
  note-producing head of every patch is the existing `PolySine`, and each stage downstream of it lowers to a real
  processor or a `Passthrough`. The graph has the shape the patch describes from the first day; later prompts change
  processors, not topology.
- **All patches share one note stream**, because `RenderPlan` delivers one. `lower_studio` says so in its notes when a
  studio assigns parts to more than one patch, rather than silently mis-routing. Per-part event routing is a later
  prompt's.
- **Two new audio processors**: `StereoGain` (sends and bus levels operate on already-panned signal) and
  `Passthrough { channels }` (the placeholder). Both were needed for the graph to be *valid*, not merely representable.
- **`examples/glass-mountain.musa` stays ASCII.** Its studio comment cites "roadmap 7.1" without the section sign: the
  Rust lexer reports byte offsets and the TypeScript tokenizer reports UTF-16 indices, and until that is reconciled a
  non-ASCII example breaks the highlighting fixture. Tracked separately.

## Stop

- No real ADSR/LFO/filters/effects DSP (prompts 30–26) — placeholders with warnings.
- No patch library imports (`use "../library/patches.musa"` — prompt 36).
- No GUI sound-graph visualization (the Sound workspace comes after prompt 31; do not build it here).
- No new synthesis techniques beyond what prompt 16/17 processors express.
