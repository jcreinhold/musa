# 58. The module layer, and the templates that share its `make`

**Status: governs nothing.** `../../../rules/language/01-surface.md` §6 and §6.1 hold the decision; this page holds the
measurement behind prompt 162's repair. Written at prompt 162, before a line of it was implemented.

Prompt 162 was written to delete four words — `signature`, `structure`, `template structure`, and `make` — on the
argument that a signature is a record type, a structure is a `let`, a functor is a function, and `make` is a call. The
argument survives. The scope does not: `make` is not the module layer's word alone, and the corpus is twice the size the
prompt states.

## 1. What the four words are, and what they become

| Was | Becomes |
| --- | --- |
| `signature S { … }` | `record S { … }` |
| `structure X: S { … }` | `let X : S = S { … }` |
| `template structure F(A: S): T { … }` | `fn F(a: S) -> T { … }` |
| `make F(X) as Y` | `let Y : T = F(X);` |

Nothing in that table is in question. `01-surface.md` §6 states the same reduction and names 162 as the prompt that
performs it.

## 2. The measurement: fifteen sites in three files, not seven in two

Every line in `stdlib/src` and `examples/` that opens one of the five declarations, read with
`grep -rnE '^\s*(private )?(signature|structure|template|make)\b'`:

| File | `signature` | `structure` | `template structure` | `template piece` | `template voice` | `make` |
| --- | --- | --- | --- | --- | --- | --- |
| `stdlib/src/context.musa` | 1 | 2 | — | — | — | — |
| `examples/module-functor-study.musa` | 1 | — | 1 | 2 | — | 3 |
| `examples/template-study.musa` | — | — | — | 1 | 1 | 3 |
| **total** | **2** | **2** | **1** | **3** | **1** | **6** |

Seven of those fifteen are the module layer proper — the three in `context.musa` and the signature, functor, and two
functor `make`s in `module-functor-study.musa` — which is the count the prompt states. The other eight are declaration
templates and their instantiations, in the same two example files.

## 3. Derived: the four-keyword scope is not executable

**Proposition.** `make` is the only instantiation form for a `template piece` and a `template voice` (`01-surface.md`
§6, and `04-templates-and-modules.md` §1's `Make` rule, whose `κ` ranges over
`{library, piece, part, voice, performance, instrument, mix, structure}`). Therefore no ordering of the prompt's four
deletions leaves a consistent language:

- Remove `make` and keep `template`: a `template piece` can be declared and never instantiated, and
  `examples/template-study.musa` has no piece. A declaration form with no elimination form is not a smaller language, it
  is a broken one.
- Keep `make` and remove the other three: the Target's "the four keywords removed from the grammar" is false, and §6's
  sentence naming 162 stands unsatisfied.

So `template` goes with them, and 162 is five words rather than four. This is not a widening of the prompt's argument —
it is the same argument reaching the word the prompt did not list.

## 4. What settles it, and what does not

`01-surface.md` §6 opens: "**`template`, `signature`, `structure`, `template structure`, and `make` are being removed,
and prompt 162 removes them.**" `docs/README.md`'s ladder puts `rules/language/` at rank 4 and `plan/prompts/` at rank
6, so where the two disagree the specification wins and the prompt is the defect. The specification is a *candidate*,
which means anything above it wins over *it* — and nothing above it mentions declaration templates at all: neither the
constitution, nor `obligations.md`, nor any file under `across-stages/`. There is no higher document to consult, so §6
is the highest one that speaks.

## 5. What the fifth word becomes

The four-line table has no row for `template piece` and `template voice`, because the reduction is not to a value — a
piece is not a value and there is no expression whose type is "declaration". The reduction is to what was already true
about files:

- **`template voice V(…) { … }` plus `make V(a) as n`** becomes `fn V(…) -> EventTrack<WrittenTime> { … }` plus
  `voice n { use V(a); }`. A voice template's body is a sequence of music statements, which is an
  `EventTrack<WrittenTime>` and nothing else; `use` is the placement form the language already has. §6 says this in its
  own words about structures — "what leaves a structure is a value like any other" — and it is as true of a voice.
- **`template piece P(…) "T" { … }` plus `make P(a, b) as n`** becomes the piece written out, with each parameter a
  `let` at the file's lexical root bound to the argument the `make` passed. This costs nothing, because a file is
  already one piece: §6's own rule is that "a `make` of a piece template stands at the file root and *is* that file's
  piece". A template `make`d once is a piece written with extra ceremony, and both corpus files `make` each piece
  template exactly once.
- **A body wanted at two different arguments** is the case the template form existed for, and it becomes two files over
  one shared function. What is reusable about a parameterized piece is its *material* — a function of a `Key` and a
  `Scale` returning an `EventTrack<WrittenTime>` — and that function is ordinary source that any number of pieces may
  import and call. What was never reusable is the piece, because a file is one piece however the piece got there.

**Judged, not derived.** The third bullet is a choice against one alternative: keeping `template piece` and deleting
only the module layer, which §4 rules out. The cost of the choice is the corpus case `01-surface.md` §9 lists as
"key-parameterized piece / parameterized voice", whose desugaring column reads "declaration-template expansion". The
equality the row promises — distinct instance `Origin`s, `≈facts` after erasure — is unchanged by the translation, since
two calls of one function at two sites are at two `Origin`s for the same reason two `make`s were. What changes is the
mechanism named in the middle column.

## 6. The Check as written cannot pass

```sh
! grep -rnE '\b(signature|structure|make)\b' stdlib/src examples --include=*.musa
```

Fifty-three lines in the corpus match that pattern and fifteen of them are declarations. The other thirty-eight are
prose: "key signature" and "time signature" in eleven files, "the structure they already read", "a constructor is not
the place to make them", "the downbeat rests are what make them audible". A word-boundary grep over a language whose
domain vocabulary *is* those words checks nothing it means to check, and no amount of deleting declarations makes it
pass.

Anchoring it to declaration position fixes it exactly, because the language has no continuation lines that begin with
one of these words and no comment that begins without `//`:

```sh
! grep -rnE '^\s*(private )?(signature|structure|template|make)\b' stdlib/src examples
```

Run against the corpus today that matches fifteen lines and nothing else — the table in §2 is its output.

## 7. The surface area, so the prompt's Target can name it

| What | Size |
| --- | --- |
| `crates/musa-compiler/src/module.rs` | 1,205 lines |
| `crates/musa-compiler/src/template.rs` | 380 lines |
| `crates/musa-syntax`: `TemplateDecl`, `MakeStmt`, `SignatureDecl`, `StructureDecl` and their parsers, formatter, highlighter | 4 node kinds |
| `crates/musa-compiler/tests/suite/module_laws.rs`, `template_laws.rs` | two law suites |
| `docs/rules/language/04-templates-and-modules.md` | §§1–4 and §6; the package-and-`mod` prose at the top is the module *tree* and stays |
| `docs/rules/language/01-surface.md` | §6, §6.1, and §9's declaration-template row |
| `docs/book/src/guide/names.md` | §2 and §3, roughly 100 lines, whose fences are quoted verbatim from the two examples |
| `docs/book/src/guide/cookbook.md` | two recipes |
| `editors/tree-sitter-musa/test/` | per-example corpus and token fixtures for both files, regenerated rather than edited |

The two examples are named for the mechanism they demonstrate. They keep their filenames: the fixtures, the book's
fences, `lsp_laws`, `session_laws`, and the wav-export determinism test are all keyed on the path, and a rename would be
a second change wearing this one's clothes. What they demonstrate after the rewrite is the replacement — a record
bundling facts that travel together, and a function over it — which is the same lesson with the layer taken out from
under it.

## 8. Found during implementation: sealing has no replacement yet

The prompt's Design says sealing "becomes 'do not export the constructor,'" on the grounds that "Musa already has
module-private definitions and `Refusal::Private` already fires for them." The first half is true of the *kernel*. The
second is not true of any program a composer can write.

`Visibility::visible_from` in `crates/musa-calculus/src/kernel/visibility.rs` reads:

```rust
(Self::Public, _, _) | (Self::Private, None, _) | (Self::Private, Some(_), None) => true,
(Self::Private, Some(home), Some(viewer)) => home == viewer,
```

A declaration with no module "was written nowhere in particular and hides from nobody". And every definition
`musa-compiler` hands the core is written nowhere in particular: `document.rs`'s `top_level` sets `module: None`, and it
is the one constructor of a source definition. The compiler owns exactly two `ModuleId`s — `prelude::PHASE` and
`prelude::SOURCE` — and `SOURCE` is where the *context* stands, not where a definition lives. So the third arm never
fires for source code, and a `private` at a file's root is nameable by everything that imports it.

Measured rather than argued. With `stdlib/src/context.musa` rewritten as this prompt asks, a file that writes

```musa
import std::context;

let mine: Option<Frame> = c_major_home;
```

checks `ok`, and `c_major_home` is marked `private` three lines above the value that reads it.

Two consequences.

- **The law this prompt's Target asks for cannot be written.** "The sealing law restated as a module-privacy law over a
  `data` declaration" would be a law about behaviour the compiler does not have. Writing it to pass would mean asserting
  that a private name *is* reachable, which is the opposite of the sentence it is restating.
- **Deleting the layer loses a capability.** `structure` sealed by listing, and `crate::module`'s resolver enforced it —
  that is what `module_laws.rs`'s `a_member_the_signature_does_not_list_is_private` was about. Nothing enforces it after
  the deletion, and `stdlib/src/context.musa`'s three `private` members are the first to notice.

The gap is in `musa-compiler`, not in the core: 136a built `Visibility` and `ModuleId`, 141n gave every member of a
group one, and `musa-calculus`'s own laws cover the rule. What no prompt ever asked for is the compiler assigning a
module to each source file it declares from. Prompt 162a is that prompt, and 162's Target now says so instead of asking
for a law about a mechanism that is not wired.
