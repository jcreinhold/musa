---
id: 127dcfb
slug: staff-edit-print
status: pending
depends_on: [127dcfah]
phase: 3
---

# Make the Staff Adapter Generative

## Task

The writing side of the first trial. Give the staff adapter `edit` and `print`, raise its declared level to generative,
land §2.6's structured edit changing one pitch and no other byte, and prove the round-trip law on the staff document.

## Read

- `docs/notes/research/language-design-closure/27-adapter-trials.md` §2.6 in full — the structured edit, what it must
  not disturb, and its last paragraph on when the printer runs.
- `docs/notes/research/language-design-closure/26-language-design-decision.md` §4 — the edit law's three parts
  (locality, agreement, preservation), the round-trip law and what it explicitly does *not* claim, and the sentence that
  a printer alone does not make an existing block safely editable.
- Prompts 127dcd and 127dce: anchors as the way a command names a node, the level declaration and where it is checked,
  and that `PrintLoss` is an answer rather than a failure.
- Prompt 127dcfa's `expand`: the printer's round trip is against it, so the two are one contract read from two sides.
- Prompt [127dcfah](127dcfah-printed-literals.md) — `text_join` and the five literal spellings. The printer is written
  with those and adds none: this prompt's "no new compiler-owned operation" stands because the operations a printer
  needs landed there, after the attempt to write this one found that the language could not build a text at all.

## Design

`edit` answers structured commands with replacements naming nodes by **anchor** — a node's pre-order position in the
region's own reading order — because an adapter may not read a source range. §2.6's edit changes one pitch; the test
asserts that every other byte of the region, including comments and layout, is unchanged, which is the preservation half
of the edit law made concrete on real notation.

`print` takes an ordinary evaluated `StaffDocument` and returns source text or a stated loss. It runs as an ordinary
total package function, not in the phase environment, so it cannot build syntax: a printer that could would be a second
way to make an expansion out of a value.

**The staff document is the first value a printer has had that it can genuinely fail to spell**, which is what makes it
the real test of `PrintLoss`. A document carrying a written fact the staff spelling has no notation for must produce a
loss that names it, not a smaller document. A printer that silently dropped it would satisfy the round-trip law by
making the value smaller, which is the failure §4 names. Prompt 127dcfah's `interval_literal` is where the unspellable
case is real rather than staged: a transposing shift with no written interval name is a document the staff cannot write
down, and the loss says so.

The round-trip law is stated as the package's own equality on `StaffDocument` rather than as text equality, because
printing is allowed to normalize: expand the trial block, print the value, expand the printed region, and compare the
two values.

The level declaration rises from readable to generative in this prompt, and prompt 127dce's import check is what makes
that a promise rather than a label.

## Target

- `edit` and `print` in `stdlib/src/adapters/staff.musa`, with `let level = "generative";`.
- Tests: §2.6's edit changing one pitch and no other byte; an edit command the adapter does not serve, refused by name;
  the round-trip law over the trial block; and a stated loss on a document the staff spelling cannot write, naming what
  it could not spell.
- `docs/book/src/reference/stdlib.md` recording that the staff adapter is generative, alongside `doubled`'s editable.
- The privilege statement in the adapter's comments extended to anything `edit` or `print` wanted and did not get, and
  revised where 127dcfah changed the answer. Two of its bullets are now out of date: "a number it computed cannot be
  written back as a numeral" names an operation that exists (`nat_literal`), and "a region is reformatted generically
  until this adapter can print one" is what this prompt closes. Say what is still true — that `expand` does not use the
  spellings, and that whether it should is prompt 127dd's to weigh — rather than deleting the bullets.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Make the staff adapter generative`.

## Stop

- No printer-driven editing. A printed region replaces nothing that already exists.
- No claim that printing preserves comments, layout, or origin, and no test that asserts it does. The edit law makes
  that claim about `edit`; the round-trip law does not make it about `print`.
- No compiler privilege and no new compiler-owned operation.
- No change to `expand` or to the staff package's data. If the printer cannot spell what expansion produced, that is a
  loss to state, not a shape to change.
- No studio adapter; prompt 127dcg carries it.
- No freeze, no proofs, and no conformance script; prompt 127dd carries those.
- No change to `docs/rules/`.
