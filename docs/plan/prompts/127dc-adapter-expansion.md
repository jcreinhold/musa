---
id: 127dc
slug: adapter-expansion
status: done
depends_on: [127da, 127db]
phase: 3
---

# Run Adapter Expansion as a Fixed Compiler Phase

## Task

Add the named delimited adapter region, the syntax import that names its owner, adapter resolution, and the expansion
phase itself, at one fixed place in the compiler order. Expansion records become `Generated` steps in the derivation
graph. This is the phase; the two real adapters are prompt 127dd.

## Read

- `docs/notes/research/language-design-closure/26-language-design-decision.md` §3 in full — the fixed compiler order,
  the fixed lexer and grouper, the adapter interface, what an adapter may not do, and the expansion record.
- `34-proof-review.md` and `37-final-blocker.md` for what the earlier account of this phase failed to establish.
- `docs/rules/language/00-semantics.md` §1 (expansion sits before inference) and
  `docs/rules/across-stages/02-derivation-diagrams.md` §6's closing paragraph: an expansion record is a source of
  `Generated` steps and nothing more.
- Prompts 127da and 127db, and their delivered modules.
- `crates/musa-language/src/{lexer,parser,formatter,highlight}.rs`,
  `crates/musa-compiler/src/{resolve,imports,elaborate}.rs`, and `editors/tree-sitter-musa` — the drift law holds the
  grammar to the real lexer.

## Design

Use one fixed compiler order and do not let any pass reach backwards:

1. lex and group the file;
2. parse the fixed module header;
3. resolve explicitly imported adapters;
4. expand named, delimited adapter regions;
5. parse each result as an ordinary expression;
6. resolve ordinary names;
7. infer and check types;
8. lower to the private evaluation core; and
9. evaluate to finite values.

A syntax import appears in the fixed module header, before any definition that uses it, and names the package that reads
a region. An ordinary import cannot change syntax. A region is **named and delimited** in the reader Musa already has:
packages do not extend the lexer, and the grouper's boundary is fixed. Whether the surface later moves to an
indentation-based file structure is prompt 127e's single clean break, not a half-step taken here.

An adapter definition is checked and evaluated in prompt 127da's phase environment, where the syntax types are in scope;
ordinary source is checked in the environment it has now, where they are not. One checker, one evaluator, two
environments.

Package adapters may emit an expression and the contents of an expression block. They may not emit imports, modules,
type declarations, value declarations, or another adapter region. That is what makes the set of module declarations
known before expansion and breaks the expansion-resolution cycle. Adapter definitions themselves contain no adapter
regions, which makes termination structural: do not add rank arithmetic.

An adapter is type-blind. It cannot inspect an expected or inferred type, read an ordinary value from the importing
module, read files, network, clock, or randomness, mutate compiler state, call audio services, or evaluate text as
source. Given an adapter version, a region header, and a region body, expansion returns one expression or one error, and
depends on nothing else — so it is deterministic and locally decidable.

Every successful expansion produces an exact record: adapter definition, exact adapter package source version, use-site
range, input syntax, output syntax, and parent expansion. Generated nodes point into that record. Diagnostics from
resolution, inference, and evaluation report against original source through it; a composer never reads a position in
text they did not write.

Charging is phase-tagged and cache-independent: expansion steps, generated syntax nodes, type constraints, and
evaluation steps are charged separately, and a cache hit replays the same logical charge as the miss it replaces. Cache
warmth cannot change whether a file is accepted.

One small adapter lives in the repository as the phase's executable fixture — enough to exercise resolution, expansion,
the record, the source map, charging, and a refusal of each forbidden emission. The complete staff and studio adapters
are prompt 127dd's, and this fixture is not a draft of them.

## Target

- The syntax-import header form and the named delimited region: grammar, tree-sitter, formatter, and highlighting, all
  in step with the real lexer.
- Adapter resolution against the resolved package graph, with a finite acyclic adapter dependency graph.
- The expansion phase at its fixed place, expansion records, phase-tagged charges, and source-mapped diagnostics.
- Expansion records appended to prompt 127db's derivation graph as `Generated` steps.
- One in-repo fixture adapter and its `examples/` piece.
- Tests: determinism and locality of expansion; termination without rank arithmetic; refusal of an emitted import,
  module, declaration, or nested adapter region; type blindness; every diagnostic landing in original source; a cache
  hit charging what the miss charged; and hygiene across a region that binds and uses one name.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Expand adapter regions in one fixed phase`.

## Stop

- No general macro system, syntax reflection in ordinary source, type-directed expansion, expected-type access, compiler
  mutation, arbitrary fresh ids, adapter-generated declarations, or adapter recursion.
- No rank arithmetic; termination comes from the adapter-free bootstrap.
- No indentation-based cutover of the surface, and no deletion of contextual `Music` — both are prompt 127e.
- No staff or studio adapter, no edit or print operation, and no proof freeze — those are prompt 127dd.
- No claim that expansion provenance alone proves musical derivation.
