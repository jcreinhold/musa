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
   determinism, and strong normalization for a fragment whose base types are exactly `bool`, `nat`, `ratio`, `duration`,
   `pitch`, and `interval`, and warns in its own words that later additions "are not smuggled into this theorem by an
   appeal to 'standard STLC.'" Prompts 100–107 then added `spelled_pc`, `pc12`, `scale`, `key`, `degree`, `frame`,
   `chord_class`, `triad`, `voicing`, `pcset12`, `row12`, and `roman`, and grew the compiler-owned primitive registry to
   sixty-nine entries. Not one has the compatibility case §5 requires. The theory block was widening an unproved
   fragment once per prompt with nothing recording that it was doing so.

2. **The module map is decorative.** `stdlib/manifest.toml` is compiled under `#[cfg(test)]` and is never consulted by
   the resolver. Actual resolution is four hand-maintained parallel lists in `crates/musa-compiler/src/imports.rs` — URI
   constants, `include_str!` constants, a `match` returning source, and a packaging array — which must be edited in
   lockstep and are checked against each other by nothing. The failure this permits is not hypothetical: at the time of
   writing, `stdlib/sequences.musa` is committed, appears in neither the manifest nor any of the four lists, is
   unreachable from any `use std::…;`, and the workspace builds green. A standard library module can be written,
   reviewed, and merged without ever being part of the standard library.

3. **Modules cannot nest, so names carry the hierarchy.** A bundled module's virtual URI is its file name, so
   `std::tonal::harmony` is unspellable and the module is called `std::tonal_harmony`. The underscore is not a naming
   preference; it is a missing feature wearing one. The same limitation makes the flat value namespace a hard constraint
   rather than a default: two modules cannot export the same name, with no way to disambiguate.

4. **`use` means two unrelated things.** `01-surface.md` carries both `import := "use" (STRING | "std" "::" IDENT) ";"`
   and `music-use := "use" expr ";"`, disambiguated only by whether the operand happens to be a string literal or a
   `std::` path. One is a file-level declaration; the other is the score's most common statement.

Two more were found by *running* corrections A–C rather than by writing musa, and they are recorded here with the first
four because they have the same cause: a spelling was chosen locally, and nothing was comparing it to the rest of the
language.

5. **`module` and `mod` are one letter apart and unrelated.** Correction B needed a word for a package's children and
   took `mod`, while §4 of `04-templates-and-modules.md` already spends `module` on the static signature/module/functor
   layer. That is fault 4 exactly — two spellings a reader must tell apart by context — committed in the act of fixing
   fault 4. One of the two words has to go.

6. **`fn` is the only declaration whose body is not a block.** `piece`, `library`, `voice`, `motif`, `fragment`,
   `signature`, `module`, and `music` all write their contents between braces; `fn` alone writes `= expr;`. This
   document's first draft **defended** that as forced — "there is no block form, because there is no statement language"
   — and that argument is wrong. A block is not a statement language. Rust's function body is a *block expression* whose
   value is its trailing expression, and admitting that shape here adds no statement form, no `return`, no sequencing,
   and no new typing rule: `{ e }` is `e`. The rule the defense actually rested on — one total expression per function,
   which is what makes strong normalization statable — is untouched by how that expression is delimited. What the `=`
   bought was two fewer characters; what it cost was the one place where a reader who knows the rest of the language has
   to be told.

## 2. Correction A — one theorem, not one proof per domain

The obligation §5 states is real and must be discharged. Discharging it twelve times, once per musical domain, and again
for every domain prompts 109–116 add, would produce a dozen near-identical inductions that no one reads and that
silently rot. The correction is to prove it **once, parametrically**, with premises a test can check.

### A.1 Three primitive families

Every compiler-owned primitive belongs to exactly one family:

- **δ-primitives.** Every argument type and the result type is a base type or a finite constructor (`option`, `list`,
  product) over base types. No arrow occurs anywhere in the signature. This is where all sixty-nine musical operations
  live except the eliminators below.
- **Structural eliminators.** `nat_fold`, `list_fold`, `option_fold`, `map`, `filter`, `range`, `repeat`. These take
  function arguments and are already proved in §5.6.
- **Music primitives.** The constructors and controlled transforms of `music`, already proved in §5.7 and constrained by
  prompt 98.

The families are disjoint and exhaustive by construction; that both properties hold is a checked law, not a comment.

### A.2 The discipline a δ-primitive must satisfy

- **D1 — inertness.** Its base types have no eliminator. A closed value of a musical base type is an opaque constant: no
  reduction rule inspects its structure, and the only pattern that may match it is a literal or a catch-all, which §5.6
  already requires to be followed by a catch-all arm. Nothing about a `scale` can be observed except by applying a
  δ-primitive to it.
- **D2 — totality.** For every tuple of closed values of the declared argument types, the primitive yields a closed
  value of the declared result type. Partiality is expressed *in the result type* as an `option`, never as a stuck term,
  a panic, or a diagnostic. `scale_chord` returning `none` for a collection that stacks to no nameable sonority is the
  intended shape; a `scale_chord` that reported an error would violate D2.
- **D3 — purity.** The result is a mathematical function of the argument values alone: no ambient context, no
  evaluation-order dependence, no hash-iteration order, no diagnostic emission.
- **D4 — finiteness.** The constructed-node count of the result is bounded by a function of the argument sizes, and that
  bound is charged to the §4 resource meter before construction begins.

### A.3 The theorem

> **Theorem 5 — conservative extension.** Let `𝔅` be the base types of the proved fragment. Adding a base type `b ∉ 𝔅`
> with no eliminator, together with any finite set of δ-primitives over `𝔅 ∪ {b}` satisfying D1–D4, preserves Theorems
> 1–4.

*Proof.* Take `R_b(t) ⟺ t : b ∧ t ∈ SN`, which is the clause §5.5 already assigns every base type, so candidate
properties (i)–(iii) hold by the existing induction with one additional leaf and no new case shape. **Preservation**: by
D2 the primitive's actual result type is its declared result type, so the `App`-analogue inversion goes through
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

It proves the *type system* stays sound as the musical vocabulary grows. It says nothing about whether the vocabulary is
musically right — that a German sixth spells F♯ rather than G♭, that `ii` is minor in major, that a harmonic-minor
`III7` is honestly absent rather than rounded to a named chord. Those are claims about music, they are checked by the
law suites in `crates/musa-compiler/tests/`, and conflating the two would be exactly the overreach
`03-musical-domains.md` exists to prevent. Both statements belong in the specification and neither substitutes for the
other.

### A.5 The registry becomes the witness

`PRIMITIVE_OWNERSHIP` gains a family classification and a declared signature, and a law suite checks the premises rather
than trusting them:

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

The four parallel lists in `imports.rs` are replaced by one table derived mechanically from the package manifest and the
`mod` tree. Source text is still embedded at build time — the standard library must remain readable at its
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

### B.5 Relation to prompt 134

The remote-package prompt (pinned imports, lockfile, offline builds, no version solver) inherits this shape rather than
inventing a second one. A remote package is the same directory layout fetched by exact pin; the roadmap's rejection of a
registry and a version-range solver is unaffected. That prompt is repaired to build on this correction.

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
notes the distinction because the overloading made it easy to misread as function-application syntax, which it has never
been.

The old spelling becomes a **hard error with an applicable fix**, not a silent accept — prompt 56's diagnostic machinery
already carries fixes, and a language that quietly accepts both spellings has three years of mixed corpus ahead of it.

## 5. Correction D — `structure` for the static layer, `module` for nothing else

Two words, one letter apart, for unrelated things. The resolution is to rename the rarer one:
`04-templates-and-modules.md` §4's static layer is ML's, and ML's word for the thing that implements a signature is
**`structure`**.

```musa
signature TonalContext { let tonic: key; let collection: scale; }

structure CMajor : TonalContext {
    let tonic = key c major;
    let collection = signature_scale(tonic);
}

template structure InKey(home: key) : TonalContext { … }
make Home = InKey(key g major);
```

`module` then means one thing — a node of a package's tree — and `mod` is its declaration keyword, spelled short because
a module file is a list of them and nothing else. `signature`, `template`, and `make` are unchanged.

The alternative was to keep `module` for both and tell the two apart by what follows the name, as Rust tells `mod foo;`
from `mod foo { }`. It was rejected because the two are not two forms of one idea: a package module is a file, and a
structure is a checked value of a signature type. Rust's two `mod`s really are the same thing written twice.

`structure` costs less than it looks like: the layer is used at four sites in `examples/` and `stdlib/`, against `mod`'s
fourteen in the standard library alone and every package written after this.

## 6. Correction E — a function body is a block expression

`fn f(x: τ) -> υ { e }` replaces `fn f(x: τ) -> υ = e;`.

The braces are a **block expression**: `{ e }` elaborates to `e`, and it is admitted as an expression form so that a
function body is one, exactly as `piece`'s body is a piece body. Nothing else follows from it. There is no statement
sequence, no `let`-in-block, no `return`, and no early exit; a block holds exactly one expression, and a second one is a
parse error that says so. `02-core-calculus.md` §5's fragment gains one derived form with the equation `⟦{ e } ⟧ = ⟦e⟧`,
which is a definitional expansion and therefore changes no proof.

The `= e;` spelling becomes a **hard error with an applicable fix**, on correction C's precedent and for its reason: a
language that accepts both has a mixed corpus forever, and the fix machinery makes one spelling affordable.

Two consequences worth stating, because both look like objections and neither is one. A body that is a `music` block
reads `fn triad(register: frame) -> music { music { … } }`, and the doubled brace is honest — the outer one delimits the
function, the inner one is a `music` value, and they are genuinely two things. And a `match` body loses its trailing
semicolon, which is the one place the old form read well; it reads better inside braces, where every other `match` in
the language already sits.

## 7. Correction F — a type is spelled with a capital

Every type name is `UpperCamelCase`, and so is every constructor of one:

```text
Unit  Bool  Nat  Ratio  Duration  Pitch  NoteName  Interval
Scale  Key  Degree  Frame  ChordClass  Triad  Roman  Voicing
Pc12  PcSet12  Row12  Music  Option  List       None  Some
```

The reason is not familiarity. Six of these words are *also* music statement keywords — `pitch`, `music`, `scale`,
`key`, `degree`, and `frame` — which is why the type parser carries a whitelist of the keywords a type is allowed to be.
One word doing two jobs in two grammars is a collision the parser papers over; a capital settles it in the lexer, where
`key c major;` and `Key` are simply different words. `music { … }` and `Music`, the expression form and the type of what
it produces, stop being homographs.

`pitchclass` becomes `NoteName`. The compiler's type table spells it `pitchclass` and `01-surface.md` §3 spells it
`spelled_pc`; capitalizing forces the question, and the answer is that neither name was right. Open Music Theory 99
defines a pitch class as a group of pitches related by octave *and enharmonic* equivalence — A♭₄, A♭₃, and G♯₂ are one
pitch class — so `Pc12` is what a theorist means by the term, and a type that keeps C♯ apart from D♭ is not a pitch
class at all. What it is is the letter and accidental as written, without an octave, which OMT 3 calls a letter name.
`NoteName` says that in words a musician already owns, and says the distinction for free: a name is a spelling.

The lowercase spellings become **hard errors with applicable fixes**, on correction C's precedent and for its reason.

This adds no type, removes none, and changes what none of them mean.

## 8. Correction G — a type parameter is angle-bracketed

`Option<τ>` and `List<τ>` replace `option[τ]` and `list[τ]`.

`[` currently means three things: a list literal `[c4, d4]`, a list pattern `[x, ..xs]`, and a type parameter
`list[Pitch]`. The first two are one idea seen from two sides; the third is unrelated and merely shares the character.
Angle brackets give the type layer its own bracket and leave `[` with exactly one job.

The ambiguity that makes `<>` expensive elsewhere does not exist here. `Option` and `List` are the only parameterized
types and both are keyword-headed, and there is no user-written type application at all — so the parser knows it is
reading a type before it reaches the `<`, and the `a < b > (c)` reading that forces Rust's turbofish cannot arise. `<`
is not currently a token. `>` is, as the accent articulation inside a bar, and a type position is never a bar.

**This is notation, not polymorphism.** §10 records that the language has no user parametric polymorphism, and that
stands: `Option` and `List` remain two constructors the compiler owns, spelled the way a reader expects rather than
generalized. Corrections F and G change spelling and nothing else.

## 9. Order and cost

The corrections are independent and are implemented in this order because it minimizes rework:

| # | Correction | Touches |
| --- | --- | --- |
| A | domain metatheory | `02-core-calculus.md` §5.8, the primitive registry, one law suite. No `.musa` source moves. |
| C | the `import` keyword | 24 import sites, the grammar, the formatter, the lexer's keyword set, tree-sitter, the LSP, lexed fixtures. |
| B | packages and module trees | the `stdlib/` layout, `imports.rs`, the reference generator, every `use std::…;` site. |
| D | `structure` for the static layer | 4 declaration sites, the grammar, the formatter, tree-sitter, the LSP, `04-templates-and-modules.md` §4. |
| E | braced function bodies | every `fn` in `examples/` and `stdlib/`, the grammar, the formatter, tree-sitter, the LSP, `02-core-calculus.md` §5. |
| F | capitalized type names | every type annotation in `examples/` and `stdlib/`, the compiler's type table, the six dual-role keywords, the grammar, the formatter, tree-sitter, the LSP, `01-surface.md` §3. |
| G | angle-bracketed type parameters | 366 `option[`/`list[` sites, one lexer token, `type_atom`, the formatter, tree-sitter, the LSP, `01-surface.md` §1. |

C runs before B so that import sites are rewritten once rather than twice. A runs first because it is the standing gate
on everything the theory block does next, and because nothing else depends on it. D and E were found by running A–C and
are implemented immediately after them, before the theory block resumes writing `.musa` source that would otherwise have
to be rewritten twice — E in particular touches every function in the standard library, and the library grows with every
prompt after this one.

F and G were found the same way, by reading the corrected language rather than by writing it, and run for the same
reason: both rewrite every type annotation in the standard library, and the library grows with every prompt after them.
F runs first because it decides what the words are and G only decides what surrounds them; run the other way, the
bracket sweep would touch `option[…]` and the casing sweep would touch `Option[…]`, which is the same file twice for no
gain.

## 10. What is explicitly not changed

- The temporal kernel, its operations, its normal forms, and its law suite.
- `music` as a context-reading recipe, the contextual-instantiation theorem, and prompt 98's higher-order constraint.
- One total expression per function, which is what makes strong normalization statable. §6 changes how that expression
  is delimited and nothing about what it is.
- The absence of user parametric polymorphism, recursion, `fix`, effects, and anonymous lambdas. §8 gives `Option` and
  `List` the brackets a reader expects of a type constructor; it does not give the language a way to write one.
- What any type *is*. §§7–8 change how types are spelled and nothing about the set of them, their inhabitants, or the
  rules that relate them.
- The static layer itself, which is a checking-time abstraction and is not a second import mechanism. §5 renames its
  middle keyword and changes nothing about what it means; §B.4 states where its qualification rule and the import rule
  deliberately differ.
- The rejection of a package registry, a semantic-version range solver, and implicit network access at compile time.
