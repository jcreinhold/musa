# How the Rust workspace implements the specification

**Status: descriptive.** These pages report on code; they never decide semantics. Where one disagrees with
`../../rules/across-stages/` or `../../rules/constitution.md`, the code is wrong or this page is stale — see
[`../../README.md`](../../README.md).

The formal specification says what Musa’s stages mean. These pages say which crate implements each stage, which details
remain private, how data is stored, and where validation occurs.

**Vocabulary note.** The event-track and dependent-language names are current. Machine, registered primitive, exact
step, `schedule`, and `PreparedMachine` name the governing runtime target; the current DSP graph names remain only until
prompts 171–173 migrate their callers. Every outstanding pair is explicitly reassigned in the
[`clean-break ledger`](../clean-break-ledger.md).

Read them in this order:

1. [stage-pipeline.md](stage-pipeline.md) maps compiler stages to crates and public APIs.
2. [identity-and-storage.md](identity-and-storage.md) explains exact encodings, hashes, caches, and saved origin data.
3. [process-runtime.md](process-runtime.md) explains how `musa-dsp` prepares a machine for the real-time engine.
4. [spec-to-implementation-map.md](spec-to-implementation-map.md) maps every language-pass capability to its current
   owner and executable evidence, then names the remaining runtime boundary.
5. [implementor-reference.md](implementor-reference.md) is the orientation for someone changing the compiler: grammar to
   event track, the laws each stage owes, and the recipe for adding a domain, an analysis kind, or an assertion.

`docs/plan/roadmap.md` remains the broad roadmap. These pages give more precise boundaries where the later cross-stage
specification changed or clarified that roadmap.
