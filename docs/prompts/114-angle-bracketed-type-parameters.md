---
id: 114
slug: angle-bracketed-type-parameters
status: done
depends_on: [113]
phase: 3
---

# A Type Parameter Is Angle-Bracketed

## Task

Replace `Option[τ]` and `List[τ]` with `Option<τ>` and `List<τ>`, so that `[` means one thing in this language — a list,
as a literal or as a pattern — and the type layer has its own bracket. This is notation. It adds no way to write a type
constructor, and the language still has exactly two.

## Read

- `docs/language/01-surface.md` §1, which governs this prompt, and `docs/scratch/60-language-decision-record.md` for why
  this is not the introduction of parametric polymorphism.
- `docs/language/01-surface.md` §1, whose `type` production carries `"option" "[" type "]"` beside a `list` expression
  production and a list pattern that both also spell `[`.
- `crates/musa-language/src/parser.rs`, `type_atom` and `expr_atom` — the two readings of `[` that this prompt
  separates.
- `crates/musa-language/src/parser.rs`, `at_articulation` — `>` is already a token, as the accent inside a bar. That is
  the one place the new bracket has to be shown not to collide.
- Prompt 109 for the migration-diagnostic shape, and prompt 80 for the tree-sitter drift law.

## Design

`[` currently reads three ways: a list literal `[c4, d4]`, a list pattern `[x, ..xs]`, and a type parameter
`List[Pitch]`. The first two are one idea seen from two sides. The third is unrelated and shares the character by
accident of an early choice.

The ambiguity that makes `<>` expensive in other languages does not exist here, and the reason is worth stating rather
than assumed. `Option` and `List` are the only parameterized types; both are keyword-headed; and there is no
user-written type application anywhere in the language. So the parser knows it is reading a type before it reaches the
`<`, and the `a < b > (c)` reading that forces Rust's turbofish has no term that could produce it. `<` is not currently
a token at all. `>` is — as the accent articulation — and a type position is never inside a bar, so the two readings
never meet.

```musa
fn root_of(t: Triad) -> Option<Pitch> { Some(chord_root(t)) }
let voices: List<Music> = [];
```

A lexed `<` is `Less` and a lexed `>` stays `Greater`; the type parser consumes the pair, and the music parser's
`at_articulation` is untouched. No `>>` token is introduced, because nesting a parameterized type inside another is
already writable — `List<Option<Pitch>>` is three tokens at the end, not one — and inventing a compound token to split
later is a cost with no payer.

The `[τ]` spelling becomes a **hard error with an applicable fix**, on prompt 109's precedent.

## Target

- `Less` in the lexer; `type_atom` reading `Option` `<` τ `>` and `List` `<` τ `>`; the CST node kinds and the formatter
  following.
- The migration diagnostic for `Option[τ]` and `List[τ]`, located at the brackets, with an applicable fix that rewrites
  the pair, and a snapshot test.
- Every type parameter rewritten across `examples/`, `stdlib/`, and the test corpora.
- `docs/language/01-surface.md` §1's `type` production, and every example in `docs/language/` that writes one.
- `editors/tree-sitter-musa` grammar, corpus, and queries updated, with the lexer drift law green.
- LSP completion and semantic tokens.
- Prompt 93's frozen compatibility baseline updated for this deliberate break, with the break *named* in the baseline
  rather than absorbed into it.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-lsp -p musa-project
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-lsp -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/tonal-construction.musa
cargo run -p musa -- check examples/serial-forms.musa
cargo run -p musa -- format examples/tonal-construction.musa --check
npm --prefix editors/tree-sitter-musa test
```

Commit as `Angle-bracket a type parameter`.

## Stop

- No user-written type constructors, no type parameters on functions, and no parametric polymorphism. `Option` and
  `List` remain the two the compiler owns.
- No `>>` token, no turbofish, and no lexer state for angle-bracket nesting.
- No change to the list literal `[e, …]` or to the list pattern `[x, ..xs]`; they keep `[` and are the reason it is
  worth freeing.
- No change to `>` as the accent articulation inside a bar.
- Do not accept both spellings, and do not add a compatibility flag or edition mechanism to allow the old one.
