---
id: 127dcf
slug: staff-package
status: pending
depends_on: [127dcea]
phase: 3
---

# Write the Staff Package as Ordinary Unprivileged Musa

## Task

The first half of the first trial. Write the staff *package* — the data an expansion produces and the ordinary functions
over it, including `realize` and `engrave` — in ordinary unprivileged Musa, with no adapter and no region yet. This is
the value shape the adapter must be able to produce, established before anything has to produce it.

## Read

- `docs/notes/research/language-design-closure/27-adapter-trials.md` §2.2 in full — the value expansion produces and
  what the fold over it is for (validation, engraving, analysis, conversion) — and §2.4, what expansion deliberately
  does *not* decide, which is the list of choices `realize` and `engrave` make instead.
- `docs/notes/research/language-design-closure/26-language-design-decision.md` §5 — written rhythm and exact time are
  different data even when they cover the same span, which is why `c5/4.` and `c5(3/8)` are two constructors and not
  one.
- `docs/rules/constitution.md` §7 and the roadmap §2 layering table: written pitch is not MIDI number, notated duration
  is not performed duration. The staff package owns staff concepts; the core owns none of them.
- Open Music Theory `001`–`012` for the distinctions the data must be able to make — spelling versus pitch class,
  written value versus exact span, meter versus hypermeter — and `docs/rules/style-guide.md` for spellings.
- Prompt 127dcea's exact-time operations: `duration_add`, `position_shift`, `position_between`, `duration_scale`,
  `follow`, `track_duration`. These are what `realize` computes with, and they are the reason this prompt can be written
  at all.
- `crates/musa-compiler/src/data.rs`, `folded` — a generated fold replaces *one constructor layer*, so a recursive
  occurrence under a container arrives unfolded. This is what makes §2.2's shape unwritable as printed and is the reason
  for the first paragraph of **Design** below.

## Design

**The value shape is direct recursion, not lists of children.** §2.2 writes `Bar(Anchor, Ratio, List<StaffItem>)` and
says the generated fold is what validation, engraving, analysis, and conversion use. In this language it is not: a
generated fold replaces one constructor layer, so a `List<StaffItem>` field arrives as the list it is, its elements
never folded, and there is no recursion to fold them with. A sequence is therefore written as direct recursive fields —
each item carries the rest of its sequence, and each nested form carries its body and its rest — which is foldable, says
the same thing, and is the shape a total traversal can actually be written over. §2.2's content is still what the trial
has to produce; only the constructors' fields change.

The data covers all fourteen items the next prompt's expansion must reach: notes, rests, chords, dots, exact durations,
ties, slurs, tuplets, grace notes, pickups, repeats, alternate endings, meter changes, and transposing instruments. It
carries written facts and an anchor, and nothing else. A note records what is written and never what it will sound like;
`WrittenDuration` records a note value and its dots, and an exact duration is a separate constructor, because §5 says
they are different data.

**`realize` and `engrave` make the later choices, and they are where a missing choice is an error.** `realize` turns
written facts into exact time — resolving a tuplet's ratio, a dot's extension, a tie's joined span, a pickup's offset —
by folding with prompt 127dcea's operations and accumulating positions. `engrave` makes layout choices. Grace timing is
a performance profile's, not this package's, and `realize` says so by refusing rather than guessing. Both return
`Result` where the choice can fail, so the diagnostic lands where the choice is made rather than where the note was
written.

Everything here is ordinary Musa: declarations, generated folds, and the compiler-owned operations any package may use.
Nothing in this prompt runs in the expansion phase, so the whole-module scope an adapter does not get is available, and
the functions may be written as ordinary sibling declarations.

## Target

- `stdlib/src/notation/staff.musa` — `WrittenDuration`, `StaffItem`, `StaffDocument`, their generated folds, and the
  ordinary functions over them including `realize` and `engrave`.
- Tests constructing each of the fourteen items directly as data and folding over it: the fold is total, `realize` gives
  the exact span the written value means, and a tie across a barline realizes as one joined span.
- Tests for each refusal: grace timing, and any other choice `realize` declines to guess.
- A test that no item inherits register or duration from an earlier item — each is read in isolation and means the same
  thing.
- `docs/book/src/reference/stdlib.md` naming the module and what it is for.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Write the staff package as ordinary unprivileged Musa`.

## Stop

- No adapter, no `syntax staff { … }` region, and no expansion; prompt 127dcfa carries those.
- No `edit`, no `print`, and no conformance level; prompt 127dcfb carries those.
- No compiler privilege and no new compiler-owned operation. If the package needs one, that is evidence against prompt
  127dcea's operation list and repairs it rather than being granted here.
- No deletion of contextual `Music` and no notation migration of the corpus — prompt 127e owns both.
- No studio package; prompt 127dcg carries it.
- No change to `docs/rules/`.
