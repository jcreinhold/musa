---
id: 127dce
slug: adapter-print
status: pending
depends_on: [127dcd]
phase: 3
---

# Give an Adapter the Print Operation and Name the Three Conformance Levels

## Task

Add the third declared adapter operation — a printer that creates a *new* region where none exists — with its round-trip
law and its stated losses. Then make the three conformance levels a thing the compiler knows rather than a paragraph in
a note: an adapter declares which of readable, editable, or generative it is, and the declaration is checked against
what it actually offers.

## Read

- `docs/notes/research/language-design-closure/26-language-design-decision.md` §4 — `print_A`, the round-trip law
  ("expanding and evaluating the printed block must return an adapter-equal value"), what the law explicitly does *not*
  claim (equal source, comments, layout, origin), and the sentence that a printer alone does not make an existing block
  safely editable.
- `docs/notes/research/language-design-closure/27-adapter-trials.md` §2.6's last paragraph and §3.5's last paragraph —
  when each trial's printer runs, and that `PrintLoss` is an answer rather than a failure.
- Prompt 127dcd's `edit`, and prompt 127dcb's refusal: three operations, one way of saying no.
- `crates/musa-project/src/template.rs` and `library.rs` — where new source is created today, which is where a printed
  region has to arrive.

## Design

`print` is the one operation whose input is an *ordinary evaluated value* rather than syntax, so it does not run in the
phase environment: it runs as an ordinary total package function whose result is source text.

```text
print : A -> Result<Text, Text>
```

The value half is the region's contents — what goes inside `syntax <adapter> { … }` — and the error half is the loss:
the adapter's own sentence about what it was handed and could not write down. `PrintLoss` is not a `Result` of
convenience. A printer that silently dropped what it could not spell would make the round-trip law true by making the
value smaller.

**The round-trip law.** For a value the printer accepts, expanding the printed region and evaluating it gives an
adapter-equal value. Equality is the package's to state, and the test states it as the fixture's own equality function
rather than as structural equality of the printed text, because printing is allowed to normalize.

**The three levels** are declared in the adapter module and checked, not inferred:

- **readable** — `expand` only;
- **editable** — `expand` and `edit`, and the edit law holds;
- **generative** — editable, and `print`, and the round-trip law holds.

An adapter that declares a level it does not reach is refused where it is imported, with the operation it is missing
named. This is what makes a level a promise a musician can rely on rather than a label.

## Target

- `print` as an ordinary declared operation, and the round-trip law tested through the real compiler: print, splice,
  expand, evaluate, compare.
- The declared conformance level in an adapter module, read at import, and refused when it overstates what the module
  offers.
- `stdlib/src/adapters/doubled.musa` declares generative and satisfies all three laws, or declares what it actually is
  and says why in its own comment.
- Tests: round-trip on a value the printer accepts; a stated loss on one it does not; a module that declares generative
  and offers no printer is refused at its import and names `print`; a readable adapter's region is read-only and its
  edit command says so.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Give an adapter the print operation and name the three conformance levels`.

## Stop

- No printer-driven editing. A printed region replaces nothing that already exists; `edit` is how existing source
  changes, and conflating the two is the failure §4 names.
- No claim that printing preserves comments, layout, or origin, and no test that asserts it does.
- No staff or studio adapter; prompts 127dcf and 127dcg carry those.
- No change to `docs/rules/`.
