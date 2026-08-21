---
id: 127dcb
slug: adapter-refusal
status: done
depends_on: [127dca]
phase: 3
---

# Let an Adapter Refuse a Region and Say Where

## Task

An adapter's `expand` currently has to answer with syntax. Give it the error half of the type its own specification
states — one refusal, carrying a message and the node it is about — and land that refusal on the source range the node
came from.

## Read

- `docs/notes/research/language-design-closure/26-language-design-decision.md` §3.4, which types the operation
  `expand: ExpansionContext × BlockSyntax -> Result<ExprSyntax, SyntaxError>`. The `Result` is not decoration: §3.4 also
  forbids an adapter from reading a source range, so a refusal is the only way it can say anything about *where*.
- `docs/notes/research/language-design-closure/27-adapter-trials.md` §2.5 and §3.4 — the diagnostics both trials are
  required to produce. "staff tie changes pitch … continuation at a9" is an adapter's sentence about an input node, not
  a compiler complaint about the region.
- `crates/musa-compiler/src/phase/mod.rs`: `expand_syntax` and the `Syntax -> Syntax` type it demands.
- `crates/musa-compiler/src/quote/mod.rs`: `SourceInfo`, which is inside a node and has no eliminator, and
  `crate::quote::read_region`, which is where an original range enters a node.
- `crates/musa-compiler/src/expand/mod.rs`: `ExpansionFailure`, `stopped_or_refused`, and how a refusal reaches a
  `Diagnostic`.
- Prompt 127dc's delivered phase, and prompt 127dd's Design.

## Design

The transformer's type becomes:

```text
expand : Syntax -> Result<Syntax, (Syntax, Text)>
```

An adapter that will not read a region answers `Err((node, message))`. The node is how it points: it has no operation
for reading a range, so it hands back a node it was *given* and the compiler reads that node's `SourceInfo`. This keeps
§3.4's prohibition exactly as it is — pointing is not reading — and it is why the error carries a node rather than a
number.

One shape, not two. The phase demands this type of every adapter, and the bundled fixture is rewritten to it, because a
compiler that accepted both would be two interfaces with one name and the trials would not know which they were proving.

Where a refusal lands:

- the node is `Original`, so the refusal lands on the range the composer wrote — which is what makes an adapter's
  complaint about the fourth bar arrive at the fourth bar;
- the node is `Generated`, so the adapter is pointing at something it built rather than at something it was handed;
  there is no composer's text under it, and the refusal lands on the region as a whole.

Say which of the two happened in the report rather than silently degrading. A caret under the whole region when the
adapter meant a note is not wrong, but a reader deserves to know that the adapter pointed at its own work.

An adapter refusal is not a compiler fault and not a limit. It gets `Code::Expansion`, the adapter's own sentence as its
message, and the adapter package's name, so a musician can tell "this package will not read what I wrote" from "the
compiler could not run this package".

The charge does not change: refusing costs what the run that refused cost. A refusal is an answer, not a stop.

## Target

- `expand_syntax` demands `Syntax -> Result<Syntax, (Syntax, Text)>` and returns the refusal separately from the
  failures that mean the adapter is broken.
- A refusal becomes one `Code::Expansion` diagnostic carrying the adapter's message, located at the pointed node's
  original range when it has one and at the region when it does not.
- `stdlib/src/adapters/doubled.musa` answers `Ok(…)`, and refuses one thing it can actually detect, so the fixture
  exercises both halves.
- Tests: a refusal lands on the pointed node's own range and not on the region; a refusal pointing at a generated node
  lands on the region and says so; a refusal is told apart from a transformer that does not check and from a run the
  meter stopped; a refusal charges what the run charged.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project
cargo clippy --all-targets -p musa-compiler -p musa-project -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Let an adapter refuse a region and point at it`.

## Stop

- No source-range eliminator, and no operation that lets an adapter read, compare, or construct a range. It points with
  a node it holds or it does not point.
- No second refusal channel, no warning level, no list of refusals — one refusal ends one expansion, which is what a
  `Result` says.
- No `edit` and no `print`; those are prompt 127dd's two remaining operations.
- No change to the source map's granularity for diagnostics that arise *after* expansion — an ordinary complaint about
  generated text still lands on the region, and whether that is enough is 127dd's evidence to produce.
- No change to `docs/rules/`.
