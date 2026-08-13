---
id: 127da
slug: bounded-syntax-adapters
status: pending
depends_on: [127c, 127d]
phase: 3
---

# Make Bounded Syntax Adapters Executable

## Task

Define, implement, and prove the small package-owned syntax-adapter phase before using it to migrate Musa notation and
studio source. Close the four exact blockers from the failed language proof rather than reviving a general macro system.

## Read

- `docs/notes/research/language-design-closure/{26-language-design-decision,34-proof-review,37-final-blocker}.md`.
- The governing adapter, origin, and derivation rules produced by prompt 127a and the inferred core from prompt 127b.
- The lossless CST, source maps, structured edit commands, module resolver, and the staff/studio surface
  implementations.
- The complete staff and studio adapter trials listed in the failed design record.

## Design

Use one fixed compiler order:

1. lex and group the file;
2. parse the fixed module header;
3. resolve explicitly imported adapters;
4. expand named, delimited adapter regions;
5. parse each result as an ordinary expression;
6. resolve ordinary names;
7. infer and check types;
8. lower to the private evaluation core; and
9. evaluate to finite values.

An adapter receives a finite `Syntax` value: missing input, token, identifier with opaque scopes, or delimited group,
each with exact source information. A path-aware generated fold visits every node and supplies its unique structural
input path. Pure builder functions derive output and binding paths only from the expansion path, an input path, a
builder role, and a finite child number. Package code cannot invent path or scope ids. The checker rejects duplicate
output paths and conflicting binders.

Adapter definitions are written in the adapter-free total core. An adapter emits one ordinary expression and cannot emit
imports, modules, types, declarations, or another adapter call. This simple bootstrap makes expansion termination
structural; do not add rank arithmetic. Builders are ordinary first-order total operations over storable syntax data,
not quotation, antiquotation, text evaluation, or mutable compiler services.

Keep exhaustive `match` as an explicit form in the private evaluation core. Do not introduce `join`, `jump`, `Leaf`,
`Bind`, or `Switch` until a measured compiler need justifies a separate proved lowering. This gives adapter code the
same executable match semantics as ordinary source.

Every expansion produces an exact record containing adapter definition and source version, use site, input syntax,
output syntax, and parent expansion. Generated nodes point into that record. Track construction and reuse then append
those source leaves to the finite derivation graph installed by prompts 127a and 127c. A reused event records both its
shared source and this generation site. A combined event has all input derivations as parents. Composition grafts an
earlier graph at each matching source leaf; it is never list concatenation.

Separate three adapter operations:

- `expand` returns ordinary expression syntax or an error;
- `edit` returns focused text edits for a structured command; and
- optional `print` creates new source or states what it cannot preserve.

Reprinting an expanded value is not a safe edit. The standard staff and studio adapters must support all three
operations; third-party adapters may be read-only.

Freeze the exact rules and prove termination, determinism, hygiene, unique path formation, source attribution, edit
locality, print round-trip where claimed, match execution, derivation coverage, and associative derivation composition.
Run hostile proof review, repair, and re-review as many times as needed. This prompt completes only when the final
review says correct under the stated contracts with no fatal, high, or medium finding.

## Target

- Public finite `Syntax`/path-aware fold and narrow pure builder facade; all compiler state and ids remain private.
- Adapter resolution, expansion records, fixed charging, source maps, edit/print contracts, and one executable match
  target.
- Finite derivation graph with reuse, combination, coverage, and composition laws.
- Unprivileged complete staff and studio trials, including structured edits, with no private parser or checker access.
- Frozen proof and a correct-under-contracts review with no unresolved fatal, high, or medium issue.

## Check

```sh
./scripts/check-syntax-adapter-conformance.sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Make bounded syntax adapters executable`.

## Stop

- No general macro system, syntax reflection in ordinary source, type-directed expansion, expected-type access, file or
  network access, compiler mutation, arbitrary fresh ids, adapter-generated declarations, or adapter recursion.
- No decision-tree or join-point target without a measured need and its own complete semantics and simulation proof.
- No claim that expansion provenance alone proves musical derivation; the two records have different jobs.
