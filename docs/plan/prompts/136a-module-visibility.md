---
id: 136a
slug: module-visibility
status: done
depends_on: [136]
phase: 3
---

# Let a Package Hide What It Maintains

## Task

Give the language one visibility marker. `private` before a top-level declaration makes that declaration nameable only
inside its own module; `private` before an enum's cases keeps the type public and makes its constructors module-local,
so a package can maintain an invariant its clients cannot break. Grammar, CST, formatter, highlighting, and tree-sitter
in `musa-language`; the resolution rule and its diagnostics in `musa-calculus`. Record the rule in
`docs/rules/language/01-surface.md` §1.3 and `docs/rules/language/04-templates-and-modules.md` §4.

## Read

- [`docs/notes/research/language-design-closure/43-dependent-language-trial.md`](../../notes/research/language-design-closure/43-dependent-language-trial.md)
  §5.1 and §14 — the finding that produced this prompt. Three of note 28's five programs rely on a constructor a client
  cannot reach, and `Chord::NamedChord(ChordSymbol::GSeven, [Spelling::C])` — a chord whose symbol contradicts its tones
  — is constructible today by anyone. The trial could not repair it, because prompt 132's Stop forbade adding a
  mechanism, so it recorded the gap and named this prompt as the answer.
- [`docs/notes/research/language-design-closure/28-five-programs.md`](../../notes/research/language-design-closure/28-five-programs.md)
  §7 — "Hidden constructors | tonal, phrase, tuning | keep", the row that has been in the corpus since before the
  dependent core and has never had a spelling.
- `docs/rules/language/04-templates-and-modules.md` §4 — the *existing* privacy mechanism, and its limits. A structure
  member the signature does not list is already private, so this prompt is not inventing privacy; it is giving privacy
  to declarations a `structure` body cannot hold. A `structure` admits `binding | function` and no type declaration, so
  no arrangement of signatures hides a constructor.
- `docs/rules/language/01-surface.md` §1.3 as prompt 130 wrote it and prompt 132 corrected it — namespaced constructors,
  named-field cases, and the bare-constructor rule in checking position, all of which this prompt has to keep working.
- `stdlib/src/adapters/staff.musa` and `stdlib/src/notation/staff.musa` — roughly 150 top-level definitions of which a
  handful are the interface. That is the second measurement, and it says whether the marker is worth having beyond
  constructors.
- `crates/musa-compiler/src/module.rs` — what a module scope has to be able to answer, and **not** where this lands.
  That `NameScope` belongs to the checker prompt 142 deletes; building the filter there would be work thrown away, and
  the Stop below forbids it.
- `crates/musa-calculus/src/{scope.rs, context.rs, family.rs, declare.rs}` — where a name is resolved *after* 142. A raw
  variable resolves against binders and against the declared groups a `Cx` was extended by, and `Cx` is flat: there is
  no module in it for a filter to test against. That is the missing prerequisite this prompt supplies, and it is why the
  filter cannot be "a filter on an existing lookup" until the lookup has a boundary to know about.
- `crates/musa-calculus/src/origin.rs` — the precedent for the shape that boundary takes. An `Origin` is "a number the
  caller assigned, and the only thing this crate does with that number is keep it attached to the right term"; a module
  identity is the same bargain, and the argument that keeps the core a leaf is the same one.
- *A Philosophy of Software Design* ch. 5 and ch. 8 — information hiding, and the argument that an invariant maintained
  by a smart constructor is decoration if the raw constructor is reachable. Ch. 7 is the reason this prompt is one
  mechanism rather than two: a "constructor privacy" feature and a "declaration privacy" feature would be the same
  abstraction one level down.

## Design

**Public by default, `private` as the explicit subtraction.** Every declaration stays visible outside its module unless
marked, so no existing program changes and prompt 142's migration adds markers where a package wants them rather than
everywhere. This is the opposite of Rust's default and the opposite of what ch. 5 would argue for a fresh language, and
the argument it loses to is specific rather than general: Musa's packages are *vocabularies* — `std::notation::staff`
exists to be named — and flipping the default would mean marking every one of `stdlib/`'s definitions inside the single
migration that is already the largest prompt in the pass. Record the counter-argument in `01-surface.md` where the rule
is stated, with the re-opening condition: a measured count of declarations that *should* have been private and were not,
taken after 145 and 146.

**One keyword, two positions.**

```musa
private fn dotted_factor(dots: Dots) -> Ratio { … }

enum Chord {
    private NamedChord(ChordSymbol, List<Spelling>),
    private AnonymousChord(List<Spelling>),
}
```

Before a top-level `let`, `fn`, `record`, `enum`, `trait`, `impl`, or `structure`, it hides the declaration. Before an
enum's cases it hides the constructors and leaves the type public, which is the shape note 28 wrote as
`pub type Chord: private NamedChord(…)` and which this grammar can say without a second modifier.

**All the cases or none of them.** A mixed enum — one private case beside a public one — is refused, naming both cases.
The reason is coverage: outside the module a `match` on a partly private type could still be written, and the arms it is
allowed to write would never exhaust the type, so every such `match` would need a catch-all for cases the author cannot
see. That is a worse thing to explain than a refusal. The evidence agrees: in the trial's `Chord`, both cases are
package-maintained. Re-opening needs a program with a genuinely public case beside a private one, and a stated answer
for what its `match` coverage means.

**Outside the module, an abstract type is not matched.** A `match` whose scrutinee has private constructors is refused
where it is written, naming the type, its module, and the fact that its constructors are private — not silently made
inexhaustive. A client eliminates through whatever the package exports (`members`, `analyze`, `write`), which is the
package's own interface and the point of hiding the constructors. The bare-constructor rule in checking position
(`01-surface.md` §1.3) is unaffected inside the module and refuses outside it for the same reason and with the same
message.

**The core has to learn what a module is, and it learns exactly one thing about it.** `Cx` is flat: a raw name resolves
against binders and against the groups the context was extended by, and nothing in `musa-calculus` can say whether an
elaboration is happening inside a declaration's own module. Give it an opaque module identity — a number the caller
assigns — carried on a `Cx` and stamped on a declared group when it is brought into scope, compared only for equality
and never interpreted. That is `Origin`'s bargain word for word (`origin.rs`: the core "does not know what a file is,
what a span is, or what a syntax node is, and it must not learn"), and it is what keeps this rule in the crate that is
provably right without importing the compiler's idea of a package. `musa-calculus` never mints one; the caller does, and
no caller mints a real one until 142.

The alternative — have the compiler build each module's `Cx` without the private declarations at all — is rejected, and
for a stated reason rather than a preference: a name that is absent is `UnknownName`, and this prompt's whole value is
in the two refusals that need the checker to know the name *exists and is hidden here*. "No such thing as `NamedChord`"
sends a reader looking for a typo; "`NamedChord` is a private case of `Chord`, which `std::theory::chord` maintains
through `build`" sends them to the interface.

**Visibility is then a filter on one lookup, not a second resolution path.** Resolution already walks the declared
groups; a private declaration is one it declines to answer with when the querying `Cx`'s module is not the one stamped
on the group, and it declines by *naming* rather than by falling through to "not found". Two things follow and both are
checkable: a private name is still in its module's scope, so a sibling definition reads it by its bare name with no
ceremony; and a private name that is never read inside its own module is dead code, which the existing
unused-declaration diagnostic should say rather than this prompt inventing a second one.

**`private` and `signature`/`structure` do not overlap and do not conflict.** A structure seals by *listing* — the
signature is the interface and everything else is private. A module hides by *marking*. They compose without a rule:
`private` inside a structure body is redundant and is refused as such, naming the signature that already hides it, which
is better than accepting a marker that means nothing.

**What this does not become.** Not a visibility *lattice* — no `pub(crate)`, no `pub(super)`, no package-visible tier.
One boundary, the module, because that is the boundary `module.rs` already has and the only one any program in the
corpus wants. A second tier is a new prompt with a program that needs it.

**Laws.** A private declaration is nameable from a sibling definition in its module and from nowhere else, including
through an `import` alias and through a re-export. A public type with private constructors is nameable, usable in a
signature, and matchable inside its module only. Marking a declaration private changes no accepted program that did not
name it. Parsing round-trips losslessly and formatting is idempotent with the marker present. A mixed-visibility enum, a
`private` structure member, and an outside `match` on an abstract type each produce their named diagnostic.

## Target

- `musa-language`: the `private` keyword, its grammar and CST positions, formatter layout, highlighting, and completion.
- `editors/tree-sitter-musa`: grammar and queries, with the drift test green.
- `musa-calculus`: an opaque module identity on `Cx` and on a declared group, the visibility a `RawData` and its cases
  carry, the resolution filter, the mixed-enum refusal, the outside-`match` refusal, and their diagnostics with `musa
  explain` codes. A context with no module named is inside every module, which is what keeps every existing test and
  every existing caller unchanged.
- `crates/musa-language/tests/suite/` and `crates/musa-calculus/tests/suite/` cases, including the `Chord` program from
  note 43 §5.1 written out as a fixture: the package builds a `NamedChord` through `build`, and the client that tries to
  build one directly is refused.
- `docs/rules/language/01-surface.md` §1.3 and §1 grammar, and `docs/rules/language/04-templates-and-modules.md` §4 —
  the rule, the public-by-default argument with its re-opening condition, and the non-overlap with sealing.
- `docs/plan/code-map/` rows for both crates.
- No `stdlib/` or `examples/` change: nothing is marked until 142.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-language -p musa-calculus
cargo nextest run --workspace
cargo clippy --all-targets -p musa-language -p musa-calculus -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Let a package hide what it maintains`.

## Stop

- No visibility tier below or above the module. No `pub` keyword, no export list, no re-export form.
- No `musa-compiler` wire-up and no migration of `stdlib/` or `examples/`; the marker reaches real packages in 142, and
  the two adapter rewrites at 145 and 146 are what measure whether it earned its keep. `musa-calculus` takes the module
  identity as a parameter and mints none, so nothing supplies a real one until then — the rule is testable here because
  a test can assign the numbers a compiler will later assign.
- **No module system in `musa-calculus`.** One opaque identity, compared for equality, is the whole of it. No path, no
  nesting, no parent relation, no `import`, no notion of a file. If a refusal needs to *print* a module's name, the
  caller supplies the text the way it supplies a span for an `Origin`.
- No change to `signature`/`structure` sealing, and no new privacy mechanism beside it.
- No abstract *type* form — a type whose definition is hidden and whose constructors are hidden with it is a different
  feature, needs a program, and would need its own conversion rule in `02-core-calculus.md`.
- No change to `01-surface.md`'s bare-constructor rule, to enum namespacing, or to record structurality. Records are
  structural (`01-surface.md` §1.2), so a hidden record field is not this prompt's form and would contradict that rule.
- No constitution or obligations amendment. This adds a surface marker to a candidate specification; it decides nothing
  the governing documents decide.
