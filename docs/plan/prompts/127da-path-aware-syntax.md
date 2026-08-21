---
id: 127da
slug: path-aware-syntax
status: done
depends_on: [127c, 127d]
phase: 3
---

# Give Syntax a Path-Aware Finite Calculus

## Task

Add the finite `Syntax` value, its path-aware fold, and its pure builder facade as a **phase-local** calculus, so that a
syntax transformer can be *written* rather than displayed. This is the first of the four blockers that stopped the
language proof, and the one every other adapter step waits on.

## Read

- `docs/notes/research/language-design-closure/37-final-blocker.md` §1 — the exact defect: `NodePath` and `BindingPath`
  were abstract with no constructors, so the staff and studio expansions were desired output, not programs.
- `26-language-design-decision.md` §§3.2–3.4 (fixed lexing and grouping, syntax values, the transformer interface) and
  `34-proof-review.md` on what the earlier builder API failed to establish.
- `docs/rules/language/00-semantics.md` §1 for where expansion sits, and
  `docs/rules/across-stages/02-derivation-diagrams.md` §§1 and 3 for what a `Generated` step later needs from an
  expansion.
- `docs/rules/language/02-core-calculus.md` §5 — the closed type grammar, the sentence "There is no `fix`, recursive
  binding, while loop, exception, mutation, I/O, reflection, **syntax value**, dynamic cast, or effect handler", and
  §5.8's four builtin families. Read it before writing a line: it decides where this calculus may live.
- Prompt 127ac (library-declared finite data and its generated fold) and prompt 127d
  (`crates/musa-compiler/src/machine.rs`, `core.rs`) — 127d is the worked precedent for adding a finite core type with
  its own builtins, kinding, exact encoding, and law suite.
- `crates/musa-syntax/src/{document,ast,syntax_kind}.rs` — the lossless CST this type is built from.
- Peyton Jones ch. 4–5 for how a fold over a finite structured type is derived from its declaration, which is what makes
  the path-aware fold a derivation rather than an invention.

## Design

**This calculus is phase-local, and that is not a detail.** `02-core-calculus.md` §5 closes the source type grammar and
says in as many words that the source language has no syntax value; §5.8 fixes four builtin families. Both stay true.
Ordinary source cannot name `Syntax`, `NodePath`, or `BindingPath`, cannot obtain one, and has no quotation form — so it
still cannot inspect itself, which is what that sentence exists to forbid. The syntax types and their builtins are in
scope only where a transformer is checked, which is also the repair `34-proof-review.md` recommends: quotation is phase
machinery, and forcing it into the ordinary value calculus is what made the last proof pretend that a load-bearing
language was a library.

One thing this does *not* mean is a second checker or a second evaluator. The transformer language is the same terms,
the same Algorithm W, and the same total evaluator, under an environment that offers more names. A separate machine
would re-open the blocker this series is closing. So: one core, one evaluator, and a phase-local *environment*, with the
syntax builtins carried in their own registry so that §5.8's four families remain the four families of the source core.

`Syntax` is a finite value with its own eliminator, not a compiler-internal tree handed out through a wrapper:

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

Each step function receives the node's structural input path together with what that node holds. A transformer therefore
handles input of any finite size without general recursion, and cannot reach a node without also holding its path.

A transformer never reads a source range. `SourceInfo` lives inside the value and has no eliminator: an adapter carries
a node in order to point at it and can neither read nor forge where it came from. That is one fewer operation, and it
makes "an adapter cannot fabricate provenance" true by construction rather than by rule.

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

This prompt adds a calculus and its environment, not a phase. So that everything here has a real caller, add one
crate-internal driver that reads a parsed region of the existing surface into a `Syntax` value, checks and evaluates a
transformer against it in the phase environment, and returns the checked expression — and drive the law suite through
it. The compiler order, the adapter import, adapter resolution, and expansion records are prompt 127dc's.

## Target

- `Syntax`, `SourceInfo`, delimiters, and opaque scopes as a finite phase-local type: kinding, exact encoding, and exact
  identity.
- `NodePath` and `BindingPath` with derivation-only construction, and no constructor from raw data at all.
- The path-aware fold and the narrow pure builder facade, including `checked_expression` and its diagnostics, in their
  own ownership registry with a hidden-information justification each.
- The phase environment: one crate-internal driver from a parsed region through checking and evaluation to a checked
  expression, with all compiler state and ids private.
- Law tests: ordinary source can neither name a syntax type nor obtain a syntax value; every node the fold visits has a
  unique path; every generated node has a unique path; one binding path denotes one name and two binders differ; folding
  is total and deterministic; a builder is a function of its displayed arguments; scopes are preserved and cannot be
  forged; a duplicate output path and a conflicting binder are each refused by name; §5.8's four source families still
  classify every source builtin exactly once; and there is exactly one match evaluator.
- Public-surface comparison and module-design audit.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-compiler
cargo clippy --all-targets -p musa-syntax -p musa-compiler -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-compiler
```

Commit as `Give syntax values paths, folds, and builders`.

## Stop

- No expansion phase, adapter import, adapter resolution, or expansion record — those are prompt 127dc.
- No grammar, tree-sitter, or formatter change; the fixed reader is not touched here.
- No quotation or syntax-reflection form in ordinary source, no syntax type nameable from ordinary source, no access to
  an expected or inferred type, no fresh-id operation, and no file, network, clock, or randomness access.
- No amendment to `docs/rules/`: if the phase-local reading of §5 turns out not to hold, stop and say so rather than
  widening the source core.
- No second checker and no second evaluator.
- No decision-tree or join-point evaluation target.
- No staff or studio migration.
