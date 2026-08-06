---
id: 04
slug: formatter
status: pending
depends_on: [03]
phase: 1
---

# Formatter

## Task

Implement the lossless formatter over the CST and wire the first two real CLI
commands, `musa format` and a structural `musa check` (parse + diagnostics only, for
now). The formatter operates on syntax, not on any semantic model, and preserves
comments predictably.

## Read

- Roadmap §11 (formatting laws), §10.1 (why trivia is kept), §15.2 (`format` facade),
  §15.8 (CLI commands), §17.3 (formatting laws to test).
- Prompt 03's CST and typed wrappers.

## Design

- `musa-language` gains:

  ```rust
  /// Format a parsed document. Lossless: every comment and token survives;
  /// only whitespace trivia is normalized.
  pub fn format(document: &ParsedDocument) -> FormattedSource;

  /// Apply text edits; the project's canonical mutation mechanism later.
  pub fn apply_edits(source: &str, edits: &[TextEdit]) -> String;
  ```

- Formatting rules: 4-space indent per block level; one statement per line; `;`
  terminates; blank line between top-level declarations; `{` on the declaration line.
  Comments stay attached to the following token (trailing comments stay on their
  line). Whatever rules you choose, encode them as insta snapshots so review is
  textual.
- `TextEdit` is a simple `(TextRange, String)` replacement type; `apply_edits` is a
  small utility now, load-bearing for score-editing in prompt 16.
- `musa-cli` gains real argument handling (hand-rolled or `clap` — if `clap`, add it
  and update `deny.toml` review in the same commit):
  - `musa format <file>` — rewrite in place, or `--check` to exit non-zero on diff.
  - `musa check <file>` — lex + parse, print diagnostics with `miette` (roadmap
    §10.4), exit non-zero on errors. Semantic checks arrive in prompt 05 and extend
    this command; structure the command so that happens by adding a pass, not
    rewriting it.

## Target

- `musa-language`: `format`, `FormattedSource`, `apply_edits`, `TextEdit`.
- `musa-cli`: `format` and `check` subcommands calling `musa_language` directly for
  now (the project-session indirection arrives in prompt 14 — per roadmap §15.8 the
  CLI must not recreate orchestration, but at this stage there is no orchestration to
  reuse; keep the call sites one-liners so the swap is trivial).
- Tests: idempotence `format(format(x)) == format(x)` and semantic preservation
  `parse(format(parse(x)))` equals `parse(x)` up to whitespace trivia, as `proptest`
  properties over a small grammar-directed generator plus the two example files.
- insta snapshots for formatted output of both examples and of comment-placement edge
  cases.

## Check

```sh
cargo nextest run -p musa-language -p musa-cli
cargo clippy --all-targets -p musa-language -p musa-cli -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- format --check examples/glass-mountain.musa
cargo run -p musa-cli -- format examples/invention.musa && git diff --exit-code examples/  # formats to itself
printf 'piece "x" { score { part p { voice v { c5 1/4 } } } }' > /tmp/bad.musa && cargo run -p musa-cli -- check /tmp/bad.musa  # exits non-zero, diagnostic points at the missing ';'
```

Commit as `Add formatter and CLI format/check commands`.

## Stop

- No semantic validation in `check` beyond syntax (that's prompt 05).
- No editor integration, no LSP, no watch mode.
- Do not normalize comment text or reorder declarations.
