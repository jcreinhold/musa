---
id: 79
slug: folding
status: done
depends_on: [77]
phase: 3
---

# Folding Ranges

## Task

Answer `textDocument/foldingRange` from the concrete syntax tree: a fold for every braced block that spans more than one
line, and one for every run of consecutive comment lines. Folding must work on half-typed source — a composer collapses
a part to work on another *while* the piece is broken — so it reads the lossless tree, never the session's facts.

## Read

- `crates/musa-syntax/src/parser.rs` and `syntax_kind.rs` — the tree is lossless and total; braces are the language's
  explicit structure (roadmap §7.2), which is what makes folding a tree walk rather than a heuristic.
- Prompt 77 — the server and its `convert` module; this prompt is one handler plus one tree walk.
- Prompt 26 — the desktop's source workspace folds too, from the same tree through CodeMirror; the two must agree on
  *what* is foldable even though they compute it on opposite sides of the wire.

## Design

A foldable range is a `{ … }` block whose opening and closing braces sit on different lines, reported with the block's
kind as `region` — `piece`, `score`, `part`, `voice`, `motif`, `studio`, `patch`, and every transform block the grammar
grows later are foldable by virtue of being braced, so the walk keys on brace pairs in the tree, not on a list of
keywords that could go stale. A run of comment-only lines folds as `comment`.

The handler needs the parsed tree and `convert`, nothing else: no session lookup, no compile. It is the cheapest handler
in the server and the one that says the most about the source's shape.

## Target

- `musa-lsp`: the handler and its tree walk; law tests folding every `examples/*.musa` and asserting the broken fixtures
  in `examples/broken/` still fold their closed blocks.

## Check

```sh
cargo nextest run -p musa-lsp
cargo clippy --all-targets -p musa-lsp -- -D warnings
cargo fmt --check
```

Behavior: `glass-mountain.musa` offers a fold per part, per motif, and for the studio block; an unclosed block offers
none (there is no closing line to fold to); single-line blocks offer none.

## Stop

- No custom folding-range kinds and no client-specific extensions.
- No indentation- or blank-line-based folding. Braces are the structure; anything else is guessing.
- No folding of imports, no folding in the desktop — CodeMirror's fold comes from the same tree, prompt 26's path.
- No changes to `musa-syntax`: the tree prompt 03 built already says everything this prompt asks.
