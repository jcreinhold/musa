# 79. Source owns the sound language

**Status: governs nothing.** This record preserves the repair that reopens prompts 174b–176 and redirects the pending
sound prompts before their Rust projections become a second language.

## The defect

Prompt 167 proved that `stdlib/src/sound/graph.musa` can declare and validate a finite studio description as ordinary
Musa data. Prompt 164 had already established the ownership test: a host operation is justified only by source-aware
provenance, direct core construction, registered primitive state, or a private finite representation/work budget. Yet
prompt 174b asked only which Rust crate should own the legacy `StudioSpec`; prompt 176 then made `musa-dsp`'s Rust table
the authoritative surface catalogue; and prompt 175 made a public Rust `WrittenQuantity` part of that vocabulary.

The same drift appeared while preparing prompt 177: fixed Rust enums for gestures, controls, connections, techniques,
and phrase groups duplicated declarations the completed source language can express. That draft was not committed.

This is not merely a crate-layering error. It creates two definitions of the sound language: ordinary `.musa`
declarations on one side and a closed Rust ontology on the other. The two can disagree, and every new control,
instrument, unit, or studio term would require a compiler release even when it needs no private host capability.

## Evidence

- `docs/rules/language/00-semantics.md` says compiler ownership is an information boundary and puts every operation
  expressible through public values in ordinary source.
- Constitution §9 refuses built-in instruments and cultural theories. Standard instruments and performance policy are
  library declarations, not constructors of the host implementation language.
- `stdlib/src/sound/graph.musa` already declares `PortKind`, exact parameter values, studio declarations, validation
  errors, and the validating functions in Musa. Note 66 measured that source program and identified the missing bridge
  to the legacy Rust path.
- Prompt 164 and note 61 moved operations from the builtin registry once data, recursion, privacy, and finite folds made
  them expressible.
- Peyton Jones, *The Implementation of Functional Programming Languages*, chapter 3, separates a rich source language
  from the smaller implementation substrate by semantics-preserving translation. It does not make every source
  constructor a target-language primitive.
- Ousterhout, *A Philosophy of Software Design*, chapters 7–8, warns that adjacent layers carrying the same abstraction
  are shallow duplication. A Rust `Gesture` ontology beside a Musa `Gesture` ontology is exactly that duplication.

## Replacement rule

The source language owns every declarable sound value and every total policy over those values. The bundled standard
library therefore owns gesture and control declarations, performance profiles, instrument signatures and mappings, exact
units and quantities, studio descriptions, sample maps, standard instruments, and presets.

The host owns only boundaries that source cannot implement:

- provenance-preserving construction and traversal of opaque event tracks;
- registered primitive identity, private state, exact port/state formats, and resource contracts;
- verified asset bytes and decoder state;
- scheduling, frame quantization, compact resolved indices, DSP conversion, and real-time execution; and
- caller-oriented projections whose exact equivalence to checked source data is tested.

A Rust projection may be optimized or shaped for a consumer, but it is never independently constructible semantic
authority. Its schema/version follows the source declaration, every field has a source derivation, and differential laws
compare it with the checked source value. Tooling reads standard-library declarations and source indexes; it reads the
primitive registry only for facts that genuinely belong to a registered primitive.

Dependent relationships are stated in source indices. In particular a control key's value kind is an index shared by the
key and value. Omitted indices are solved only by the existing Miller-pattern unifier: distinct local-variable spines,
occurs/scope checks, postponement, and refusal of unresolved constraints. Sound elaboration gains no special inference
table, fallback, or host-side guess.

## Examples affected

No accepted `.musa` example is intentionally removed. The existing staff and studio adapter fixtures continue to use
ordinary declarations. What stops being a supported public construction path is Rust code that manually builds
`StudioSpec`, `WrittenQuantity`, fixed gesture/control enums, or an instrument schema without a checked source value.
Repository callers migrate to opaque checked artifacts and project facts derived from source.

The `q` to `resonance` source migration remains: it is a spelling decision in the standard-library declaration and its
source fix, not evidence that Rust owns the public catalogue.

## Dependency-cone repair

- Reopen 174b: establish the generic checked canonical-data bridge and prove it on prompt 167's deliberately small
  `StudioDescription` value without claiming premature production parity.
- Reorder 174c after the source cutover, then restate the crate-role check without calling `musa-dsp` the owner of
  editable source vocabulary.
- Reopen 175: retain exact arithmetic and the one float boundary, but make exact quantity a source declaration.
- Reopen 176: split source declarations/documentation from the private primitive registry.
- Repair 177–181: gestures, indexed controls, profiles, signatures, mappings, instruments, and defaults are ordinary
  source; host code performs only the owned bridges above.
- Add 180a: after 175–180 supply production parity, cut the legacy Rust `StudioSpec` path over to checked source and an
  opaque DSP preparation projection, then delete the compiler-to-DSP edge.
- Repair 184–186: native/SFZ/SoundFont adapters produce the source-declared sample-map/instrument contract before
  private preparation.
- Repair 189–193: editor facts and documentation derive from source declarations, and conformance mechanically rejects
  authoritative Rust mirrors.

Prompts outside that cone still inherit the ownership rule from the prompt README.

Note 80 records why the bridge and cutover cannot be the same early prompt: the prompt-167 trial does not yet express
the production path's patches, buses, sends, modulation, or full processor vocabulary.

## Migration and identity

Source files remain canonical. Existing Rust `StudioSpec` and provisional gesture values are not released stored formats
and acquire no compatibility status. During the cutover they may serve only as differential oracles. Stored identity is
recomputed from the checked, versioned source value and complete preparation arguments. A digest may locate a candidate
but never substitutes for exact source-value equality.

No private primitive identity, machine state, decoded asset, frame schedule, or callback object moves into source.
