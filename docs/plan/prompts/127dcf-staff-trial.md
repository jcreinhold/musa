---
id: 127dcf
slug: staff-trial
status: pending
depends_on: [127dce]
phase: 3
---

# Write the Staff Adapter as an Unprivileged Package

## Task

The first of the two trials that prove the boundary. Write the complete staff adapter and the staff package it expands
into, as ordinary unprivileged Musa, and discharge the fourteen-item coverage list item by item.

## Read

- `docs/notes/research/language-design-closure/27-adapter-trials.md` §2 in full — the source block, the value it
  produces, how the two folds work, what expansion deliberately does *not* decide, the two required diagnostics, and the
  structured edit. §2.1's block is paper Musa and its layout is not this language's; the *coverage* is what transfers,
  not the indentation.
- `docs/notes/research/language-design-closure/26-language-design-decision.md` §5 — written rhythm and exact time are
  different data even when they cover the same span, which is why `c5/4.` and `c5(3/8)` are two constructors and not
  one.
- `docs/rules/constitution.md` §7 and the roadmap §2 layering table: written pitch is not MIDI number, notated duration
  is not performed duration. The staff package owns staff concepts; the core owns none of them.
- Open Music Theory `001`–`012` for the distinctions the trial must be able to make — spelling versus pitch class,
  written value versus exact span, meter versus hypermeter — and `docs/rules/style-guide.md` for spellings.
- Prompts 127dcc, 127dcd, and 127dce: anchors, `edit`, `print`, and the conformance levels the staff adapter must reach.
- `crates/musa-compiler/src/data.rs`, `folded` — a generated fold replaces *one constructor layer*, so a recursive
  occurrence under a container arrives unfolded. This is what makes §2.2's shape unwritable and is the reason for the
  first paragraph of **Design** below.
- `crates/musa-compiler/src/core.rs`, `run_transformer` — an adapter's `expand` is checked as one expression with no
  symbols, no modules, and no world, which is the second.

## Design

The staff package declares the data (`WrittenDuration`, `StaffItem`, `StaffDocument`) and the ordinary functions over
it; the adapter declares only `expand`, `edit`, and `print`. Nothing in the compiler learns a staff concept, and the
adapter never receives an inferred type, a private parser, or compiler state. If it needs one, that is evidence against
the boundary and repairs 127da or 127dc rather than being granted.

**The value shape is direct recursion, not lists of children.** §2.2 writes `Bar(Anchor, Ratio, List<StaffItem>)` and
says the generated fold is what validation, engraving, analysis, and conversion to `Music` use. In this language it is
not: a generated fold replaces one constructor layer, so a `List<StaffItem>` field arrives as the list it is, its
elements never folded, and there is no recursion to fold them with. A sequence is therefore written as direct recursive
fields — each item carries the rest of its sequence, and each nested form carries its body and its rest — which is
foldable, says the same thing, and is the shape a total traversal can actually be written over. §2.2's expansion is
still what the trial has to produce; only the constructors' fields change.

**The adapter is one expression and has no scope of its own.** Its `expand`, `edit`, and `print` are each read alone: no
sibling `let` in the same module resolves, no `data` the module declares is in scope, and there is no local `let` inside
a block. Factoring is by immediately-applied lambda — `(fn (name) { … })(value)` — which is a real encoding of `let` and
is what the fourteen items are built with. This is a privilege the trial wants and does not get, and Target's last
bullet is where it is written down; granting it would repair 127dcb rather than this prompt.

Expansion records *written facts and nothing else*. `realize` and `engrave` are ordinary package functions that come
later and make the later choices, so a missing choice is an error where the choice is made rather than where the note is
written. Grace timing is a performance profile's, not the adapter's.

The coverage list is discharged item by item, each with its own fixture and its own test: notes, rests, chords, dots,
exact durations, ties across bars, slurs, tuplets, grace notes, pickups, repeats, alternate endings, meter changes,
transposing instruments, the two diagnostics of §2.5, and the structured edit of §2.6. No note inherits register or
duration from an earlier note: the block is locally readable, and a test asserts it by reading each event in isolation.

## Target

- `stdlib/src/notation/staff.musa` — the staff data and its ordinary functions, including `realize` and `engrave`.
- `stdlib/src/adapters/staff.musa` — the adapter, declared generative, with all three operations.
- `examples/staff-page.musa` — the trial block, covering all fourteen items, compiling and rendering.
- Tests: one per coverage item; the two §2.5 diagnostics landing on the composer's own text; the §2.6 edit changing one
  pitch and no other byte; the round-trip law for the printer.
- A statement, in the adapter's own comments, of any privilege it wanted and did not get.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Write the staff adapter as an unprivileged package`.

## Stop

- No compiler privilege, no private parser or checker access, and no inferred type reaching the adapter.
- No deletion of contextual `Music` and no notation migration of the corpus — prompt 127e owns both.
- No studio adapter; prompt 127dcg carries it.
- No freeze, no proofs, and no conformance script; prompt 127dd carries those.
- No change to `docs/rules/`.
