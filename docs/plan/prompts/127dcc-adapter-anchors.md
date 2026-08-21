---
id: 127dcc
slug: adapter-anchors
status: done
depends_on: [127dcb]
phase: 3
---

# Let an Adapter Carry a Source Anchor into the Value It Produces

## Task

An adapter can now point at a node while it is refusing. It cannot put a place into the value it *produces*, so nothing
a later ordinary package function says about a region can be traced back to the composer's text. Give the phase an
anchor: a number the adapter emits, and a compiler-owned table from that number to the range it names.

## Read

- `docs/notes/research/language-design-closure/27-adapter-trials.md` §2.2 and §3.2 — `a0`–`a31` and `s0`–`s18` are
  *fields of the values the adapters produce*, one per staff item and one per studio declaration. §3.4 is what they are
  for: "connection at s15 joins control to audio(2)" is `validate`'s sentence about a value, produced long after
  expansion, and it is only about the composer's block because the value remembers where it came from.
- `docs/notes/research/language-design-closure/26-language-design-decision.md` §3.4 — `anchor_at`, and the rule that an
  adapter may not read a source range. Emitting an anchor is not reading one, for the same reason pointing with a node
  is not: the adapter never learns what the range *is*.
- `docs/rules/language/02-core-calculus.md` §5 — the closed source type grammar and its "no syntax value" sentence. An
  anchor that needed a new source type would be an amendment under `docs/rules/README.md` rather than a prompt.
- `crates/musa-compiler/src/syntax.rs`: `SourceInfo`, `print`, and `Printed`, which is where a node becomes text.
- `crates/musa-compiler/src/expand/mod.rs`: `ExpansionRecord` and `SourceMap` — what one expansion already remembers.
- Prompt 127dcb's `Refused`, which is the other half of the same rule: pointing during expansion.

## Design

**An anchor is a `Nat`.** The package declares its own field — `data StaffItem { Note(at: Nat, …) }` — and the number is
the anchor. Nothing enters the source type grammar, nothing new is nameable, and a package that never wants anchors
never sees them. The compiler owns the table from number to range; the number alone means nothing without it, which is
what keeps a forged anchor a mislocated sentence rather than a hole in anything.

One new phase builtin, with the shape `syntax_at` already has and for the same reason:

```text
syntax_anchor : Syntax × NodePath × NodePath -> Option<Syntax>
```

It answers with the anchor of the *input* node at the first path, as an `Integer` token built at the second, and `None`
where the path addresses no input node. An adapter holds a node or it does not; it may not invent a place.

**Why a node and not the number itself.** A transformer's answer is an expression, and the only route from a number into
an expression is a literal the ordinary parser reads. There is no `Nat -> Text` in the source core and none should be
added for this, so a builtin answering `Option<Nat>` would hand back a number the adapter could never emit. Answering
with a node is also the stricter reading of §3.4: the adapter receives something to splice and nothing to compare, so it
still never learns what the range is — and the anchor's value *in the emitted expression* is an ordinary `Nat`, which is
what the package's own field declares.

**The number is a function of the region alone.** It is the node's position in the region's own reading order — not a
counter, not the region's ordinal in the file, and not anything the compiler allocates. Prompt 127dc's law that one
region has one answer wherever it is written, and the cache that replays a miss's charge, both depend on this: an anchor
that counted from the file's start would make two identical regions expand to two different expressions.

`ExpansionRecord` gains the table, `anchors: Vec<SourceSpan>`, indexed by that number: one entry per node of the region,
in reading order. It belongs on the record rather than in a compilation-wide map because a number is only meaningful
with the region that minted it, and the record is already what a reader has in hand. The table is a function of the
region alone, so the cache-hit path and the cache-miss path build the same one and neither has to remember it — and no
run can enlarge it, because what an adapter asked for does not change what the region is.

## Target

- `syntax_anchor` in `SYNTAX_OWNERSHIP`, with its type, its evaluation, and its charge, and the phase-privacy tests
  extended to cover it exactly as they cover the other ten.
- `ExpansionRecord::anchors`, filled during expansion, and a way to read a range back out of a record given a number.
- `stdlib/src/adapters/doubled.musa` emits, for each group it rebuilds, the anchor of the node it rebuilt it from — so
  the fixture expands to `(repeat(…, 2), n)` and exercises the whole route. The book's two sentences about the fixture
  and `examples/doubled.musa` say so.
- Tests: an anchor names the range of the node it was taken from; two identical regions in one file mint the same
  anchors, so determinism and the cache are unchanged; an anchor for a node the adapter built is `None`; the table is
  the region's own nodes, one per node, and no run enlarges it.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project
cargo clippy --all-targets -p musa-compiler -p musa-project -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Let an adapter carry a source anchor into the value it produces`.

## Stop

- No `Anchor` type in the source type grammar, and no operation that lets an adapter read, compare, or construct a
  range. It emits a number it was given or it emits nothing.
- No compilation-wide anchor allocator, and nothing that makes an anchor depend on where in a file its region stands.
- No route from a package's error *value* to a compiler diagnostic — a package's complaint is a value, and what a piece
  does with one is the trials' business, not this prompt's.
- No `edit` and no `print`; those are prompts 127dcd and 127dce.
- No change to `docs/rules/`.
