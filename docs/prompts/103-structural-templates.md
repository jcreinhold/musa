---
id: 103
slug: structural-templates
status: pending
depends_on: [63, 98, 101]
phase: 3
---

# Typed Templates for Pieces and Voices

## Task

Implement declaration-level templates for families of pieces and voices parameterized by typed values, functions, and
contextual `music`. This supplies the concrete key/scale/subject/transform parameterization requested by the design
without turning a piece, part, voice, context track, studio graph, or source AST into an ordinary value.

## Read

- `docs/language/04-templates-and-modules.md` and the staging diagram in `00-semantics.md`.
- Current piece/score/part/voice parsing and identity allocation; prompt 63's authority rules for key/meter/tempo/clef;
  prompt 67 realization identity; prompt 84/the-project-is-the-unit.
- The key-parameterized `study` and transformed-voice examples in `docs/language/01-surface.md`.

## Design

Implement the separate judgment `Γ ⊢ template(params) D : params => declaration κ`, for `κ` equal to piece or voice
in this prompt. A template is not a value; instantiation evaluates arguments in the total core, substitutes values
into a typed declaration template, assigns stable generative identities derived from template definition plus
instance site, and yields ordinary declarations before context tracks and `music` instantiate.

The dependency graph is finite and acyclic. Duplicate/recursive instance paths, identity collisions, missing/extra
arguments, and structural-kind mismatch are diagnostics with both definition and instance labels. A parameterized voice
may contain key/meter/tempo/clef statements because each instance has one structural placement and identity; a
`music` parameter remains context-neutral. Templates accept higher-order arguments such as `music -> music` but cannot
return/take first-class `piece` or `voice` values or inspect their source.

Prove/test determinism, stable identity under unrelated edits, distinct identity for distinct instance sites,
alpha-renaming, substitution/type preservation, acyclic termination, and equivalence with handwritten declarations
under musical equality plus a specified template-instance Origin step.

## Target

- Surface grammar/formatter/tree-sitter for parameterized piece/voice declarations and `make`/instance syntax chosen by
  the spec.
- Private structural-template checker/expander before context-track construction.
- Stable identity/provenance integration through compiler/project/editor facts.
- `examples/template-study.musa`: two key/scale instances of one piece template and a voice template taking a
  transposition function.
- `crates/musa-compiler/tests/template_laws.rs`: substitution, identity, context authority, equivalence, cycles, and
  negative type/kind cases.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- check examples/template-study.musa
cd editors/tree-sitter-musa && tree-sitter test
cargo bench -p musa-compiler
```

Commit as `Add typed piece and voice templates`.

## Stop

- No first-class `piece`, `voice`, `part`, `module`, or declaration value.
- No reflection, arbitrary declaration rewriting, macros, recursive templates, or runtime instantiation.
- No module parameter/signature/functor yet — prompt 104.
- Do not leak template HIR or identity allocation through the compiler facade.
