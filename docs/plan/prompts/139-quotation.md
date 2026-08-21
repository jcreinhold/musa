---
id: 139
slug: quotation
status: done
depends_on: [138]
phase: 3
---

# Implement Quotation, Splicing, and Automatic Provenance

## Task

Implement `quote at here { … }`: a quote body parsed by the real parser, elaborated into a `Syntax<Cat>` construction,
with `$x` splicing a value at a category-checked position, `$..xs` splicing a sequence where a sequence is grammatical,
and provenance minted by the elaborator rather than allocated by the author. This is the prompt that makes `call1`–
`call7` and every hand-written `syntax_group`/`syntax_token` assembly unnecessary.

## Read

- `docs/rules/language/11-quotation.md` in full, and prompt 132's staff-construction program — the specification and the
  program it was written against. If the trial's program does not compile at the end of this prompt, the prompt is not
  done.
- `docs/rules/language/01-surface.md` §7 — the kernel quote, its typed antiquotation, and its four writer's rules. Two
  of those rules generalize and this implementation must satisfy them: a quote is commented like the file around it, and
  the quotation locus is where a hole is *instantiated*, which differs from where its result lands under `let`.
- `crates/musa-syntax/src/parser.rs` — the quote body is parsed by this parser and nothing else. A separate template
  parser is the sublanguage-by-subtraction root `AGENTS.md` forbids, and it is also how a quote starts disagreeing with
  the language about what an expression is.
- Prompt [138](138-typed-syntax.md)'s `Derived` representation and its identity law — quotation is the thing that mints
  derived nodes at scale, so a bug here shows up as two nodes that should be distinct sharing an identity.
- `stdlib/src/adapters/staff.musa`'s `// ---- writing` section — the code this replaces, and the measurement prompt 145
  makes.
- `crates/musa-compiler/src/expand/mod.rs` — where the phase runs, and where a quote's `at here` node comes from.

## Design

**The body is real Musa, parsed by the real parser, with holes.** `$x` and `$..xs` are the only additions to the grammar
inside a quote. Everything else is the language, which is what makes a quote readable as the thing it produces rather
than as a tree-building program. The formatter formats quote bodies as the code they are.

**Categories are checked at the splice, not at the end.** `$x` at an expression position requires `Syntax<Expr>`; the
mismatch names both categories and points at the splice. `$..xs` requires a sequence of the position's category and is
only grammatical where the surface grammar has a repetition — an argument list, a block's items, a pattern list — which
means the check is a fact about the grammar rather than a special case list to maintain by hand.

**Hygiene, stated as two rules.** An identifier written literally in a quote refers to what it referred to at the
quote's site; an identifier arriving through a splice keeps its own binding. Spliced syntax is never captured by a
binder written in the quote. This is the same discipline §7's kernel quote already keeps between kernel and host
identifiers, and the implementation should share the alpha-renaming machinery rather than growing a second one.

**Provenance is minted, not passed.** Every node written literally in a quote gets `Derived { origin, quotation, path }`
computed from its position in the quote's own tree; every spliced node keeps the identity it arrived with. The author
writes no path, no index, and no role integer, and there is no operation that lets them. The property test worth
writing: two structurally identical sub-quotes inside one quote have distinct derived identities, and the same quote
elaborated twice produces the same ones.

**The locus rule is the one that will be got wrong.** A hole in a `let` value is instantiated once, at the `let`'s own
locus, and each later reference places the finished syntax where it is written. Copying §7's rule into a test rather
than into prose is the difference between it holding and it being documented.

**Cost and budget.** A quote elaborates to a construction whose size is the quote's size; splicing is not substitution
into a copied tree, so a large quote spliced many times must not multiply. Say what the representation shares and what
it copies, and put the phase budget on the construction the way every other phase operation is charged (prompt 127dcec's
rule: a value is charged where it is constructed).

**Laws.** A quote with no splices elaborates to a `Syntax<Cat>` whose printed form is the quote body. Splicing a
`quote { e }` into a hole is the same as writing `e` there — the substitution law, and the one that makes quotes
composable. Category mismatch is a compile error, per pair. Hygiene: the capture program does not capture. Derived
identity is distinct per literal position and stable across runs. Charging is at construction.

## Target

- `musa-syntax`: the quote grammar, CST, formatter, and highlighting; tree-sitter and its drift test.
- The phase-side elaboration of a quote into a `Syntax<Cat>` construction, with splice checking, hygiene, and minted
  provenance.
- New `Code` variants and `musa explain` text for category mismatch at a splice, a sequence splice at a non-repetition
  position, and a capture refusal.
- `crates/musa-compiler/tests/suite/quotation_laws.rs` with the substitution, hygiene, identity, locus, and charging
  laws, plus the compile-fail cases.
- Prompt 132's staff-construction program, compiling, as a fixture.
- `docs/plan/code-map/` rows.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-syntax -p musa-compiler -p musa-calculus
cargo nextest run --workspace
cargo clippy --all-targets -p musa-syntax -p musa-compiler -p musa-calculus -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Implement quotation, splicing, and automatic provenance`.

## Stop

- No pattern form. Prompt 140 owns destructuring, and the two are separable: one builds, one matches.
- No merge of `quote at here { … }` with `kernel T { … }`, and no third quotation form.
- No string-to-syntax operation, no runtime `eval`, no procedural macro over token streams, no unhygienic escape, and no
  way to read provenance from inside a quote.
- No fresh-name operation. If a program needs one, that is a finding and a repair of `11-quotation.md`, not an addition
  made here.
- No rewrite of `stdlib/src/adapters/staff.musa`. Prompt 145.
- No template dialect and no second parser for quote bodies, in any form, including "a small one just for tokens".
