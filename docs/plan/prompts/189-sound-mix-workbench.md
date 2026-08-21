---
id: 189
slug: sound-mix-workbench
status: pending
depends_on: [124, 174, 181, 183, 185, 186, 188]
phase: 4
---

# Sound and Mix Show Musical Objects First

> **Governed by the event-track and machine core installed by prompts 127a–127e and 171–174.** The workbench edits
> source that constructs instruments and machine wiring; it owns no second graph.

## Task

Rebuild the existing Sound and Mix workspaces around instruments, exposed musical controls, part outputs, rooms/buses,
assets, and media lanes. An ordinary musician chooses a sound and shapes expression without seeing private primitives;
an instrument author can deliberately open the machine source and physical parameters. Every interaction edits Musa
source and consumes immutable project/compiler facts.

## Read

- Governing `docs/rules/desktop/`, especially source authority, selection, state/voice, Origin, accessibility, and
  budgets; roadmap §14.4; prompts 31 and 119.
- Prompts 175, 179–186 facts/edit commands; current Sound/Mix Svelte components, Tauri/project boundary, screenshots,
  and Playwright fixtures.

## Design

Write the interaction/spec repair before UI code. Progressive disclosure has three levels:

1. **Compose inspector:** selected part's instrument/profile, stable default, and a source-edit action to change it.
2. **Sound:** instrument name/origin/support, exposed controls in musical terms, technique/fallback status, sampled-bank
   preset, and asset health. A deliberate disclosure shows implementation details read-only or opens their source.
3. **Mix:** part-output strips, media-source strips, rooms/buses, sends, routes, and main output. A projected strip does
   not assert that a part is a mixer track; label the binding and retain distinct identities.

Imported/built-in instruments navigate to read-only source/support facts. Asset failures and offline packages have
loading/error/remediation states. Parameter/control gestures replace source tokens and commit once; playback may use a
separate prepared-plan update only if source has already become authoritative and prompt 191 measurement requires it.
Private machine wiring is not editable from a generic property grid or free-form canvas.

All terms use prompt 175's authoritative catalogue and the compiler/project facts from prompts 183–188; prompt 190
carries those same facts into editors and the handbook. Include keyboard navigation, screen-reader grouping, focus
preservation on recompile, narrow layouts, stale/last-valid plan indication, and reduced-motion behavior.

## Target

- Governing interface additions for instrument selection, exposed controls, assets/media, part-output identity, and
  deliberate access to machine source.
- Desktop Compose/Sound/Mix interactions using project edit commands and immutable facts.
- Unit, Playwright, accessibility, and screenshot tests with native, SFZ, SoundFont, room/send, custom control, missing
  asset, offline package, fixed cue, and musical clip fixtures.
- No second machine model, mutable preset state, or TypeScript semantic computation.

## Check

```sh
cargo nextest run -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
cd apps/musa-desktop/ui && npx playwright test
cargo insta test --workspace --unreferenced=reject
```

Commit as `Integrate instruments and media into Sound and Mix`.

## Stop

- No free-form node canvas, waveform editor, DAW timeline, hidden preset database, or UI-owned automation.
- No visual identification of part with track, instrument declaration with instance, or score mark with control value.
- No redesign of Compose engraving unrelated to sound selection and media/cue projection.
