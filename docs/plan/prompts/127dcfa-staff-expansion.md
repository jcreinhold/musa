---
id: 127dcfa
slug: staff-expansion
status: pending
depends_on: [127dceb, 127dcf]
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
- Prompt 127dceb, which this prompt's first attempt caused: an adapter module is a module, its own `data` and `fn` are
  in scope, `Syntax` has a spelling there, `syntax_number` answers with the rational the lexer already read, and
  `text_equal` compares two spellings. Write the adapter as ordinary Musa; if some part of it still wants an encoding no
  reader would recognise, that is a finding to report and not a puzzle to solve.
- `crates/musa-compiler/src/expand.rs`, the region grouper: `raw_group` makes one node per matched delimiter pair and no
  other rule, and `Whitespace` is one undifferentiated trivia kind. An adapter cannot see lines, and the staff spelling
  has to be readable without them.

## Design

The adapter declares `expand` and nothing else, and declares itself **readable**: its regions are read-only until prompt
127dcfb gives it `edit` and `print`. Under-promising is what prompt 127dce made legal, and it is what lets this prompt
end at a real boundary instead of a half-written adapter.

**The adapter is written the way a package is.** Prompt 127dceb made the module its scope: it declares the `data` its
reader carries, the `fn`s that read one bar and one event, and `expand` on top of them. A reader of the adapter should
be able to see the notation's grammar in the shapes of those declarations. Nesting the fourteen items inside one
expression, or folding each node to a function of a state, would hide exactly that, and the first attempt at this prompt
is the record of what it costs.

**The value expansion emits records written facts and nothing else.** A missing choice is not an error here: it is an
absence the package's `realize` refuses later, which is what §2.4 is about. No exact span, no resolved tuplet, and no
grace timing appears in the emitted expression; those are the package's answers, and an adapter that pre-computed them
would be deciding at read time what §2.4 defers.

**Both of §2.5's diagnostics are expansion's, and each is landed on the composer's own node.** "Bar at a8 has length
7/8; meter requires 1" is arithmetic over what the tokens spell, which `syntax_number` now supplies exactly — including
`c5(3/8)`, the arbitrary rational in the very bar that diagnostic reports on, which no finite table over note values
could have covered. "Staff tie changes pitch: f5 tied to f#5, continuation at a9" compares two spellings the composer
wrote, which `text_equal` now compares. Both are refusals in prompt 127dcb's sense: `Err((node, text))`, pointing at the
node the adapter was handed.

The adapter checks these and the package does not, and the reason is worth stating once. The package could find the
length fault — it computes spans and holds each bar's stated meter — but could not say *where*, because its refusal is a
`Text` and only the adapter holds nodes. Where a check needs a place, it belongs to whoever has one.

Everything else in §2.3's list is a check against a literal the adapter wrote: an unknown word, a note that states no
written value, a note that states two, an empty chord, a zero tuplet number, a duplicate header, and a tie with nothing
sounding after it anywhere in the region.

**The block is locally readable.** No note inherits register or duration from an earlier note, and a test asserts it by
reading each event in isolation. This is the spelling prompt 127e keeps, so getting it wrong here is expensive later.

The coverage list is discharged item by item, each with its own fixture and its own test: notes, rests, chords, dots,
exact durations, ties across bars, slurs, tuplets, grace notes, pickups, repeats, alternate endings, meter changes, and
transposing instruments.

## Target

- `stdlib/src/adapters/staff.musa` — the adapter, declared readable, with `expand`.
- `examples/staff-page.musa` — the trial block, covering all fourteen items, compiling and rendering.
- Tests: one per coverage item, each asserting the expansion's value against the package's own data; every diagnostic of
  **Design**, including both of §2.5's, each landing on the composer's own node; and the local-readability test.
- A statement, in the adapter's own comments, of any privilege it wanted and did not get, with what closing each would
  take.

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
