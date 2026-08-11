---
id: 127
slug: studio-vocabulary
status: pending
depends_on: [92, 122, 125]
phase: 3
---

# One Discoverable Studio Vocabulary

> **Contingent on prompt 125.** The core-boundary decision may repair this prompt's Design, fold it into another, or
> replace it. Read `docs/core-boundary.md` first.

## Task

Make every built-in studio processor, parameter, control type, unit, and routing term discoverable from one
authoritative surface catalogue. A musician encountering `oscillator`, `envelope`, `resonance`, `bus`, or `send` gets a
plain first sentence, a typed signature, units/defaults/range, its built-in origin, and a short example; compiler, LSP,
desktop, and generated reference material consume the same facts.

## Read

- `docs/language/08-performance-and-sound.md`; roadmap §§7.2, 13.6–13.7, 14.4; prompts 29–31 and 84.
- `crates/musa-compiler/src/studio.rs`, especially `Processor::params`/`ParamSpec`; `musa-audio` parameter descriptors;
  keyword docs and `musa-lsp/src/features/hover.rs::at_studio`.
- Existing Sound/Mix facts and all hard-coded processor/parameter name matches. Count them before choosing an owner.

## Design

The compiler-side surface catalogue owns processor spelling, musician-facing summary, longer technical note, parameter
names, aliases, unit, written range/default, signal role, and example. The audio descriptor continues to own the
post-conversion DSP range, smoothing, and combination policy. Join them by a checked stable key; do not force two
different questions into one descriptor and do not introduce a new crate for a table.

Canonicalize filter `resonance`; accept `q` as a source-compatible deprecated alias with a certain code action. Hover
explains that resonance is conventionally represented by quality factor Q. Existing source and audio remain unchanged
until explicitly edited. Terms such as `bus`, `send`, `patch`, `room`, and `main` are documented as studio concepts, not
presumed prior knowledge.

Plain identifiers used as processor calls receive hover/signature/completion just as keywords do. Imported declarations
show their defining source; built-ins say `builtin`. Generate reference tables and UI labels from the catalogue. Add an
exhaustive law that every parser/compiler-recognized built-in and every editable parameter has exactly one catalogue
entry and compatible audio descriptor.

## Target

- Authoritative compiler catalogue and compatibility checks against `musa-audio` descriptors.
- LSP hover/signature/completion for processors and parameters, including invalid/half-typed studio source.
- Sound/Mix labels, descriptions, accessible names, and generated reference page from the same facts.
- `q` deprecation diagnostic/fix and migrated canonical examples without breaking old fixtures.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-audio -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-audio -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
rg -n "resonance|quality factor|builtin" docs/language crates/musa-lsp apps/musa-desktop
```

Commit as `Make the studio vocabulary discoverable`.

## Stop

- No new processor merely to make the catalogue look complete.
- No public DSP registry, dynamic processor plug-in API, or public compiler HIR.
- No change to score-driven controls or routing; prompts 131–132 own those semantics.
