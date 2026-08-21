---
id: 127ac
slug: nominal-data
status: done
depends_on: [127ab]
phase: 3
---

# Let a Library Declare Finite Data

> **Governed by the event-track and machine core installed by prompts 127a–127e and 170–173.** Third of the five prompts
> that replace the source checker and evaluator; the chain is 127aa, 127ab, 127ac, 127ad, 127b.

## Task

Let a library declare finite, strictly positive nominal data types with named fields, private constructors, and abstract
members, and generate one structural fold per declaration. This is the piece that lets a music-theory package own its
own data instead of asking the compiler for another built-in type.

## Read

- `docs/rules/language/02-core-calculus.md` §1 and §1.1 — nominal types `N[τ, …]`, the once-per-group storable-data
  check, and the strict positivity requirement.
- Research `05-selected-calculus.md` §2 and §2.1, in particular the declaration-group check and build-local nominal
  identity.
- `docs/rules/language/04-templates-and-modules.md` §4 — the existing signature and structure system these declarations
  are sealed behind, and which already implements transparent by-name matching.
- `crates/musa-compiler/src/{module,resolve,template}.rs`, and the `Shape`/`Base` registry in `core.rs` that the
  built-in musical domains use today.
- `crates/musa-syntax` and `editors/tree-sitter-musa`, held together by the drift law.

## Design

A declaration is finite and strictly positive: a type may not occur to the left of an arrow in its own group. Group
mutually recursive declarations, check the group once, reject any stored function field, and accept the group as
storable data when every field leaving it is already storable data. The check terminates because the declaration graph
is finite. Report a rejection at the field that caused it, not at the group.

Nominal identity is build-local: two declarations with the same name in different packages are different types, and the
identity is derived from the resolved declaration rather than the spelling. It is not hashed source text.

Generate exactly one structural fold per declaration and give it the scheme the declaration determines. Do not generate
a second eliminator, a pattern-match compiler, or a visitor: `match` from prompt 127ab is the eliminator a reader uses,
and the fold is what a total traversal is written with.

Private constructors and abstract members are resolved and sealed before lowering, exactly as signatures and structures
already are. The private evaluation core sees ordinary constructors; it does not carry abstraction.

Records are a nominal declaration with one constructor and named fields, spelled and projected by name. They are not a
structural row type and there is no width or depth subtyping.

Nominal ids, constructor tables, and the sealed declaration tables stay private to `musa-compiler`.

## Target

- `data` declarations with named fields, parameters, private constructors, and abstract members; records as the
  one-constructor case.
- The once-per-group finiteness, strict-positivity, and storable-data check, with per-field diagnostics.
- One generated structural fold per declaration, with its inferred scheme.
- Build-local nominal identity, derived from the resolved declaration.
- Lexer, keywords, CST, AST accessors, formatter, highlighting, and `editors/tree-sitter-musa` updated together.
- Compile-fail tests for a non-positive declaration, a stored function field, a private constructor used outside its
  module, and a bad data instantiation; law tests for fold and for the group check's termination.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
find examples stdlib -name '*.musa' -print0 | xargs -0 -n1 cargo run -q -p musa -- check
```

Commit as `Let a library declare finite nominal data`.

## Stop

- No general recursion, no non-strictly-positive declaration, no type class, and no deriving mechanism beyond the one
  generated fold.
- No second decision-tree evaluator or pattern-match compiler beside 127ab's `match`.
- No migration of the built-in musical domains onto `data`; they keep their registry until a later prompt argues the
  move on evidence.
- No deletion of partial calls or default parameters; that is 127ad.

## Note

Moving the existing musical domains from the `Base`/`Shape` registry onto library-declared `data` is the obvious next
question and is deliberately not asked here. `02-core-calculus.md` §5.8's conservative-extension theorem is stated over
the registry; changing what the theorem ranges over is a separate argument with its own evidence, not a refactor to do
while the declarations are new.
