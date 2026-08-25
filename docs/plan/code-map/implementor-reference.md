# Implementor's reference

**Status: descriptive.** This is orientation for someone changing the compiler. Governing decisions live in
`docs/rules/`; the code map says where their implementation lives.

## Boundaries

```text
musa-syntax ──┐
              ├──► musa-compiler ──► {musa-notation, musa-dsp} ──► musa-playback ──► musa-project
musa-calculus ┤          ▲
musa-events ──┘     musa-score
```

`musa-calculus` and `musa-events` are leaves. `musa-lsp` also reads `musa-syntax`, because highlighting and completion
must work on half-typed text. The public seams are deliberately narrow:

| Crate | Owns | Keeps private |
| --- | --- | --- |
| `musa-syntax` | lossless tokens/CST, parser, formatter, text edits | Rowan |
| `musa-calculus` | dependent terms, NbE, conversion, bidirectional elaboration, recheck | values, environments, evaluator, quotation, unifier |
| `musa-events` | coordinate-indexed exact finite tracks and laws | representation and normalization internals |
| `musa-score` | musical values, snapshots, provenance, diagnostics, analysis | no parsing or pass machinery exists here |
| `musa-compiler` | imports/names, host registrations, lowering, quotation/adapters, realization | pass types, resolution tables, calculus internals |
| `musa-notation` | notation plan and exports | planning internals |
| `musa-dsp` | studio validation, machine/scheduling target, offline rendering | primitive state, buffers, schedules |
| `musa-playback` | device negotiation, transport, callback | CPAL and callback internals |
| `musa-project` | documents, revisions, commands, derived-result coordination | compiler internals |

No public item exists without a caller. Source text is canonical; there is no second editable AST. Musical time remains
exact rational until the performance/DSP edge.

## Source to checked core

1. `musa-syntax` lexes and parses every byte into a lossless CST.
2. `musa-compiler::imports` and `resolve` load the real module tree and assign names; `lower` reads the CST as
   `musa_calculus::RawProgram` without inventing another typed IR.
3. `musa-calculus` elaborates bidirectionally. Introduction forms check, eliminations synthesize, and NbE conversion
   compares expected and inferred values.
4. Omitted arguments create scoped contextual metavariables. The unifier solves only distinct-local-variable Miller
   spines, preserving permutation and weakening by stable de Bruijn levels; it postpones blocked equations, retries
   after progress, and refuses unresolved survivors. It never guesses a higher-order solution.
5. The prompt-149 kernel rechecker checks the completed term independently. Elaboration is not trusted to certify its
   own output; `crates/musa-calculus/TRUST.md` names the trusted half.
6. `musa-compiler::elaborate` realizes the checked term through registered finite base values and δ-rules, producing
   caller-ready facts and `EventTrack<WrittenTime, ScoreFact>` values with provenance.

The compiler's phase-local studio descriptor inference is not source-language typing. Do not add a source construct to
`infer.rs`, reimplement conversion in the compiler, or expose a calculus `Value` to make a downstream pass convenient.

## Definitions, data, and names

Functions are ordinary dependent Π values. Source recursion is structural: generated case trees check coverage and
decreasing recursive calls. Indexed `data` declarations refine result indices; record telescopes may be dependent and
have η. Private constructors are visible only inside their defining module.

Modules are lexical and loaded explicitly. `import std::list;` loads a module; `use list::map;` brings a member into
scope. Ordinary namespace definitions provide inherent methods—`T.equal(a, b)` or its operator spelling—not implicit
trait dictionaries. The earlier signature/functor/template and source trait/instance designs were deleted; do not revive
their terminology in code or documentation.

Reusable music is an ordinary value. A fragment has type `EventTrack<WrittenTime, ScoreFact>` and a motif can be a
function returning one. Lexical context is explicit in a closure or parameter; saving a fragment does not arrange
dynamic capture.

## Storage and resource acceptance

`Storable` is a generated structural constraint, not a trait authors implement. It rejects source functions at every
depth and is checked again at payload boundaries. Canonical finite encoding is separate: it fixes exact versioned bytes
for equality and storage. Do not replace either with the other.

The language budget charges reduction steps, nesting, constructed nodes, and logical bytes before work occurs. Reusing a
named or projected value does not charge its contents again. Exhaustion is deterministic and publishes no partial
declaration graph. Any operation whose finite loop or allocation the evaluator cannot see needs an up-front registered
charge; wall-clock timeouts are cancellation, not language semantics.

## Typed quotation and adapters

An adapter receives indexed `Syntax<TokenTree>` and must return one checked `Syntax<Expr>`. `quote at anchor { … }` and
splices build syntax; typed token/delimiter operations classify what the parser already read. `recurse_syntax` is the
sealed bottom-up boundary, and ordinary folds process the finite children it returns. Adapter state is ordinary Musa
data. An adapter cannot inspect inferred types, compiler ASTs, or a hidden role allocation.

The two reference clients are `stdlib/src/adapters/staff.musa` and `stdlib/src/adapters/graph.musa`. Changes to the
adapter interface must preserve both suites and note 67's frozen theorem: one input type, one output type, no hidden
compiler input, complete provenance, and the same reader/checker as ordinary source.

## Event track and provenance

`musa-events` owns `empty`, `event`, `follow`, `together`, `map_payloads`, `duration`, exact-time operations, queries,
normalization, versioned encoding, and semantic hash. It knows neither syntax nor musical policy. A surface convenience
must elaborate to the fixed basis; it cannot quietly become another event-track operation.

Every created occurrence records its declaration/source anchor and derivation steps. Origins remain plural, foreign
spans retain their URI, and generated facts are navigable but not directly editable. `musa-project` attaches revisions
and keeps last-valid results; expanding an origin chain does not recompile.

## Runtime boundary

The governing runtime pipeline is checked track → schedule → prepared machine → one-frame step. Prompts 171–173 still
own that migration. Until then, current `StudioGraphSpec`, `compile_graph`, and `RenderPlan` are production APIs but are
not evidence that the machine calculus or one-frame rule is implemented. Do not delete them before callers migrate, and
do not describe their caller-block feedback or modulation as the governing semantics.

Runtime preparation consumes already rechecked finite language values. It may validate primitive descriptors, port
types, formats, capacities, memory, and work bounds; it must not evaluate source or add a second payload-admission rule.

## Extension recipes

**Base type or builtin.** Add a host registration only for source-aware provenance, direct core construction, registered
primitive state, or a private finite representation/work budget. Add the domain's meaning and falsifying example to
`docs/rules/language/03-musical-domains.md`. Familiarity and speed alone are not admission grounds.

**Standard-library operation.** Prefer ordinary Musa in `stdlib/src/`, with its public documentation and module entry.
The registry survey in note 61 is the precedent: if public data, recursion, privacy, and finite folds can express it, it
does not belong in Rust.

**Diagnostic.** Add a broken fixture and snapshot the complete rendered report. A diagnostic raised in an imported
adapter becomes a structured `Cause` with that document's spans, not concatenated prose.

**Surface construct.** Decide its elaboration in `docs/rules/events/06-surface-elaboration.md` first, then add parser,
formatter, tree-sitter, positive, negative, and corpus evidence. If it cannot elaborate to the fixed core, that is a
specification decision rather than permission to add a private shortcut.

**Runtime primitive.** Wait for prompt 171's registry unless working that prompt. A primitive fixes exact ids/versions,
port and state formats, deterministic initialization/step, and memory/work bounds; arbitrary closures and hidden input
are not primitives.
