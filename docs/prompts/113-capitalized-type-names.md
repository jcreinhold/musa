---
id: 113
slug: capitalized-type-names
status: done
depends_on: [110, 112]
phase: 3
---

# A Type Is Spelled With A Capital

## Task

Give every type name and every constructor of one an `UpperCamelCase` spelling — `Pitch`, `Music`, `Triad`, `Pc12`,
`Option`, `List`, `Some`, `None` — so that the type layer has a spelling of its own and stops sharing six words with the
music statement grammar. `pitchclass` becomes `NoteName`, settling a name the compiler and the specification disagree
about — and that neither of them had right. No type is added, none is removed, and none changes meaning.

## Read

- `docs/language/01-surface.md` §1, which governs this prompt, and `docs/scratch/60-language-decision-record.md` for what it does not touch.
- `docs/language/01-surface.md` §3, which lists the primitive value types and spells the pitch-class type `spelled_pc`
  where `crates/musa-compiler/src/core.rs` spells it `pitchclass`.
- Open Music Theory `099-pitch-and-pitch-class.md` and `003-reading-clefs.md`, which decide what this type is called:
  the first defines a pitch class as octave *and enharmonic* equivalence, the second calls the octave-free spelled thing
  a letter name.
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
Unit  Bool  Nat  Ratio  Duration  Pitch  NoteName  Interval
Scale  Key  Degree  Frame  ChordClass  Triad  Roman  Voicing
Pc12  PcSet12  Row12  Music  Option  List        None  Some
```

`pitchclass` becomes `NoteName`, and neither `PitchClass` nor `SpelledPc` is the answer. OMT 99 defines a pitch class as
a group of pitches related by octave *and enharmonic* equivalence, so a type in which C♯ and D♭ differ is not a pitch
class — `Pc12` is. What this type holds is a letter and an accidental with the octave dropped, which OMT 3 calls a
letter name. `NoteName` is that in a word every musician already has, and it carries the distinction without explaining
it: a name is a spelling, so of course two names spell two things.

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
- `pitchclass` renamed to `NoteName` in the compiler and in `docs/language/01-surface.md` §3, with the two documents
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
