---
id: 127ab
slug: text-and-sums
status: pending
depends_on: [127aa]
phase: 3
---

# Add Text, Sums, and One Structural Result

> **Governed by the event-track and machine core installed by prompts 127a–127i.** Second of the five prompts that
> replace the source checker and evaluator; the chain is 127aa, 127ab, 127ac, 127ad, 127b.

## Task

Add `text`, binary sums, and one structural `Result` to the source language, with the surface syntax and exhaustive
`match` that make them usable, so that a primitive which can fail returns an ordinary value instead of opening a second
error channel.

## Read

- `docs/rules/language/02-core-calculus.md` §1, §1.1, and §2 — the type grammar, storable data, and the typing rules.
- Research `05-selected-calculus.md` §2, and its rule that a failing source primitive returns an ordinary `Result`.
- `docs/rules/style-guide.md` for the spelling of the new keywords and constructors.
- `crates/musa-language/src/{lexer,keywords,ast,highlight}.rs` and the formatter, and `editors/tree-sitter-musa` — the
  drift law holds the grammar to the real lexer, so a new keyword lands in both places in this commit.
- The inference engine installed by prompt 127aa, and its finding that **no source position in this type set mints a
  data variable** — the storable-data condition is proved at the unifier and first attached to a source position by
  `EventTrack[C, δ]` at prompt 127c.
- `stdlib/src/post_tonal/serial.musa`, whose `row`, `repeated_positions`, and `missing_classes` are the diffuse second
  error channel this prompt closes.
- *Open Music Theory*, `108-basics-of-twelve-tone-theory.md` and `110-row-properties.md` — a row is a bijection from the
  twelve order positions onto the twelve pitch classes, which is why failing to be one has exactly the two reasons
  `serial.musa` already names.

## Design

Add `text` as a base type with a literal form, an equality, and an exact encoding. It is storable data. It is not a
`Music` payload escape hatch: nothing in this prompt lets text carry structure the type system would otherwise check.

Add the binary sum `τ + τ` with its two injections, and define `Result` structurally over it rather than as a nominal
type with privileged compiler support — `Result` is what a sum is used for, not a new kind of thing. Both are storable
data exactly when their members are, by the structural rule of §1.1.

The sum gets **one** surface spelling. `Result<T, E>` writes the type, `Ok(v)` and `Err(e)` are its two injections, and
both elaborate to the one sum type, which is what "not a privileged compiler type" means here. A second neutral spelling
— `Either` with `Left` and `Right` — is not added: it would be a second name for one type, and every sum this language
has found a use for is a failure. A later prompt that needs a neutral sum spells it over the same type.

`match` is exhaustive. A non-exhaustive `match` is a located error naming the uncovered case; there is no catch-all that
silences the check and no runtime failure arm. Reuse the coverage machinery the checker already applies to the existing
patterns rather than adding a second decision-tree evaluator.

Sum elimination in the private evaluation core is direct: a case tree over the injections, evaluated strictly, charged
by the existing work meter. Do not invent join points.

Migrate the primitives that currently signal failure by returning `option` where the caller genuinely needs to know
*which* failure occurred; leave the rest returning `option`, and say in the prompt's completion note which moved and
why. A primitive that gains a `Result` result type states its error type in the ownership registry.

`row12_of` is the one that moves. `serial.musa` already documents the two exact reasons a sequence fails to be a row —
order positions whose pitch class already appeared, and pitch classes the sequence never names — and exposes each as a
separate function the caller runs *again on the original input* to learn which. That is the second error channel in its
diffuse form. Its result becomes `Result<Row12, (List<Nat>, List<Pc12>)>`, the two reasons together rather than a choice
between them, because a sequence of the wrong length can have either without the other. Every remaining `option` result
has exactly one reason and stays `option`.

Keep the formatter, highlighting, and tree-sitter grammar in step with the lexer in the same commit.

## Target

- `text`, its literal, its equality, and its exact encoding, admitted as storable data.
- Binary sums with injections, and structural `Result` defined over them.
- Exhaustive `match` over sums, with a located non-exhaustiveness diagnostic.
- Lexer, keywords, CST, AST accessors, formatter, highlighting, and `editors/tree-sitter-musa` updated together.
- Property tests for encoding round-trip and for sum/`Result` storable-data classification; a compile-fail test for a
  non-exhaustive `match`; and a refusal test **at the unifier** for a sum holding a function, since no source position
  demands storable data until 127c and a compile-fail test would therefore have nothing to refuse.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
find examples stdlib -name '*.musa' -print0 | xargs -0 -n1 cargo run -q -p musa -- check
```

Commit as `Add text, sums, and a structural Result`.

## Stop

- No nominal `data` declaration, record, private constructor, or generated fold; that is 127ac.
- No exception, effect handler, or hidden error channel beside the returned value.
- No deletion of partial calls or default parameters; that is 127ad.
- No `EventTrack` rename, machine type, scheduler, or DSP change.
