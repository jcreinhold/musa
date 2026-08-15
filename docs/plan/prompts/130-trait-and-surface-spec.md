---
id: 130
slug: trait-and-surface-spec
status: done
depends_on: [129]
phase: 3
---

# Specify Records, Enums, Traits, and the Surface That Uses Them

## Task

Say what an author actually types. Rewrite `docs/rules/language/01-surface.md` for the core prompt 129 specified, and
add `docs/rules/language/10-traits.md`: nominal records with projection and nested `with` update, namespaced typed
enums, traits as dependent records with coherence and no search, operators and indexing routed through traits, inherent
methods and type namespaces under exact-receiver lookup, and the surface forms for building and iterating collections.

## Read

- `docs/rules/language/01-surface.md` in full, and `04-templates-and-modules.md` for the declaration forms this grammar
  has to sit beside without duplicating.
- `docs/rules/language/02-core-calculus.md` as rewritten by prompt 129 — every form added here elaborates into it, and a
  surface form with no elaboration is the defect this prompt exists to avoid.
- Prompt [127dcfac](127dcfac-record-update.md) — `with` update already landed on nominal data for one field at one
  level. This prompt widens it, so read what it deliberately did *not* do and why.
- `stdlib/src/adapters/staff.musa`: `data Pending`'s eight-field destructure, the `text_equal(kind, "PitchLiteral")`
  dispatch table, and — the sharpest piece of evidence — the `Untied` constructor that collides with the staff package's
  own `Tie`. `crates/musa-compiler/src/core.rs`'s `names_a_phase_type` exists only because constructor names are
  currently flat within a module.
- `crates/musa-compiler/src/core.rs`'s builtin registry — the 131 entries are the exact set the operator and method
  design has to cover, and prompt 143 is the prompt that deletes them.
- Peyton Jones ch. 3 (translating a high-level language into the core) and ch. 4–5 (structured types, the semantics of
  pattern matching) — the desugarings in this document are the same kind of translation, and ch. 3's argument that a
  language for programmers needs abstractions and local definitions is the argument in root `AGENTS.md` under "No
  sublanguage by subtraction".
- `docs/rules/style-guide.md`, to see what a rule there costs — every rule names its diagnostic, which is why this
  prompt adds none.

## Design

**Records.** A record is a nominal declaration of named fields, with projection `p.field`, construction by field name,
and `with` update along a *path*: `p with { region.anchor = a }`. Prompt 127dcfac's single-field, single-level update
was the smallest thing that unblocked one program; the evidence that it was too small is `Pending`, an eight-field
product written as a single-constructor `data` and destructured in full to read one field. Record types are Σ-with-names
in the core (129), so η holds and two records with the same fields are convertible.

**Enums, and namespaced constructors.** An enum is a nominal sum whose constructors live in the type's namespace:
`TokenKind::PitchLiteral`, not a bare `PitchLiteral` competing for the module's one flat name space. This is not
cosmetic. The printer splice in prompt 127dcfb failed because the staff adapter's `Untied` and the staff package's
`Untied` were the same name, and the compiler now carries `names_a_phase_type` to work around it. Namespaced
constructors delete the collision at its source. A bare constructor name is still accepted where the expected type is
known — that is the check direction 129 already gives us, not an inference heuristic.

**Traits: coherent, dictionary-elaborated, and deliberately boring.**

- A trait is a dependent record of methods with an optional set of laws stated in prose; an impl is a value of it.
- **Coherence**: at most one instance per (trait, head type). No overlap, no specialization, no defaulting.
- **Orphan rule**: an impl lives in the package that declares the trait or the package that declares the head type.
- **No search**: instance selection is a lookup on the head type's constructor, matched structurally. Instance heads
  carry a decreasing measure so matching terminates; a context reduction that does not decrease it is refused at the
  declaration, not at the use.
- **Ambiguity is an error**, never a default. An unresolved instance names the type it could not find and the two
  candidates when there are two.
- **`where` on every public generic signature.** A generic function states its constraints; nothing is inferred into a
  public signature.
- **Local dictionaries beat global impls.** A dictionary bound by an enclosing `where` is used in preference to a global
  instance for the same head, so a generic function's behaviour is fixed by its own signature.

**`Eq` and `DecEq`, and a deviation recorded.** The approved plan wrote `PartialEq` for the `==` trait. This prompt
narrows it to **`Eq`**: the language is total and exact, every instance is a decision procedure, and a partial equality
is an artifact of languages that must accommodate floats and NaN — Musa's floats live at the DSP edge, outside the
source language. `DecEq A` is the dependent strengthening that returns a proof or a refutation, and it is what `Vec`
index reasoning uses. Propositional `=` (129's `Id`, introduced by `refl`) and computational `==` stay distinct words
for distinct things, and the document says which one a reader is looking at every time either appears.

**Operators.** `==`, `<`, `+`, `-`, `*`, `/`, and indexing are surface syntax for trait methods. Two rules keep this
from becoming overloading-by-another-name: an operator resolves only when the concrete head type is known or a `where`
constraint supplies the dictionary, and **an operation that can fail keeps its failing shape** — `ratio_sub` returns
`Result` today and its operator form returns `Result` tomorrow. A partial operator is how a total language quietly grows
a hole.

**Inherent methods and type namespaces, under exact-receiver lookup only.** `x.add(y)` resolves when `x`'s concrete type
is known. A value of a generic parameter `A` never acquires `.add` from anywhere; the caller writes the constraint or
the qualified path. `Duration::of(n)` is how a value of a named type is built — **not** return-type-directed
overloading, which is search in disguise and makes elaboration order-dependent. Explicit qualification
(`Trait::method(x)`, `Type::item`) is always available and always resolves.

**Collections at the surface only.** A list literal `[a, b, c]`, the `Buildable`/`Iterable` trait pair, and method-style
`map`/`filter`/`fold`/`collect`. That is the grammar; prompt 141 owns the library and the length-indexed vector. No
comprehension in v1: a comprehension is sugar over `map` and `filter` (Peyton Jones ch. 7), and adding sugar before the
thing it sugars has a user is the wrong order.

**The acceptance corpus grows with the grammar.** §9's corpus correctness relation is what makes this document
checkable, so each added form gets its corpus rows — accepted, rejected, and the diagnostic the rejection names.

## Target

- `docs/rules/language/01-surface.md`: §1's added grammar extended with record, enum, trait, and impl declarations,
  method-call and path forms, operator forms, `where` clauses, and collection literals; each with its desugaring into
  129's core; §9's corpus extended with the accepted and rejected rows for every one.
- `docs/rules/language/10-traits.md`, new: coherence, the orphan rule, instance lookup and its termination measure,
  dictionary elaboration, `where` and local-beats-global, the operator-to-trait table covering the builtin set prompt
  143 will collapse, inherent methods and type namespaces with exact-receiver lookup, `Eq` versus `Id`, `DecEq`, and an
  explicit list of what is refused and why.
- `docs/rules/language/README.md`: the document-map row for `10-traits.md`, and the graduation list if it now names a
  different set of documents.
- `docs/rules/language/citations.md`: the new claims — dictionary elaboration, coherence, orphan rules, and the
  case-tree desugaring of method syntax.

## Check

```sh
python3 scripts/renumber-prompts.py audit
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
/Users/jcreinhold/.cargo/bin/mdwright check docs/rules docs/plan
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
git diff --check
```

Commit as `Specify records, enums, traits, and their surface`.

## Stop

- No code, no grammar file, no fixture, no `stdlib/` change.
- `01-surface.md` §7 (kernel documents and quotation) is not rewritten here. Prompt 131 generalizes it, and rewriting it
  twice would leave two accounts of the same construct in one release.
- No style-guide rule. Every rule in `docs/rules/style-guide.md` names its diagnostic, and none of these diagnostics
  exist yet; prompt 137 adds the rules with the code that reports them.
- No trait search, no overlapping instances, no specialization, no defaulting, no return-type-directed overloading, no
  auto-deref, no method resolution on a generic parameter. Each of those turns a lookup into a search, and the whole
  point of this design is that there isn't one.
- No comprehension, no `do`-notation, no operator sections, no user-defined operator symbols.
- No collection library and no `Vec A n`. That is prompt 141, and it needs the core built first.
