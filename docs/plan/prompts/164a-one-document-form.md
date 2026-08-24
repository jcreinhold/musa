---
id: 164a
slug: one-document-form
status: in-progress
depends_on: [164]
phase: 3
---

# One Document Form

## Task

A musa file is currently three things, and the compiler has to decide which before it can read one:

```text
document := (import | binding | function | data | record | enum | impl)*
            (piece | library)
          | module-file
```

The three are not three languages. `library_decl` and `piece_decl` in
[`crates/musa-syntax/src/parser/documents.rs`](../../../crates/musa-syntax/src/parser/documents.rs) are the same
dispatch loop written twice, and the set of statements a `library` accepts is a **strict subset** of the set a `piece`
accepts — identical on imports, `motif`, `fragment`, `let`, `fn`, type declarations, `impl`, `performance`, `studio`,
and the misplaced-`private` error, and lacking exactly five: `tempo`, `meter`, `key`, front matter, and `score`. The
parser says so in a comment: *"A library holds what can be shared. Music belongs to a piece, which is why `score` is not
in this list."*

That is `AGENTS.md`'s **no sublanguage by subtraction** named in its own words. A `library` is not a different kind of
document; it is the source language with five statements removed, and every author who writes one pays for the removal.
The third shape is the same mistake once more: a **module file** is not a shape at all, it is a file whose declarations
all happen to be `mod` and which declares no piece.

Collapse the three into one:

```text
document := declaration* piece?
```

A file is a sequence of declarations, optionally followed by one piece. `library { … }` goes away and its contents stand
at the file root. `stdlib/src/lib.musa` stops being an exception and becomes an ordinary file with five declarations and
no piece. The elaborator stops asking which of three shapes it holds and elaborates the root, then builds a snapshot if
a piece is there.

**This prompt does not move `piece` into the standard library.** That is the decision this one clears the ground for,
and it belongs after [167](167-studio-rewrite.md) and before [168](168-adapter-freeze.md), for the reason recorded in
the Design below. This prompt takes only what needs no judgment about the staging semantics: one document form, one
parser loop, one elaborator path.

## Read

- [`crates/musa-syntax/src/parser/documents.rs`](../../../crates/musa-syntax/src/parser/documents.rs) — `piece_decl` and
  `library_decl`, the two loops, and the comment that states the subtraction as though it were a design.
- [`crates/musa-syntax/src/parser/engine.rs`](../../../crates/musa-syntax/src/parser/engine.rs)'s `Shape` and its
  `declares_modules` flag — the third shape, decided by lookahead before anything is parsed.
- [`crates/musa-compiler/src/elaborate/mod.rs`](../../../crates/musa-compiler/src/elaborate/mod.rs)'s
  `elaborate_parsed`, whose three-arm dispatch and *"this file declares no piece"* backstop are what one form deletes;
  and `elaborate_material`, the library path that merges with the piece path's declaration half.
- [`docs/rules/language/01-surface.md`](../../rules/language/01-surface.md) §1's `document` production and §6's closing
  paragraphs — "A file is one piece or one library" and "A **module file** is the third document shape". Both are this
  prompt's to restate.
- [`docs/rules/language/00-semantics.md`](../../rules/language/00-semantics.md) §2 — the declaration-kind list
  `library`, `piece`, `part`, `voice`, `performance`, `instrument`, `mix`, `structure`. `structure` left at
  [162](162-delete-the-module-layer.md) and the list did not notice; `library` leaves here, and the list is restated
  with both gone.
- [`docs/rules/language/04-templates-and-modules.md`](../../rules/language/04-templates-and-modules.md) §1 — what a
  package's `lib.musa` and `mod.musa` are, which stops being a special shape and stays exactly as true.
- [`crates/musa-compiler/tests/suite/document_shape_laws.rs`](../../../crates/musa-compiler/tests/suite/document_shape_laws.rs)
  — the laws that hold the parser's count and the elaborator's count equal. They survive as laws about one count.
- `AGENTS.md`'s **no sublanguage by subtraction**, and *Philosophy of Software Design* ch. 7 — a `library` block is the
  pass-through variant of the smell: a wrapper that adds a brace and subtracts five statements.
- [Prompt 162](162-delete-the-module-layer.md) — the precedent for deleting a layer rather than repairing it, and the
  migration shape a twenty-file de-indentation follows.

## Design

**One production, and the shapes become facts about a file rather than kinds of file.**

```text
document := declaration* piece?
```

Where `declaration` is `library`'s list — imports and `use`, `motif`, `fragment`, `let`, `fn`, `data`/`record`/`enum`,
`impl`, `mod`, `performance`, `studio` — and `piece` is unchanged. A file with a piece is a piece; a file without one
exports its declarations; a file whose declarations are all `mod` is a module tree. Nothing is decided by lookahead and
nothing is refused for standing in the wrong wrapper.

**`piece` keeps its own five statements, and this prompt does not move the declarations out of it.** `tempo`, `meter`,
`key`, front matter, and `score` are the piece's ambient state and its music, and they stay inside the piece block.
Twenty-two of the fifty-seven files in `examples/` put `fn` and `let` inside the piece too, and two put them before it;
that redundancy is real and is **not** this prompt's, because the prompt that moves `piece` into the standard library
will redesign what a piece holds, and churning fifty-seven fixtures twice for one answer is worse than churning them
once for the right one.

**The elaborator gets one path.** `elaborate_parsed` stops choosing between `PieceDecl`, `LibraryDecl`, and
`ModDecl::all_at_root`. It elaborates the root's declarations — which is what `elaborate_material` already does — and
then, if the root holds a piece, builds the snapshot. `DocumentKind` stays, because a *caller* still wants to know
whether a compilation produced a score, but it becomes an observation about the answer rather than a decision taken
before reading. The *"this file declares no piece"* diagnostic and its help text go: there is no such fault any more.

**The five refusals become checks with reasons, not a grammar that cannot say them.** A `score` or a `meter` at a file
root is still wrong, and it is now wrong the way a type error is wrong — parsed, then refused with a message that says
what the statement needs (`meter` sets a piece's ambient meter; this file declares no piece) rather than with *"expected
a declaration"* from a loop that never had the arm. That is the enrichment direction `AGENTS.md` requires, and it is
what makes the collapse an improvement rather than a permission.

**Why this lands before [165](165-diagnostics-and-performance.md).** 165 rewrites every diagnostic from 134 through 140
and pins each message on a realistic program. Pinning a message against a surface that is about to lose a wrapper and
gain five refusals means writing the tests twice. The order is: collapse the surface, then make its failures legible.

**Why the `piece`-into-stdlib decision is not here.** It amends §2's staging judgment `Σ ⊢piece Δ ⇓ t :
Term[ScoreFact]`, the private fragment `K`, and `follow`/`together` — the semantics, not the syntax. The evidence for
what that costs is [166](166-staff-rewrite.md)'s staff adapter and [167](167-studio-rewrite.md)'s studio adapter, the
two proofs that a domain sublanguage can be an unprivileged package. 166 is done and 167 is not, so the decision waits
for the second number. It must land **before** [168](168-adapter-freeze.md), because 168 freezes the adapter rules under
hostile review and `piece` would be their largest client; freezing them with that client outside the room is what this
note exists to prevent.

**Migration.** Twenty files under `stdlib/src/` lose a `library {` line, a closing brace, and one level of indentation;
`cargo run -p musa -- format` writes the indentation once the wrapper is gone. Five module files change not at all.
`examples/album/library/motifs.musa` and `patches.musa` lose theirs the same way — the two files under `examples/` that
open one — and no file under `examples/` that declares a piece changes at all.

## Target

- `library` deleted: the keyword, `SyntaxKind::LibraryKw` and `LibraryDecl`, `ast::LibraryDecl`, `library_decl`, the
  highlighter and LSP entries, and the tree-sitter grammar's rule with its queries.
- `Shape::declares_modules` and the lookahead that sets it deleted; `crates/musa-syntax/src/parser/engine.rs` reads one
  document form.
- `elaborate_parsed` reduced to one path: elaborate the root, then build the snapshot if a piece stands there. The
  `Code::Misplaced` *"this file declares no piece"* diagnostic and its help text removed.
- The five piece-only statements refused at a rootless file with a diagnostic that names what each needs, each with
  `musa explain` text and a test.
- `docs/rules/language/01-surface.md` §1's `document` production and §6's two closing paragraphs restated for one form.
- `docs/rules/language/00-semantics.md` §2's declaration-kind list restated with `library` and `structure` both gone.
- `docs/rules/language/04-templates-and-modules.md` §1 restated: a `lib.musa` is an ordinary file, not a shape.
- `stdlib/src/`'s twenty `library { … }` wrappers removed and the files reformatted.
- `document_shape_laws.rs` restated as laws about one form: a file with a piece, a file without one, a file of `mod`
  declarations, and the standard library's own root, all compiling by the same path.
- `docs/book/`'s pages that teach `library { … }` updated, and `docs/plan/code-map/` rows.

## Check

```sh
cargo build --workspace
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- check stdlib/src/*.musa
cargo run -p musa -- format --check stdlib examples
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Make a musa file one document form`.

## Stop

- **`piece` is not moved into the standard library.** The staging judgment, the fragment `K`, and `follow`/`together`
  are untouched. That prompt is written after 167 and runs before 168.
- **No declaration is moved out of the `piece` block.** `tempo`, `meter`, `key`, front matter, and `score` stay where
  they are, and no file under `examples/` that declares a piece is rewritten. The two that open a `library` lose the
  wrapper because the keyword is gone, which is Target and not scope.
- No new language feature. This prompt removes a wrapper and merges two loops; anything it makes newly *expressible*
  beyond declarations standing at a file root is out of scope.
- No behaviour change to what a piece means: same snapshot, same rendered output, same exports for the same
  declarations.
- No weakening of the module-file rules. A file whose declarations are all `mod` still elaborates to a module tree and
  no exports, and a binding beside a `mod` at a package root is still refused — now as a check with a reason rather than
  as a shape that could not hold it.
