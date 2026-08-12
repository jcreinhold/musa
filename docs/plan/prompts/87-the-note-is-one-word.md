---
id: 87
slug: the-note-is-one-word
status: done
depends_on: [86, 57, 62]
phase: 2
---

# The Note Is One Word

## Task

Make a duration cost one character. Across `examples/`, 35% of non-blank lines are single-note statements averaging 7.5
characters, of which 54% is the duration — so the language spends more ink on how long a note lasts than on which note
it is. This prompt adds `c4/4` as an exact synonym for `c4 1/4`, adds the augmentation dot, and introduces the
`Duration` node the shorthand needs in order not to corrupt every note in the compiler. The corpus is not touched: this
prompt is green with every example exactly as it stands.

## Read

- `docs/plan/roadmap.md`, the duration section: *"Canonical duration syntax should be fractions of a whole note… The
  editor may display familiar note symbols and accept shortcuts such as `q`, `h`, or `e`, but those should elaborate
  into exact values."* A shorthand that elaborates to the same rational is already sanctioned; this is that sentence
  taken up, with `/4` instead of `q` because `/4` says which fraction.
- `crates/musa-language/src/lexer.rs` — `logos`, maximal munch with backtracking. `c4` and `1/4` are each **one** token,
  there is no `/` token at all, and a lone `/` lexes as an error.
- `crates/musa-compiler/src/resolve.rs` `parse_duration`, and `crates/musa-language/src/edits.rs` `set_duration`. Both
  take *the first `Rational`-or-`Integer` token under the statement node*. That pattern is why this prompt introduces a
  node.
- `crates/musa-render/src/plan.rs` `beam_unit`.

## Design

### Why `/` and not `:`

`:` already carries three meanings — a named argument (`ratio: 2`), a motif parameter's type (`root: pitch`), and a
measure-beat position (`section "Exposition" at 1:1`). The third is decisive: a reader who has learned that `:` joins
two numbers into a *position* meets `c4:4` and has to unlearn it. `/` means exactly one thing in musa today, and it is
this one — `1/4`, `meter 7/8`, `tuplet 3/2`, `tempo 1/4 = 96`. It is also unshifted, it sits better beside `[c3 g3]/2`,
and it lets the short and long forms use the same character for the same idea.

### Beat groups belong at the bottom, and there is a bug proving it

`plan::beam_unit` returns `1/8` for 7/8: it has no irregular-meter table, so an engraved bar of 7/8 beams as seven
separate eighths. Prompt 90's formatter needs the same fact to group `2+2+3` in the text, and two readers disagreeing
about one musical fact is what this repo's drift tests exist to prevent. So the table goes at the bottom of the graph,
in `musa-language`, and `beam_unit` calls it:

```rust
/// How a bar of `numerator/denominator` divides into the groups a player
/// hears — the fact a beam draws and a beat group is spaced by. Each group is
/// its length counted in `1/denominator` units, so the groups sum to the
/// numerator and `7/8` answers `[2, 2, 3]`.
pub fn beat_groups(numerator: u32, denominator: u32) -> Vec<u32>
```

`7/8 → 2+2+3`, `5/8 → 3+2`, `9/8 → 3+3+3`, `6/8 → 3+3`, `12/8 → 3+3+3+3`, `5/4 → 3+2`, `7/4 → 2+2+3`. Otherwise groups
of three when the denominator is 8 and the numerator is a multiple of 3 greater than 3, else groups of one.

The groups are counted in denominator units rather than returned as `Ratio<i64>` because both callers already hold the
denominator and neither is helped by the fraction: `beam_unit` divides an onset by the group anyway, and prompt 91
accumulates exact `(u64, u64)` pairs precisely so that `musa-language` — the bottom of the graph — does not grow
`num-rational` for one table. Every group in every case above is a whole number of denominator units, so nothing is
lost.

### Five tokens

`Slash` `/`, `Pipe` `|`, `Greater` `>`, `Caret` `^`, `Hash` `#`. Four are for prompts 88 and 89 and are added here
because a new `SyntaxKind` is a compile error in five exhaustive matches, and doing that once is cheaper than four
times. None needs a `priority`: none collides with another pattern at equal length, and maximal munch keeps every
existing spelling. That claim is a test, not a comment — `//x`, `/*…*/`, an unterminated `/*`, `1/4`, `->`, `|>`, `a-1`
and `0.55` each get a case.

### A `Duration` node

Four navigators find a duration by taking the first `Rational`-or-`Integer` token under a statement node, and every one
of them breaks **silently** under `c4/4`, where that token is the numeral `4`:

- `resolve::parse_duration` — on the path of every note, rest and chord. Every quarter would become a whole note.
- `edits::set_duration` — would write `c4/3/8`.
- `ast::NoteStmt::duration`, and `RestStmt`/`ImproviseStmt` beside it.
- the formatter, where `spaced_before` lists `Dot` in `closes_right`, so `c4/4. d4/4` would print `c4/4.d4/4`.

A node turns four silent wrong answers into four `None`s, and it collapses `ast::held_to` — which exists only to avoid
counting tokens by position — into "the second value inside `Duration`".

```
Duration[ Rational | Integer | Identifier ]     c4 3/8, c4 1, a parameter reference
Duration[ Slash, Integer, Dot* ]                c4/4, c4/4., c4/4..
```

Scoped to the four productions that call `Parser::duration()`. **Not** `tuplet 3/2`, `stretch`, or `meter 4/4`, whose
`Rational`s are ratios and meters — a node there would claim a kinship that does not exist.

`/N` with `d` dots is `(1/N)·(2 − 2⁻ᵈ)`: `/4.` is 3/8, `/4..` is 7/16. Dots are legal **only** after a `Slash` numeral.
`c4 3/8.` is refused, because a dotted 3/8 is 9/16 and the long form already writes that.

### The spelling a duration stores

`NotatedDuration::spelling` reaches diagnostics, the desktop inspector, and every kernel golden. `c4/4.` stores `"3/8"`,
not `"1/4."`: two spellings of one duration must not become two facts. The CST is the record of what the composer typed,
and `score.rs`'s doc comment saying otherwise is amended in this commit.

## Target

- `crates/musa-language/src/meter.rs` (new): `beat_groups`, exported from `lib.rs` and re-exported by `musa-compiler`
  beside `musa_kernel::SemanticHash`, so `musa-render` reaches it without a new edge in the graph. `musa-render`'s
  `beam_unit` loses its own answer and becomes `beat_group_at`, which returns the group a given onset falls in — a
  uniform unit cannot describe 2+2+3.
- `crates/musa-language/src/lexer.rs`, `syntax_kind.rs`, `highlight.rs` (`SPELLINGS` and `TokenClass::of`),
  `keywords.rs`, and `tests/tree_sitter_fixtures.rs`'s `tree_sitter_name`: five tokens, five exhaustive matches.
- `crates/musa-language/src/parser.rs`: `Parser::duration` wraps a `Duration` node and accepts the short form.
- `crates/musa-language/src/ast.rs`: a `Duration` wrapper; `NoteStmt`/`RestStmt`/`ImproviseStmt::duration` and `held_to`
  go through it.
- `crates/musa-language/src/formatter.rs`: `Slash` tight both sides; `Dot` leaves `closes_right`; `ParamPath` joins
  `Position | ChordSymbol` in the tight-node list, which is where the `Dot` rule actually belonged.
- `crates/musa-compiler/src/resolve.rs`: `parse_duration` reads the node, and understands `/N` and dots.
- `crates/musa-language/src/edits.rs`: `set_duration` replaces the `Duration` node's value range — which stops before
  `to`, so rewriting a note's value leaves the performer's bound alone. The `spell_duration` helper this prompt once
  listed here arrives in prompt 89 instead, with the `Statement::text()` rewrite that is its only caller: a helper
  landed a commit before anything calls it is a public item with no caller, which this repo does not keep.
- `editors/tree-sitter-musa/grammar.js` + regenerated `src/parser.c`, and `queries/highlights.scm`.
- `apps/musa-desktop/ui/src/lib/session/generated/spellings.json`, regenerated.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
for f in examples/*.musa examples/album/pieces/*.musa; do cargo run -p musa -- check "$f"; done
```

New laws: every maximal-munch claim above, one case each; `c4/4` and `c4 1/4` elaborate to the same fact; `/4.` is 3/8
and `/4..` is 7/16; `c4 3/8.` is a diagnostic, not 9/16; and a bar of 7/8 beams 2+2+3.

The corpus is unchanged, so every kernel golden, compiler and render snapshot, lexed fixture and tree-sitter fixture
must pass untouched — that is this prompt's real check, and if one moves, something silently changed meaning. The two
exceptions are named in advance because they are the change itself: `parser__invention_parses_cleanly` and
`parser__glass_mountain_parses_cleanly` are pictures of the parse tree, and this prompt adds a node to it. Their diff
must be nothing but a `Duration` wrapper appearing around durations that are otherwise byte-identical.

## Stop

- **No corpus conversion.** `c4 1/4` stays legal and stays exactly as written; prompt 89 converts.
- **No `musa format` rewriting `c4 1/4` into `c4/4`.** The formatter rewrites whitespace, and the law that says so
  (`format_preserves_semantics`, whose oracle is the non-whitespace token sequence) is worth more than the convenience.
- **No `q`/`h`/`e` letter shorthands.** `/4` says which fraction; a letter has to be learned.
- **No sticky or carried-forward duration.** Notation does not do it, and it costs working memory the reader needs for
  the music.
- **No use of `|`, `>`, `^` or `#` yet.** They are lexed here and given meaning in 88 and 89.
