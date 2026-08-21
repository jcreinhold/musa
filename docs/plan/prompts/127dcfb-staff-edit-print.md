---
id: 127dcfb
slug: staff-edit-print
status: done
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
- `crates/musa-compiler/src/phase/mod.rs`'s `AdapterModule` doc comment — "**A module and not an expression** … a reader
  written without local definitions is a reader nobody can follow (Peyton Jones ch. 3)". It says that about `expand` and
  `edit`, and `print` is the one operation 127dce exempted from it.
- Peyton Jones ch. 3 (`~/Code/papers/logic-and-computation/software-engineering/`
  `implementation-of-functional-programming-languages/03-translating-a-high-level-functional-language-into-the-lambda-calculus.md`)
  — why a language handed to a programmer is the calculus *enriched* with local definitions rather than the bare
  calculus, and root `AGENTS.md`'s "No sublanguage by subtraction", which is that argument as a standard of this repo.
- `crates/musa-syntax/src/parser.rs`'s `block_expr` — "Exactly one expression, because there is no statement here to be
  the second one". This is the fact that turns the point above from a preference into an obstruction: musa has no `let`
  expression, so an operation read as a bare expression has *no* way to bind a local name.
- `docs/notes/research/language-design-closure/39-totality-and-structural-abstraction.md` §1, §4, §6.4–6.5 and §12.3 —
  the course correction this prompt sits after. All five of its recommendations have landed; per-type duplication is
  §6.5's accepted cost with a stated reopening rule, and more total recursion is §12.3's open question. Neither is this
  prompt's to reopen, and the Design section says why the obstacle here is smaller than both.

## Design

`edit` answers structured commands with replacements naming nodes by **anchor** — a node's pre-order position in the
region's own reading order — because an adapter may not read a source range. §2.6's edit changes one pitch; the test
asserts that every other byte of the region, including comments and layout, is unchanged, which is the preservation half
of the edit law made concrete on real notation.

`print` takes an ordinary evaluated `StaffDocument` and returns source text or a stated loss. It runs as an ordinary
total package function, not in the phase environment, so it cannot build syntax: a printer that could would be a second
way to make an expansion out of a value.

**Where a printer is read.** 127dce settled that the printer is "read where it is run, against the value it is handed",
and read it in an *empty* scope: no imports, no world, and none of its module's declarations. That was enough for a
fixture whose value is a text, and the first printer over real package data is what shows it is not enough for anything
else. Two halves of "where it is run" have to be filled in, and both are completions of that sentence rather than
departures from it.

*The world the value's type is declared in.* `Document(…)` is not a name an empty world holds — neither the subject
expression nor a `match` over it can be checked there — so the printer and its subject are read with the imports of
`at`, the document the region will be written into. `adapter_print` already takes `at` for exactly this reason, and its
imports are the world the printed region will itself be read in, so this is the same scope on both sides of the
round-trip law. Nothing about `A` changes: it is still settled by unification against the subject, and the phase still
never learns the package's type.

*The module's own declarations the printer reads.* A musa block holds exactly one expression and the language has no
`let` expression, so a printer read as a bare expression cannot bind a single local name — the language minus local
definitions, which is the sublanguage by subtraction root `AGENTS.md` forbids and the shape Peyton Jones ch. 3 enriches
the calculus to avoid. `walked` in `stdlib/src/notation/staff.musa` is what a seven-case fold over `StaffItem` costs
when it *can* name `stopped`, `nested`, and `tupleted`; the same fold with those inlined is what a printer would
otherwise have to be, four times over. So `print` is spliced together with the module declarations it names,
transitively.

A declaration that belongs to the phase does not thereby become available, and *which* declarations those are has to be
decided by the splice rather than left to the checker. "The declarations it names" is read off the identifiers a
declaration writes down, which over-reaches on purpose — a spliced declaration the printer did not need only costs a
little checking. That is true of every declaration except one kind. An adapter shares constructor names with the package
it reads — the staff's own `Tying` has an `Untied` and so does the package's `Tie` — so a printer that writes `Untied`
drags in a `data` whose fields are `Syntax`, and the piece is then refused for a type the printer never mentioned. So a
declaration that writes down one of the phase's own types stays with the phase, always. A printer that genuinely reached
one is still refused, for the name it wrote rather than for a type it did not: the same boundary, said in the printer's
own words.

**Why note 39 did not catch this, and why the answer is not a language change.** Note 39 §4's four facilities were
derived from note 38 §10, whose evidence was the staff adapter's `expand` — a declaration *of a module*, which already
reads the module's own `let`, `fn`, and `data`. `print` is the one operation that is not, and it was not in the evidence
set, so the missing local definitions never showed up. That makes the repair the narrow one: give `print` the module
every other operation already has, rather than add a `let` expression the rest of the language has not asked for. This
is not §12.3's "more total recursion" and not §6.4's container reopening; neither is touched here.

**The staff document is the first value a printer has had that it can genuinely fail to spell**, which is what makes it
the real test of `PrintLoss`. A document carrying a written fact the staff spelling has no notation for must produce a
loss that names it, not a smaller document. A printer that silently dropped it would satisfy the round-trip law by
making the value smaller, which is the failure §4 names. Prompt 127dcfah's `interval_literal` is where the unspellable
case is real rather than staged: a transposing shift with no written interval name is a document the staff cannot write
down, and the loss says so.

The round-trip law is stated on the *value* rather than on the text, because printing is allowed to normalize: expand
the trial block, print the value it produced, expand the printed region, and compare the two expansions. `pickup (1, 4)
{ … }` printed back as `bar (1, 4) { … }` is the normalization the law has to permit — the package has no pickup,
because a pickup is a bar with fewer beats in it — and comparing the values is what permits it.

The anchors are the one part of the value the law does not carry across, and saying why is better than leaving it
implied. An anchor is a node's position in the region's own reading order — trivia included, because the region's nodes
are what the compiler hands an adapter — so an anchor is a fact about *text*. A printed page is new text with its own
layout, so it earns its own numbers, and a printer could only reproduce the value's numbers by reproducing the page byte
for byte, which is the claim §4 is explicit about not making. So the law is stated on the whole value and checked
through `realize` — every span's onset and written value, compared exactly — which catches a printer that reorders,
drops, or mis-spells a page. That is also all the compiler exposes of one: a realized span's anchor reaches no compiled
artifact.

The trial value states the anchors that page mints anyway, because a `StaffItem` carries one and a fixture written with
placeholders would be a fixture saying something untrue about the reader. The adapter takes an item's anchor at the node
the item begins with, so a note carries its pitch, a chord carries its bracket, and a bar carries its `bar` — which is
also the edit test's number.

The level declaration rises from readable to generative in this prompt, and prompt 127dce's import check is what makes
that a promise rather than a label.

## Target

- `print` read in the scope the Design section fixes: `crates/musa-compiler/src/phase/mod.rs`'s `print_value` and
  `run_printer` compile one small piece holding `at`'s ordinary imports, the module declarations the printer names
  transitively, the subject, and the printer — under `Reading::Source`, so the phase environment is as absent as the
  empty scope made it, and with a declaration that writes down one of the phase's own types kept out of the splice.
  `crates/musa-compiler/src/expand/mod.rs`'s `adapter_print` passes `at` and the import sources through.
  `AdapterModule`'s doc comment loses the sentence about a printer not calling the module's declarations, because it now
  can.
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
