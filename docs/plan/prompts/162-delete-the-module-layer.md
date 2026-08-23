---
id: 162
slug: delete-the-module-layer
status: in-progress
depends_on: [161]
phase: 3
---

# Delete the Module Layer, and the Templates That Share Its `make`

## Task

`signature`, `structure`, `template structure` and `make` are an ML module system bolted onto a dependently typed
language that can already express everything they do. Delete them: a signature is a record type, a structure is a `let`,
a template structure is a function, and `make` is a call. `template` goes with them, because `make` is its only
instantiation form and there is no language in which one of the two survives alone. **Five words, fifteen declaration
sites, three files.**

## Read

- `docs/rules/language/01-surface.md` §6 and §6.1, which prompt 145 marked deprecated-and-owned-by-162, and §9's
  declaration-template row.
- `docs/rules/language/04-templates-and-modules.md` §§1–4 and §6. The package-and-`mod` prose above §1 is the module
  *tree* and is not this prompt's; §4's own opening says the two are not alternatives.
- The fifteen sites, found by `grep -rnE '^\s*(private )?(signature|structure|template|make)\b' stdlib/src examples`.
- `crates/musa-compiler/src/module.rs` — including its member walk, which reads a structure's `let` and `fn` members
  only and never registered a `data` member. That gap is why one law about structure-internal data was deleted rather
  than fixed during this overhaul.
- `crates/musa-compiler/src/template.rs`, and `docs/book/src/guide/names.md` §2 and §3, whose fences are quoted verbatim
  from the two examples and so move with them.
- [`58-the-module-layer-and-its-make.md`](../../notes/research/language-design-closure/58-the-module-layer-and-its-make.md)
  — the measurement this prompt's scope was repaired against, §8 being the sealing gap found while implementing it.
- `crates/musa-calculus/src/kernel/visibility.rs` and `crates/musa-compiler/src/document.rs`'s `top_level`, which
  together are why a `private` at a file's root is not enforced today.

## Design

**The translation, in four lines.**

| Was | Becomes |
| --- | --- |
| `signature S { … }` | `record S { … }` |
| `structure X: S { … }` | `let X : S = S { … }` |
| `template structure F(A: S): T { … }` | `fn F(a : S) -> T { … }` |
| `make F(X) as Y` | `let Y : T = F(X);` |

**And the fifth word, which the four lines have no row for.** A piece is not a value, so there is no expression a
`template piece` reduces to. It reduces to what was already true about files:

- `template voice V(…) { … }` with `make V(a) as n` becomes `fn V(…) -> EventTrack<WrittenTime> { … }` with
  `voice n { use V(a); }`. A voice template's body is a sequence of music statements, which is an
  `EventTrack<WrittenTime>` and nothing else.
- `template piece P(…) "T" { … }` with `make P(a, b) as n` becomes the piece written out, each parameter a `let` at the
  file's lexical root bound to the argument the `make` passed. This costs nothing: §6's own rule is that a `make` of a
  piece template *is* that file's piece, so a template made once is a piece written with ceremony, and both corpus files
  make each piece template exactly once.
- A body wanted at two different arguments becomes two files over one shared function. What is reusable about a
  parameterized piece is its *material* — a function of a `Key` and a `Scale` returning an `EventTrack<WrittenTime>` —
  and that is ordinary source any number of pieces may import. The piece was never the reusable part, because a file is
  one piece however the piece got there.

**The abstract-type question, answered here rather than assumed.** A signature's `data Hidden;` withholds a constructor.
A record field of type `Type` carries the same information — and it is exactly the construct that puts a record at
`Type 1`, which is why prompt 152 comes first and why this prompt is the one that *measures* whether the hierarchy was
needed. If the stdlib after this rewrite never needs a third level, say so in the commit; if it does, that is 152 paying
for itself.

**Sealing is module privacy, not a second mechanism — and the compiler does not wire it yet.** What `signature` bought
that a record does not is that a constructor could be withheld. The replacement is one rule instead of a layer: do not
export the constructor. `Visibility` and `ModuleId` exist in the kernel (136a), every member of a group carries one
(141n), and `Refusal::Private` fires on the rule. What no prompt ever asked for is `musa-compiler` giving a source
definition a module: `document.rs`'s `top_level` writes `module: None`, which the kernel reads as "written nowhere in
particular and hides from nobody". So a `private` at a file's root is nameable by everything that imports it, measured
in note 58 §8. Deleting the layer therefore *loses* sealing rather than restating it, and losing it silently is the one
thing this prompt may not do. Prompt 162a owns the wiring and the law; this prompt names the gap and stops there.

**Why the layer was wrong in the first place, recorded so it does not return.** *No sublanguage by subtraction* —
AGENTS.md's standing rule — cuts both ways. The module layer was a sublanguage by *addition*: a second, weaker
abstraction mechanism beside the one dependent records already provide, with its own scoping, its own matching rule, and
its own diagnostics, none of which composed with the rest of the language.

**The scope repair, recorded.** *Repaired before implementation; note 58 is the measurement.* Three of this prompt's
sentences did not survive contact with the corpus.

- *"`signature`, `structure`, `template structure` and `make` — all four go"* — `make` is also the instantiation form
  for a `template piece` and a `template voice` (`04-templates-and-modules.md` §1's `Make` rule ranges over eight
  declaration kinds, `structure` being one of eight). Removing `make` and keeping `template` leaves a declaration form
  with no elimination form; keeping `make` leaves the Target's own sentence false. `01-surface.md` §6 settles which way:
  it names all five words and names this prompt, and `docs/README.md`'s ladder puts a candidate specification above the
  prompt queue.
- *"Seven declaration sites in the whole corpus"* — seven is the module layer's share. Fifteen lines open one of the
  five declarations, in three files rather than two: note 58 §2 is the table.
- The Check's `! grep -rnE '\b(signature|structure|make)\b'` — fifty-three lines match that pattern and thirty-eight of
  them are prose, mostly "key signature" and "time signature". It cannot pass, and deleting every declaration does not
  make it pass. Anchored to declaration position it matches the fifteen and nothing else.

*Repaired again during implementation, on the evidence in note 58 §8.* A fourth sentence did not survive: *"the sealing
law restated as a module-privacy law over a `data` declaration"*. Module privacy is not enforced for source
declarations, so the law would assert behaviour the compiler does not have. The Target now deletes the two law suites
and hands the law to 162a with the wiring it needs.

**The two examples keep their filenames.** `examples/module-functor-study.musa` and `examples/template-study.musa` are
named for the mechanism they demonstrate, and both names go stale — but the tree-sitter fixtures, the book's verbatim
fences, `lsp_laws`, `session_laws`, and the wav-export determinism test are all keyed on the path. A rename is a second
change wearing this one's clothes, and renaming is cheap later and cheap never. What they demonstrate afterwards is the
replacement: a record bundling facts that travel together, and a function over it.

## Target

- `crates/musa-syntax`: the five words removed from the lexer, parser, CST node kinds (`TemplateDecl`, `MakeStmt`,
  `SignatureDecl`, `StructureDecl`), formatter, and highlighter; `editors/tree-sitter-musa` kept in step under the drift
  law, its per-example fixtures regenerated rather than edited.
- `crates/musa-compiler/src/module.rs` and `crates/musa-compiler/src/template.rs`: deleted, their callers routed through
  ordinary resolution.
- `stdlib/src/context.musa`: the signature and two structures rewritten as a record and two `let`s.
- `examples/module-functor-study.musa`, `examples/template-study.musa`: rewritten under the translation above, still
  compiling and rendering.
- `docs/rules/language/01-surface.md` §6, §6.1 and §9's declaration-template row;
  `docs/rules/language/04-templates-and-modules.md` §§1–4 and §6; `docs/book/src/guide/names.md` §2 and §3 and
  `docs/book/src/guide/cookbook.md`'s two recipes: the sections removed and what replaced them stated once.
- `crates/musa-compiler/tests/suite/`: `module_laws.rs` and `template_laws.rs` deleted. No sealing law here — see the
  Design and note 58 §8: sealing's replacement is not wired, and [`162a`](162a-module-privacy-for-source.md) restates
  the law once it is.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- format --check stdlib/src examples
! grep -rnE '^\s*(private )?(signature|structure|template|make)\b' stdlib/src examples
pnpm -r test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

`pnpm -r test` is here because the grammar moves and the tree-sitter drift law is checked on the other build system.
`make fmt-check` replaces `cargo fmt --check`, which reads only the Rust half and let four commits ship unwrapped
Markdown before 161. `--run-ignored all` carries the 30 staff-budget failures prompt 166 owns; the count must not move.

Commit as `Delete the ML module layer`.

## Stop

- No new visibility mechanism, and no wiring of the one that exists. Giving a source file a `ModuleId` is 162a's whole
  Target, and it needs its own answers about aliased imports and about a document whose root and piece are two sources
  of one file.
- No functor-like abstraction added back under another name, and no expression form for a declaration.
- No rename of the two example files, and no rename of `04-templates-and-modules.md`. Both are cheap later.
- No work on the package tree, `mod`, or `import` — that is the module *tree*, and `04-templates-and-modules.md`'s own
  opening says it is not this layer.
