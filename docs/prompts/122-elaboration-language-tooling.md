---
id: 122
slug: elaboration-language-tooling
status: pending
depends_on: [104, 118, 119, 121]
phase: 3
---

# Semantic Tooling for Score Elaboration

## Task

Make the score/elaboration portion of the candidate language understandable in editors: type-aware hover, signature help, completion,
definition/references/rename, symbols/folding, diagnostics/fixes, standard-library navigation, analysis requests, and
kernel-document/quote support. All answers come from compiler/project facts or the lossless syntax of half-typed source;
the LSP and editor extensions do not grow a second type checker or theory engine. This prompt establishes the generated
fact/documentation seam that prompt 141 extends to instruments, controls, processors, assets, and packages; it does not
claim whole-language tooling closure.

## Read

- Prompts 77–84 and the current LSP/tree-sitter/VS Code/Zed/desktop language adapters.
- `docs/language/01-surface.md`, `04-templates-and-modules.md`, `07-analysis.md`.
- The compiler's new definition/type/reference/source-map facts and project last-valid-artifact behavior.

## Design

Extend the existing thin LSP boundary, not its dependency graph. Compiler/project expose only caller-oriented immutable
facts needed by more than one surface; do not publish HIR, closures, environments, unification variables, module
tables, or raw `Music`. Syntax-only features continue to use `musa-language` on invalid source; semantic answers use
the last valid compilation and clearly label staleness where relevant.

Required behavior:

- hover shows inferred local type, annotated public signature, domain distinction (`pitchclass` versus `pc12`, key
  versus scale, chord class versus voicing), Origin role, and standard-library source link;
- signature help/completion knows partial application, named/default parameters, modules/templates, assertion policies,
  scale/chord/analysis names, and kernel quote holes;
- definition/references/rename cross user libraries/templates/modules but bundled stdlib is read-only; generated facts
  navigate to definition and instance site;
- symbols/folding include functions, modules, signatures, templates, and kernel composition/lets;
- a code action can insert explicit missing context/register/policy only when the diagnostic supplies a certain fix;
- analysis requests use a command/code-lens surface and display typed findings/evidence without publishing them as
  compiler errors.

Generate editor vocabularies/queries from authoritative Musa sources. Add protocol-level laws with
`Connection::memory()` and token drift tests; do not rely only on a manual editor smoke.

Design the generated item-documentation record so later declaration kinds can use it without publishing compiler HIR:
stable source identity, user-facing name/kind, summary, signature, source span or read-only virtual document, and
deprecation/origin metadata. Do not add audio variants before prompt 141 has real compiler callers, but do not hard-code
the record around only functions and theory declarations.

## Target

- `musa-lsp` feature extensions and focused law tests for every behavior above.
- Any minimal compiler/project fact surface with documented invariants and two real consumers; public-surface audit.
- Tree-sitter query updates and generated assets in `../vscode-musa` and `../zed-musa`.
- Standard-library virtual-document/read-only navigation and analysis-result rendering in both editor clients.
- Manual smoke scripts/checklist for VS Code and Zed, in addition to automated protocol/query tests.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cd editors/tree-sitter-musa && tree-sitter test
git -C ../vscode-musa diff --check
git -C ../zed-musa diff --check
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-compiler
```

Commit each repository intentionally and record cross-repository commit ids. Commit Musa as
`Teach editor tooling the elaboration language`.

## Stop

- No second parser/type checker/evaluator/theory algorithm in LSP, TypeScript, tree-sitter, VS Code, or Zed.
- No rename of bundled standard-library definitions and no edit to generated music facts.
- No incremental compiler or semantic-token protocol optimization without prompt 125's measurement.
- No desktop interaction redesign — prompt 123.
- No instrument/control/processor/asset/package hover or completion yet — prompt 141 adds those from their eventual
  authoritative registries.
