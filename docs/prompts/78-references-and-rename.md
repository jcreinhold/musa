---
id: 78
slug: references-and-rename
status: done
depends_on: [77]
phase: 3
---

# References and Rename

## Task

Teach the language server to answer "where is this name used" and "rename this everywhere" for the names a composer
gives: motifs, parts, voices, patches. The answers must come from the resolver, never from textual search — a rename
that rewrites a string that happens to match is a rename that breaks a piece the day a name appears in a comment.

## Read

- `crates/musa-compiler/src/resolve.rs` and `scope.rs` — every name use is already resolved; this prompt makes
  resolution leave a trace. Recording spans is additive; re-deriving them anywhere else is a second resolver.
- Prompt 39 — how compiler facts become session facts; compiler types stop at the `musa-project` boundary, and the
  reference index restates rather than re-exports for the same reason diagnostics did (prompt 56).
- Prompt 77 — the server's handlers; this prompt adds three more.
- Prompt 36 — imports. A name can be declared in a library file; that case is stated honestly below, not half-done.

## Design

### The resolver remembers

`musa-compiler` gains a reference record: for every named thing, its kind (motif, part, voice, patch), its declaration
span, and the span of every use the resolver resolved to it. This is data the resolver already holds at the moment it
acts; the work is keeping it, not finding it. Unresolved names record nothing — a name that does not resolve has no
references, which is the truth.

`musa-project` restates the record as facts on the snapshot, keyed by name, in the crate's own span vocabulary. The
facts are empty when the source does not compile: references describe the last valid reading, like every other fact.

### Three handlers

- `textDocument/references` — the recorded use spans plus the declaration, converted once by prompt 77's `convert`.
- `textDocument/prepareRename` — succeeds when the position is inside a recorded declaration or use span, so the client
  learns *before* prompting whether rename is possible.
- `textDocument/rename` — a workspace edit replacing exactly the recorded spans. Before answering, the new name is
  checked against the reference record itself: colliding with an existing name in scope, or spelling something the lexer
  would not read as a name, is refused with a reason — not applied and diagnosed afterwards.

### One document

Rename and references cover the open document. A motif used from an imported library resolves to a declaration in a file
the session treats as read-only; reporting its uses there is honest, rewriting them is not. Cross-file rename is
deferred to the day imports get a workspace model — say so in the refusal, not in silence.

## Target

- `musa-compiler`: the reference record in `resolve.rs`/`scope.rs`; exposed through the compile result.
- `musa-project`: restated reference facts on the snapshot.
- `musa-lsp`: the three handlers; tests for rename-through-transform (a motif used inside `transpose`), references from
  a use and from the declaration, refusal cases (collision, non-name, imported declaration).

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
```

Behavior: renaming the motif in `glass-mountain.musa` rewrites its declaration and every `use` and nothing else;
references on `glass_pad` in the studio find the patch declaration and the `assign`; renaming to an existing name or to
`4x` is refused; a document that does not compile answers references from the last valid compile, flagged as such.

## Stop

- No cross-file rename and no workspace-wide symbols — imports have no workspace model yet.
- No rename of header fields, keywords, or units — they are the language's vocabulary, not the composer's.
- No occurrences as rename targets: an occurrence is an act of expansion, not a name (prompt 39's distinction).
- No textual fallback of any kind. If the resolver did not record the span, the edit does not touch it.
