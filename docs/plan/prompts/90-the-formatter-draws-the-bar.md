---
id: 90
slug: the-formatter-draws-the-bar
status: done
depends_on: [89, 87]
phase: 2
---

# The Formatter Draws The Bar

## Task

Put one bar on one line, and show the beat groups in the whitespace. Prompt 89 made `|` a separator; newlines are
trivia, so where a bar goes is the formatter's decision and this is where it is made. The device is notation's own: a
beam groups the beats a player hears, and a double space is what a beam looks like in text.

## Read

- `docs/plan/roadmap.md`: *"The formatter should operate on syntax, not on the expanded semantic model."* That is the
  constraint the whole design bends around — the formatter may not ask `musa-render` for a `NotationPlan`, because that
  is upward in the dependency graph.
- `crates/musa-language/src/formatter.rs` — all of it, especially `MEASURE`'s nineteen lines of rationale, `inline_bar`,
  `format_token`, and `spaced_before`.
- Prompt 87's `beat_groups`, which is the shared answer this prompt spaces by and `beam_unit` beams by.

## Design

### Meter cannot be tracked lexically

`part_decl` accepts `meter` in any order, so a part's meter can be written *after* its voices and still governs them. A
single left-to-right pass would get that wrong. So: one pre-walk, still purely syntactic, building `piece_meter` and
`part_meter` per `PartDecl`; then inside a voice a `VoiceItem::Meter` overrides lexically from where it is written,
because that one *is* a change written where it happens. `meter none;` means no grouping.

### The measurability rule, as a whitelist

Stated once, and as a whitelist so that a statement kind added next year defaults to the safe answer instead of a wrong
guess:

> A bar is **measurable** when every direct item is a `NoteStmt`, `RestStmt` or `ChordStmt` — each contributing its
> written duration — or a `DynamicStmt` or `MarkStmt`, each contributing nothing; when every duration in it is a
> `Rational` or `Integer` literal; when a meter is in force and is not `none`; and when the durations sum exactly to the
> measure. **Otherwise every gap in that bar is a single space.**

Per bar, not per file: one unmeasurable bar does not silence its neighbours. What falls back, and why each is right:

- a bar containing `use foo()` — the material's duration is a compiler fact and `musa-language` does not have it;
- a duration that is a parameter reference — same reason;
- `improvise` — it frames unnotated music, and drawing its interior to scale would claim something false;
- a `tuplet`, `grace`, `slur`, `repeat`, `ending`, hairpin, or any other nested block;
- a bar that does not add up — the grouping would be a lie about music `check_bar_length` is about to complain of.

### Wrapping

A `|`-bar has no braces, so `MEASURE`'s overflow path can no longer be "break like any other block". A bar too wide
wraps at its double-space boundaries, with continuation lines indented two further so they align under the first event
after `| `. With no boundaries to wrap at, the line runs long — a long line is better than a wrong one.

### One spacing table, not two

`spaced_before` is a second declarative copy of `format_token`'s rules, kept in step by hand so the single-line bar path
can reuse them. Prompt 87 and prompt 89 between them add six tokens, which would mean six edits in each of two places
that must agree. Delete it: render the bar through a `Writer` with line breaks suppressed, so there is one table and it
cannot drift from itself.

## Target

- `crates/musa-language/src/formatter.rs`: the meter pre-walk, `bar_line`, the measurability whitelist, the wrap rule,
  and `spaced_before` deleted.
- `crates/musa-language/tests/suite/formatter.rs`: `examples_format_to_themselves` re-pinned; the proptest `item()`
  generator producing `|`-bars.
- `examples/*.musa` re-formatted by the formatter itself, which is the check that it agrees with prompt 89's hand
  conversion.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo run -p musa -- format --check examples/bulgarian.musa
```

The two existing laws stay: `format_is_idempotent`, and `format_preserves_semantics` over the non-whitespace token
sequence. New: a measurable bar's gaps fall exactly on `beat_groups`' boundaries; an unmeasurable bar is spaced with
single spaces throughout; a bar whose durations do not sum to the measure gets no grouping.

By eye, which is the question the prompt exists to answer: `bulgarian.musa` reads as fifteen bars, and its 2+2+3 is
visible without the comment that used to explain it.

## Stop

- **No proportional spacing.** That is prompt 91, and it is optional; this prompt's spacing is the one every file gets.
- **No line-duration configuration.** `MEASURE` is already decided, in writing, at duration.
- **No token rewriting.** The formatter rewrites whitespace.
- **No asking `musa-render` anything.** The dependency points the other way, and `beat_groups` is why it does not need
  to.
- **No cross-voice alignment.** Voices are separate blocks in the text and cannot be aligned vertically; the property
  this prompt buys is that beat *n* is in the same column on every line of *one* voice.
