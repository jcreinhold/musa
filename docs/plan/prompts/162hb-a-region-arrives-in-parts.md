---
id: 162hb
slug: a-region-arrives-in-parts
status: in-progress
depends_on: [162ha]
phase: 3
---

# A Region's Literal Arrives in Parts, and the Staff Adapter Reads Them

## Task

Prompt [162h](162h-literal-parts.md) gave `c#5`, `M3` and `3/8` the parts the lexer found, in the CST, and prompt
[162ha](162ha-a-literal-is-one-lexeme.md) taught the expansion phase the shape that holds them — a fused group, written
back with nothing between its children. What neither did is hand a *region's* literal over in parts, because the one
adapter that reads regions dispatches on `TokenKind.PitchLiteral` and would stop recognizing a note the day the token
became a group. Turn the reading over and move `stdlib/src/adapters/staff.musa` onto it in the same commit, which is
what makes `time 4/4` writable: the numerator and the denominator arrive apart, so `4/4` is no longer the `1/1` it
reduces to.

## Read

- [162ha](162ha-a-literal-is-one-lexeme.md)'s Design, the paragraph **"Reading waits for the reader that must read it"**
  — this prompt is the half it defers, and the measurement it defers on.
- `crates/musa-compiler/src/quote/read.rs`'s `read_node` — the one line that changes, and `read_region`'s doc comment,
  which describes the tree an adapter receives.
- `stdlib/src/adapters/staff.musa`'s file-head, the paragraph beginning "For a related reason `time (4, 4)` is two
  numbers rather than `4/4`" — the standing evidence, in the adapter's own words: "the lexer keeps `4/4` whole as one
  reduced rational, so `4/4` and `1/1` would arrive identical". The parts are what make that sentence false, and the
  paragraph comes out with the workaround it describes.
- `stdlib/src/adapters/staff.musa`'s `entering`, `group_read`, `token_read`, `sighted` and `kinded` — the reading path a
  fused group arrives at. `entering` answers `Ignoring` for a delimiter it does not know and `group_read` drops that
  group's children, so a pitch that becomes a fused group is *silently dropped* rather than diagnosed. Prompt 162ha
  measured it: 37 tests fail and the adapter refuses every region it is handed, saying "this staff field says nothing".
- `stdlib/src/adapters/staff.musa` around the tie, the comment beginning "checked by spelling because a spelling is all
  this adapter can see: `f5` tied to `f#5` is two texts" — which reads like the same kind of workaround as the `time`
  note and is **not** one. The parts partition the lexeme, so comparing the whole spellings *is* comparing the parts,
  and the verdict is already right. What parts would buy is a message naming which part differs, and the Design says why
  that is not this prompt's.
- `crates/musa-compiler/src/registry/rules.rs`'s `SyntaxOp::Number` — the operation that reads `3/8` as an exact
  rational, written against a `Token` and answering `None` for a group. `c5(3/8)` goes through it, so it moves with the
  representation or the exact span stops being readable.
- `crates/musa-compiler/tests/suite/staff_expansion_laws.rs` and `staff_writing_laws.rs` — the behaviour this rewrite
  must not move. **Those tests survive unchanged**, for prompt [166](166-staff-rewrite.md)'s reason: they were written
  against behaviour and not against implementation, so a rewrite that needs them edited has changed the adapter's
  meaning and has to say which.
- Prompt [166](166-staff-rewrite.md) — the staff rewrite, and the reason this is a prompt of its own rather than an
  amendment to it. 166 is `done`: it is the acceptance gate the constitution's amendment record points at, and its
  measurement is a finished measurement. Moving the adapter onto a representation that did not exist when it ran is new
  work, not a re-opening of that gate.
- Peyton Jones ch. 5 §5.2 — reading a constructor's arguments rather than its printed form, which is what an adapter
  that meets a numerator and a denominator is finally doing instead of comparing the whole spelling it was handed.

## Design

**One line turns the reading over.** `read_node` already asks `delimited` what a node's delimiter is, and `delimited`
already answers `Fused` for a composite literal — 162ha put it there for the quote body's sake. What remains is that
`read_node` stops special-casing the three kinds into a token, and the region an adapter receives has the letter, the
accidental and the octave as ordinary token children with ordinary paths. No new operation, and nothing the fold did not
already reveal.

**The adapter meets a lexeme where it met a token.** A fused group arrives at the traversal's group branch with its
parts as children, and it is intercepted there *before* that branch decides anything else: the parts are folded into the
kind and the spelling the adapter already dispatches on, and what comes out is handed to the very path a token took, so
the note path is the note path it already is. The kind comes from the parts' own kinds — a `PitchLetter` means a pitch,
a `RationalNumerator` means a rational — because a fused group has no kind of its own and its parts are what the lexer's
own pattern found.

The interception cannot live in `entering`. That function is consulted only in a body, and a chord's brackets hold
pitches while a form's parentheses hold a rational, so all three states meet a fused group and all three have to meet it
as a lexeme. What changes underneath is what the adapter may then ask — a pitch's letter apart from its accidental, and
a rational's numerator apart from its denominator — and one place takes it up:

1. **`time 4/4`.** The file-head's own note says why `time (4, 4)` is two numbers, and the reason was that
   `syntax_number` hands back a reduced `Ratio`, so `4/4` and `1/1` are one value. The numerator and the denominator
   arrive apart now, so the written form is readable, and `time (4, 4)` stays admitted beside it rather than being
   replaced — a staff already written is a staff that still reads.

   Reading them apart needs one thing of the reading operation: a `RationalNumerator` spells a numeral and
   `SyntaxOp::Number` refused it, because the operation asked whether the *lexer* called the lexeme a number and a part
   is minted by the parser. It answers for a part that spells one, by the same argument that put the part kinds in the
   phase's `TokenKind` — an adapter that can reach a numerator and cannot read the number it spells has been handed half
   an operation. The question is self-limiting rather than a list: a part whose text is not a numeral answers `None` by
   failing to parse, so `PitchLetter` needs no case of its own.

The tie is **not** the second place, though it reads like one. A tie compares the note it joins to the note that sounds
next by their spellings, and the comment says that is "all this adapter can see". The parts partition the lexeme, so
comparing the two whole spellings *is* comparing their parts, and the verdict was already right: `f5` tied to `f#5` is
refused, for the reason it is wrong. What parts would buy is a *message* naming which part differs, and that is a
sentence, not a verdict — Stop keeps it out. The comment is corrected; the code is not.

**What the parts do not buy.** They do not make a splice able to stand inside a literal: `$letter#5` is four tokens to
the lexer and there is no `PitchLiteral` for the parser to split, which 162ha measured and recorded. They do not give
the adapter a pitch value either — `docs/rules/language/11-quotation.md` §2's "no operation reads a text as source"
still holds, and a letter is a token like any other.

**Measure rather than assert.** The commit message carries the adapter's line count before and after. This prompt adds a
form rather than deleting one, so the number is expected to go up, and saying by how much is what keeps prompt 166's
gate honest about what has happened to the file since it ran.

## Target

- `read_node` reads the three composite literal nodes as fused groups, and 162ha's deferral comment comes out.
- `read_region`'s doc comment says what an adapter now receives.
- `SyntaxOp::Number` reads a fused group as the lexeme its parts spell, so `c5(3/8)` still names an exact span, and
  reads a part that spells a numeral as that numeral, so `4/4`'s two numbers are two numbers.
- `stdlib/src/adapters/staff.musa` reads a fused group in all three of its states, and every existing law in
  `crates/musa-compiler/tests/suite/staff_expansion_laws.rs` and `staff_writing_laws.rs` passes **unchanged**.
- `time 4/4` is admitted by the staff adapter, beside `time (4, 4)`, and the file-head's paragraph about why it could
  not be is replaced by what it now does. The tie's comment is corrected to say that a spelling comparison is a part
  comparison rather than a workaround for not having one.
- Laws in `musa-compiler`: a region's `c#5` arrives as a fused group whose children are the letter, the accidental and
  the octave; `time 4/4` and `time 1/1` are two different staves rather than one; `c5(3/8)` still names its exact span.
- One `examples/` fixture writes `time 4/4`, so the form is a regression fixture and not only a law.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-compiler -p musa-syntax
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Expansion snapshots move where a composite literal is inside a region, and each moved one must show a fused group and
nothing else. `tests/fixtures/elaboration-compatibility.txt` and every `examples/` rendering must be byte-identical
apart from the one fixture this prompt adds `time 4/4` to.

```sh
cargo nextest run --run-ignored all
```

Carries the reds prompt [164](164-builtin-collapse.md)'s Check enumerates; a *new* red is this prompt's.

Commit as `Hand a region's literal over in parts, and read them in the staff`.

## Stop

- No second reading operation. A part is a token child and the fold reads it; nothing gains a way to ask for a pitch's
  accidental other than by looking at the child that is one.
- No splice at a part, and no lexer change to make one possible. That is 162ha's measurement and it stands.
- No part data in `Sound`, `Hanging` or `Voice`. The tie's verdict is already right and its message is not worth that
  change; a prompt that wants the better sentence can have it later.
- No change to `Delimiter`, to the gate, or to `11-quotation.md` — 162ha owns all three and this prompt consumes them.
- No new staff notation. `time 4/4` is a spelling of a form the adapter already has; a form it does not have is not this
  prompt's.
- `doubled.musa` is untouched: it reads no literal.
