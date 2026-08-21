# How the Rust workspace implements the specification

**Status: descriptive.** These pages report on code; they never decide semantics. Where one disagrees with
`../../rules/across-stages/` or `../../rules/constitution.md`, the code is wrong or this page is stale — see
[`../../README.md`](../../README.md).

The formal specification says what Musa’s stages mean. These pages say which crate implements each stage, which details
remain private, how data is stored, and where validation occurs.

**Vocabulary note (prompt 127a).** These pages now use the governing names — event track, `follow`, `together`,
`map_payloads`, duration, machine, registered primitive, step, `schedule`, `PreparedMachine`. The Rust identifiers in
the workspace still carry their pre-127a spellings until prompts 127b–127d, 142, and 150–153 land, and every pair is
listed in [`../clean-break-ledger.md`](../clean-break-ledger.md). Where a page quotes literal current output or a
fixture, it says so.

Read them in this order:

1. [stage-pipeline.md](stage-pipeline.md) maps compiler stages to crates and public APIs.
2. [identity-and-storage.md](identity-and-storage.md) explains exact encodings, hashes, caches, and saved origin data.
3. [process-runtime.md](process-runtime.md) explains how `musa-dsp` prepares a machine for the real-time engine.
4. [spec-to-implementation-map.md](spec-to-implementation-map.md) marks each planned feature as implemented, partial, or
   absent.
5. [implementor-reference.md](implementor-reference.md) is the orientation for someone changing the compiler: grammar to
   kernel, the laws each stage owes, and the recipe for adding a domain, an analysis kind, or an assertion.

`docs/plan/roadmap.md` remains the broad roadmap. These pages give more precise boundaries where the later cross-stage
specification changed or clarified that roadmap.
