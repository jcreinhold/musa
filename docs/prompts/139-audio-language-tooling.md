---
id: 139
slug: audio-language-tooling
status: pending
depends_on: [122, 124, 130, 132, 134, 135, 136, 138]
phase: 4
---

# The Sound Language Explains Itself

## Task

Complete editor tooling and the handbook for performance profiles, gestures, instruments, exposed controls, private
graphs, mix routing, assets, exact packages, sampled formats, and recorded media. Musicians learn by sound-making task;
language developers can recover the types, staging, lowering, ownership, real-time laws, format support, and extension
boundaries from generated facts and tested examples.

## Read

- Prompts 120 and 122 seams; all of `docs/language/08-performance-and-sound.md` and `09-assets-and-packages.md`;
  prompts 124–138 completion/repair notes.
- OMT chapters cited by prompt 92/133; SFZ sources cited by prompt 134; SoundFont 2.04 source cited by prompt 135.
- Current LSP, VS Code, Zed, desktop virtual documents, keyword docs, generated stdlib docs, and handbook checker.

## Design

Hover/signature/completion/definition/references cover:

- profiles and the standard gesture/control meaning produced by each rule;
- instruments, signatures, supported techniques/fallbacks, exposed controls, units/defaults/ranges, and origin;
- private-node access diagnostics and navigation from an exposed mapping to its implementation;
- buses/rooms/sends/routes/main with identity and channel information;
- asset/package identity, locked origin, offline/missing/digest state, and certain fetch/fix commands;
- SFZ/SoundFont support summaries and named unsupported opcodes/generators;
- musical clips versus fixed-media cues and their tempo/transform behavior.

Generate these from compiler/project catalogues and format support matrices. Syntax answers on invalid source remain in
`musa-language`; semantic answers may use last-valid artifacts with explicit staleness. No editor parses SFZ/SoundFont,
resolves packages, or interprets control curves independently.

Extend the handbook's musician path with choosing/swapping sounds, expression/articulation, rooms/sends, sample banks,
field recordings, and online libraries. Extend the implementor path with the exact semantic equations and laws, deep
module boundary, preparation lifecycle, RT rules, assets/lockfile, compatibility matrices, and how to add one built-in
processor or import adapter without widening public internals. Every fenced Musa example compiles; every source/citation
and generated table is checked.

## Target

- LSP and editor-extension coverage plus protocol/token/navigation tests.
- Generated sound-language reference and two-path handbook additions; no copied external specification text.
- Executable positive/negative examples and plain-language diagnostic goldens.
- `scripts/check-language-docs.sh` extended to validate audio catalogue/support matrices, local OMT citations, external
  specification links, and all examples.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
./scripts/check-language-docs.sh
cd editors/tree-sitter-musa && tree-sitter test
git -C ../vscode-musa diff --check
git -C ../zed-musa diff --check
find examples -name '*.musa' -print0 | xargs -0 -n1 cargo run -q -p musa -- check
```

Commit each affected repository intentionally and record cross-repository commit ids. Commit Musa as
`Teach the sound language to explain itself`.

## Stop

- No handwritten duplicate processor/format tables, second compiler in an editor, or unsupported-feature marketing.
- No long quotation from OMT or external format specifications; cite and state Musa's own rule.
- No plug-in hosting, waveform editing, or feature added only to complete a documentation example.
