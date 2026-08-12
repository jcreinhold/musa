# Implementation architecture

These documents explain how the Rust workspace is intended to realize `docs/spec/`. The specification owns semantics;
architecture owns crate boundaries, private representations, validation points, storage, and runtime strategy. If an
architecture shortcut changes a specified judgment, the shortcut is wrong or the specification needs a deliberate
amendment.

Start with:

1. [stage-pipeline.md](stage-pipeline.md) — presentations, passes, and crate ownership;
2. [identity-and-storage.md](identity-and-storage.md) — canonical data, caches, and artifact registries;
3. [process-runtime.md](process-runtime.md) — private process IR and audio execution; and
4. [spec-to-implementation-map.md](spec-to-implementation-map.md) — implemented versus missing target rows.

`docs/initial-design-roadmap.md` remains the broad implementation roadmap. These documents refine the cross-stage
architecture where the later formal specification is more precise.
