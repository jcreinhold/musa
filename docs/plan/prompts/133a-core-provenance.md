---
id: 133a
slug: core-provenance
status: done
depends_on: [133]
phase: 3
---

# Let a Core Term Say Where It Came From

## Task

Give every `musa-calculus` term an **origin**: the surface node it was elaborated from, carried by the representation,
preserved by evaluation and quotation, and invisible to conversion. This is
[`docs/rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §7, which prompt 133 built the
crate without and which no prompt in the stack currently owns.

## Read

- `docs/rules/language/02-core-calculus.md` §7 in full — the three reasons it is normative, the preservation clause
  ("preserved by substitution, by instantiation, and by `quote`"), the η clause, the metavariable-solution clause, and
  the exclusion clause ("Origins are **not** part of conversion").
- Prompt [133](133-core-crate.md)'s Design and the code it produced:
  `crates/musa-calculus/src/{term,value,eval,quote}.rs`. This prompt changes the representation those four share, so
  read them as one thing.
- `docs/rules/across-stages/` on origin paths, and `crates/musa-compiler/src/derivation.rs` — the derivation graph an
  origin will eventually name a node in. This prompt does **not** connect the two; it fixes the shape of the hole.
- `docs/rules/desktop/` on the Origin view, for what an origin is ultimately read by.
- *A Philosophy of Software Design* ch. 8 — this is the "pull complexity downward" case: the alternative is every
  consumer reconstructing an origin by matching a term against the syntax it might have come from, which §7 §2 calls a
  heuristic and which would make the identity law unprovable.

## Design

**An origin is opaque to this crate.** `musa-calculus` is a leaf and does not know what a surface node is. An `Origin`
is therefore an identifier the caller supplies and this crate only carries, plus one distinguished value for a term
nobody wrote. Making it a `u32` newtype rather than a span keeps `musa-calculus` from acquiring a notion of *file*,
which is the first step toward acquiring a notion of source text.

**Terms wrap rather than widen.** A `Term` becomes `{ origin, node }` over an enum of shapes, rather than every variant
growing a field. Three things follow, and they are the reason for the shape:

1. There is one place a term's origin is read and written, so preservation is checkable by reading one file.
2. The `Arc` moves into the wrapper, so a shape's children are `Term` rather than `Arc<Term>`, which is less noise than
   what prompt 133 wrote and not more.
3. `PartialEq` on `Term` is written by hand to compare shapes only. That is §7's exclusion clause enforced by the type
   rather than remembered by each caller, and it is what keeps a file move from changing what a program means.

**Values carry origins too, or quotation cannot preserve them.** §7 says the normal form of a term carries the origins
of the terms it was built from. A normal form is produced by `quote` from a `Value`, so a value that had thrown its
origin away could not put one back. Where a value is built by evaluation the origin is the term's; where `quote`
η-expands, §7 fixes the answer — the expansion carries the origin of the term it expanded — so the λ that `quote` writes
at a Π and the record literal it writes at a record type both take the origin of the value being expanded, not of the
type driving the expansion.

**One rule for a term this crate invents.** Some terms have no surface node: a variable quotation names, the base case
of a fresh binder. They take `Origin::UNKNOWN`, and the crate never fabricates a plausible-looking one. A wrong origin
is worse than an absent one — it points a reader at code that is not the cause.

**Laws, and they are the deliverable.** Conversion ignores origin (two terms differing only in origins are convertible,
and their normal forms are equal). Normalization preserves origin (every node of a normal form carries an origin that
some node of the input carried, `UNKNOWN` aside). η-expansion carries the expanded term's origin. δ carries the
definition's. These are stated over prompt 133's existing corpus, extended with origins, so the same terms answer both
prompts' questions.

## Target

- `Origin` in `musa-calculus`'s public interface, with `UNKNOWN` and a caller-supplied constructor.
- `Term` restructured as origin plus shape, with a hand-written `PartialEq` that compares shapes, and prompt 133's
  constructor helpers carrying an origin argument.
- Origins on values, and preservation through `eval` and `quote` including both η cases.
- `crates/musa-calculus/tests/suite/provenance_laws.rs`, and prompt 133's three suites still passing unchanged in what
  they assert.
- `docs/plan/roadmap.md` §15.12 and `docs/plan/code-map/`: the row and the interface sketch updated.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus
cargo nextest run --workspace
cargo clippy --all-targets -p musa-calculus -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Let a core term say where it came from`.

## Stop

- No elaboration, no metavariables, no unification. Prompt 134 attaches origins; this prompt makes there be somewhere to
  attach them to.
- No connection to `crates/musa-compiler/src/derivation.rs`, no `Derived { origin, quotation, path }`. Prompt 138.
- No span, no file identity, no source text in `musa-calculus`.
- No origin in conversion, and no "compare origins too" option. A knob here is a second semantics.
- No change to `musa-compiler`, `musa-language`, or any shell.
