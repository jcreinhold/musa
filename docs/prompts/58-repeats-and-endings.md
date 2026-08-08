---
id: 58
slug: repeats-and-endings
status: pending
depends_on: [49, 56, 57]
phase: 2
---

# Repeats and Endings

## Task

`repeat` becomes notation. A repeated passage is engraved once between repeat barlines and played the number of times
it says, instead of being copied onto the page; `ending 1 { … }` and `ending 2 { … }` give it first and second endings.
What the composer writes once, the page prints once, and the performance plays twice.

## Read

- Roadmap §2's layer table — *motif definition ≠ its expansions*, *notated duration ≠ performed duration*. A notated
  repeat and its performance are two representations of one statement, and this prompt is the case that proves the
  table is load-bearing.
- Prompt 06 (`repeat n { … }` as it exists), prompt 49 (it is already a kernel `let` referenced n times — the sharing
  this prompt needs is already in the term).
- Prompt 07 / `crates/musa-render/src/plan.rs` — where barlines are decided.
- Prompt 57 — bars. An ending is a run of bars, and repeat barlines fall on barlines.
- `docs/interface/02-engraving.md` — repeat barlines and volta brackets are Verovio's `<ending>` and `@right="rptend"`.

## Design

### The one idea

**`repeat` already means the right thing; it was never notated.** `repeat 4 { … }` says "play this four times", and
musa's answer today is four copies on the page. No engraver writes that, and no composer proofreads it. The fix is not a
new construct — it is to stop expanding the *notation* while going on expanding the *performance*.

This is the layer table's own example. One statement, two projections: the score prints `|: … :|`, the performance plays
it through four times. Nothing about the source changes, which means every existing `repeat` in the corpus is engraved
correctly the moment this lands.

### Endings

```
repeat 2 {
    bar { a4 1/2; c5 1/2; }
    ending 1 { bar { e5 1;   } }
    ending 2 { bar { a5 1;   } }
}
```

Pass *k* plays the body, then the ending numbered *k*. Rules, each with its own diagnostic:

- Endings come last in the body and are consecutive from 1.
- The number of endings may not exceed the repeat count. `ending 3` under `repeat 2` names a pass that never happens,
  and the diagnostic says so with the count's span as its second label.
- Fewer endings than passes is legal: the last ending covers the remaining passes, which is what `1.–3.` means on a
  volta bracket.
- An ending outside a `repeat` is an error whose help is the two-line example above.

### What reaches each side

| | Score | Performance |
| --- | --- | --- |
| body | once, between `|:` and `:|` | *n* times |
| `ending k` | once, under a volta bracket labelled *k* | on pass *k* only |

The timeline keeps the notes for every pass — playback, `musa render --to wav`, and the semantic hash are unchanged by
this prompt, and that is the invariant its tests assert. What changes is that the notation plan reads the `let` prompt
49 already emits, prints the bound material once, and marks the span with repeat barlines instead of walking every
reference.

### Why not a new keyword

A separate `reprise { }` for notated repeats and `repeat n { }` for expansion would let a composer write the same music
two ways and get two different pages from it. There is one way to say "play this again" and it is the one that is
already there.

## Target

- `crates/musa-language`: `EndingKw`; `EndingStmt`; parsing and formatting inside `RepeatStmt`; recovery fixtures.
- `crates/musa-compiler`: ending placement and count rules with their diagnostics (`ending-outside-repeat`,
  `ending-past-the-count`, `endings-out-of-order`); passes bound to endings in elaboration.
- `crates/musa-render`: repeat barlines and volta brackets in `NotationPlan`, and in the MEI, LilyPond, and MusicXML
  backends; MIDI unchanged, because performance was already correct.
- `examples/`: a fixture using `repeat` with two endings; goldens at every backend.
- `docs/initial-design-roadmap.md` §5: the repeat section, and the note that it is notation.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-render
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-render -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- render examples/repeats.musa --to mei -o -    # one body, |: :|, two endings
cargo run -p musa-cli -- render examples/repeats.musa --to midi        # every pass, as before
```

The performance must be byte-identical to what it is today for every existing fixture. A repeat that sounds different
after this prompt is a bug in it.

## Stop

- No `D.C.`, `D.S.`, `Fine`, `Coda`, or segno. They are a jump table over the whole piece, not a bracket over a passage,
  and they need the form model prompt 35 started — their own prompt.
- No repeat of a non-contiguous selection, and no nested repeats. Nesting is legal notation and rare enough to wait for
  someone who wants it.
- No playback-only or engraving-only repeats. One statement, two projections, or the layer table means nothing.
