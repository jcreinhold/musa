---
id: 58
slug: repeats-and-endings
status: done
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
    ending 1 {
        bar { e5 1; }
    }
    ending 2 {
        bar { a5 1; }
    }
}
```

Pass *k* plays the body, then the ending numbered *k*. Rules, each with its own diagnostic:

- Endings come last in the body and are consecutive from 1.
- The number of endings may not exceed the repeat count. `ending 3` under `repeat 2` names a pass that never happens,
  and the diagnostic says so with the count's span as its second label.
- Fewer endings than passes is legal: the last ending covers the remaining passes, which is what `1.–3.` means on a
  volta bracket.
- An ending outside a `repeat` is an error whose help is the shape above.

All four reuse `Code::Misplaced` from prompt 56 rather than taking codes of their own. A code is a rule a composer can
look up with `musa explain`, and "this is not where that goes" is one rule; four codes for four spellings of it would
be four pages saying the same sentence. The sentence that differs is the message, and the message is where the
difference belongs.

`ending` parses wherever a note does, and the compiler — not the parser — reports one written outside a repeat. A
syntax error there could only say the grammar disagreed; the compiler can say what the composer meant to write.

### What reaches each side

| | Score | Performance |
| --- | --- | --- |
| body | once, between `|:` and `:|` | *n* times |
| `ending k` | once, under a volta bracket labelled *k* | on pass *k* only |

The timeline keeps the notes for every pass — playback, `musa render --to wav`, and the semantic hash are unchanged by
this prompt, and that is the invariant its tests assert. What changes is that the repeat also *states on the timeline
that it is one*, as a `FactKind::Repeat { times }` over all its passes and a `FactKind::Ending { bracket, pass }` over
each ending's region, and the notation plan reads those statements and prints the bound material once.

Reading the `let` prompt 49 emits would not have been enough. The plan sees a `ScoreSnapshot`, which is already the
kernel evaluated — the sharing is gone by then, and the alternative was to thread terms into a layer whose whole job is
that it does not have them. A fact survives evaluation, which is what facts are for.

### Notated position ≠ performed position

Printing the body once means the page is shorter than the performance, and everything downstream of the plan —
measure numbers, tempo and section marks, barlines — is positioned by absolute time. So the plan computes a **fold**:
the sorted list of performed intervals the page drops (each pass after the first, minus the endings that print), and
one function that maps a performed moment to its notated one by subtracting the dropped length before it. The kept
intervals in performed order turn out to be exactly print order, so the fold reorders nothing.

The fold lives in `plan.rs` and nowhere else. This is the point: two clocks are tolerable in one function and
intolerable spread across four backends, so the plan is the only code that ever holds both, and `NotationPlan` is
stated wholly in notated time.

### One system, one repeat

A repeat barline is drawn across the whole system, so a repeat that one voice writes and another does not cannot be
drawn at all. Only repeats that *every sounding voice* states identically fold; the rest are written out on the page,
which is exactly what musa did before this prompt, so nothing regresses. The composer gets a warning — `Code::Ignored`, since something
was skipped and the piece still plays — that says the passage is written out and why.

This is a real limit, not a shortcut. Two voices repeating different spans is not a page that exists.

### Why not a new keyword

A separate `reprise { }` for notated repeats and `repeat n { }` for expansion would let a composer write the same music
two ways and get two different pages from it. There is one way to say "play this again" and it is the one that is
already there.

### LilyPond writes the brackets by hand

`\repeat volta 2 { … } \alternative { … }` is the idiomatic spelling and it is unusable here: it needs the repeat to
be a syntactic container in the emitted `.ly`, and musa emits a flat run of measures with barlines between them.
Reshaping the LilyPond backend around one construct's nesting would complicate every other thing it prints. Instead
each barline carries a `\set Score.repeatCommands`, which is the escape hatch LilyPond provides for exactly this, and
the commands at one barline are merged into a single `\set` — two adjacent `\set`s do not compose, the second wins,
and an `end-repeat` silently lost that way is a wrong page.

Volta brackets are written in the topmost staff's first lane only, as engravers write them.

## Target

- `crates/musa-language`: `EndingKw`; `EndingStmt`; parsing inside `RepeatStmt`; highlighting; recovery through
  `VOICE_RECOVERY`.
- `crates/musa-compiler`: ending placement and count rules, all as `Code::Misplaced`; `FactKind::Repeat` and
  `FactKind::Ending` with their `factext` spellings; `RepeatRegion`/`EndingRegion` on `AnnotationStore`; the
  every-voice-agrees rule and its warning.
- `crates/musa-render`: the fold, `RepeatMark`/`VoltaMark` on `NotationPlan`, and repeat barlines and volta brackets in
  the MEI, LilyPond, and MusicXML backends; MIDI unchanged, because performance was already correct.
- `examples/repeats.musa`: two parts, two endings; goldens at every backend.
- `examples/broken/ending-outside-repeat.musa` and `examples/broken/ending-past-the-count.musa`: the rendered reports,
  snapshotted as prompt 56 established.
- `docs/initial-design-roadmap.md` §2 (the position row and what varies) and §7.2 (the repeat section).

Found along the way, and repaired here:

- `musa format --check` wrote a `.recovery` copy beside every file it rejected. It formatted the document to find out,
  and an unsaved edit is autosaved. `ProjectSession::is_formatted` answers the question without making the edit.
- `format_check_passes_on_canonical_examples` checked one example, so the rest had drifted out of canonical form.
  It now checks all of them, and `canon.musa` and `variation.musa` are reformatted to match.
- Four stray `.recovery` copies had been committed by earlier prompts. Deleted, and `*.recovery` is now ignored: an
  autosave copy sits beside the file it recovers and is deleted on save, so none of them belongs in history.

An `ending`'s block breaks across lines like every other block; only `bar` prints on one line. An ending is a
container of bars, and the one-line rule is about bars.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-render
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-render -- -D warnings
cargo fmt --check
cargo run -p musa -- render examples/repeats.musa --to mei -o -    # one body, |: :|, two endings
cargo run -p musa -- render examples/repeats.musa --to midi        # every pass, as before
```

The performance must be byte-identical to what it is today for every existing fixture. A repeat that sounds different
after this prompt is a bug in it.

## Stop

- No `D.C.`, `D.S.`, `Fine`, `Coda`, or segno. They are a jump table over the whole piece, not a bracket over a passage,
  and they need the form model prompt 35 started — their own prompt.
- No repeat of a non-contiguous selection, and no nested repeats. Nesting is legal notation and rare enough to wait for
  someone who wants it.
- No playback-only or engraving-only repeats. One statement, two projections, or the layer table means nothing.
