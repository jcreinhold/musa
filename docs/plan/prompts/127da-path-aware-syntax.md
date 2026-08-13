---
id: 127da
slug: path-aware-syntax
status: pending
depends_on: [127c, 127d]
phase: 3
---

# Give Syntax a Path-Aware Finite Calculus

## Task

Add the finite `Syntax` value, its path-aware fold, and its pure builder facade to the source core, so that a syntax
transformer can be *written* rather than displayed. This is the first of the four blockers that stopped the language
proof, and the one every other adapter step waits on.

## Read

- `docs/notes/research/language-design-closure/37-final-blocker.md` §1 — the exact defect: `NodePath` and `BindingPath`
  were abstract with no constructors, so the staff and studio expansions were desired output, not programs.
- `26-language-design-decision.md` §§3.2–3.4 (fixed lexing and grouping, syntax values, the transformer interface) and
  `34-proof-review.md` on what the earlier builder API failed to establish.
- `docs/rules/language/00-semantics.md` §1 for where expansion sits, and
  `docs/rules/across-stages/02-derivation-diagrams.md` §§1 and 3 for what a `Generated` step later needs from an
  expansion.
- Prompt 127ac (library-declared finite data and its generated fold) and prompt 127d
  (`crates/musa-compiler/src/machine.rs`, `core.rs`) — 127d is the worked precedent for adding a finite core type with
  its own builtins, kinding, exact encoding, and law suite.
- `crates/musa-language/src/{document,ast,syntax_kind}.rs` — the lossless CST this type is built from.
- Peyton Jones ch. 4–5 for how a fold over a finite structured type is derived from its declaration, which is what makes
  the path-aware fold a derivation rather than an invention.

## Design

`Syntax` is a finite core value, not a compiler-internal tree handed out through a wrapper:

```text
SourceInfo = Original(source range)
           | Generated(expansion, node path)

Syntax = Missing(SourceInfo)
       | Token(SourceInfo, token kind, exact text)
       | Identifier(SourceInfo, name, scopes)
       | Group(SourceInfo, delimiter, List<Syntax>)
```

`delimiter` is `Parentheses`, `Brackets`, `Braces`, or `Layout`. `scopes` is opaque: package code may compare two names
and preserve the scopes it received, and has no operation that constructs a scope. `Syntax` is storable data — no arrow
at any depth — so the kind system already refuses a syntax value that hides a closure, exactly as it does for a machine
port.

The whole point of this prompt is that **paths are derived, never invented**. A `NodePath` is obtained in one of two
ways: the path-aware fold supplies the unique structural path of each input node it visits, and a builder derives an
output path from an expansion path, an input path, a builder role, and a finite child number. There is no public
constructor from a number, a string, or a counter, and no operation that mints a fresh id. A `BindingPath` is derived
the same way, which is what makes "calling one local binding path twice denotes the same name" a fact about the API
rather than a convention.

The fold is the only way into a syntax value. Give it the shape prompt 127ac already generates for finite data, plus the
path argument:

```text
fold_syntax(missing, token, identifier, group, subject) -> A
```

Each step function receives the node's `SourceInfo` and its structural input path. A transformer therefore handles input
of any finite size without general recursion, and cannot reach a node without also holding its path.

The builder facade is a small set of total first-order builtins over storable data — construct a token, an identifier, a
group, a binder, a reference to a binder — each a pure function of its displayed arguments. A node built at output path
`p` receives `Generated(expansion, p)`; existing input nodes keep their source information unchanged.
`checked_expression` is the gate: it rejects duplicate output paths and two binder declarations at one binding path,
with a diagnostic that names the path and both sites. There is no quotation, antiquotation, text-to-syntax, or
syntax-reflection form in ordinary source.

Blocker 2 is closed by *stating* what 127b already decided rather than by adding machinery. Exhaustive `match` stays an
explicit form in the private evaluation core; there is no `join`, `jump`, `Leaf`, `Bind`, or `Switch` target, so adapter
code and ordinary source have one executable match semantics and one evaluator. State it as a law here, where the
transformer language first exists to be confused with a second one.

This prompt adds values and operations, not a phase. So that the type has a real caller, add one crate-internal driver
that turns a parsed region of the existing surface into a `Syntax` value and back to a checked expression, and use it in
the law suite. The compiler order, the adapter import, adapter resolution, and expansion records are prompt 127dc's.

## Target

- `Syntax`, `SourceInfo`, delimiters, and opaque scopes as a finite core type: kinding, exact encoding, exact identity,
  and its entries in the builtin-ownership registry.
- `NodePath` and `BindingPath` with derivation-only construction, and no public path constructor from raw data.
- The path-aware fold and the narrow pure builder facade, including `checked_expression` and its diagnostics.
- One crate-internal driver from a parsed region to `Syntax` and back, with all compiler state and ids private.
- Law tests: every node the fold visits has a unique path; every generated node has a unique path; one binding path
  denotes one name and two binders differ; folding is total and deterministic; a builder is a function of its displayed
  arguments; scopes are preserved and cannot be forged; a duplicate output path and a conflicting binder are each
  refused by name; and there is exactly one match evaluator.
- Public-surface comparison and module-design audit.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler
cargo clippy --all-targets -p musa-language -p musa-compiler -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-compiler
```

Commit as `Give syntax values paths, folds, and builders`.

## Stop

- No expansion phase, adapter import, adapter resolution, or expansion record — those are prompt 127dc.
- No grammar, tree-sitter, or formatter change; the fixed reader is not touched here.
- No quotation or syntax-reflection form in ordinary source, no access to an expected or inferred type, no fresh-id
  operation, and no file, network, clock, or randomness access.
- No decision-tree or join-point evaluation target.
- No staff or studio migration.
