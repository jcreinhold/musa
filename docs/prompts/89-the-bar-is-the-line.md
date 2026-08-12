---
id: 89
slug: the-bar-is-the-line
status: done
depends_on: [88, 57, 62]
phase: 2
---

# The Bar Is The Line

## Task

Make the bar the unit of a line. Music's unit is the beat group; musa's is the note, and the consequence is
`examples/bulgarian.musa`, which spends 96 lines on fifteen bars and carries a comment apologising in prose for what its
layout hides. This prompt replaces the anonymous `bar { … }` block with a `|` separator, drops the semicolon from
events, replaces `chord [c3, g3] 1/2;` with `[c3 g3]/2`, gives accent and marcato the marks notation gives them, and
converts the corpus.

## Read

- `docs/roadmap.md` on `bar { }`: *"A bar means nothing… What the brace buys is the assertion."* The assertion is what
  survives; the brace is what does not. Also *"Newlines should be trivia, not syntax"* — which is why one bar per line
  is prompt 90's decision and not this one's.
- `docs/style-guide.md` §1 — a named bar "plays where it stands, so its name is an *address*". A named bar is material
  and keeps its block; an anonymous one is not and does not.
- `crates/musa-language/src/parser.rs` — `voice_items`, `bar_stmt`, `nested_bar`, `articulations`, `VOICE_RECOVERY`.
- `crates/musa-language/src/ast.rs` — `voice_items`, which already reads both a `Block` child and direct children.
- `crates/musa-language/src/edits.rs` — the whole mutation API, and the one place that *writes* note syntax.

## Design

### `|` reuses `BarStmt`

```
BarStmt[ Pipe, <items…> ]                  anonymous — a separator, as notation draws it
BarStmt[ BarKw, Identifier?, Block[…] ]    named — material, and it keeps its address
```

No new node kind, and `ast.rs` needs no change at all: `voice_items` already reads both shapes. Three consequences worth
asserting rather than assuming:

- `BarStmt::name()` reads only **direct** identifier tokens, and a `|`-bar's direct tokens are the pipe and trivia —
  every identifier is inside a `NoteStmt` or an `ArticulationList`. So a `|`-bar is anonymous **by tree shape**, not by
  a check that could be forgotten.
- `content_end()`, `lint::copied_bars` and `elaborate_bar`'s bar-length check all keep working unchanged, which means
  `|` inherits the assertion the brace was there for.
- `voice_items` gains `Pipe` to its exit condition **and** its dispatch, exit checked first, so `| a | b` closes one bar
  before opening the next and never trips `nested_bar`. What else closes it is below.

`bulgarian.musa` has no bars today and so has never been length-checked. Giving it barlines subjects it to
`check_bar_length` for the first time. It passes — eight bars of 7/8 and seven of 4/4 both come to 7 — but it is a
behaviour change and it belongs in the commit message.

### The semicolon leaves events, and stays everywhere else

Note, rest and chord lose their `;`. Context statements keep theirs and keep their own line: `dynamic mf;`,
`meter 7/8;`, `clef treble;`. That split is not arbitrary — it is where notation puts them, layered below the staff
rather than in the run of noteheads. Stated once, in the words the language should use everywhere it explains itself:

> **A note, a rest and a chord end themselves; every other statement ends with `;` or `}`.**

That sentence is the help line on the stray-`;` parse error and the rule paragraph under `musa explain syntax`. Both
readers meet it at the moment they need it, and neither gets a different version of it.

**A grace note ends itself too.** `grace { c5 staccato d5 }`, not `grace { c5 staccato; d5; }`. A grace note is a pitch
and the marks written on it — the same word an event is, minus the duration it does not have — so the same reason
applies, and applying it only to the three *statements* would leave the one place a pitch is written without a duration
as the one place a `;` is still required. The group's `}` ends the last grace note, which is what the rule's second
clause already says. Two consequences, both wanted: the formatter writes a grace group on one line for the same reason
it writes a bar on one line — a run of events is horizontal — and `grace_stmt` eats leading trivia before opening its
node, so the comment above a group stays above it rather than inside the line.

The `grace` *rule* in a `performance` profile — `grace { steal = 1/8; from = principal; }` — is untouched. Those are
settings, not events, and `steal = 1/8` is exactly the kind of statement whose `;` the rule keeps.

An event is self-delimiting, with one exception that has to be handled rather than hoped about. `articulations()` is
greedy over `Identifier`, and a pitch *reference* is also a bare `Identifier` — `examples/tuplet-fixture.musa` writes
exactly that shape inside a motif (`root 1/8 tenuto; d5 1/8;`). The `;` is what tells them apart today.

> **`articulations()` stops before an `Identifier` whose next significant token can open a duration** — `Slash`,
> `Rational` or `Integer`.

One token of lookahead on a kind, using the `nth_significant` helper that already exists for the same class of problem
in the studio grammar. `Identifier Identifier` needs no rule: a note's duration is read *before* the articulations are,
and the duration is mandatory, so the second word is the duration and there is nothing left to guess at.

### Where a `|` bar ends

A brace said where a bar ended. A pipe has to be told, and "at the next pipe" is not enough: `ending 1 { … }` written
under a `|` would otherwise be swallowed by the bar above it, and the bar would be an ending too long.

> **A `|` bar ends at the next `|`, at the end of its block, or before a statement that is itself at least a bar long**
> — `bar`, `repeat`, `ending`, `senza`, `mobile`, `improvise`, `transpose`, `retrograde`, `invert`, `stretch`, `phrase`.

Stated as a whitelist of what *continues* a bar — the events, and the things written among them: `use`, `dynamic`,
`clef`, `meter`, `key`, `tempo`, `mark`, `crescendo`, `diminuendo`, `tuplet`, `slur`, `grace`. A statement kind added
next year therefore ends the bar, which is wrong where the composer can see it, rather than lengthening the bar, which
is wrong where only the length check notices.

A brace resets it: a `tuplet` inside a `|` bar reads its own block, so `block()` clears the flag and restores it.

One consequence, and it is the right one: a voice that plays whole bars by name — `bar refrain { … }` followed by
`use refrain;` — keeps the braced form throughout, because `use` continues a bar and a named bar ends one. That is
`examples/refrain.musa`, which is the file about named bars, and it stays as it is written.

### Recovery gets better, not worse

`|` cannot appear inside an event, so it is a stronger resync anchor than `;` ever was: one malformed note poisons one
bar instead of running to the next statement keyword. `VOICE_RECOVERY` gains `Pipe` and `LBracket`.

For the semicolon a reader will type out of habit, `voice_items` gains an explicit arm: *"a note does not end in `;`"*,
with `with_fix("remove `;`", "")`. Deletion fixes already work — `SyntaxError::with_fix` replaces the error's own span
and `apply_edits` accepts an empty replacement — so this costs ten lines and is the whole migration story for every file
written before today.

### Chords, and two marks

`[c3 g3]/2`. The bracket already means chord in ABC and in GUIDO; the keyword and the commas were both saying what the
bracket says. `ChordKw` is deleted everywhere.

`>` is accent and `^` is marcato — notation's marcato *is* a caret — postfix and attached: `a5/4>`, `g5/2.^`. They go
**inside** `ArticulationList`, so `ast::articulation_names` stays the single reader and `elaborate::articulations_of`
needs no change at all. The mapping is a new `shorthand: Option<&'static str>` column on `MarkDef`, because a table is
what `marks.rs` is for.

Tenuto gets no mark, deliberately: `-` collides with `Minus`, with the negative octave in `a-1`, and `c4/4->` would lex
as the existing `Arrow`. Staccato gets no mark because `.` is spent on the augmentation dot, and notation tells those
two apart by vertical position, which text cannot. **Every articulation word stays legal**; these two are shorthand, not
a replacement.

### `edits.rs`, where the breakage is silent

| Site | What happens | Fix |
| --- | --- | --- |
| `Statement::text()` | writes `;`, `chord [a, b]`, and a long duration — the one place that *writes* note syntax | rewrite around prompt 87's `spell_duration` |
| `set_duration` | range anchored at the duration token | anchor at the **previous** token's end, so the space is part of the edit and long↔short both work |
| `extract_motif` | `head.parent() != tail.parent()` now fails for any two notes in different bars — the normal case | compare the nearest non-`BarStmt` ancestor |
| `insert` | writes `"\n{indent}{text}"`, splitting every one-line bar | inside a `BarStmt`, write `" {text}"` at `content_end()` — which is what that method's doc says it is for |
| `Anchor::EndOfVoice` | same | same |
| `set_pitch`, `item_syntax`, `is_statement` | **safe** — the pitch is still the first pitch-or-identifier token, and `VoiceItem::Bar` is reused | none; say so, so review does not chase them |

### The corpus is converted by hand

Not by a tool. These files are executable specifications, each conversion embeds a judgement about where the beat groups
fall, and a `musa migrate` subcommand would be dead surface the moment it finished.

```
voice melody {
    dynamic mf;
    | a4/8 b4/8  c5/8 b4/8  a4/8 g4/4
    | b4/8 c5/8  d5/8 c5/8  b4/8 a4/4
}
```

bulgarian's explanatory comment is deleted in the same commit, because the grouping it describes is now visible.

## Target

- `crates/musa-language/src/lexer.rs`: `ChordKw` deleted.
- `crates/musa-language/src/parser.rs`: `pipe_bar_stmt`, `voice_items`' exit and dispatch, `chord_literal`, the
  articulation lookahead, the marks inside `ArticulationList`, `VOICE_RECOVERY`, the stray-`;` arm and the rule in its
  help line, `grace_stmt`'s items and its leading trivia.
- `crates/musa-language/src/{ast,highlight,keywords}.rs`: `ChordKw` removed from the exhaustive matches; the marks
  mapped in `articulation_names`.
- `crates/musa-language/src/edits.rs`: the six rows above.
- `crates/musa-language/src/formatter.rs`: `>` and `^` close up to the note the way `/` and `.` do; an event ends its
  own line — nothing inside it does that now the `;` is gone; and `inline_bar` becomes `inline_run`, covering the grace
  group for the same reason it covers the bar. None of that is bar *layout*; that is still prompt 90.
- `crates/musa-compiler/src/marks.rs`: `MarkDef::shorthand`.
- `crates/musa-project/src/diagnostic.rs`: `explain("syntax")` states the rule, in the same words the parse error uses.
- `editors/tree-sitter-musa/grammar.js` + regenerated `src/parser.c`, and `queries/highlights.scm`.
- Every `examples/**/*.musa`, converted. `examples/broken/extra-semicolon.musa` is added — the `;` a reader who learned
  the old syntax types after a note. `broken/missing-semicolon.musa` stays: `clef treble` without its `;` is still a
  genuinely missing terminator, and it is the fixture `crates/musa-lsp/tests/lsp_laws.rs` reads, which this prompt may
  not touch. `broken/bar-too-short.musa` is rewritten with `|`.
- Every golden, snapshot and fixture that follows.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
UPDATE_KERNEL_GOLDENS=1 cargo test -p musa-compiler
for f in examples/*.musa examples/album/pieces/*.musa; do cargo run -p musa -- check "$f"; done
```

New laws: a `|`-bar's `BarStmt::name()` is `None` **by tree shape**, over generated sources; `| a | b` does not trip
`nested_bar`; every `MarkDef::shorthand` lexes as exactly one token; `a5/4>` and `a5/4 accent` elaborate to the same
fact; a stray `;` after a note produces the removal fix and applying it yields a source that parses clean;
`grace { c5 staccato d5 }` parses as two grace notes, the first carrying the mark.

Rendered output must not change except where the source's own spelling appears — this is a surface change, and
`bulgarian.musa` newly passing a bar-length check it was never subject to is the one intended difference.

## Stop

- **No `musa migrate` subcommand.** By hand, once.
- **No mark for tenuto, staccato, or slur.** The collisions are real and the words already work.
- **No `bar 5/4 { … }` sugar and no pickup syntax.** Both are rejected in the roadmap and stay rejected.
- **No removal of the named `bar`.** It is an address, and the style guide says why.
- **No one-bar-per-line formatting.** That is prompt 90; this prompt makes it possible and does not do it.
- **No `;` removed from `use`, `mobile`, a `performance` profile's `grace` rule, or any context statement.** A grace
  *note* loses its `;` because it is an event; a `steal = 1/8` setting keeps its because it is not.
