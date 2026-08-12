---
id: 88
slug: sharps-and-flats
status: done
depends_on: [87]
phase: 2
---

# Sharps And Flats

## Task

Spell an accidental the way a musician does. `fs3` and `ef4` are decoded; `f#3` and `eb4` are read — `#` and `b` are
what every DAW, chord chart and lead sheet uses, and the letters `s` and `f` were only ever a workaround for a character
set nobody is short of. This prompt changes the pitch literal, adds the `PitchClass` node the change needs where an
accidental appears without an octave, and migrates the whole corpus.

## Read

- `crates/musa-language/src/lexer.rs` — the pitch regex, and the test asserting a bare `a`–`g` lexes as an `Identifier`
  and not a pitch.
- `crates/musa-compiler/src/pitch.rs` — `WrittenPitch::parse`, `PitchClass::parse`, and their `Display`s.
- `crates/musa-language/src/parser.rs` — `key_stmt`, which expects two `Identifier`s, and `chord_symbol`, which takes an
  `Identifier`-or-`PitchLiteral` then an optional `Integer`.

## Design

### The literal

```
[a-g](##|bb|[#bn])?-?[0-9]+
```

`b` is both a letter and a flat, and there is no ambiguity because the letter is always at position 0: `b2` is B, `bb2`
is B♭, `bbb2` is B𝄫. A pitch is letter, then optional accidental, then octave, and nothing else can start where a pitch
starts.

**The octave stays required.** Making it optional so a bare `a`–`g` could be a pitch would break `key a minor;`,
`mobile { a; b; c; }`, and every motif parameter named `a` — the lexer asserts this deliberately today, and it stays
asserted.

### Where an accidental has no octave

Two places, and both break under the naive change:

- `key g# minor;` would lex as `Identifier("g")`, `Hash`, `Identifier("minor")` — three tokens where the parser wants
  two. Worse than uniformly broken: `key bb major;` still lexes as one identifier, so flats would keep working while
  sharps silently stopped.
- `f#m7` as a chord symbol would be four tokens.

So a `PitchClass` node, built by the parser as `Identifier Hash{0,2}` — flats need no `Hash` because `bb`, `gb` and
`abb` are already one identifier — and printed tight by the same mechanism `Position` and `ChordSymbol` already use.
`resolve::parse_key` reads the node's text rather than one token, and `chord_symbol` accepts `Hash`.

This is the asymmetry the design has to live with: a sharp is a separate token and a flat is not. It is invisible in the
source and confined to one node.

## Target

- `crates/musa-language/src/lexer.rs`: the regex, and cases for `b2`/`bb2`/`bbb2`/`f#3`/`c##3`/`en5`/`a-1`.
- `crates/musa-language/src/parser.rs`, `ast.rs`, `syntax_kind.rs`: the `PitchClass` node, used by `key` and
  `chord_symbol`. **Not `invert around`**, whose axis is a whole pitch (`invert around c5`) and so is one `PitchLiteral`
  however it is spelled — the node exists only where the octave is absent.
- `crates/musa-language/src/formatter.rs`: `PitchClass` joins the tight-node list.
- `crates/musa-compiler/src/pitch.rs`: `WrittenPitch::parse`, `PitchClass::parse`, both `Display`s.
- `crates/musa-compiler/src/resolve.rs` and `elaborate.rs`: the two diagnostics that offer "an optional `s` or `f`".
- `crates/musa-language/tests/suite/formatter.rs`: the proptest `pitch()` generator.
- `editors/tree-sitter-musa/grammar.js` (`pitch_literal`, `pitch_class`, `chord_symbol`) + regenerated `src/parser.c`
  and the corpus expectations; `apps/musa-desktop/ui/src/lib/lang-musa/tokenize.ts` (`bb2` already works; `f#3` needs
  the `#`).
- The three places outside the compiler that spell an accidental in the language's own vocabulary and would otherwise
  keep writing `s`/`f`: `crates/musa-project/src/midi.rs` (`accidental`), the desktop's `steps.ts` (`ACCIDENTALS` and
  the accidental `CYCLE`) and `entry.svelte.ts` (the `Accidental` type). LilyPond's `s`/`f` and MEI's `s`/`x`/`f` are
  those formats' own spellings and do not move.
- Every `examples/**/*.musa`, and every golden, snapshot and fixture that follows: 24 `*.musa.kernel`, the compiler and
  render `insta` snapshots, `fixtures/lexed/*.json`, `*.snapshot.json`, and the tree-sitter token fixtures.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
UPDATE_KERNEL_GOLDENS=1 cargo test -p musa-compiler
for f in examples/*.musa examples/album/pieces/*.musa; do cargo run -p musa -- check "$f"; done
```

The migration is a respelling and nothing else, so the check that matters is that **no rendered output changes**:
regenerate the goldens, and every MEI, LilyPond, MusicXML and MIDI snapshot must differ only where a pitch name is
printed in the source's own spelling.

## Stop

- **No optional octave**, at any point, for any reason. `c4` is scientific pitch and the digit is the octave.
- **No Unicode accidentals** (`♯`, `♭`) in source. They are output, not input.
- **No `x` for a double sharp.** `##` composes; `x` is a fourth thing to learn.
- **No change to how a pitch is spelled anywhere but the source language** — MEI, MusicXML and the kernel keep their own
  spellings, which are not musa's to choose.
