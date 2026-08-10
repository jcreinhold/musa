# Language correction

**Status: governing over `docs/language/` from this document forward.** It corrects the candidate specification in
`docs/language/` the way `docs/course-correction.md` corrects the roadmap: where this document and a candidate section
disagree, this document wins, and the candidate section is repaired in the prompt that implements the correction.

It does not touch the temporal kernel, the score denotation, or the sound pipeline. Every correction here is about the
*language above* the kernel — how its types are justified, how its source is organized, and how its two most common
statements are spelled.

## 1. Why

Prompts 92–107 built the elaboration language quickly and well, and in doing so accumulated four faults that are
individually small and collectively make the surface read as improvised. They were found by writing
`examples/tonal-construction.musa` and `stdlib/tonal_harmony.musa`, not by auditing:

1. **A base type per prompt, none of them justified.** `02-core-calculus.md` §5 proves preservation, progress,
   determinism, and strong normalization for a fragment whose base types are exactly `bool`, `nat`, `ratio`,
   `duration`, `pitch`, and `interval`, and warns in its own words that later additions "are not smuggled into this
   theorem by an appeal to 'standard STLC.'" Prompts 100–107 then added `spelled_pc`, `pc12`, `scale`, `key`, `degree`,
   `frame`, `chord_class`, `triad`, `voicing`, `pcset12`, `row12`, and `roman`, and grew the compiler-owned primitive
   registry to sixty-nine entries. Not one has the compatibility case §5 requires. The theory block was widening an
   unproved fragment once per prompt with nothing recording that it was doing so.

2. **The module map is decorative.** `stdlib/manifest.toml` is compiled under `#[cfg(test)]` and is never consulted by
   the resolver. Actual resolution is four hand-maintained parallel lists in `crates/musa-compiler/src/imports.rs` — URI
   constants, `include_str!` constants, a `match` returning source, and a packaging array — which must be edited in
   lockstep and are checked against each other by nothing. The failure this permits is not hypothetical: at the time of
   writing, `stdlib/sequences.musa` is committed, appears in neither the manifest nor any of the four lists, is
   unreachable from any `use std::…;`, and the workspace builds green. A standard library module can be written,
   reviewed, and merged without ever being part of the standard library.

3. **Modules cannot nest, so names carry the hierarchy.** A bundled module's virtual URI is its file name, so
   `std::tonal::harmony` is unspellable and the module is called `std::tonal_harmony`. The underscore is not a naming
   preference; it is a missing feature wearing one. The same limitation makes the flat value namespace a hard
   constraint rather than a default: two modules cannot export the same name, with no way to disambiguate.

4. **`use` means two unrelated things.** `01-surface.md` carries both `import := "use" (STRING | "std" "::" IDENT) ";"`
   and `music-use := "use" expr ";"`, disambiguated only by whether the operand happens to be a string literal or a
   `std::` path. One is a file-level declaration; the other is the score's most common statement.

A fifth item was examined and **kept**: `fn f(x: τ) -> υ = expr;`. It is not a shorthand for a block form; there is no
block form, because there is no statement language. `02-core-calculus.md` §5 depends on it — "There is no
anonymous-function surface syntax. A checked named `fn` supplies the lambda" — and one total expression per function is
what makes strong normalization statable at all. It stays, and this paragraph exists so it is not re-litigated.

## 2. Correction A — one theorem, not one proof per domain

The obligation §5 states is real and must be discharged. Discharging it twelve times, once per musical domain, and
again for every domain prompts 109–112 add, would produce a dozen near-identical inductions that no one reads and that
silently rot. The correction is to prove it **once, parametrically**, with premises a test can check.

### A.1 Three primitive families

Every compiler-owned primitive belongs to exactly one family:

- **δ-primitives.** Every argument type and the result type is a base type or a finite constructor (`option`, `list`,
  product) over base types. No arrow occurs anywhere in the signature. This is where all sixty-nine musical operations
  live except the eliminators below.
- **Structural eliminators.** `nat_fold`, `list_fold`, `option_fold`, `map`, `filter`, `range`, `repeat`. These take
  function arguments and are already proved in §5.6.
- **Music primitives.** The constructors and controlled transforms of `music`, already proved in §5.7 and constrained
  by prompt 98.

The families are disjoint and exhaustive by construction; that both properties hold is a checked law, not a comment.

### A.2 The discipline a δ-primitive must satisfy

- **D1 — inertness.** Its base types have no eliminator. A closed value of a musical base type is an opaque constant:
  no reduction rule inspects its structure, and the only pattern that may match it is a literal or a catch-all, which
  §5.6 already requires to be followed by a catch-all arm. Nothing about a `scale` can be observed except by applying a
  δ-primitive to it.
- **D2 — totality.** For every tuple of closed values of the declared argument types, the primitive yields a closed
  value of the declared result type. Partiality is expressed *in the result type* as an `option`, never as a stuck
  term, a panic, or a diagnostic. `scale_chord` returning `none` for a collection that stacks to no nameable sonority
  is the intended shape; a `scale_chord` that reported an error would violate D2.
- **D3 — purity.** The result is a mathematical function of the argument values alone: no ambient context, no
  evaluation-order dependence, no hash-iteration order, no diagnostic emission.
- **D4 — finiteness.** The constructed-node count of the result is bounded by a function of the argument sizes, and
  that bound is charged to the §4 resource meter before construction begins.

### A.3 The theorem

> **Theorem 5 — conservative extension.** Let `𝔅` be the base types of the proved fragment. Adding a base type `b ∉ 𝔅`
> with no eliminator, together with any finite set of δ-primitives over `𝔅 ∪ {b}` satisfying D1–D4, preserves
> Theorems 1–4.

*Proof.* Take `R_b(t) ⟺ t : b ∧ t ∈ SN`, which is the clause §5.5 already assigns every base type, so candidate
properties (i)–(iii) hold by the existing induction with one additional leaf and no new case shape. **Preservation**:
by D2 the primitive's actual result type is its declared result type, so the `App`-analogue inversion goes through
unchanged. **Progress**: an application of a δ-primitive whose arguments are all values steps by D2; one with a
non-value argument steps by `Context`; so no δ application is stuck, and D1 removes the only other way a value of `b`
could appear at a redex position. **Determinism**: D3 makes the primitive a function, and the leftmost evaluation
context decomposition of §5.4 is unchanged because no new context former is introduced. **Strong normalization**: by D2
a δ redex whose arguments are values contracts to a *value* in one step, and values are normal, so the fundamental
lemma's new case is immediate — reducible arguments give a reduct in `R_b`. The arrow, product, option, list, and
`music` cases of §5.5–§5.7 quantify over the base-type set without inspecting it and therefore carry over verbatim. ∎

**Corollary.** A future musical domain needs no new proof. It needs a base type with no eliminator, primitives whose
signatures contain no arrow, and a discharge of D1–D4.

### A.4 What this does not prove

It proves the *type system* stays sound as the musical vocabulary grows. It says nothing about whether the vocabulary
is musically right — that a German sixth spells F♯ rather than G♭, that `ii` is minor in major, that a harmonic-minor
`III7` is honestly absent rather than rounded to a named chord. Those are claims about music, they are checked by the
law suites in `crates/musa-compiler/tests/`, and conflating the two would be exactly the overreach
`03-musical-domains.md` exists to prevent. Both statements belong in the specification and neither substitutes for the
other.

### A.5 The registry becomes the witness

`PRIMITIVE_OWNERSHIP` gains a family classification and a declared signature, and a law suite checks the premises
rather than trusting them:

1. every primitive appears exactly once and is classified into exactly one family;
2. every δ-primitive's declared signature contains no arrow in any position;
3. every base type reachable from a δ-primitive signature is on the inert list, and the checker admits no destructuring
   pattern for it;
4. every δ-primitive, evaluated over a generated finite sample of its argument domains, returns a value of its declared
   type — never a panic, never a diagnostic, and never a Rust-level absence unless the declared result is an `option`;
5. the sample is exhaustive where the domain is finite and generated to a documented bound where it is not.

### A.6 The standing budget

A new base type is admissible only with all three of: a stated reason no existing domain can carry the distinction, a
D1–D4 discharge with its registry entries, and a row in `03-musical-domains.md` giving its definition, source, and a
counterexample it rules out. A prompt that adds a domain without these three is mis-scoped and is repaired before it is
implemented. This is the guard the theory block did not have.

## 3. Correction B — packages, module trees, and a manifest that is load-bearing

### B.1 Shape

A **package** is a directory containing `musa.toml` and a source root. The bundled standard library becomes one:

```text
stdlib/
  musa.toml
  src/
    lib.musa
    list.musa
    option.musa
    pitch.musa
    scale.musa
    tonal/
      mod.musa
      harmony.musa
      sequences.musa
      schemas.musa
```

`lib.musa` declares its children, a directory module declares its own in `mod.musa`, and both are ordinary Musa source
read by the ordinary parser:

```musa
mod list;
mod option;
mod pitch;
mod scale;
mod tonal;
```

### B.2 Declaration, not discovery

Module resolution follows declarations, never a directory scan. Two errors follow, and both are the point:

- a `.musa` file under the source root that no `mod` reaches is rejected as **declared nowhere**;
- a `mod` naming no file is rejected as **missing**.

Either one would have caught `stdlib/sequences.musa` at the commit that introduced it.

### B.3 One source of truth

The four parallel lists in `imports.rs` are replaced by one table derived mechanically from the package manifest and
the `mod` tree. Source text is still embedded at build time — the standard library must remain readable at its
`musa-stdlib:/std/…` URIs with no filesystem access — but the embedding is generated from the traversal rather than
transcribed beside it. Whether that generation is a build script or a checked-in generated file is an implementation
decision for the prompt; that it has exactly one hand-edited input is not.

### B.4 Paths, and why binding stays flat

Import paths are qualified to any depth: `import std::tonal::harmony;`. The `std::IDENT` production becomes
`std::IDENT ("::" IDENT)*`, and `std::tonal_harmony` becomes `std::tonal::harmony`.

Binding, however, stays **flat by default**. An import brings the module's public names into the current flat value
namespace, so a score writes `numeral_chord(home, five)` rather than `harmony.numeral_chord(home, five)`. This is a
deliberate departure from Rust and from `04-templates-and-modules.md` §4's `Module.member` rule for static modules, and
the reason is that the two constructs have different readers. A `signature`/`module`/`template module` is read by
someone building an abstraction and qualification is information; a bundled theory module is read inside a score by a
musician, where `harmony.` before every call is noise charged on every line to prevent a collision that has not
happened.

The collision is handled where it occurs rather than everywhere it might:

```musa
import std::tonal::harmony;
import std::jazz::harmony as jazz;
```

Importing two modules that export the same name is an error naming both, and an `as` alias on either resolves it by
qualifying that one. Aliasing is therefore required exactly at a real conflict and absent otherwise.

### B.5 Relation to prompt 127

The remote-package prompt (pinned imports, lockfile, offline builds, no version solver) inherits this shape rather than
inventing a second one. A remote package is the same directory layout fetched by exact pin; the roadmap's rejection of
a registry and a version-range solver is unaffected. That prompt is repaired to build on this correction.

## 4. Correction C — `import` for imports, `use` for material

The import production is respelled `import`; `use` keeps its musical meaning alone.

```musa
import "motifs.musa";
import std::tonal::harmony;

voice upper {
    use subject;
    use canon(theme(), transpose(P5), 1/2);
}
```

The reasons, in order of weight: `use e;` is the score's most common statement and the one a musician writes — it
appears at a hundred and thirty-two sites against the import form's twenty-four — and it deserves the shorter, more
musical word; `import` is already what `01-surface.md` calls the production; and `import path;` needs no lookahead to
distinguish from anything.

`use` remains a splice, not an application: `use e;` checks `e : music`, instantiates it at the current cursor, and
sequences it. `use name(args);` is that same rule applied to a call, not a second invocation mechanism. This document
notes the distinction because the overloading made it easy to misread as function-application syntax, which it has
never been.

The old spelling becomes a **hard error with an applicable fix**, not a silent accept — prompt 56's diagnostic
machinery already carries fixes, and a language that quietly accepts both spellings has three years of mixed corpus
ahead of it.

## 5. Order and cost

The corrections are independent and are implemented in this order because it minimizes rework:

| # | Correction | Touches |
| --- | --- | --- |
| A | domain metatheory | `02-core-calculus.md` §5.8, the primitive registry, one law suite. No `.musa` source moves. |
| C | the `import` keyword | 24 import sites, the grammar, the formatter, the lexer's keyword set, tree-sitter, the LSP, lexed fixtures. |
| B | packages and module trees | the `stdlib/` layout, `imports.rs`, the reference generator, every `use std::…;` site. |

C runs before B so that import sites are rewritten once rather than twice. A runs first because it is the standing gate
on everything the theory block does next, and because nothing else depends on it.

## 6. What is explicitly not changed

- The temporal kernel, its operations, its normal forms, and its law suite.
- `music` as a context-reading recipe, the contextual-instantiation theorem, and prompt 98's higher-order constraint.
- `fn f(x: τ) -> υ = expr;` — see §1.
- The absence of user parametric polymorphism, recursion, `fix`, effects, and anonymous lambdas.
- The static `signature` / `module` / `template module` layer, which is a checking-time abstraction and is not a second
  import mechanism. §B.4 states where its qualification rule and the import rule deliberately differ.
- The rejection of a package registry, a semantic-version range solver, and implicit network access at compile time.
