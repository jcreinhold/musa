# What is implemented today

This table distinguishes proved design targets from current Rust code. A green test suite cannot implement a row whose
data type does not yet exist.

| Feature | Owner | Current state | Next evidence |
| --- | --- | --- | --- |
| Total non-recursive source expressions | `musa-compiler` | implemented for the current value types and folds | whole-language conformance tests |
| User-defined nominal data, private constructors, and abstract type members | `musa-compiler` | absent; research candidate still incomplete after review 54 | repair active package-version selection and `Music` stability, then review again |
| Closing contextual music into kernel terms | compiler to kernel | implemented | existing differential and closure tests |
| Finite `Timeline<A>` operations, including unequal-length overlay | `musa-kernel` | implemented | current 62 kernel tests and final conformance audit |
| Versioned exact bytes for timeline equality | `musa-kernel` | implemented | migration test when a persisted reader is added |
| Gesture timeline payload | `musa-compiler` | absent | prompts 130 and its admission tests |
| Engraving plan and current exports | `musa-render` | implemented for current score facts | language graduation matrix |
| Analysis packages with their own hidden value types and evidence | `musa-compiler` | partial built-in analyses; no general package mechanism | accepted source type design and real package examples |
| Valid whole-node audio process graph | `musa-audio` | partial and not conforming to the new schedule rule | private graph, known schedule counterexample, process-law tests |
| Feedback through a fixed stored delay | `musa-audio` | current behavior depends on caller buffer size | fixed semantic step and partition tests |
| Complete `prepare_execution` operation | `musa-audio` | absent in the specified form | instrument and preparation prompts |
| Cache that confirms complete audio arguments after hash lookup | audio/project | absent | exact `ExecArgs` record and forced-collision test |
| Versioned registry of source and derived representations | `musa-project` | absent | exact descriptor and merge validation |
| Complete origin paths with loss records | compiler/render/audio/project | partial source provenance only | generated-event and multi-pass path prototype |
| Structured editing that changes source | project/interface | architectural boundary implemented; feature set incomplete | interface and language graduation audits |

## Recommended implementation order

1. Finish or reject the small source-language design for theory-owned data. Do not implement it while stable package
   selection remains undefined.
2. Add the gesture timeline using the now-implemented payload admission rule.
3. Implement audio preparation with the complete argument list.
4. Replace caller-buffer-based graph feedback with the private whole-node graph and fixed semantic step.
5. Add the versioned representation registry and complete origin paths.
6. Run the repaired audio and whole-language conformance prompts before public release.

None of these steps requires call-by-push-value, dependent timeline types, first-class “worlds,” or one common musical
data model.
