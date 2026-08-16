---
id: 141
slug: collections
status: done
depends_on: [140]
phase: 3
---

# Let a List Be Built

## Task

Give the language collection construction: list literals, the `Buildable`/`Iterable` trait pair, `map`, `filter`,
`fold`, and `collect`, and the length-indexed `Vec A n` for the places where a length is load-bearing. Note 41 §7
recorded that a list cannot be constructed, which is why the staff adapter's whole reading algorithm runs backwards.
This is the prompt that closes it.

## Read

- `docs/notes/research/language-design-closure/41-staff-on-the-repaired-interface.md` §7 — the finding, in the words of
  the trial that hit it, and the algorithm it forced. Read it before designing the fix; the shape of the workaround says
  what the fix has to remove.
- `docs/rules/language/01-surface.md` as rewritten by prompt 130 — the collection grammar, the trait pair, and the
  deliberate absence of a comprehension.
- `stdlib/src/adapters/staff.musa`'s `from_the_end` and `read_body` — the reversed traversal, which exists only because
  the reader cannot accumulate forwards.
- Prompt [135](135-inductive-families.md)'s indexed families — `Vec A n` is the first real user of an index, and if the
  index is awkward here it will be awkward everywhere.
- Prompt [136](136-records-and-enums.md)'s Design paragraph on where each half lands. Collections split the same way,
  for the same reason, and 142 closes the seam.
- `crates/musa-compiler/src/core.rs`'s existing `list` type and its eliminators, and prompt
  [127dcfaa](127dcfaa-list-fold-direction.md), which decided which end a fold runs from. That decision stands; this
  prompt adds the other direction of travel, not a second fold.

## Design

**Construction is the missing half.** The language can take a list apart and cannot put one together, and every
consequence in note 41 §7 follows from that asymmetry. A list literal `[a, b, c]`, `cons`-style extension, and a
`Buildable` trait that accumulates are the three forms that close it; the traversal that had to run from the end runs
forwards afterwards, which is prompt 145's measurement to make.

**`Buildable` and `Iterable` are two traits, not one.** Building and traversing are different capabilities: a `Vec A n`
is iterable at every length and buildable only into a `Vec A (n+1)`. Keeping them apart is what lets `collect` be typed
honestly — it consumes an `Iterable` and produces a `Buildable`, and the two need not be the same container.

**`Vec A n` earns its index or it does not ship.** The test is prompt 132's trial: if no program needed a length in a
type, `Vec` is a mechanism nobody asked for and this prompt builds `List` alone. If a program did — the staff reader's
fixed-arity constructions and the studio adapter's parameter lists are the candidates — then `Vec` ships with that
program as its fixture. Do not ship it on the strength of it being the standard example of a dependent type.

**Where each half lands.** Grammar, CST, formatter, and highlighting in `musa-language`; the families, the trait pair,
and their laws in `musa-core`; the `.musa` prelude modules that expose them to authors in prompt 142's migration, with
everything else. This is the split prompts 136 and 137 established, for the same reason: a second checking path through
the old compiler would be built and deleted within six prompts.

**Totality reaches indexing.** `Vec` indexing with an in-range proof is total; `List` indexing is not, and returns an
`option` rather than acquiring a partial operator. That is the same rule prompt 137 applied to failing arithmetic, and
it is worth stating twice because indexing is where languages usually make the exception.

**Laws.** A literal's elements are its elements, in order. `collect` after `map` is `map` then `collect`. Folding a
built list gives back what was built — the round trip that says construction and elimination agree. `Vec`'s length is
its length, and a length mismatch is a type error rather than a runtime one. Indexing is total where it is claimed to
be. And the direction law from 127dcfaa still holds, unchanged.

## Target

- `musa-language`: list-literal grammar, CST, formatter, highlighting; tree-sitter and its drift test.
- `musa-core`: `List`, `Vec A n` (if the trial justified it), `Buildable`, `Iterable`, `map`, `filter`, `fold`,
  `collect`, indexing, and their laws.
- `crates/musa-core/tests/suite/collection_laws.rs`, with note 41 §7's forward-accumulating traversal as a fixture.
- A recorded answer on `Vec`: shipped with its program, or not shipped, with the reason.
- `docs/plan/code-map/` rows.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-language -p musa-core
cargo nextest run --workspace
cargo clippy --all-targets -p musa-language -p musa-core -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Let a list be built`.

## Stop

- No comprehension, no `do`-notation, no lazy sequence, no infinite structure, no iterator protocol with state.
- No map, set, or dictionary container. A container without a program that needs it is a container to add later.
- No partial indexing operator on `List`, and no panicking access anywhere.
- No `musa-compiler` wire-up and no `stdlib/` or `examples/` change. Prompt 142.
- No second fold direction and no reversal of 127dcfaa's decision.
- No rewrite of `stdlib/src/adapters/staff.musa`. Prompt 145 measures it.
