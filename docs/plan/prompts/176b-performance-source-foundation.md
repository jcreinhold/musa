---
id: 176b
slug: performance-source-foundation
status: done
depends_on: [176, 176a]
phase: 3
---

# Make Source-Owned Performance Expressible

## Task

Before prompt 177 replaces the Rust gesture oracle, make the governing `std::performance` module path and its dependent
data expressible through the ordinary module system and checker. Declare the exact finite performance vocabulary and
neutral interpretation policy in source, prove its control indices use only the general Miller-pattern unifier, and
expose one generic checked-artifact request. This prompt establishes the language-owned input/output contract; prompt
177 moves the production track bridge onto it.

## Read

- `docs/rules/language/{00-semantics,02-core-calculus,04-templates-and-modules,08-performance-and-sound}.md`, prompt
  176a's payload admission rule, and note 79.
- OMT 007 for the separation of notation from realization and OMT 114 for why a crescendo is not a gain operation.
- Peyton Jones chapters 3–6 for source data/pattern translation and Ousterhout chapters 7–8 for pushing reusable
  complexity below callers.
- The package tree, qualified-path parser, checked-source bridge, `Progress`, and the existing general unification laws.

## Design

`performance` remains reserved where it starts the legacy declaration block, but may name a module in a qualified path.
A directory module is itself importable from its `mod.musa`, as well as providing the namespace of its children; source
must not need a dummy leaf facade to spell `import std::performance`.

Declare `ControlKind`, `ControlKey<K>`, `ControlValue<K>`, existential `SomeControl`, standard control values, gesture
identity, note/control-curve/phrase/release gesture data, symbolic techniques, a complete public notation view, profile
rules/results, and the neutral policy in `stdlib/src/performance/`. Occurrence spans will own time in prompt 177, so no
gesture payload carries an absolute coordinate. Provenance remains a separate host projection.

The helper that packages a heterogeneous control has an omitted implicit `K`. Its two indexed arguments determine that
metavariable through the existing Miller-pattern rule. Add focused source laws for successful inference, incompatible
indices, and an unresolved index; retain the calculus suite's postponement, flex-flex, and escaping-scope laws as the
mechanism evidence. Do not add a performance-specific unifier, kind table, coercion, or default.

The neutral source function owns exact dynamic/articulation intent and the affine hairpin law. It emits abstract
`expression`, `emphasis`, `separation`, and `sustain`, plus symbolic technique/group data—not gates, seconds, gain,
velocity, or DSP addresses. A checked vocabulary artifact is the only host request added here; there is no Rust mirror
or decoder yet.

## Target

- Importable `std::performance` backed by `stdlib/src/performance/mod.musa`.
- Indexed source controls, gesture/profile data, neutral policy, and exact hairpin function.
- Generic checked-source vocabulary request and focused pattern-unification laws.
- Package/parser laws for keyword module segments and importable directory modules.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-calculus -p musa-compiler
cargo clippy --all-targets -p musa-syntax -p musa-calculus -p musa-compiler -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
cargo insta test --workspace --unreferenced=reject
rg -n "data Gesture|data ControlKey|some_control|let neutral" stdlib/src/performance
```

Commit as `Make source performance declarations checkable`.

## Stop

- No production gesture lowering, instrument implementation, routing, DSP mapping, or frame scheduling.
- No host interpretation of dynamics, marks, grouping, or techniques.
- No closure in an artifact and no Rust enum mirroring a source family.
- No special unifier, omitted-index default, or dynamically tagged substitute for the dependent relation.
