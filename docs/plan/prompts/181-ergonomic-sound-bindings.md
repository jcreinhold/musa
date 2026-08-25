---
id: 181
slug: ergonomic-sound-bindings
status: pending
depends_on: [125, 174, 178, 179, 180, 180a]
phase: 3
---

# Choosing a Sound Is One Musical Action

> **Governed by the event-track and machine core installed by prompts 127a–127e and 171–174.** Surface conveniences must
> elaborate to ordinary values, event tracks, and machines.

## Task

Implement the candidate's musician-facing sound-selection and room/mix surface over the independent declarations now
supported by the compiler. A part can choose an instrument and performance profile in one readable action, hear a stable
default when it says nothing, and graduate to exposed controls or the instrument-authoring language without first
learning its private wiring.

## Read

- `docs/rules/language/01-surface.md` and `08-performance-and-sound.md` spellings/desugarings; roadmap §2's Warm Pad
  example and §14.4 progressive disclosure.
- Current `profile` binding, `assign`/`route`/`send` syntax, default studio, structured studio edits, examples, style
  guide, prompts 177–180, and 180a's completed source cutover.
- Note 79 and the prompt README ownership rule: every convenience resolves to source declarations; only registered
  primitive/runtime boundaries remain host-owned.

## Design

Implement exactly the surface forms settled by prompt 92; do not reopen punctuation here. The simple form desugars to
three independent facts: performance-profile selection, part→instrument binding, and part-output→main route. A room
convenience desugars to an ordinary named bus/effect/send. The expert surface retains explicit instruments, exposed
controls, buses, sends, routes, and private native machine bodies.

Provide a small versioned standard-library instrument package and one stable edition-specific default. Absence of an
explicit studio therefore remains audible and deterministic, with the effective default visible in hover/inspector and a
source action to make it explicit. Presets are ordinary read-only Musa declarations, never generated Rust mirrors or
opaque UI blobs.

Keep only the chosen canonical expert forms. Removed `patch`, `q`, or other old spellings are hard errors with certain
source fixes and are absent from the evaluator and runtime. Rewrite all repository source. Diagnostics lead with
musician language: missing sound, unsupported technique, incompatible control, or unconnected output; technical
identity/signature detail follows.

## Target

- Parser/CST/formatter/tree-sitter/compiler support for the settled simple and expert forms and their desugarings.
- Default/basic instrument library as ordinary inspectable source declarations with stable versioning and provenance;
  generated documentation may derive from them, but generated declarations may not derive from a Rust catalogue.
- Project edit commands for choosing/replacing an instrument/profile and making defaults explicit.
- Negative fixtures for removed studio syntax and positive fixtures showing the canonical source is concise for ordinary
  selection, room, and send tasks.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-compiler -p musa-dsp -p musa-project -p musa-lsp
cargo clippy --workspace --all-targets -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
find examples -name '*.musa' -print0 | xargs -0 -n1 cargo run -q -p musa -- check
```

Commit as `Make sound selection a musical action`.

## Stop

- No second editable sound assignment in project/UI state.
- No automatic rewriting of existing expert machine definitions into presets.
- No claim that a part is a mixer track or an instrument declaration is an instrument instance.
