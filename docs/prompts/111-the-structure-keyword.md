---
id: 111
slug: the-structure-keyword
status: done
depends_on: [110]
phase: 3
---

# `structure` for the Static Layer, `module` for a Package Node

## Task

Rename `04-templates-and-modules.md` §4's static module layer from `module` to `structure`, and `template module` to
`template structure`, so that "module" means one thing in this language: a node of a package's tree, declared with
`mod`. `signature`, `make`, sealing, and every rule about what a static module *is* are unchanged — this prompt changes
one keyword and everything that spells it.

## Read

- `docs/language/04-templates-and-modules.md`, which governs this prompt, and `docs/scratch/60-language-decision-record.md` fault 5 for how the collision was introduced.
- `docs/language/04-templates-and-modules.md` §4, the layer being renamed, including its qualification rule and its
  generative-identity rule.
- `docs/language/01-surface.md`, which states where the static layer's qualification and the import rule
  deliberately differ. That distinction is the reason the two words were confusable and the reason they must not be.
- Prompt 109 for the migration-diagnostic shape a keyword change takes here, including its applicable fix.
- Prompt 80 for the tree-sitter drift law: the grammar is held to the real lexer token-for-token, so a keyword change is
  a change in two places that a test compares.

## Design

`mod tonal;` and `module CMajor : TonalContext { … }` are one letter apart and unrelated: the first names a file, the
second is a checked value of a signature type. That is prompt 109's fault repeated inside its own correction, and it is
worse than 109's, because `use`'s two meanings at least both concerned material.

The rarer word moves. §4's layer is ML's, and ML calls the thing that implements a signature a **structure** — so
`structure` is not an invention, it is the name this construct has had since 1984. It is written at four sites in the
whole corpus, against `mod`'s fourteen in the standard library alone and every package written after prompt 110.

```musa
signature TonalContext { let tonic: key; let collection: scale; }

structure CMajor : TonalContext {
    let tonic = key c major;
    let collection = signature_scale(tonic);
}

template structure InKey(home: key) : TonalContext { … }
make Home = InKey(key g major);
```

Keeping `module` for both and telling them apart by what follows the name — as Rust tells `mod foo;` from `mod foo { }`
— is rejected. Rust's two forms are the same idea written twice; these are two ideas. A reader who has to scan past a
name to learn which construct they are in is being charged for a decision the language could have made.

Nothing else moves. `M.member` still qualifies, a sealed structure still hides what its signature omits, generative
identity is still the digest of the instance, and `make` still names an instantiation. The compiler's `module.rs` keeps
its name: it is about the static layer, and the crate has no package-tree module of its own to confuse it with —
`package.rs` is that.

The migration is a hard error with an applicable fix, on prompt 109's precedent and for its reason.

## Target

- `structure` and `template structure` in the lexer, parser, CST node kinds, and formatter; `module` removed from that
  position.
- The migration diagnostic for `module Name : Sig { … }` and `template module`, each with a located applicable fix and a
  snapshot test.
- Every declaration site rewritten across `examples/`, `stdlib/`, and the test corpora, including
  `examples/module-functor-study.musa` and `stdlib/src/context.musa`.
- `docs/language/04-templates-and-modules.md` §4 repaired to the new spelling throughout, including its grammar
  production and every example.
- `editors/tree-sitter-musa` grammar, corpus, and queries updated, with the lexer drift law green.
- LSP completion, semantic tokens, and prompt 84's exhaustive-by-construction keyword documentation table.
- Prompt 93's frozen compatibility baseline updated for this deliberate break, with the break *named* in the baseline
  rather than absorbed into it.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-lsp -p musa-project
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-lsp -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/module-functor-study.musa
cargo run -p musa -- check examples/tonal-construction.musa
npm --prefix editors/tree-sitter-musa test
```

Commit as `Rename the static module layer to structure`.

## Stop

- No change to what a static structure means: qualification, sealing, generative identity, functor application, and the
  checking-time-only rule are all exactly as §4 states them.
- Do not accept both spellings, and do not add a compatibility flag or edition mechanism to allow the old one.
- No change to `signature`, `make`, or `template` as words, and no `functor` keyword.
- No change to `mod`, to package trees, or to import resolution; prompt 110 owns those and they are done.
- Do not rename `crates/musa-compiler/src/module.rs`. It is the static layer's module and the rename is about the
  surface language.
