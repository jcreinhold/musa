---
id: 180a
slug: source-studio-cutover
status: pending
depends_on: [174b, 175, 176, 177, 178, 179, 180]
phase: 3
---

# Cut Production Studio Semantics Over to Checked Source

## Task

Converge the temporary parallel paths after the source sound language reaches production parity. Make every accepted
studio, performance, instrument, route, and control form elaborate into the declarations and total functions of
`std::sound`/`std::performance`, then derive the private exact DSP preparation projection from that one checked value.
Delete the independently constructible Rust `StudioSpec` language and the `musa-compiler -> musa-dsp` dependency.

## Read

- `docs/rules/language/08-performance-and-sound.md` in full; notes
  [`79`](../../notes/research/language-design-closure/79-source-owns-the-sound-language.md) and
  [`80`](../../notes/research/language-design-closure/80-bridge-before-cutover.md).
- Prompts 174b and 175–180, their checked-artifact/projection laws, and the production-gap inventory 174b records.
- Every remaining `StudioSpec`, `WrittenQuantity`, `Processor`, `Unit`, patch/bus/send/route/modulation catalogue, and
  construction caller across compiler, DSP, project, LSP, desktop facts, tests, and examples.
- Prompt 167's adapter path and the ordinary surface lowerer: both must reach the same source declarations and checked
  value, not two host schemas.
- Prompt 164/note 61 and Peyton Jones chapter 3: a rich source construct lowers semantics-preservingly to the smaller
  implementation substrate. Ousterhout chapters 7–8: the migration oracle is deleted once the layers converge.

## Design

There is one semantic route:

```text
lossless source/CST
  -> ordinary std::sound/std::performance declarations and values
  -> shared elaboration, Miller-pattern unification, checking, zonking, evaluation
  -> checked canonical source artifact
  -> private exact preparation projection
  -> DSP preparation
```

Surface `studio` syntax may remain as an edition compatibility spelling until prompt 181, but it desugars into ordinary
source constructors before validation. It does not build a Rust graph. The graph adapter produces the same constructors
directly. Every validation/default/mapping decision is a source declaration or function; the host decoder only verifies
artifact schema and registered-primitive agreement.

The projection is opaque and nonconstructible except from the checked artifact. It carries complete exact source bytes,
schema/version, provenance/lineage references needed by callers, and resolved host-owned primitive identities only after
agreement checking. Source spelling and edit ranges remain in the CST/reference index, not in a mutable semantic AST. No
float, frame, processor state, or compact runtime index enters before its owned preparation boundary.

Move orchestration to the lowest existing layer that may name both siblings. `musa-project` may pass the compiler's
generic checked artifact to `musa-dsp`; `musa-compiler` itself no longer depends on or returns a DSP type. Shells that
compile without audio never instantiate a DSP projection.

Use the legacy Rust path only as a differential oracle while migrating. Exercise every row in 174b's gap inventory and
compare complete exact projections, diagnostics, identities, project facts, structured edits, and rendered fixtures.
Then delete the oracle, public constructors, catalogue remnants, and snapshots that serialize its debug shape. A digest
never substitutes for exact artifact equality.

Indexed controls, quantities, signatures, and mappings are already settled before this prompt. Their omitted arguments
must have been solved by the existing Miller-pattern unifier. The decoder sees zonked canonical data and contains no
sound-specific inference, default, coercion, retry, or fallback.

## Target

- One production semantic path through checked source declarations for studio, performance, instruments, routing, and
  controls, shared by ordinary surface and adapter-produced values.
- Exact opaque DSP preparation projection plus complete source/projection differential laws over the gap inventory.
- Compiler/project/LSP/desktop facts and structured edits derived from checked source plus CST/resolution spans.
- Deletion of public Rust `StudioSpec`, `WrittenQuantity`, `Processor`, `Unit`, routing/node construction schemas, and
  the authoritative legacy catalogue/oracle.
- Removal of the normal `musa-compiler -> musa-dsp` dependency; project/session orchestration joins the sibling outputs.
- Code map, roadmap, crate docs, snapshots, and examples describing source as semantic owner and host values as private
  preparation/runtime state.

## Check

```sh
cargo build --workspace
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
python3 scripts/check-layers.py
rg -n "pub (struct|enum) (StudioSpec|WrittenQuantity|Processor|Unit)|musa-dsp" crates/musa-compiler
```

The final `rg` is empty. Commit as `Cut studio semantics over to checked source`.

## Stop

- No new sound feature, sample format, GUI workflow, processor, control, or unit; this converges the implemented set.
- No second evaluator, source parser, source validator, special unifier, or display-text decoder in `musa-dsp`.
- No public calculus `Value`, primitive state, editable host projection, or compiler-to-DSP dependency.
- No removal of compatibility surface beyond the exact deprecations/fixes already assigned to prompt 181.
