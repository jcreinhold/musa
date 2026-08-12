# How the elaboration language was decided, and then corrected

**Status: research record. Governs nothing.** `docs/rules/language/` is the specification. This file holds the two
things that were only ever in the memos it was built from — why each decision was made, and what was deliberately left
alone — so that the specification can be read without a stack of amendment documents in front of it.

Two documents were dissolved into `docs/rules/language/`:

- **The elaboration-language essay** (prompt 92's input) — evaluated whether Musa needed a typed total language above
  the temporal kernel, decided it did, and was split into the candidate specification. It was already marked "design
  history, no longer an input", and where it and the candidate differed the candidate was right: it was written to be
  implemented, the essay was written to decide what to implement.
- **The language correction** (prompts 108–113) — corrected seven faults the candidate accumulated through prompt 107.
  Its corrections are now stated in the specification itself.

## Why the corrections were needed

Prompts 92–107 built the elaboration language quickly and well, and accumulated faults that were individually small and
collectively made the surface read as improvised. **They were found by writing `examples/tonal-construction.musa` and
`stdlib/tonal_harmony.musa`, not by auditing** — which is the reusable lesson, and the reason the standard library and
the example corpus are held to the same checks as the compiler.

1. **A base type per prompt, none of them justified.** `02-core-calculus.md` §5 proved preservation, progress,
   determinism, and strong normalization for a fragment whose base types were exactly `bool`, `nat`, `ratio`,
   `duration`, `pitch`, and `interval`, and warned in its own words that later additions "are not smuggled into this
   theorem by an appeal to 'standard STLC.'" Prompts 100–107 then added twelve more base types and grew the
   compiler-owned primitive registry to sixty-nine entries, not one with the compatibility case §5 required.
2. **The module map was decorative.** `stdlib/manifest.toml` was compiled under `#[cfg(test)]` and never consulted by
   the resolver; actual resolution was four hand-maintained parallel lists in `crates/musa-compiler/src/imports.rs`,
   checked against each other by nothing. The failure was not hypothetical: `stdlib/sequences.musa` was committed,
   appeared in neither the manifest nor any of the four lists, was unreachable from any import, and the workspace built
   green. A standard-library module could be written, reviewed, and merged without ever being part of the standard
   library.
3. **Modules could not nest, so names carried the hierarchy.** A bundled module's virtual URI was its file name, so
   `std::tonal::harmony` was unspellable and the module was called `std::tonal_harmony`. The underscore was a missing
   feature wearing a naming preference.
4. **`use` meant two unrelated things** — a file-level import declaration and the score's most common statement —
   disambiguated only by whether the operand happened to be a string literal.

Three more were found by *running* corrections A–C rather than by writing musa, and have the same cause: a spelling was
chosen locally and nothing compared it to the rest of the language.

5. **`module` and `mod` were one letter apart and unrelated.** Correction B needed a word for a package's children and
   took `mod`, while `module` was already spent on the static signature/functor layer — fault 4 exactly, committed in
   the act of fixing fault 4.
6. **`fn` was the only declaration whose body was not a block.** The correction's own first draft *defended* this as
   forced — "there is no block form, because there is no statement language" — and that argument is wrong. A block is
   not a statement language. `{ e }` is `e`: admitting it adds no statement form, no `return`, no sequencing, and no new
   typing rule. The rule the defense actually rested on — one total expression per function, which is what makes strong
   normalization statable — is untouched by how that expression is delimited.
7. **Types were spelled like keywords.** Six type names were also music statement keywords, so one word did two jobs in
   two grammars and the type parser carried a whitelist to paper over it.

## The order they were run in, and why

A, C, B, D, E, F, G. C ran before B so import sites were rewritten once rather than twice. A ran first because it was
the standing gate on everything the theory block did next and nothing depended on it. D and E were implemented
immediately after A–C, before the theory block resumed writing `.musa` source that would otherwise be rewritten twice —
E touches every function in the standard library, which grows with every prompt. F ran before G because F decides what
the words are and G only decides what surrounds them; run the other way, the bracket sweep would touch `option[…]` and
the casing sweep would touch `Option[…]`, which is the same file twice for no gain.

The largest single sweep was G: 366 `option[`/`list[` sites.

## What the corrections deliberately did not change

Recorded because each one looks like a consequence of a correction and is not:

- The temporal kernel, its operations, its normal forms, and its law suite.
- `music` as a context-reading recipe, the contextual-instantiation theorem, and prompt 98's higher-order constraint.
- One total expression per function. Correction E changed how that expression is delimited and nothing about what it is.
- The absence of user parametric polymorphism, recursion, `fix`, effects, and anonymous lambdas. Correction G gives
  `Option` and `List` the brackets a reader expects of a type constructor; it does not give the language a way to write
  one.
- What any type *is*. Corrections F and G change how types are spelled and nothing about the set of them, their
  inhabitants, or the rules relating them.
- The static layer, which is a checking-time abstraction and not a second import mechanism. Correction D renames its
  middle keyword; where its qualification rule and the import rule deliberately differ is stated in
  `docs/rules/language/01-surface.md`.
- The rejection of a package registry, a semantic-version range solver, and implicit network access at compile time.
