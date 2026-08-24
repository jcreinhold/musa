---
id: 162c
slug: nested-patterns-parse
status: pending
depends_on: [155, 161]
phase: 3
---

# Let the Parser Read the Nested Patterns the Rules Already Admit

## Task

`01-surface.md` §1 says "**Patterns nest.** A sub-position holds another pattern rather than only a binder", and
`02-core-calculus.md` §6.2 is where coverage for one is decided. The parser has not been told. `Wrap(Loud(count))` is
refused with "missing `)`", pointing at the inner `(`, because a constructor pattern's arguments are read as bare
binders. Every reader in the corpus therefore writes the depth-one form the rule replaced — a match, then a match inside
the arm — and pays a case tree per level. Read the nested form.

## Read

- `crates/musa-syntax/src/parser/patterns.rs` — where a constructor pattern's argument list is read, and the one place
  the depth-one assumption lives.
- `docs/rules/language/01-surface.md` §1's **Patterns nest** paragraph, which is the governing sentence and is already
  written: "This replaces the earlier depth-one rule, whose whole argument was that the case-tree compiler had not
  earned its place; §6.2 states what changed and why." The compiler earned it at prompt [155](155-case-trees.md); this
  is the surface catching up.
- `docs/rules/language/02-core-calculus.md` §6.2 — the case tree, its coverage rule, and its refusals. A nested pattern
  compiles by splitting the sub-position; nothing in §6.2 needs to change, and the Check below asserts that by
  exercising an inexhaustive nested match and reading the existing diagnostic.
- `crates/musa-compiler/src/lower/case.rs` (and `crates/musa-calculus/src/elaboration/case.rs`) — what the compiler does
  with the patterns it is given, so the prompt can tell which half is missing rather than guessing.
- `crates/musa-syntax/src/ast/patterns.rs` — the CST shape a pattern node has, and whether it already admits a pattern
  child or only a binder token.
- `editors/tree-sitter-musa/grammar.js` — held to the real lexer by the drift law, and to the real grammar by the corpus
  tests. A pattern the parser reads and the grammar does not is drift.
- `stdlib/src/adapters/staff.musa` and `stdlib/src/notation/staff.musa` — the two files that write the depth-one
  workaround most often. Neither is rewritten here (see Stop); they are read so the prompt's Check can name a real arm
  that becomes one arm.

## Design

**One rule: a constructor pattern's argument is a pattern.** Not "a pattern of depth two", not "a pattern except a
literal". The grammar is recursive because §6.2's case tree is, and any bound would be a second thing to remember.

**List and record patterns nest too, or the rule is not the rule.** `[Loud(n), .. rest]` and
`Pending { read = Loud(n) }` are the same sub-position question in two spellings, and a fix that reached only the
positional form would leave the corpus with two answers to one question.

**No change to coverage, and that is checkable.** A nested match's exhaustiveness is §6.2's already: the case tree
splits the sub-position and reports what it always reported. The Check writes an inexhaustive nested arm and asserts the
*existing* diagnostic code, so a fix that quietly widened what is accepted fails.

**The tree-sitter grammar moves in the same commit.** The drift law is not a style rule — an editor that cannot
highlight a pattern the compiler reads is the same file disagreeing with itself.

## Target

- `Wrap(Loud(count))` parses, at any depth, in positional, list, and record patterns alike.
- The CST admits a pattern child where it admitted a binder token, and the lowering passes it through.
- `editors/tree-sitter-musa` reads the same shape, with a corpus entry.
- Laws: a nested match answers what the two-level match answered; an inexhaustive nested match is refused with the
  diagnostic §6.2 already names; a nested list pattern and a nested record pattern each parse and lower.

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

And the proof, which must go from "missing `)`" to a clean compile:

```sh
printf 'library {\n  data Inner { Quiet, Loud(count: Nat) }\n  data Outer { Wrap(held: Inner) }\n  fn read(o: Outer) -> Nat {\n    match o { Wrap(Loud(count)) -> count, Wrap(Quiet) -> 0, }\n  }\n}\n' > /tmp/nested.musa
cargo run -q -p musa -- check /tmp/nested.musa
```

`cargo nextest run --run-ignored all` and `cargo insta test --workspace` carry the reds prompt
[164](164-builtin-collapse.md)'s Check enumerates and [166b](166b-per-context-memo-stamp.md) owns; a *new* red under
either is this prompt's.

Commit as `Let the parser read the nested patterns the rules already admit`.

## Stop

- No guards, no conditional equations, no pattern on the left of a definition. §1 rules out all three by name and this
  prompt does not reopen them.
- No or-patterns. Prompt [162d](162d-or-patterns.md).
- No change to `02-core-calculus.md` §6.2's coverage rule or to any diagnostic it names.
- No rewrite of `stdlib/` to use the new form. The staff adapters are prompts [166](166-staff-rewrite.md) and
  [167](167-studio-rewrite.md)'s, and 166 is done; a sweep here would be that prompt reopened.
- No formatter opinion beyond keeping `make fmt-check` green — how a deep pattern wraps is `docs/rules/style-guide.md`'s
  question and nobody has asked it yet.
