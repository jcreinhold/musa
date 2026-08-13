---
id: 127dcfa
slug: staff-expansion
status: in-progress
depends_on: [127dcf]
phase: 3
---

# Expand a Staff Region into the Staff Package, Item by Item

## Task

The second half of the first trial's reading side. Write the staff adapter's `expand`, discharge the fourteen-item
coverage list item by item with a fixture and a test each, land the two §2.5 diagnostics on the composer's own text, and
compile and render the trial block.

## Read

- `docs/notes/research/language-design-closure/27-adapter-trials.md` §2 in full — §2.1's source block, §2.2's value,
  §2.3's two folds, §2.4's deliberate silences, and §2.5's two diagnostics. §2.1's block is paper Musa and its layout is
  not this language's; the *coverage* is what transfers, not the indentation.
- Prompt 127dcf's staff package: this prompt produces its data and adds nothing to it.
- Prompts 127dcc and 127dcb: how an anchor reaches the value, and how a refusal points at the node it is about. Both
  diagnostics of §2.5 are refusals in that sense.
- `crates/musa-compiler/src/core.rs`, `run_transformer` — an adapter's `expand` is checked as one expression with no
  symbols, no modules, and no world, which is the reason for the second paragraph of **Design** below.
- `crates/musa-compiler/src/expand.rs`, the region grouper: `raw_group` makes one node per matched delimiter pair and no
  other rule, and `Whitespace` is one undifferentiated trivia kind. An adapter cannot see lines, and the staff spelling
  has to be readable without them.

## Design

The adapter declares `expand` and nothing else, and declares itself **readable**: its regions are read-only until prompt
127dcfb gives it `edit` and `print`. Under-promising is what prompt 127dce made legal, and it is what lets this prompt
end at a real boundary instead of a half-written adapter.

**The adapter is one expression and has no scope of its own.** `expand` is read alone: no sibling `let` in the same
module resolves, no `data` the module declares is in scope, and there is no local `let` inside a block. Factoring is by
immediately-applied lambda — `(fn (name) { … })(value)` — which is a real encoding of `let` and is what the fourteen
items are built with. This is a privilege the trial wants and does not get, and Target's last bullet is where it is
written down; granting it would repair prompt 127dcb rather than this prompt.

**The value expansion emits records written facts and nothing else.** A missing choice is not an error here: it is an
absence the package's `realize` refuses later, which is what §2.4 is about. No exact span, no resolved tuplet, and no
grace timing appears in the emitted expression; those are the package's answers, and an adapter that pre-computed them
would be deciding at read time what §2.4 defers.

**Only one of §2.5's two diagnostics is expansion's, and the other one is the trial's first real finding.** A tie whose
two ends spell different pitches is a comparison of two token texts, so the adapter refuses it and points at the second
note. A bar that does not fill the meter it states is arithmetic on the numbers those tokens spell, and **an adapter
reads spellings, never numbers**: `syntax_fold` hands a token its kind and its text, and no operation turns `Text` into
a `Nat` or a `Ratio`. §2.3's "closing a bar adds exact spans and checks the stated length" assumed a privilege the phase
does not grant. A finite table from `"4"` to `1/4` would cover the note values and still not cover `c5(3/8)` — the very
bar §2.5 reports on — because an exact span is an arbitrary rational the composer wrote.

Nor can the package refuse it in expansion's place. The package computes spans and holds each bar's stated meter, so it
could find the fault; what it could not do is say *where*. A refusal there is a `Text`, an anchor is a `Nat`, and the
same missing conversion keeps the two apart — only `Err((node, text))` carries a node, and only the adapter holds nodes.
So this prompt lands the tie diagnostic and records the bar-length one under Target's last bullet, which is what that
bullet is for. Closing it needs a refusal that can carry an anchor, and deciding that is not this trial's to make.

**The block is locally readable.** No note inherits register or duration from an earlier note, and a test asserts it by
reading each event in isolation. This is the spelling prompt 127e keeps, so getting it wrong here is expensive later.

The coverage list is discharged item by item, each with its own fixture and its own test: notes, rests, chords, dots,
exact durations, ties across bars, slurs, tuplets, grace notes, pickups, repeats, alternate endings, meter changes, and
transposing instruments.

## Target

- `stdlib/src/adapters/staff.musa` — the adapter, declared readable, with `expand`.
- `examples/staff-page.musa` — the trial block, covering all fourteen items, compiling and rendering.
- Tests: one per coverage item, each asserting the expansion's value against the package's own data; §2.5's tie
  diagnostic landing on the composer's own text; and the local-readability test.
- A statement, in the adapter's own comments, of any privilege it wanted and did not get — including §2.5's bar-length
  diagnostic, which is one of them, with what it would take to close.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Expand a staff region into the staff package, item by item`.

## Stop

- No compiler privilege, no private parser or checker access, and no inferred type reaching the adapter.
- No `edit` and no `print`; prompt 127dcfb carries both, and the declared level says so.
- No change to the staff package's data or functions. If expansion cannot produce a shape prompt 127dcf declared, that
  repairs 127dcf.
- No deletion of contextual `Music` and no notation migration of the corpus — prompt 127e owns both.
- No studio adapter; prompt 127dcg carries it.
- No freeze, no proofs, and no conformance script; prompt 127dd carries those.
- No change to `docs/rules/`.
