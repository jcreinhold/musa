---
id: 140
slug: syntax-patterns
status: done
depends_on: [139]
phase: 3
---

# Match Syntax by Quoting the Shape You Mean

## Task

Implement quotation as a pattern: `match node { quote { $a + $b } => … }`. This is the inverse of prompt 139 and the
second of the two ways into a syntax value that prompt 131 amended `00-semantics.md` §2 to allow. It is what deletes the
string-dispatch table from adapter code.

## Read

- `docs/rules/language/11-quotation.md`'s pattern section and its two rules — a pattern quote binds only splice
  variables, and matching is on syntactic shape rather than provenance. Both are refusals with teeth and both need a
  test. Its two example programs are normative about where a spread may stand: the second is a block, so a spread in a
  pattern is not restricted to the comma-separated positions construction restricts it to.
- `docs/rules/language/00-semantics.md` §2 as amended — why a second descent is allowed and what makes this one
  controlled. If the implementation ends up needing more than one level of decomposition per pattern, that is evidence
  against the amendment and a repair, not a widening made in passing.
- Prompt [135](135-inductive-families.md)'s case-tree compilation and coverage checker — a syntax pattern is a pattern,
  and it goes through the same compiler. Do not build a second matcher; a syntax pattern that misses coverage checking
  is a `match` with different rules from every other `match` in the language.
- `stdlib/src/adapters/staff.musa`'s dispatch table and prompt 132's staff-dispatch program — the code this replaces and
  the program that says what the replacement should read like.
- Prompt [127dca](127dca-text-patterns-match.md) and `crates/musa-compiler/tests/suite/literal_pattern_laws.rs` —
  literal patterns and their laws, which a `TokenTree` pattern sits beside.
- Prompt [127dcfaf](127dcfaf-syntax-step-recursor.md) — `recurse_syntax` stays for traversal of unknown shape, and this
  prompt must say in its tests where each of the two belongs, or authors will pick by habit.

## Design

**A pattern quote is a pattern, in the ordinary `match`.** It compiles through prompt 135's case tree, so coverage,
unreachability, and refutability all work the way they do everywhere else. The scrutinee's category is known from its
type, so the pattern's category is checked, not guessed.

**Only splice variables bind.** `quote { $a + $b }` binds `a` and `b`; nothing else in the pattern binds anything. The
alternative — literal identifiers in a pattern binding whatever they name — reads well in one example and is a trap in
every other, because a typo in a literal token silently becomes a wildcard. The refusal message says which identifier
was taken literally and how to bind it if that was meant.

**Matching sees shape, not provenance.** A node derived by prompt 139's quotation and a node read from source match the
same pattern when they have the same shape. This keeps expansion results interchangeable with source, which is the
property the whole adapter design rests on, and it means there is deliberately no way to ask "was this written by a
human" from inside a pattern.

**Trivia is not shape.** A pattern matches modulo trivia — comments and whitespace between tokens do not defeat a match
— because the alternative makes every adapter fragile against formatting. Say what "modulo trivia" means precisely,
including for a sequence splice, and test the case where trivia sits between two spliced elements.

**Sequence patterns bind sequences.** `$..xs` binds the run of siblings it stands among, and it may stand among any
group's children — which is *wider* than construction, where a spread needs the comma its position supplies.
`11-quotation.md` §4's second example is `quote { { $..items } }` and a block separates nothing, so the narrower rule
would refuse the specification's own program. The asymmetry is not an oversight: writing a run needs a separator to put
between its elements, and reading one needs only a run to bind. Exactly one open sequence splice per group, because two
would need search, and search is what this language keeps refusing.

**When to use which, written down.** A pattern quote decides a known shape; `recurse_syntax` traverses an unknown one.
The tests carry one program of each kind side by side, and the doc comment on each names the other. This is the cheapest
possible defence against a future adapter that reimplements traversal out of patterns or dispatch out of the recursor.

**Laws.** Match after build is the identity: matching `quote { $a + $b }` against a value built by `quote at here { $x +
$y }` binds `a` to `x` and `b` to `y`. Coverage and unreachability behave as for any other pattern. A literal identifier
does not bind. Provenance does not affect matching, tested with a derived and a source node of the same shape. Trivia
does not affect matching. Two open sequence splices in one group are refused.

## Target

- `musa-syntax`: pattern-position quote grammar, CST, formatter, highlighting; tree-sitter and its drift test.
- Pattern elaboration through prompt 135's case-tree compiler, with category checking, splice binding, sequence
  patterns, and trivia-insensitive matching.
- New `Code` variants and `musa explain` text for a literal identifier in a pattern quote, two open sequence splices,
  and a category mismatch in pattern position.
- `crates/musa-compiler/tests/suite/syntax_pattern_laws.rs` with the laws above, the side-by-side pattern/recursor
  programs, and the compile-fail cases.
- Prompt 132's staff-dispatch program, compiling, as a fixture.
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

Commit as `Match syntax by quoting the shape you mean`.

## Stop

- No second matcher. Syntax patterns go through the one case-tree compiler or this prompt is not done.
- No provenance observation from a pattern, no "is this derived" question, no origin binding.
- No pattern guards, no view patterns, no or-patterns beyond what the language already has, and no regular-expression
  matching over token trees.
- No more than one open sequence splice per repetition, and no backtracking.
- No deletion of `recurse_syntax` or the string-dispatch builtins. Prompt 143 removes what is dead after 145 and 146
  prove it is.
- No rewrite of `stdlib/src/adapters/staff.musa`. Prompt 145.
