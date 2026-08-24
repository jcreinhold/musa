---
id: 162e
slug: let-in-a-block
status: done
depends_on: [155a, 161]
phase: 3
---

# `let` Inside a Block, Because Naming a Value Should Not Cost a Declaration

## Task

`{ e }` holds one expression and `01-surface.md` §1 says so by name: "no `let`, no `return`, no `;`-separated sequence".
The consequence is that the only way to name an intermediate value is to declare a function for it at the top of the
file, or to write the expression twice. The standard library shows both: `stdlib/src/adapters/staff.musa` declares
ninety-seven functions, many of them one-line readers used at one site, and its refusal paths recompute values the
success path already had. Admit `let` in a block — the core has had the term since the beginning (`Raw::Let`, and `with`
already elaborates to one) — and amend §1, which currently forbids it.

## Read

- `docs/rules/language/01-surface.md` §1's block paragraph, in full: "`{ e }` is a block, it holds exactly one
  expression, and it means that expression — `⟦{ e }⟧ = ⟦e⟧` (`02-core-calculus.md` §2). … There is no statement
  language inside it: no `let`, no `return`, no `;`-separated sequence, and a second expression in a block is a static
  error naming the rule." That sentence is what this prompt amends, and *only* the `let` clause of it: `return` and a
  bare sequence stay refused, for the reason the Design gives.
- `docs/rules/language/02-core-calculus.md` §2 and §1's `let` — the term already exists, it already has a typing rule,
  and `crates/musa-calculus/src/elaboration/elab/check.rs`'s `RawShape::Let` arm already checks one. So this prompt adds
  no core term; it adds a spelling for a term the core has.
- `docs/rules/language/01-surface.md` §9.1 — path update, which "elaborates to a `let` binding the subject and one
  record literal per segment". The desugaring is already written in terms of a `let` the surface cannot spell, which is
  the clearest statement that the gap is in the surface alone.
- `crates/musa-compiler/src/lower/values.rs` — how a block is lowered today, and where the second-expression refusal is
  raised.
- `crates/musa-syntax/src/parser/expressions.rs`'s block reader and `at_field_init` — the three-token lookahead that
  already tells a record literal from a block, which is the precedent for deciding this in the parser rather than with a
  flag.
- Peyton Jones ch. 3, §3.2 and §3.3 — the enriched lambda calculus, and why a language for programmers gets local
  definitions before anything else. Root `AGENTS.md`'s "no sublanguage by subtraction" cites the same chapter, and a
  language whose blocks cannot name a value is that subtraction applied to itself.
- Prompt [155a](155a-case-tree-bodies.md) and [155aa](155aa-lift-local-recursion.md) — what a definition body may hold
  now, and how a local recursion is lifted. A `let` is not a `rec`, and 155aa is the reason it does not have to be.

## Design

**One binding, one body, and the `;` is part of the `let` and not a sequence.** `let name: T = e; body` is one
expression: the `;` terminates the binding, exactly as it does at the top level, and a block still holds exactly one
expression. So `{ e1; e2 }` stays the static error §1 names — nothing gained a sequence, and "a second expression in a
block is a static error naming the rule" is still true, because a `let` is not a second expression.

**Several bindings are several `let`s, nested rightward.** `let a = …; let b = …; body` is `let a = … in (let b = … in
body)`, which is what the core term already means. No block-level scope table, no mutual recursion, no shadowing rule
beyond the one a nested binder already has.

**The type annotation is optional and inference is the ordinary one.** `let doubled = x + x;` infers. Where the value is
uninferable the refusal is `Uninferable`, pointing at the value, which is the diagnostic the core already gives; a `let`
is not a place to invent a second inference mode.

**No `rec`.** A `let` whose value names itself is refused, naming prompt 155aa's lifting as the thing to write instead.
The core's `Raw::Let` is non-recursive and the termination checker's measure (`02-core-calculus.md` §2.4) is stated over
definitions; a recursive local binding would need both changed and neither is this prompt's.

**It is a real saving only if it also lands where the value is used twice.** The Target names two conversions rather
than none, and the commit message carries the line counts, because the argument for the feature is that the corpus
repeats itself and the evidence is that the repetition goes away.

## Target

- `let name = value;` and `let name: T = value;` inside any block, at any depth, nesting rightward.
- A self-naming `let` is refused, pointing at 155aa.
- `{ e1; e2 }` is still refused, with §1's own diagnostic unchanged.
- `docs/rules/language/01-surface.md` §1 amended: the `let` clause goes, the `return` and sequence clauses stay, and the
  paragraph says which is which and why.
- `editors/tree-sitter-musa` reads it, with a corpus entry.
- Laws: a `let` means what substituting the value means; a nested `let` scopes rightward; an inner `let` shadows an
  outer binder; a self-naming `let` is refused; a `let` in a `match` arm and in a `quote at here { … }` both parse.
- Two conversions as the evidence, both in `stdlib/src/notation/staff.musa`, with before and after line counts in the
  commit message. That file carries two private helpers whose own comments say they exist only because a block cannot
  name a value — `dotted_by` ("`dotted_span`'s step has no `let` to name it with") and `placed_span` ("a parameter is
  how this language names a value used twice"). Both go.

  `stdlib/src/tonal/schemas.musa` was named here and is not a conversion. It is a file of table literals: every schema
  is one `degrees_of([…])` call, and the only repeated subexpression in it is the `degree_of(1)` that
  `rule_ascending_chord` and `rule_descending_chord` each write twice. Hoisting that into a `let` costs a line and
  evaluates a chord for every bass degree that is not a tonic — which is prompt [162f](162f-lazy-methods.md)'s problem,
  not evidence for this one. A conversion that made a file worse would not be evidence that a block should name a
  value.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-syntax -p musa-calculus -p musa-compiler
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test -p musa-syntax -p musa-compiler --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

```sh
printf 'library {\n  fn twice(x: Nat) -> Nat { let doubled: Nat = x + x; doubled + doubled }\n}\n' > /tmp/let.musa
cargo run -q -p musa -- check /tmp/let.musa
```

The second line needs `Nat.add`, which prompt [164](164-builtin-collapse.md) declares; until it runs, write the same
proof over `text_join` instead and say so in the report.

`cargo nextest run --run-ignored all` and `cargo insta test --workspace` carry the reds prompt 164's Check enumerates
and [166b](166b-per-context-memo-stamp.md) owns; a *new* red under either is this prompt's.

Commit as `Let a block name a value`.

## Stop

- No `return`, no `;`-separated sequence of expressions, no statement language. Only `let`.
- No recursive `let`. Prompt 155aa lifts a local recursion and that is where it stays.
- No mutual `let`, no `let` pattern destructuring (`let (a, b) = …`), no `let` at the top of a `match` arm's *pattern*.
- No new core term. If the implementation reaches for one, that is evidence the design is wrong and the prompt needs
  repair, not a term.
- No sweep of `stdlib/`. Two conversions are the evidence; the adapters are prompts [166](166-staff-rewrite.md) and
  [167](167-studio-rewrite.md)'s. `stdlib/reference.md` is generated (`UPDATE_FIXTURES=1`) and is regenerated, not
  edited.
