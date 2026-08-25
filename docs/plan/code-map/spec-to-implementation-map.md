# What is implemented today

**Status: descriptive.** Governing decisions live in `docs/rules/`; this page maps the language-pass specification to
the code that implements it. `implemented` means the public compiler path reaches the mechanism and its laws exist.
`partial` and `pending` name a real remaining boundary rather than an anticipated API.

## Language-pass ledger

| Capability | State | Owner and executable evidence |
| --- | --- | --- |
| Lossless surface syntax, declarations, records, data, modules, visibility, patterns, operators, and typed quotation | implemented | `musa-syntax`; parser/formatter/tree-sitter laws |
| Dependent terms, Π types, universes, records with η, lets, and registered base types | implemented | `musa-calculus::kernel`; conversion and rechecker laws |
| Bidirectional elaboration and implicit retained indices | implemented | `musa-calculus::elaboration`; bidirectional, numeral, and index laws |
| Scoped metavariables and Miller-pattern unification | implemented | `musa-calculus::elaboration::unify`; scope, permutation, weakening, flex-flex, occurs, and postponement laws |
| Normalization by evaluation and definitional equality | implemented | `musa-calculus::kernel`; quotation/evaluation/conversion laws |
| Indexed inductive families, constructors, structural recursion, and case trees | implemented | `musa-calculus`; family, constructor, recursor, coverage, and termination laws |
| Records, nominal data, private constructors, modules, and inherent namespace methods | implemented | `musa-calculus`; program/module/privacy/method laws |
| Structural `Storable` admission and canonical finite payloads | implemented | `musa-calculus` plus compiler registrations; storage and payload-boundary laws |
| Deterministic work, nesting, constructed-node, and logical-byte limits | implemented | `musa-calculus::Budget`; exact-boundary and corpus-budget laws |
| Host registrations for source base types, constructors, and δ-rules | implemented | `musa-compiler::registry` and `prelude`; ownership and conformance laws |
| CST-to-raw lowering, whole-document elaboration, and independent kernel recheck | implemented | `musa-compiler::{lower,document,elaborate}`; prompt-149 trusted boundary |
| Imports, real module tree, `private`, and generated reference documentation | implemented | `musa-compiler::{imports,package,reference}` and `stdlib/` |
| Typed syntax values, provenance-preserving quotation, anchors, splices, and adapter expansion | implemented | `musa-compiler::{quote,phase,expand}`; adapter and origin laws |
| Compiler diagnostics with causes across documents | implemented | compiler → project → LSP/desktop; rendered diagnostic laws |
| Collections needed by committed programs | implemented | surface list forms, core lists, and `std::list`; collection/corpus laws |
| Finite coordinate-indexed event tracks and exact versioned encoding | implemented | `musa-events`; algebra, normalization, encoding, and hash laws |
| Staff and studio adapters on one frozen interface | implemented | `stdlib/src/adapters/{staff,graph}.musa`; adapter law suites |
| Source-owned sound declarations versus host-owned runtime boundaries | pending | notes 79–80; checked bridge at 174b, parity at 175–180, cutover at 180a, audits through 193 |
| Tonal and post-tonal packages over indexed families | implemented | `stdlib/src/{tonal,post_tonal}`; generic-row and corpus laws |
| Source-to-adapter-to-event-track provenance | implemented | compiler derivation records and prompt-169 K1–K20 matrix |
| Provenance composition through scheduling and audio | implemented | provisional exact gesture projections retain complete origins; opaque occurrence handles reach audio; note 77 R14 audits derivation reuse and associative stage composition; prompt 177 replaces the provisional payload vocabulary with source declarations |
| Exact machine reference step and primitive registry | implemented | `musa-dsp/src/{machine,primitive,plan}.rs`; structural and native one-frame registries |
| Checked event-track scheduling into frame sources | implemented | `musa-dsp::schedule`; exact-map, decision, collapse, merge, countdown, seek, and bound laws |
| One-frame DSP meaning and opaque prepared machine | implemented | exact gestures → checked `Schedule` → `PreparedAudio`; live/offline repeated-step partition and RT laws |

## Trusted boundary

`musa-calculus` owns the only implementation of core evaluation, conversion, and checking. Elaboration may create
metavariables and solve them, but an accepted term crosses the facade only after the independent kernel rechecker has
checked it from scratch. `Value`, environments, neutral terms, evaluator control frames, quotation, and unification are
private. `crates/musa-calculus/TRUST.md` names the trusted portion precisely; prompt 149's rechecker laws and prompt
169's K1–K20 matrix are the executable evidence.

The compiler is a host, not a second calculus. It supplies finite base values, type constructors, and registered δ-rules
through `registry` and `prelude`; reads the lossless CST into `Raw`; asks the calculus to elaborate and recheck; then
realizes the checked result into event-track and score values. The phase-local `infer.rs` machinery checks legacy studio
descriptors and does not define source-language typing.

## Elaboration and unification

Elaboration is bidirectional: introduction forms check against an expected type, eliminations synthesize, and conversion
compares NbE values. Omitted arguments create scoped contextual metavariables. A metavariable occurrence may be solved
only when its spine is a list of distinct local variables (a Miller pattern). Inversion maps those stable de Bruijn
levels back into the metavariable's declaration scope; permutations and weakening are preserved, duplicate, foreign,
omitted, escaping, and cyclic spines are refused as solutions. Blocked comparisons are postponed and retried after
progress; unresolved survivors become a deterministic diagnostic. No branch guesses a higher-order solution.

This is the governing pattern-unification discipline in `docs/rules/across-stages/01-stage-judgments.md`, implemented in
`musa-calculus::elaboration::unify`. The identity-spine hot path allocates nothing; short duplicate checks stay inline,
and larger spines use a set, so the discipline does not turn ordinary elaboration quadratic.

## Definitions, data, and modules

Top-level groups are collected before bodies are checked, giving recursive declarations stable identities. Functions are
total: recursion is accepted only through generated structural case trees, with coverage and decreasing calls checked
before evaluation. Indexed constructors refine their result indices; numeral syntax elaborates at the expected counting
family rather than through an untyped integer detour.

Records elaborate to dependent field telescopes and support η. Nominal `data` declarations generate constructors and
recursors; a `private` constructor remains usable inside its defining module but cannot be named or matched outside it.
Modules form a real lexical tree. `import` loads a module; `use` selects names from one already loaded. Ordinary
definitions in a type's namespace provide inherent methods such as `equal`; the deleted trait/instance experiment and
its implicit dictionaries are not a surviving semantic path.

## Storage and finite work

`Storable` is generated, not author-implemented. Its structural check rejects a source function at every depth and is
rechecked where a value crosses an event-payload boundary. Canonical encoders separately produce versioned finite bytes;
storage admission and encoding are related checks, not aliases for one another.

One deterministic meter covers reduction steps, nesting, constructed value nodes, and logical bytes. The published
language budget is 2,000,000 steps, 1,000,000 nodes, and 16 MiB. Charges occur before work or construction, shared
values are not recharged merely because they are named or selected, and no partial declaration graph is published after
an exhaustion. The largest retained measurements and their derivation are in note 64.

## Typed quotation and adapters

The parser produces a lossless token tree. `musa-compiler::quote` exposes it to a phase as indexed `Syntax<TokenTree>`
and requires an adapter to return one checked `Syntax<Expr>`. Anchors, binders, references, built nodes, token kinds,
and delimiters are typed operations over that value; an adapter cannot reach inferred source types or compiler-private
ASTs. Quotation constructs source syntax, splices checked fragments, and preserves source and generated provenance.

`stdlib/src/adapters/staff.musa` and `stdlib/src/adapters/graph.musa` are two materially different clients of the same
interface. Both use the sealed bottom-up traversal, ordinary finite folds, records for state, and quote/splice for
construction. Neither reparses token spelling after the reader has already classified it, allocates syntax roles by
hand, or calls a compiler-only convenience path. Note 67 freezes this boundary; prompt 169's audit found no second
adapter or evaluator path.

## Musical domains and the registry

Compiler registrations remain only where an operation needs source provenance, direct core construction, a registered
primitive's private state, or a private finite representation/work bound. Prompt 164 removed 22 modulus-specific
operations: modular arithmetic, packed post-tonal representations, and finite searches now live in ordinary Musa over
indexed `Pc(n)`, `PcSet(n)`, and `ToneRow(n)`. The two spelling bridges remain registered because written spelling is
private `musa-score` information, not arithmetic modulo twelve. Note 61 records every registry entry and the measured
cost.

The standard library is a version-matched Musa package with a real module tree. `std::context` exports a `TonalContext`
record and the `c_major` and `a_natural_minor` values; tonal and post-tonal packages use the same language as a piece.
There is no privileged library evaluator.

## Event-track and runtime boundary

The checked written result elaborates to `EventTrack<WrittenTime, ScoreFact>`. `musa-events` owns exact positions and
durations, typed occurrences, `empty`, `event`, `follow`, `together`, `map_payloads`, `duration`, queries,
normalization, exact versioned encoding, and semantic hash. It is a leaf: syntax, musical domains, machines, and audio
do not enter it.

The reference machine step and checked scheduling are implemented in `musa-dsp`. `machine.rs` prepares the closed
primitive registry and interprets every structural form one exact step at a time. `schedule.rs` converts an admitted
finite event track through an exact finite time map and explicit versioned policy into immutable frame batches, a full
decision record, and an allocation-free cursor/countdown source. Its bounds cover map entries, occurrences, messages,
batches, and frame representation; its merger injects opaque handles into recursive disjoint namespaces before sorting.

Production audio lowers directly to exact `EventTrack<PerformedTime, Gesture>` lanes, checks them through `Schedule`,
then prepares registered one-frame native instruments. Prompt 174 deleted the legacy frame-scheduled performance value;
MIDI and debug consumers now read the same exact `GesturePlan` and choose no audio-frame lattice. Note 77 records the
complete derivation audit. The graph flattening is private and has no caller-defined block width or public compilation
API.
