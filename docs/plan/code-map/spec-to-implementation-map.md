# What is implemented today

This table distinguishes proved design targets from current Rust code. A green test suite cannot implement a row whose
data type does not yet exist. Feature names use the vocabulary installed by prompt 127a; the Rust identifiers still
carry their pre-127a spellings, and the pairs are in [`../clean-break-ledger.md`](../clean-break-ledger.md).

| Feature | Owner | Current state | Next evidence |
| --- | --- | --- | --- |
| Total non-recursive source expressions | `musa-compiler` | implemented for the current value types and folds | whole-language conformance tests |
| Definitional equality over the dependent core, decided by normalization by evaluation | `musa-core` | implemented for universes, Π, dependent records with η, `Id`/`refl`/`J`, and `let`, under the §4 budget | prompt 135 adds families and the termination checker; prompt 148 owes the metatheory matrix |
| Bidirectional elaboration: check/infer over a surface-independent raw term, contextual metavariables, pattern-fragment unification with postponement, implicit-argument insertion | `musa-core` | implemented; the elaborated term is re-checked by an independent checker (`well_typed`) as the suite's primary gate. No level metavariables — an unannotated binder's type meta is created at `Type 0`. Introduction forms check only, so a record literal has no inference rule and an elimination applied directly to one is outside the re-checker's reach | prompt 135 adds level metavariables with level-polymorphic families; nothing calls this yet — prompt 142 cuts the compiler over |
| Per-term provenance: an origin on every core term, preserved by evaluation and quotation and invisible to conversion | `musa-core` | implemented as an opaque `Origin` on `Term`, on values, and on context assumptions; carried through elaboration and named by both sides of a conversion mismatch | prompt 138 relates it to the compiler's `Derived` graph |
| User-defined nominal data, private constructors, and abstract type members | `musa-compiler` | absent; research candidate still incomplete after review 54 | repair active package-version selection, then review again |
| Building and closing a fragment into a core term | compiler to kernel | implemented, but over the deleted contextual `music` type | prompt 142 rebuilds it over ordinary values; differential and closure tests |
| Finite `EventTrack<C,A>` operations, including unequal-duration `together` | `musa-kernel` | implemented at the old names and without the coordinate index | prompts 127b–127c; current 62 kernel tests and final conformance audit |
| Versioned exact bytes for event-track equality | `musa-kernel` | implemented; the coordinate tag is not yet in the encoding | prompt 127c, then a migration test when a persisted reader is added |
| `Machine<K,A,B>` as a core value of the source language | `musa-compiler`/`musa-audio` | absent | prompts 150–152 |
| `schedule(format, policy, time map, track)` with a recorded decision list | `musa-audio` | absent | prompt 152 |
| Gesture event track | `musa-compiler` | absent | prompt 156 and its admission tests |
| Engraving plan and current exports | `musa-render` | implemented for current score facts | language graduation matrix |
| Analysis packages with their own hidden value types and evidence | `musa-compiler` | partial built-in analyses; no general package mechanism | accepted source type design and real package examples |
| Valid whole-machine step order | `musa-audio` | partial and not conforming to the new ordering rule | known ordering counterexample, machine-law tests |
| Feedback through initialized one-step state | `musa-audio` | current behavior depends on caller buffer size | one-frame step, `batch` contract, and partition tests (R1-batch) |
| Complete `prepare_execution` operation returning a `PreparedMachine` | `musa-audio` | absent in the specified form | instrument and preparation prompts |
| Cache that confirms complete audio arguments after hash lookup | audio/project | absent | exact `ExecArgs` record and forced-collision test |
| Versioned registry of source and derived representations | `musa-project` | absent | exact descriptor and merge validation |
| Complete origin paths with loss records | compiler/render/audio/project | partial source provenance only | generated-event and multi-pass path prototype |
| Structured editing that changes source | project/interface | architectural boundary implemented; feature set incomplete | interface and language graduation audits |

## Recommended implementation order

1. Carry out the clean break of prompts 127b–127d, 142, and 150–153: the event-track rename and coordinate index, the
   deletion of the contextual `music` type, machines as core values, and `schedule`.
2. Finish or reject the small source-language design for theory-owned data. Do not implement it while stable package
   selection remains undefined.
3. Add the gesture event track using the now-implemented payload admission rule.
4. Implement audio preparation with the complete argument list.
5. Replace caller-buffer-based feedback with the whole-machine step order and the one-frame step.
6. Add the versioned representation registry and complete origin paths.
7. Run the repaired audio and whole-language conformance prompts before public release.

None of these steps requires call-by-push-value, dependent track types, first-class “worlds,” or one common musical data
model.
