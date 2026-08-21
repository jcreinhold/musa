---
id: 165a
slug: equality-for-declared-types
status: pending
depends_on: [165]
phase: 3
---

# Give Declared Types an Equality, or Decide They Do Not Get One

## Task

The compiler owns `Eq` and supplies exactly five instances — `Text`, `Ratio`, `Duration<WrittenTime>`, `Pitch`,
`Interval` — a list closed by what a **literal pattern** can name rather than by taste. No *declared* type has one, on
the recorded reasoning that `true` and `7` are constructor patterns and an instance for them "would be a second way to
ask a question ι already answers." Prompt 163 makes `==` the spelling for equality; prompt 165 rewrites the standard
library into declared types. This prompt decides, on that corpus, whether a declared type gets an equality — derived,
written, or neither — and implements the answer.

## Read

- `crates/musa-compiler/src/prelude.rs`, `eq_class` and `eq_instances`, and **both** doc comments in full. The first
  says why the compiler owns `Eq` rather than `stdlib/` (a piece that imports nothing still writes `==` and still writes
  literal patterns). The second says why the instance list is five and why `Bool` and `Nat` are deliberately not on it.
  That second argument is this prompt's central question, because it generalizes: a derived `Eq` for a declared type is
  a second route to a question the case tree already answers, and admitting it is a decision rather than a convenience.
- `docs/rules/language/10-traits.md` §2 and §9. §2's coherence — at most one instance per `(trait, head)` — is what
  makes "derived, but overridable" **not** an available option, and §9's `overridable default method bodies` row already
  refuses that shape for the reason that applies here verbatim: two impls with the same declaration produce different
  dictionaries and "nothing at the use site says which happened." §7 as 142db leaves it, which is where the three
  equalities a reader must tell apart are named.
- `docs/plan/prompts/163-builtin-collapse.md`, the lowering `x == y` ⇝ `Eq::equal(x, y)`. After 143 there is one
  spelling, so a declared type without an instance is a type `==` does not work on.
- The corpus prompt 165 produced. **Count before deciding**: how many declared types the rewritten standard library has,
  how many of them a program actually compares, how deep their structure is, and how many would need a hand-written
  `equal` that is a fold over fields and nothing else. The count is this prompt's evidence and its Target records it
  whichever way the decision goes.
- `~/Code/Idris2/libs/base/Decidable/Equality.idr`. Idris 2 writes its twelve `DecEq` instances **by hand** and keeps
  derivation out of the core entirely — `%runElab derive` is the external `idris2-elab-util` package. Note line 138's
  `[FromEq] Eq a => DecEq a`: a *named* instance, which is how Idris avoids incoherence while having two routes. Musa
  refuses named instances (§2), so that escape is closed here and the choice is genuinely three-way rather than
  four-way.
- The Haskell 2010 Report §11.1 on derived instances, for the structural specification a derived `Eq` would follow if
  this prompt derives one: constructor tags compared first, then fields left to right, and a type whose fields are not
  all equatable does not derive.

## Design

**Count first, decide second.** The three outcomes are all admissible, and which one the corpus supports is not knowable
before this prompt runs. Writing the prompt to reach a predetermined answer would make the count decorative.

- **Derived.** The compiler generates `equal` structurally at the declaration: constructor tag, then fields in order,
  bottoming out at the compiler-owned five. A hand-written `impl Eq<T>` for a type that derives is **refused**, not
  preferred — §2 admits one instance and §9 refuses the shape where the use site cannot tell which it got.
- **Written.** An author writes `impl Eq<T>` and the compiler generates nothing, exactly as Idris 2 does. Cheap to
  build, and honest when the count is small.
- **Neither.** Declared types compare by `match`, `==` stays the base-type spelling, and the prelude's ι argument is
  taken at its word for every declared type rather than only for `Bool` and `Nat`.

**What the count has to distinguish.** Many types, or deeply nested ones, make hand-writing a fold over fields the kind
of boilerplate that is the same abstraction one level down. Few and shallow makes derivation machinery that earns
nothing. A corpus where nothing compares a declared value at all makes both wrong, and *neither* is the answer.

**Three questions the derived route must answer before it is chosen**, because each is a place a structural derivation
quietly stops being structural:

- **A field of function type.** Not equatable, so the type does not derive, and the refusal names the field. Deriving
  what cannot be decided is how a total language acquires a partial operation.
- **A field of an indexed type.** The index is erased at quotation (§1.5) and cannot contribute to a comparison of
  values. `Row(12)` and `Row(24)` compare as `Row`s, and that has to be stated where a reader meets it rather than
  discovered.
- **A field of an unregistered base type.** The five are the compiler's; a base type the host registered without an
  equality has none, and the derivation stops there rather than inventing one.

**Whichever route is chosen, the prelude's five do not move.** They are what a literal pattern needs, they are reached
by source that imports nothing, and this prompt neither extends nor collapses them.

## Target

- **The count, recorded** in `docs/notes/research/language-design-closure/`, and linked from the code map: declared
  types in the rewritten library, those a program compares, their depth, and the hand-written `equal` bodies avoided or
  not avoided. Written whichever way the decision goes, because a decision without its evidence is the drift this repo
  forbids.
- `docs/rules/language/10-traits.md`: the decision stated — which route a declared type takes, that it is fixed at the
  declaration, and that the other route is refused for that type with the diagnostic that says so.
- `crates/musa-compiler/`: the implementation, if the decision is *derived* or *written*. If the decision is *neither*,
  the deliverable is the recorded count, the §10 sentence, and no code.
- The three refusals above, each with a fixture, if the decision is *derived*.
- `stdlib/`: the instances or the derivations the corpus actually needs, and no others.

## Check

```sh
cargo nextest run -p musa-compiler
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
```

And the decision's own check, which is that the corpus exercises it: every declared type the rewritten library compares
is compared through the route this prompt chose, and no type has two.

## Stop

- **No `Ord`, no `Hash`, no `Show`, and no general `deriving` mechanism.** If the decision is *derived*, it derives `Eq`
  and nothing else; a second derivable trait needs its own count and its own caller.
- **No named instances**, no instance priority, no overlap, and no overridable default. §2 admits one instance per
  `(trait, head)` and §9 has already refused the alternatives.
- No change to the compiler-owned five, and no new literal pattern.
- No propositional equality, no `DecEq`, no proof that `equal` agrees with conversion. §1.4 stays deleted and this
  prompt gives no evidence to re-open it; the agreement is a law suite under `05-verification.md` §4, as `10-traits.md`
  §7 already says.
- No change to how `==` lowers. That is 143's, and it is done before this prompt starts.
- Do not derive for base types. The host registers what a base type can do, and equality is one of the things it
  registers.

Commit as `Decide how a declared type gets an equality`.
