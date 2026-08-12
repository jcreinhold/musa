---
id: 109
slug: the-import-keyword
status: done
depends_on: [99, 108]
phase: 3
---

# `import` for Imports, `use` for Material

## Task

Separate the two unrelated statements that share the word `use`. `01-surface.md` carries both
`import := "use" (STRING | "std" "::" IDENT) ";"` and `music-use := "use" expr ";"`, told apart only by whether the
operand happens to be a string literal or a `std::` path. Respell the import production as `import`, leave `use` with
its musical meaning alone, add the `as` alias clause the module-tree prompt will need, and make the old spelling a hard
error carrying an applicable fix.

## Read

- `docs/language/01-surface.md` §1, which governs this prompt, and `docs/scratch/60-language-decision-record.md` for why `use` rather than `import` keeps the
  musical meaning.
- `docs/language/01-surface.md` §1 (the repaired grammar) and §2 (`use e;` as a splice at the cursor).
- `docs/language/04-templates-and-modules.md`'s source-library boundary paragraph.
- Prompt 56 for the diagnostic shape this migration error takes, including its applicable fix.
- Prompt 80 for the tree-sitter drift law: the grammar is held to the real lexer token-for-token, so a new keyword is a
  change in two places that a test compares.

## Design

`use e;` is the score's most common statement and the one a musician writes — a hundred and thirty-two sites against the
import form's twenty-four — so it keeps the shorter, more musical word. `import` is already what `01-surface.md` calls
the production, and `import path;` needs no lookahead to tell it from anything else.

`use` is a splice and not an application: it checks `e : music`, instantiates it at the current cursor, and sequences
it. `use name(args);` is that same rule applied to a call rather than a second invocation mechanism. Nothing about that
changes here; the overloading simply made it easy to misread, which is part of why the two are being separated.

The migration is a hard error, not a silent acceptance of both spellings. A language that quietly takes either has a
mixed corpus forever, and this project's own examples are the corpus its behavior is defined by. The diagnostic is
located at the `use` token, names both meanings, and carries an applicable fix that rewrites the token — which is what
prompt 56's machinery exists for and what makes a hard error affordable here.

The `as` alias clause lands in this prompt even though its only caller is prompt 110, because it belongs to the same
production and touching the import grammar twice would mean regenerating the editor grammars twice. It parses, formats,
highlights, and round-trips — and it binds nothing, because there is nothing yet to bind it into. Today an import is
flat and total: the library's declarations are merged into the importing file's namespace and the module itself has no
name in scope, so an alias has no referent to rename. §3's `as` qualifies *one* module's names to resolve a collision
against another, which needs both the qualified namespace and the collision, and prompt 110 brings both. Adding a public
`alias()` accessor here would be a reader with no reader — the CST is lossless, so the token is preserved for prompt 110
whether or not anything asks for it today.

Everything downstream of the token is mechanical and must all move together, or the drift laws will say so: the lexer's
keyword set, the CST node kind, the formatter's rendering, the tree-sitter grammar and its corpus, the highlight
queries, the LSP's completion and semantic tokens, the lexed fixtures, and every `.musa` file in `examples/` and
`stdlib/`.

## Target

- The `import` keyword in the lexer, the repaired import production in the parser, and its CST node; `import` accepts a
  quoted relative path or a module path, with an optional `as IDENT` that parses and round-trips and binds nothing.
- The migration diagnostic for `use "…";` and `use std::…;`, with a located applicable fix, and a snapshot test for each
  of the two old shapes.
- Every import site rewritten across `examples/`, `stdlib/`, and the test corpora.
- `editors/tree-sitter-musa` grammar, corpus, and queries updated, with the lexer drift law green.
- Formatter rendering plus its idempotence and round-trip laws; LSP completion, semantic tokens, and keyword
  documentation (prompt 84's exhaustive-by-construction table gains the keyword and cannot be left short).
- Prompt 93's frozen compatibility baseline is updated for this deliberate break, with the break named in the baseline
  rather than absorbed into it — `docs/language/README.md`'s graduation criterion 2 requires exactly this.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-lsp -p musa-project
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-lsp -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/tonal-construction.musa
cargo run -p musa -- render examples/canon.musa --to musicxml -o /tmp/canon.musicxml
npm --prefix editors/tree-sitter-musa test
```

Commit as `Separate the import keyword from the music splice`.

## Stop

- No change to what `use e;` means, to where it is legal, or to how it sequences.
- Do not accept both spellings, and do not add a compatibility flag or edition mechanism to allow the old one.
- No module nesting, no path segments beyond what exists today, and no collision-resolution behavior; prompt 110 owns
  all three. The `as` clause parses here and nothing more — no accessor, no binding, no meaning.
- No prelude, no implicit import, no re-export form.
