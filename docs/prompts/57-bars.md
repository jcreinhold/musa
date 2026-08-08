---
id: 57
slug: bars
status: done
depends_on: [07, 40, 49, 56]
phase: 2
---

# Bars

## Task

The language gains the unit every musician already thinks in. `bar { … }` groups a run of events, checks that it adds
up to one measure, and can be named where it is written so a later bar can play it again. A bar is a brace you can
select, copy, and paste, and a name you can reuse — and, because it declares what it claims to be, it is the first
construct in musa that can be *wrong* in a way musa can point at.

## Read

- `docs/course-correction.md` §§3–5 — the surface elaborates into the kernel; it does not add to the ontology. A bar
  adds no operation, no payload, and no second notion of time.
- Prompt 07 (`NotationPlan`) — measures are **already** computed from the meter. This prompt must not create a second
  measure representation; AGENTS.md's layer table forbids exactly that collapse.
- Prompt 40 and `MeterMap` — read what meter actually is today before designing on top of it. It is one meter for the
  piece, not a map, and that is what defers irregular lengths (see Design).
- Prompt 49 — `repeat` and motifs elaborate to kernel `let`. A named bar is a `let`, bound where it is written.
- Prompt 56 — the secondary-label and help machinery. The bar-length diagnostic is unreadable without it.
- `crates/musa-language/src/{lexer,parser,ast,formatter}.rs`; `crates/musa-compiler/src/elaborate.rs`.

## Design

### The one idea

**A bar is an assertion with music inside it.** Musa can already tell where the barlines fall — it does it on every
page. What it cannot do is notice that the composer disagreed with it. A voice is a flat stream of durations, so a
dropped `1/4` in the fourth bar does not produce an error; it produces every later bar being wrong, silently, and a page
the composer has to proofread against their own intentions. Writing the bar down turns that from proofreading into a
diagnostic.

Everything else the brief asks for — copy, paste, name, reuse — follows from having a delimiter. The check is what
makes the delimiter worth typing.

### The grammar

```
bar { c5 1/4; e5 1/4; g5 1/4; e5 1/4; }     // one measure of the prevailing meter
bar head { a4 1/2; c5 1/2; }                // sounds here, and binds `head`
use head;                                    // plays it again, anywhere later
```

`bar` is a voice item, legal wherever a note is. Bars do not nest, and the parser is where that is said: a bar claims to
be one measure, and a measure inside a measure is not a thing the notation has a mark for. A voice may mix bars and
loose events — adopting bars is per-bar, which is what keeps every existing `.musa` file valid and lets a composer bar
the passage they are arguing with and leave the rest alone.

### Named bars are named material

A named bar joins the namespace motifs already live in, and `use name;` — no parentheses — plays it. One namespace,
because "material with a name" is one idea, and because the composer who mistypes it should get one diagnostic that
knows about both:

```
error: cannot find `hed`
help: did you mean `head`?
```

Names are piece-scoped like motifs, declared where they sound. A second declaration of the same name is an error whose
secondary label is the first one. A bar declared in one voice can be used in another; the point of naming a bar is that
the cello can answer the violin without the notes being typed twice.

Where a motif may be used is settled by declaration order; where a *bar* may be used is settled by position, because a
bar both declares and sounds. One comparison covers it: a `use` that starts before the bar's span ends is either a
forward reference (`use head;` above `bar head { … }` — move one of them) or the bar quoting itself (`bar loop { use
loop; }` — write the notes out). Each gets its own sentence, and between them recursion is impossible without a
cycle-checker.

### What a bar means

Nothing. A bar elaborates to exactly what its contents elaborate to — the braces are erased after they are checked. A
named bar elaborates to a `let` and its uses to references, which is prompt 49's machinery unchanged. There is no bar
in the kernel, no bar in `ScoreSnapshot`, and no bar in the notation plan, because the notation plan already knows where
measures are.

### What the check can honestly say

A bar's **contents** are checked against the prevailing meter. Its **position** is not. Inside a motif body the absolute
position is unknowable — that is the point of a motif — so "does this bar start on a barline" is a question this prompt
cannot answer for the general case and does not pretend to. Checking the total is the whole of the value anyway: a
correct total is exactly the condition under which the barline the engraver draws lands where the composer put the
brace.

### Irregular lengths are deferred, and the reason is honest

*(Paid by prompt 64. `MeterMap` is a `ContextTrack<Meter>`, the meter changes where the music changes, and all four
exporters write the change. An irregular bar is written as the two meter statements it is; the `bar 5/4 { … }` sugar
and the pickup stayed out, for the reasons prompt 64 records.)*

This prompt was designed with `bar 5/4 { … }` and a `bar 1/4 { … }` pickup, on the argument that an irregular length is
a meter occurrence and goes where prompt 40 put meter. Prompt 40 did not put it there. `MeterMap` holds **one** meter
for the piece, and `plan.rs`, MEI, LilyPond, and MusicXML all read it as one — no exporter can emit a meter change
mid-piece. Accepting `bar 5/4 { … }` today would produce a page that disagrees with the source, which is worse than not
accepting it. Mid-piece meter needs its own prompt: `MeterMap` becomes a map, and every exporter learns to write the
change. Until then the syntax is not accepted, so nothing has to be un-taught later.

### The diagnostic this exists for

```
  × this bar is 1/4 too long
    ╭─[examples/broken/bar-too-long.musa:12:17]
 11 │                 bar { c4 1/4; d4 1/4; e4 1/4; f4 1/4; }
 12 │                 bar { g4 1/4; a4 1/4; b4 1/4; c5 1/4; d5 1/4; }
    ·                 ───────────────────────┬───────────────────────
    ·                                        ╰── these add up to 5/4
 13 │             }
    ╰────
  help: shorten a duration, or move the last of these into the next bar
  note: `meter 4/4` makes a bar 1
```

The rule is a **note**, not a second label. It was drafted as a label on the `meter` statement, which reads well on this
page and badly on a real one: `meter` is thirty lines up, so miette draws two framed snippets and the composer's eye
leaves the bar that is wrong. One sentence carries the same information without moving the reader.

Under-full says `short` and offers the rest that would fill it as a fix, since that edit is unambiguous — inserted at
the bar's last content token, not at its `}`, so it lands inside the braces on the same line. Over-full offers no fix:
which note to remove is the composer's decision, and prompt 56's rule is that an uncertain fix is a help line.

The code is prompt 56's existing `does-not-add-up`, not a new pair. A bar that misses its measure and a tuplet that
misses its bracket are the same mistake at different scales, and the composer who has read one explanation has read
both.

Advice about a piece that does not compile is advice about a piece that does not exist, so the measure-sanity and
tuplet warnings are suppressed once an error has been reported. Without that, a bar one quarter long also produces
"voice `right` in part `piano` stops part-way through measure 3" — true, useless, and stacked on top of the diagnostic
that explains it.

### Copy and paste is a formatting decision

A bar that fits on one line is written on one line:

```
bar { c5 1/4; e5 1/4; g5 1/4; e5 1/4; }
```

That is the difference between a bar you can select with a double-click and drag into the next voice, and six lines you
have to count. The formatter keeps a bar inline while it fits and breaks it the way a block breaks when it does not — and
a bar carrying a comment always breaks, because a `//` swallows everything after it on the line.

The budget is 96 columns, not the 48 of `01-visual-language.md` §8. A bar sits four levels in — piece, score, part,
voice — so it starts at column 16, and four quarter notes with pitches and durations is another 40; at 48 no real bar
would ever fit and the rule would be decoration. 96 fits a bar of eight eighths, which is the case worth fitting. The
consequence is stated rather than hidden: a barred piece has lines past the 48-column measure, and the source column
scrolls them, as it already does for comment prose and long method chains. §8's 48 was measured on a pre-bars corpus and
is now the narrower claim; widening the column is an interface change and is not this prompt.

## Target

- `crates/musa-language`: `BarKw`; the `BarStmt` syntax kind and AST wrapper; parsing with recovery and the
  bars-do-not-nest error; `use name;` without parentheses; the formatter's inline-bar rule and its `proptest`
  idempotence check. No `BarRef` kind — a bar is played by the `use` that already exists.
- `crates/musa-compiler`: bar-length checking against the prevailing meter with the diagnostic above, under prompt 56's
  `does-not-add-up`; named bars registered into the motif namespace as zero-parameter material, distinguished by a
  `Material` tag so a diagnostic can say "bar" or "motif"; forward reference and self-quotation as one span comparison;
  warnings suppressed once an error is reported.
- `examples/`: `twinkle.musa`, `invention.musa`, and `counterpoint.musa` rewritten in bars — they are executable
  specifications, so the feature is not real until they use it — plus `refrain.musa`, which is the named bar's
  specification: a three-note refrain declared once and played three times, in two parts. `glass-mountain.musa` keeps
  its unbarred voices on purpose; something has to go on proving that loose events still work.
  `examples/broken/bar-too-long.musa` and `bar-too-short.musa` for the goldens.
- `docs/initial-design-roadmap.md` §7.2 (source language design): the bar section. Not §5 as first written — §5 is the
  algebraic foundation, and a bar adds nothing to the algebra. That it has no §5 section is the design being true.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler
cargo clippy --all-targets -p musa-language -p musa-compiler -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- check examples/refrain.musa                   # a named bar, played three times
cargo run -p musa-cli -- check examples/broken/bar-too-long.musa       # the diagnostic above
cargo run -p musa-cli -- format examples/glass-mountain.musa --check
```

Semantic identity is the real check: barring an existing example must not change the timeline's shape, its times, or its
payloads. Provenance spans do move, necessarily — the text moved — so the assertion is on everything else, and the
kernel goldens' diff is the evidence. A bar that changes the music is a bug in this prompt.

## Stop

- No repeat barlines, no endings — prompt 58.
- No multi-bar named runs. A name binds one bar; a run of bars is what `repeat` and motifs are for.
- No automatic barring of existing files, and no lint that asks for bars. Adopting them is per-bar and voluntary.
- No bar numbers in the surface. `bar 12 { … }` would be a position, and positions are computed, never written.
- No irregular bar lengths and no pickups — they need mid-piece meter, which needs its own prompt (see above).
- No barline-alignment check. Only the bar's own total is checked.
