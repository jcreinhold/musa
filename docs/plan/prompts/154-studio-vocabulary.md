---
id: 154
slug: studio-vocabulary
status: pending
depends_on: [122, 153]
phase: 3
---

# One Discoverable Studio Vocabulary

> **Governed by the event-track and machine core installed by prompts 127a–127e and 150–153.** Read the revised rules
> and the core-calculus conformance report before this prompt's Design.

## Task

Make every built-in studio processor, parameter, control type, unit, and routing term discoverable from one
authoritative surface catalogue. A musician encountering `oscillator`, `envelope`, `resonance`, `bus`, or `send` gets a
plain first sentence, a typed signature, units/defaults/range, its built-in origin, and a short example; compiler, LSP,
desktop, and generated reference material consume the same facts.

## Read

- `docs/rules/language/08-performance-and-sound.md`; roadmap §§7.2, 13.6–13.7, 14.4; prompts 29–31 and 84.
- `crates/musa-compiler/src/studio.rs`, especially `Processor::params`/`ParamSpec`; `musa-dsp` parameter descriptors;
  keyword docs and `musa-lsp/src/features/hover.rs::at_studio`.
- Existing Sound/Mix facts and all hard-coded processor/parameter name matches. Count them before choosing an owner.
- The revised machine and audio specifications and `docs/plan/code-map/process-runtime.md`; the catalogue's stable
  processor/port descriptors join the build-local primitive registry established by prompts 150–152.

## Design

The compiler-side surface catalogue owns processor spelling, musician-facing summary, longer technical note, parameter
names, aliases, unit, written range/default, signal role, and example. The audio descriptor continues to own the
post-conversion DSP range, smoothing, and combination policy. Join them by a checked stable key; do not force two
different questions into one descriptor and do not introduce a new crate for a table.

The catalogue entry also fixes the versioned processor identity and public port/parameter schema which preparation will
validate. It does not expose private machine state in this prompt. A built-in which cannot supply a first-order
total-transition/resource contract is marked unavailable to the native process registry rather than admitted through a
callback-shaped escape hatch.

Canonicalize filter `resonance`. The removed `q` spelling is a hard error with a certain code action; it is not accepted
as an alias. Hover explains that resonance is conventionally represented by quality factor Q. Rewrite repository
fixtures to the one canonical spelling. Terms such as `bus`, `send`, `instrument`, `room`, and `main` are documented as
studio concepts, not presumed prior knowledge.

Plain identifiers used as processor calls receive hover/signature/completion just as keywords do. Imported declarations
show their defining source; built-ins say `builtin`. Generate reference tables and UI labels from the catalogue. Add an
exhaustive law that every parser/compiler-recognized built-in and every editable parameter has exactly one catalogue
entry and compatible audio descriptor.

## Target

- Authoritative compiler catalogue and schema-agreement checks against `musa-dsp` descriptors.
- LSP hover/signature/completion for processors and parameters, including invalid/half-typed studio source.
- Sound/Mix labels, descriptions, accessible names, and generated reference page from the same facts.
- Hard-error `q` diagnostic/fix and migrated canonical examples, with no alias in the checker or runtime.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-compiler -p musa-dsp -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-syntax -p musa-compiler -p musa-dsp -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
rg -n "resonance|quality factor|builtin" docs/rules/language crates/musa-lsp apps/musa-desktop
```

Commit as `Make the studio vocabulary discoverable`.

## Stop

- No new processor merely to make the catalogue look complete.
- No public DSP registry, dynamic processor plug-in API, or public compiler HIR.
- No change to score-driven controls or routing; prompts 158–159 own those semantics.
