---
id: 117
slug: typed-kernel-quotation
status: pending
depends_on: [97, 112, 116]
phase: 3
---

# Typed Kernel Quotation and Hygienic Antiquotation

## Task

Add the controlled assembly-level escape inside ordinary Musa: quote a kernel composition expression as contextual
`music` and splice typed host `music` with hygienic `${...}` antiquotation. Check closure, exact extent, payload type,
and material context-neutrality; mark every resulting fact with `KernelSplice`; and state precisely which surface
music-theory guarantees the raw term bypasses.

## Read

- `docs/language/00-semantics.md`, `01-surface.md`, and `05-verification.md` quotation sections.
- `docs/kernel/01-grammar.md`, K1–K7 in `02-static-semantics.md`, T1–T6 in `10-term-calculus.md`, prompt 49's
  reference marks, and prompt 116's grammar ownership.
- Prompt 112 assertion placement and prompts 63–65 context-authority facts.

## Design

The local form quotes exactly the kernel `composition-expression` grammar extended only with typed holes; it does not
contain a version header or top-level composition declaration. `${e}` requires `e : music`. At quote instantiation,
instantiate each hole once under the host environment at the structurally computed quotation locus: sequence adds
exact prefix extents, overlay preserves the locus, shift translates, positive time scale scales the relative offset,
restriction does not relocate starts, and a `let` value begins at its enclosing locus.

Bind each hole to a compiler-generated fresh kernel name, close the quote around those bindings, and reject capture by
construction. Raw references/operators may later reuse, move, stretch, or observe the frozen facts; they do not
reinstantiate a hole or reread context at every reference. This assembly semantics is why an outer raw transform can
invalidate placement-sensitive surface checks previously passed inside a hole.

Before a quote has type `music`, every raw payload parses as `ScoreFact`, and key/meter/tempo/clef facts are rejected:
local `music` remains context-neutral. Then `close` and `Term::check` must succeed. Each exiting fact retains its inner
Origin and gains a `KernelSplice` step. Exact outer extent and payload typing remain guaranteed; bar/scale/chord/theory
relationships inside raw text do not, unless an explicit assertion wraps the completed quote.

## Target

- Quotation/hole syntax, lossless formatting, highlighting/tree-sitter, typed elaboration, fresh-name hygiene, quote
  locus computation, context-neutral payload validation, and `KernelSplice` provenance.
- `examples/kernel-splice.musa`: raw and antiquoted material, sharing, shift/scale/restrict, outer assertion, and Origin
  inspection; broken fixtures for capture attempts, open refs, payloads, and context facts.
- `crates/musa-compiler/tests/kernel_quote_laws.rs`: hygiene, closure, exact extent, quotation-locus reference model,
  frozen-hole behavior, provenance, guarantees/bypasses, and direct-term agreement.

## Check

```sh
cargo nextest run -p musa-kernel -p musa-language -p musa-compiler -p musa-lsp
cargo clippy --all-targets -p musa-kernel -p musa-language -p musa-compiler -p musa-lsp -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- check examples/kernel-splice.musa
cargo run -p musa-cli -- kernel examples/kernel-splice.musa
cd editors/tree-sitter-musa && tree-sitter test
cargo bench -p musa-compiler
```

Commit as `Add typed kernel quotation and splicing`.

## Stop

- No host expression other than typed antiquotation inside the raw grammar.
- No context-authoritative ScoreFact in local `music`, unknown payload, free name, or unchecked term.
- No promise that raw scale/shift/restrict preserves surface assertions placed inside the quote.
- No general syntax quotation, macro expander, assembler optimizer, or new kernel form.
