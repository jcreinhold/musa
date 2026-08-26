# 86. Audio-language conformance

**Status: governs nothing.** This is prompt 192's executable audit record. The governing claims remain the constitution,
event-track rules, and candidate language specifications. `scripts/check-audio-language-conformance.sh` runs every row
below; a green row is bounded falsifying evidence, not a mechanized universal proof.

## Matrix

| Row | Normative claim and owner | Evidence kinds and named falsifiers | Result |
| --- | --- | --- | --- |
| **A1** | Declarable quantities, performance policy, controls, instruments, studios, sample maps, and media policy are ordinary standard-library source; Rust holds exact read-only projections. `stdlib/src/{performance,sound}`, compiler checked-source bridge, DSP private preparation. | Differential/source-schema: `source_quantities_keep_exact_dimensions_until_dsp_preparation`, `the_production_studio_projection_retains_the_complete_checked_value`, vocabulary/instrument/sample/media projection laws. | Green; no independently authorable `StudioSpec`, `InstrumentSpec`, or `WrittenQuantity` public type remains. |
| **A2** | A control key and value share a source index solved by the ordinary scoped Miller-pattern unifier. Calculus elaboration owns acceptance. | Positive, postponed, disagreement, unresolved, and scope-escape laws selected by the script. | Green; no sound-specific inference table, coercion, or default is involved. |
| **A3** | Gesture support, tempo realization, frame choices, curve endpoints, profiles, complete preparation arguments, and presentation lineage remain exact and distinct. Compiler gesture lowering, scheduler, preparation. | Exact/differential: tempo boundary, hairpin midpoint/endpoints, profile comparison, adjacent support, prompt-191 R1/option/lineage laws. | Green. |
| **A4** | Signatures check source declarations while implementations stay private; each part has an isolated instance and explicit route/send; swapping, seek, and reinstall retain binding identity. Compiler/DSP/playback. | Positive/negative/end-to-end routing, private-body, send-level, swap, seek, and reinstall laws. | Green. |
| **A5** | Asset roots and complete bytes are locked exactly; digest is candidate identity only; packages are exact-pinned, offline, deterministic, read-only, and bounded. Project asset/package owners. | Positive, fault-injection, relocation, traversal, symlink, corruption, manifest-drift, offline-package, and package-asset laws. | Green. |
| **A6** | Native maps, `sfz@1`, and `sf2@1` adapt foreign bytes into the checked source-owned sample schema and refuse every unsupported sound-changing construct. Project adapters; DSP sampler preparation. | End-to-end native/SFZ/SoundFont project witnesses plus unsupported opcode/generator, escape, malformed, and resource-bound controls. | Green. |
| **A7** | Note-driven samples, beat-fitted clips, and fixed cues are three semantics; selection, release/pedal/looping, media transforms, seek, routing, and host partition are deterministic. Compiler/DSP. | Law/differential: round-robin, weighted selection, stealing, source media fit, fixed duration, crop/loop/rate, seek, partition, overlap, route/send. | Green. |
| **A8** | One frame is reference meaning; offline/live paths agree; output is deterministic and finite; callback and retirement obey RT constraints. DSP/playback. | Byte equality, explicit seed, NaN/refusal, shared-step, callback/media allocation, retirement saturation, and silence-underrun laws; prompt 191 measurements. | Green; 128-frame measured p95 42.875 µs, max 67.334 µs, zero misses on the recorded machine. |
| **A9** | Source edits remain authoritative; project facts, Origin navigation, generated help, accessibility/stale states, and editor vocabulary derive from checked declarations. Project/LSP/desktop/editor tooling. | Source-edit and stale-state laws; LSP declaration/navigation/help laws; prompt 190 tree-sitter/editor checks; prompt 191's 191/191 Playwright budget run; prompt 192 full Playwright run. | Green. |
| **A10** | The runtime boundary has one event track, one finite machine semantics, one-frame execution, storable versioned payloads, explicit seeds, checked scheduling, and no public lift or alternate interpreter. Events/calculus/DSP. | Imported R1–R14 matrix and its positive/negative/differential tests; mechanical public-surface and dependency scans. | Green. |

## Composite end-to-end fixture

No single foreign file can truthfully be both SFZ and SoundFont, and a locked package fixture necessarily owns a
separate repository. The end-to-end witness is therefore one **composite project fixture**, not one invented omnibus
source file:

- `glass-mountain.musa` supplies functions, templates/theory use, two part bindings, profiles, controls, room, send, and
  source-owned studio routing;
- `a_project_prepares_only_a_checked_map_from_the_exact_verified_read` supplies the native source sample map;
- `sfz_inheritance_layers_sequences_and_release_regions_cross_one_checked_map` supplies strict SFZ adaptation;
- `session_resolves_the_fragment_only_after_the_verified_bank_identity` supplies a generated valid SoundFont bank;
- `package_asset_addresses_resolve_to_verified_locked_metadata` supplies the exact locked package asset and offline
  sample read; and
- compiler/DSP media laws supply both the beat-fitted musical clip and the fixed cue through authored routing.

Each component crosses source checking, exact asset identity where applicable, private preparation, and an observable
consumer. The script names every component so deleting one makes conformance red.

## Closure findings

The prompt-93 expected-change ledger is empty: processor help now comes from checked studio vocabulary. Every
clean-break row is discharged; there is no prepared-plan cache to migrate. Mechanical scans find only source-owned data
projections or consumer facts for the audited sound names. `musa-events` has no musical, compiler, project, or DSP
dependency.

The public projections that remain are justified individually by A1: `Gesture` is the storable event payload with an
exact source derivation; `SampleMap` and studio/instrument contracts retain checked source bytes and have no public
semantic field constructors; project facts are read-only UI/LSP observations; prepared sampler/media/audio objects are
opaque operations. Primitive registration contains host-owned ids, ports, private state/work bounds, and frame-step
functions only. Adding a declarable policy there would make A1 fail.

The audit introduced no feature and found no governing contradiction. The performance comparison and cache-absence
result remain the measured record in `docs/rules/language/10-audio-performance.md`.
