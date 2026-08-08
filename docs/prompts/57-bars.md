---
id: 57
slug: bars
status: pending
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
- Prompt 40 — meter is an occurrence on the one timeline. An irregular bar's length goes there and nowhere new.
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
bar 1/4 { g4 1/4; }                          // an explicit length: a pickup
```

`bar` is a voice item, legal wherever a note is. Bars do not nest. A voice may mix bars and loose events — adopting
bars is per-bar, which is what keeps every existing `.musa` file valid and lets a composer bar the passage they are
arguing with and leave the rest alone.

### Named bars are named material

A named bar joins the namespace motifs already live in, and `use name;` — no parentheses — plays it. One namespace,
because "material with a name" is one idea, and because the composer who mistypes it should get one diagnostic that
knows about both:

```
error: cannot find `hed`
help: the bar `head` is declared in this voice
```

Names are piece-scoped like motifs, declared where they sound. A second declaration of the same name is an error whose
secondary label is the first one. A bar declared in one voice can be used in another; the point of naming a bar is that
the cello can answer the violin without the notes being typed twice.

### What a bar means

Nothing. A bar elaborates to exactly what its contents elaborate to — the braces are erased after they are checked. A
named bar elaborates to a `let` and its uses to references, which is prompt 49's machinery unchanged. There is no bar
in the kernel, no bar in `ScoreSnapshot`, and no bar in the notation plan, because the notation plan already knows where
measures are.

The one exception is length. `bar 5/4 { … }` states that *this* measure is 5/4, which is a meter occurrence at that
position and is the same thing an engraver writes. It goes where prompt 40 put meter and nowhere else.

### The diagnostic this exists for

```
error: this bar is a quarter too long
    ╭─[song.musa:12:9]
  4 │     meter 4/4;
    │           ─┬─
    │            ╰── a bar here is one whole note
    ·
 12 │         bar { c5 1/4; e5 1/4; g5 1/4; e5 1/4; c5 1/4; }
    │             ────────────────────┬───────────────────
    │                                 ╰── these add up to 5/4
    │
    help: drop one quarter, or write `bar 5/4 { … }` if this bar really is longer
```

Under-full says `short by` and offers the rest that would fill it as a fix, since that edit is unambiguous. Over-full
offers no fix: which note to remove is the composer's decision, and prompt 56's rule is that an uncertain fix is a help
line.

### Copy and paste is a formatting decision

A bar that fits on one line is written on one line:

```
bar { c5 1/4; e5 1/4; g5 1/4; e5 1/4; }
```

That is the difference between a bar you can select with a double-click and drag into the next voice, and six lines you
have to count. The formatter keeps a bar inline while it fits the source measure and breaks it the way a block breaks
when it does not. This is the one formatting exception in the language and it earns itself: `01-visual-language.md` §8
already says musa source is a narrow, tall thing made of short statements, and a bar is the one statement that is
naturally horizontal, because that is the direction music is read in.

## Target

- `crates/musa-language`: `BarKw`; `BarStmt` and `BarRef` syntax kinds and AST wrappers; parsing with recovery; the
  formatter's inline-bar rule and its `proptest` idempotence check.
- `crates/musa-compiler`: bar-length checking against the prevailing meter with the diagnostic above
  (`bar-too-long`, `bar-too-short`); named bars into the material namespace with `duplicate-name` and the
  cross-referencing "did you mean" from prompt 56; irregular length as a meter occurrence.
- `examples/`: `glass-mountain.musa` and `counterpoint.musa` rewritten in bars — they are executable specifications, so
  the feature is not real until they use it; `examples/broken/bar-too-long.musa` for the golden.
- `docs/initial-design-roadmap.md` §5 (language reference): the bar section.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler
cargo clippy --all-targets -p musa-language -p musa-compiler -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- check examples/glass-mountain.musa            # unchanged meaning
cargo run -p musa-cli -- check examples/broken/bar-too-long.musa       # the diagnostic above
cargo run -p musa-cli -- format examples/glass-mountain.musa --check
```

Semantic identity (prompt 43) is the real check: barring an existing example must not change its hash. A bar that
changes the music is a bug in this prompt.

## Stop

- No repeat barlines, no endings — prompt 58.
- No multi-bar named runs. A name binds one bar; a run of bars is what `repeat` and motifs are for.
- No automatic barring of existing files, and no lint that asks for bars. Adopting them is per-bar and voluntary.
- No bar numbers in the surface. `bar 12 { … }` would be a position, and positions are computed, never written.
