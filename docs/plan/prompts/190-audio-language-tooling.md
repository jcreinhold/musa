---
id: 190
slug: audio-language-tooling
status: done
depends_on: [125, 174, 176, 181, 183, 185, 186, 187, 189]
phase: 4
---

# The Sound Language Explains Itself

> **Governed by the event-track and machine core installed by prompts 127a–127e and 171–174.** Tooling explains inferred
> values, event tracks, machine types, scheduling, and private primitive boundaries from compiler facts.

## Task

Complete editor tooling and the handbook for performance profiles, gestures, instruments, exposed controls, private
machine bodies, mix routing, assets, exact packages, sampled formats, and recorded media. Musicians learn by
sound-making task; language developers can recover the types, staging, lowering, ownership, real-time laws, format
support, and extension boundaries from generated facts and tested examples.

## Read

- Prompts 122 and 122 seams; all of `docs/rules/language/08-performance-and-sound.md` and `09-assets-and-packages.md`;
  prompts 177–189 completion/repair notes.
- OMT chapters cited by prompt 92/182; SFZ sources cited by prompt 185; SoundFont 2.04 source cited by prompt 186.
- Current LSP, VS Code, Zed, desktop virtual documents, keyword docs, generated stdlib docs, and handbook checker.
- Note 79 and repaired 175: declarable vocabulary comes from source declarations/indexes; host registries contribute
  only registered primitive and foreign-format support facts.

## Design

Hover/signature/completion/definition/references cover:

- profiles and the standard gesture/control meaning produced by each rule;
- instruments, signatures, supported techniques/fallbacks, exposed controls, units/defaults/ranges, and origin;
- private-primitive access diagnostics and navigation from an exposed mapping to its implementation;
- buses/rooms/sends/routes/main with identity and channel information;
- asset/package identity, locked origin, offline/missing/digest state, and certain fetch/fix commands;
- SFZ/SoundFont support summaries and named unsupported opcodes/generators;
- musical clips versus fixed-media cues and their tempo/transform behavior.

Generate these from standard-library/imported declaration indexes, compiler/project facts, primitive support facts, and
format support matrices, each labelled by owner. Syntax answers on invalid source remain in `musa-syntax`; semantic
answers may use last-valid artifacts with explicit staleness. No editor parses SFZ/SoundFont, resolves packages,
interprets control curves independently, or reads a handwritten Rust surface catalogue.

Extend the handbook's musician path with choosing/swapping sounds, expression/articulation, rooms/sends, sample banks,
field recordings, and online libraries. Extend the implementor path with the exact semantic equations and laws, deep
module boundary, machine preparation lifecycle, RT rules, assets/lockfile, support matrices, and how to add one source
primitive wrapper plus registered primitive or one import adapter without widening public internals. Every fenced Musa
example compiles; every source/citation and generated table is checked.

## Target

- LSP and editor-extension coverage plus protocol/token/navigation tests.
- Generated sound-language reference and two-path handbook additions; no copied external specification text.
- Executable positive/negative examples and plain-language diagnostic goldens.
- `scripts/check-docs.sh` extended to validate audio catalogue/support matrices, local OMT citations, external
  specification links, and all examples.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --workspace --all-targets -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
./scripts/check-docs.sh
cd editors/tree-sitter-musa && tree-sitter test
git -C ../vscode-musa diff --check
git -C ../zed-musa diff --check
find examples -name '*.musa' ! -path 'examples/broken/*' -print0 | xargs -0 -n1 cargo run -q -p musa -- check
```

Commit each affected repository intentionally and record cross-repository commit ids. Commit Musa as
`Teach the sound language to explain itself`.

## Cross-repository record

- VS Code extension: `7f0ff0a` (`Cover sound language tooling in the editor smoke`).
- Zed extension: `fa63357` (`Cover sound language tooling in the editor smoke`).

## Stop

- No handwritten duplicate processor/format tables, second compiler in an editor, or unsupported-feature marketing.
- No long quotation from OMT or external format specifications; cite and state Musa's own rule.
- No plug-in hosting, waveform editing, compatibility alias, or feature added only to complete a documentation example.
