# Stage pipeline and ownership

## 1. The pipeline

```text
.musa source
  │ parse / resolve / total elaborate
  ▼
contextual Music + closed Term<ScoreFact>
  │ kernel evaluate
  ▼
Timeline<ScoreFact>
  ├──────────────► NotationPlan ─► MEI/LilyPond/MusicXML
  ├──────────────► Analysis_T + evidence
  │ interpret profile / realization
  ▼
Timeline<Gesture>
  │ bind instruments, studio, seed, complete options
  ▼
PreparedExecution (private finite ProcessDefinition + state/resources)
  │ allocate / tick
  ▼
observed audio history
```

This is a family of typed passes, not one lowering IR. A practice may add another path—for example phrase intent
directly to gestures—without pretending its values are `ScoreFact` or Western pitch.

## 2. Crate ownership

| Stage | Owner | Public boundary |
| --- | --- | --- |
| Tokens/CST/source edits | `musa-language` | `parse`, formatter/edit operations |
| Resolution, total core, theory owners, score/gesture passes | `musa-compiler` | narrow `compile`/snapshot facts |
| Exact finite temporal algebra | `musa-kernel` | `Term`, `Timeline`, constructors/queries/identity |
| Notation planning/export | `musa-render` | `render_notation` and export results |
| Studio checking, preparation, private process IR | `musa-audio` | `prepare_execution`, opaque prepared audio plan |
| Device allocation/tick transport | `musa-engine` | `AudioEngine`, transport commands |
| Source/revision authority and artifact coordination | `musa-project` | `ProjectSession` |
| CLI/LSP/desktop/wasm | shells | caller-oriented commands/results only |

No crate above `musa-audio` sees process nodes, buffer indices, processor states, or graph scheduler internals. The
engine receives one opaque RT-safe plan interface. No audio/engine type points back into compiler score types.

## 3. Pass ownership

Each pass owner defines:

- exact source and target presentation types;
- the admitted semantic projection it may inspect;
- operation/schema version;
- complete finite options;
- canonical diagnostics;
- source/target anchor mapping;
- losses or approximations; and
- determinism/resource contracts.

Lineage storage does not require a public generic `Pass` trait. Current pass families are closed and concrete. Shared
path/registry records remain private implementation data until two crate-level callers require a stable facade.

## 4. Artifact coordination

`musa-project` is the natural owner of one session's versioned artifact registry because it already owns documents,
revisions, commands, and last-valid artifacts. Pass-producing crates return caller-oriented artifacts plus private or
crate-internal anchor facts. The project layer qualifies them with the versioned source/target presentation refs and
validates registry merges.

This coordination must remain deep:

- the UI sees stable selection/origin ids and queries, not path storage structs;
- render/audio crates see only the source presentation slices their pass needs;
- a derived artifact never becomes an editable AST; and
- invalidation uses exact presentation/operation identity, not widget state.

## 5. Source theory ownership

If the nominal theory-module candidate graduates, its parser and checker live in `musa-compiler`'s existing total core
and static-module pass. Nominal stamps, constructor metadata, bidirectional checking, and sealed target definitions stay
private. The public compiler facade does not export a general typechecker object or runtime structure value.

Static structures disappear before ordinary evaluation. Theory-owned values remain ordinary finite values. Cross-owner
translation is an explicitly exported function/pass, not a Rust trait-object registry.
