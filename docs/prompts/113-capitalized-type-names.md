---
id: 113
slug: capitalized-type-names
status: pending
depends_on: [110, 112]
phase: 3
---

# A Type Is Spelled With A Capital

## Task

Give every type name and every constructor of one an `UpperCamelCase` spelling — `Pitch`, `Music`, `Triad`, `Pc12`,
`Option`, `List`, `Some`, `None` — so that the type layer has a spelling of its own and stops sharing six words with the
music statement grammar. `pitchclass` becomes `SpelledPc`, settling a name the compiler and the specification disagree
about. No type is added, none is removed, and none changes meaning.

## Read

- `docs/language-correction.md` §7, which governs this prompt, and §10's last two bullets for what it does not touch.
- `docs/language/01-surface.md` §3, which lists the primitive value types and spells the pitch-class type `spelled_pc`
  where `crates/musa-compiler/src/core.rs` spells it `pitchclass`.
- `crates/musa-language/src/parser.rs`, `type_atom` — the whitelist of six keywords a type name is allowed to be is the
  evidence for this prompt, not an incidental detail.
- Prompt 109 for the migration-diagnostic shape a spelling change takes here, including its applicable fix.
- Prompt 80 for the tree-sitter drift law, and prompt 84 for the keyword documentation table that is exhaustive by
  construction.

## Design

The six words `pitch`, `music`, `scale`, `key`, `degree`, and `frame` are each a type name *and* a music statement
keyword. `key c major;` sets a key; `key` is also the type of what it sets. The parser survives this by letting
`type_atom` accept a fixed list of keywords in type position, which is a whitelist that must grow every time the two
grammars touch the same word again. A capital settles it in the lexer instead: `Key` is a type, `key` is a statement,
and no lookahead is needed to tell them apart.

The full vocabulary after this prompt:

```text
Unit  Bool  Nat  Ratio  Duration  Pitch  SpelledPc  Interval
Scale  Key  Degree  Frame  ChordClass  Triad  Roman  Voicing
Pc12  PcSet12  Row12  Music  Option  List        None  Some
```

`pitchclass` becomes `SpelledPc` rather than `PitchClass`. `01-surface.md` §3 already calls it `spelled_pc`, and that
name carries the distinction that matters: in a `SpelledPc`, C♯ and D♭ differ; in a `Pc12` they do not. A reader who
sees `PitchClass` beside `Pc12` has to be told which is which.

`Some` and `None` move with their type, as in Rust, because they are constructors of `Option` and not free words. The
alternative — capitals for types, lowercase for their constructors — asks a reader to hold two rules where one will do.

Casing is not enforced on user names. This prompt fixes the spelling of the types the compiler owns; there is no
user-defined type in the language, so there is nothing to lint. When one arrives, the convention is already set.

Every old spelling becomes a **hard error with an applicable fix**, on prompt 109's precedent and for its reason: a
language that accepts both spellings has a mixed corpus forever. Each removed word stays lexed so the diagnostic can
point at it, exactly as `module` does in prompt 111.

## Target

- The capitalized vocabulary in the lexer, `type_atom`, the CST node kinds, the compiler's type table (`core.rs`'s name
  → `Type` mapping and its `Display`), and the formatter.
- `pitchclass` renamed to `SpelledPc` in the compiler and in `docs/language/01-surface.md` §3, with the two documents
  agreeing afterwards.
- The migration diagnostic for every lowercase type name and for `some`/`none`, each located at the word, carrying an
  applicable fix, with snapshot tests.
- Every type annotation, constructor, and `match` arm rewritten across `examples/`, `stdlib/`, and the test corpora.
- `editors/tree-sitter-musa` grammar, corpus, and queries updated, with the lexer drift law green.
- LSP completion, semantic tokens, and prompt 84's keyword documentation table.
- Prompt 93's frozen compatibility baseline updated for this deliberate break, with the break *named* in the baseline
  rather than absorbed into it.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-lsp -p musa-project
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-lsp -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/tonal-construction.musa
cargo run -p musa -- check examples/module-functor-study.musa
cargo run -p musa -- format examples/tonal-construction.musa --check
npm --prefix editors/tree-sitter-musa test
```

Commit as `Spell a type with a capital`.

## Stop

- No change to the set of types, to their inhabitants, or to any typing rule. This prompt changes how types are written.
- No user-defined types, no type aliases, and no casing lint on user names.
- Do not change the bracket around a type parameter; prompt 114 owns that, and `Option[Pitch]` is the correct
  intermediate spelling until it runs.
- Do not accept both spellings, and do not add a compatibility flag or edition mechanism to allow the old ones.
- No change to the music statement keywords themselves: `key c major;`, `scale …`, and `music { … }` are untouched.
- No renaming of Rust items. `Type::PitchClass` in the compiler may keep its name; this prompt is about the surface.
