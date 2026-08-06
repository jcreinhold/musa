---
id: 19
slug: studio-language
status: pending
depends_on: [18]
phase: 2
---

# Studio Language

## Task

Implement the `studio` block of the language: patch declarations with signal-chain syntax, buses,
`assign`/`route`/`send` bindings, and `modulate` connections — compiled into a `StudioSpec` that `musa-audio` can turn
into a `StudioGraphSpec`. The bridge stays narrow: parts are assigned to patches; the studio never sees notes (roadmap
§6.5).

## Read

- Roadmap §6.5 (the narrow bridge: `PartId → PatchId`, `PatchOutput → BusId`), §7.1 (the full `studio` example:
  patch/oscillator/envelope/lowpass/output, `|>` chains, `lfo |> scale |> bias`, `modulate`, `bus hall { reverb(...) }`,
  assign/route/ send with `at -18 dB`), §7.2 (units are syntax; no raw backend escapes), §13.7 (modulation is a typed
  connection).
- Prompt 11's `StudioGraphSpec`/`PortKind`, prompt 05's unit-checking skeleton.

## Design

- Language additions: `studio { ... }` with `patch`, `bus`, `assign`, `route`, `send`, `modulate`; signal expressions as
  `name = construction |> stage |> output;` chains; processor constructions `oscillator(sine, ratio: 2)`,
  `envelope(adsr(attack: 30 ms, ...))`, `lowpass(cutoff: 1400 Hz, q: 0.7)`, `reverb(room: 0.82, damping: 0.55)`,
  `gain(-15 dB)`, `scale(250 Hz)`, `bias(1400 Hz)`, `mix(a, b)`. This is the Pratt-parser prompt (§10.2): a small
  expression parser for signal chains and arguments.
- Unit enforcement (§7.2) activates fully here: `1400 Hz`, `30 ms`, `-18 dB`, `0.08 Hz` are required with units; bare
  numbers for unit-bearing parameters are diagnostics. The prompt-05 unit skeleton becomes the real table: each
  processor parameter has a declared `Unit` — shared with prompt 11's `ParameterDescriptor`, so language units and DSP
  parameters check against **one** declaration, not two.
- Compiler produces `StudioSpec` (§6.3/§10.6 pipeline: `ScoreSnapshot + StudioSpec`): patches (node trees from the `|>`
  chains — desugared to spec nodes + connections), buses, bindings (`assign violin -> glass_pad`), routing, sends,
  modulation connections. All names resolved; unknown part/patch/parameter → diagnostic with spans.
- `musa-audio` gains `From<StudioSpec>`-style lowering to `StudioGraphSpec` (a function `studio_spec_to_graph`,
  direction: audio depends on compiler per §15.1). Desugaring rules: `a |> gain(x) |> output` = node chain; `mix(...)` =
  mixer node; `modulate lfo -> glass_pad.lowpass.cutoff` = control connection to a parameter port. Processors that don't
  exist yet in DSP (envelope, lowpass, reverb, lfo, scale, bias — prompts 20–21) are represented in the graph as typed
  placeholders that render as pass-through/silence **with a compile warning**, so language work isn't blocked on DSP
  work. Remove placeholders as 20–21 land.
- Default studio (no `studio` block) is unchanged: every part → default sine polysynth → master. Once a `studio` block
  exists, only `assign`-ed parts leave the default (document the rule; §14.8's zero-setup guarantee must survive partial
  studio blocks).

## Target

- `musa-language`: studio grammar, expression parser, typed wrappers, formatter.
- `musa-compiler`: `StudioSpec`, resolution, unit table wired to audio descriptors, diagnostics.
- `musa-audio`: spec→graph lowering, placeholder processors with warnings.
- Restore the §7.1 `studio` block in `examples/glass-mountain.musa` (it compiles and renders audio end-to-end,
  placeholders and all).
- Tests: snapshot StudioSpec debug rendering; unit diagnostics (bare `1400` rejected); unknown-name diagnostics;
  graph-lowering structure tests.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-audio
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-audio -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- check examples/glass-mountain.musa
cargo run -p musa-cli -- render examples/glass-mountain.musa --to wav -o /tmp/gm5.wav
```

Commit as `Add studio language and StudioSpec`.

## Stop

- No real ADSR/LFO/filters/effects DSP (prompts 20–21) — placeholders with warnings.
- No patch library imports (`use "../library/patches.musa"` — prompt 26).
- No GUI sound-graph visualization (the Sound workspace comes after prompt 21; do not build it here).
- No new synthesis techniques beyond what prompt 11/12 processors express.
