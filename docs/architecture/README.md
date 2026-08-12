# How the Rust workspace implements the specification

The formal specification says what Musa’s stages mean. These pages say which crate implements each stage, which details
remain private, how data is stored, and where validation occurs.

Read them in this order:

1. [stage-pipeline.md](stage-pipeline.md) maps compiler stages to crates and public APIs.
2. [identity-and-storage.md](identity-and-storage.md) explains exact encodings, hashes, caches, and saved origin data.
3. [process-runtime.md](process-runtime.md) explains how `musa-audio` prepares a graph for the real-time engine.
4. [spec-to-implementation-map.md](spec-to-implementation-map.md) marks each planned feature as implemented, partial, or
   absent.

`docs/roadmap.md` remains the broad roadmap. These pages give more precise boundaries where the later
cross-stage specification changed or clarified that roadmap.
