---
id: 176
slug: studio-vocabulary
status: done
depends_on: [122, 174b, 175]
phase: 3
---

# One Discoverable Source Studio Vocabulary

> **Reopened by note 79.** The first execution centralized public studio vocabulary in a Rust `musa-dsp` catalogue. The
> primitive registry remains host-owned; the musician-facing declarations and documentation move to ordinary Musa
> source.

## Task

Make every standard studio processor wrapper, parameter, control type, unit, and routing term discoverable from one
edition-pinned standard-library declaration tree. A musician encountering `oscillator`, `envelope`, `resonance`, `bus`,
or `send` gets a plain first sentence, typed signature, units/defaults/range, primitive origin where applicable, and a
short example. Compiler, LSP, desktop, and generated reference material index those declarations rather than a parallel
Rust surface catalogue.

## Read

- `docs/rules/language/{00-semantics,04-templates-and-modules,08-performance-and-sound}.md`, especially §0; note 79.
- Repaired 174b/175, note 82, and `stdlib/src/sound/{mod,graph}.musa`; the source reference generator and declaration
  index.
- `musa-dsp`'s primitive registry and private graph descriptors: identity, port/state formats, DSP ranges, smoothing,
  combination policy, and resource contracts are the host half and must not be copied into source as private state.
- Prompt 164/note 61's per-entry builtin survey; Peyton Jones chapter 3 and Ousterhout chapters 7–8.
- Note 77's `q`→`resonance` migration. Its spelling decision remains; only the claimed Rust ownership changes.

## Design

`std::sound` owns source-facing names, data types, exact written domains/defaults, docs, examples, and processor
constructor contracts. Those contracts use prompt 175's source quantity declarations. This prompt makes them
discoverable and joins primitive-backed declarations to host-owned registrations; prompt 178 supplies the executable
source wrappers that construct machine values and the private instrument bodies that use them. A later wrapper's private
implementation may name a stable primitive registration, but the primitive's state, exact runtime formats, bounds, and
step stay in `musa-dsp`.

Join a primitive-backed source declaration to its primitive registration by stable id/version and check the facts that
cross the boundary: ports, configuration inputs, and declared resource premises. Units, written defaults/ranges, names,
ordering, and documentation are source facts, not duplicated registry columns. The source declaration is authoritative
for what an author writes and what tooling presents; the registration is authoritative for what the host can prepare and
step. Neither side silently supplies facts owned by the other. The agreement check established here is reused by prompt
178 when an executable wrapper constructs a machine value.

LSP completion/hover on invalid or half-typed source uses the standard-library source index plus CST context. Imported
declarations navigate to their source. A primitive-backed wrapper reports both its defining source and registered
primitive identity; it is not labelled as an uninspectable language builtin.

Keep `resonance` as the only accepted filter spelling. `q` remains a hard error with the exact source fix. Generate
reference pages and UI labels from source declarations and checked primitive support facts. Add an exhaustive law that
every parser-recognized standard sound name resolves to one declaration, and every primitive wrapper names one matching
registration.

## Target

- Edition-pinned `std::sound` declaration tree containing the public vocabulary, docs, examples, exact schemas, and
  processor constructor contracts over prompt 175's quantities.
- Primitive-registration/source-declaration agreement checks without exposing private state.
- LSP hover/signature/completion and desktop/reference facts generated from source indexes.
- Hard-error `q` diagnostic/fix and canonical `resonance` corpus.
- Deletion of the authoritative Rust surface catalogue; any remaining registry table documents only host-owned facts.

## Check

```sh
cargo nextest run -p musa-calculus -p musa-compiler -p musa-dsp -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-calculus -p musa-compiler -p musa-dsp -p musa-project -p musa-lsp -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
rg -n "resonance|quality factor|registered primitive" stdlib/src/sound docs/rules/language crates/musa-lsp apps/musa-desktop
```

Commit as `Make the source studio vocabulary discoverable`.

## Stop

- No new processor merely to make the declaration tree look complete.
- No public DSP registry, dynamic native plug-in API, public compiler HIR, or source access to primitive state.
- No executable primitive wrapper or private instrument machine body; prompt 178 owns both.
- No score-driven controls or part routing; prompts 177–180 own those semantics.
- No handwritten Rust duplicate of a declaration merely to make tooling convenient.
