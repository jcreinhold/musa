---
id: 141i
slug: constrained-definitions
status: done
depends_on: [135, 137, 141g]
phase: 3
---

# Give a Free Definition Its Dictionary

## Task

`crates/musa-compiler/src/lower/items.rs` refuses a `where` clause on a free `fn`, on a `record`, and on an `enum`,
saying "a constraint here has no core spelling yet". `01-surface.md` §1's grammar writes `where-clause?` on all three,
§1.2 says what one on a `record` means, §1.4 writes `fn same<A>(x: A, y: A) -> Bool where Eq<A>` as an ordinary program
and elaborates it to "an extra parameter holding the dictionary", and `10-traits.md` §4 step 1 answers a constraint from
"a dictionary bound by an enclosing `where` clause". Give the core the binder those four sentences already describe, and
delete the refusal.

This is drift, not a feature. `musa-calculus`'s own `Constraint` doc already lists the three positions its arguments may
be read under — "a trait's own context under its parameters, an instance's under the instance parameters, **a function's
`where` under its type parameters**" — and the third has no door.

## Read

- [`../../rules/language/01-surface.md`](../../rules/language/01-surface.md) §1's grammar (`function`, `record`, `enum`,
  `trait`, `trait-item`, `impl` — six productions, six `where-clause?`), §1.2's bullet "**Parameters and `where` are
  allowed and are ordinary**: `record Cell<A> where Eq<A> { at: Nat; value: A; }` requires the constraint at every
  construction and carries it to every reader", §1.4's `same` and the paragraph "It adds no term to the calculus", and
  §1.5's "The caller writes the constraint or the qualified path". Those are the four places the language already admits
  this, and the last is why it cannot be worked around: a generic parameter acquires no method without one.
- [`../../rules/language/10-traits.md`](../../rules/language/10-traits.md) §4 — the three lookup steps, step 1's local
  dictionary, and postponement. §4's termination measure is checked **at an instance** and is not this prompt's
  business: a free definition's `where` binds a dictionary, it does not declare one, so there is no descent to measure.
- `crates/musa-calculus/src/dictionary.rs` — `requirements`, and `method_at`'s `Kind::Derived` arm. `requirements` is
  this prompt's rule one level down: it walks a derived method's `where`, elaborates each constraint, **discharges** the
  key into the scope as well as assuming a binder for it, and hands back the scope the body is written in. `method_at`'s
  derived arm is the use-site half: the trait's arguments and the method's own parameters become metavariables, and each
  of the method's own constraints goes through `resolve`. Both halves already exist; what is missing is the door a
  definition reaches them through.
- `crates/musa-calculus/src/term.rs` — `Shape::Pi`'s `plicity` field and its doc: "**no core rule reads this** — it is
  here because a type reached by projection or by substitution has been through the semantic domain, and elaboration
  still has to be able to ask whether the binder it found was implicit". That sentence is the design.
- `crates/musa-calculus/src/elab/mod.rs` — `inserted`, `lambda`, `abstracted`, and `function_type`. `inserted` is the
  loop that fills an implicit binder with a metavariable; it is where a constraint binder is filled by `10-traits.md` §4
  instead.
- `crates/musa-calculus/src/family/mod.rs` — `Binder`, `Group::params`, `Constant::ty`, and the `split_off(params)`
  arithmetic that a family's parameters are counted by. This is where `enum` costs more than `fn` and `record`, and the
  cost is named in the Design below rather than discovered.
- `crates/musa-compiler/src/lower/items.rs` — `unconstrained`, whose doc argues for the refusal on exactly the ground
  this prompt removes, and `function`, `structural`, `enumeration`, and `instance`. Also the module doc's rule that a
  type declaration's parameter is explicit and a function's is implicit, which fixes where a constraint binder goes.
- `crates/musa-compiler/src/lower/laws.rs` — `a_constraint_on_a_free_definition_is_refused_where_it_is_written`, the law
  that changes sides.
- `docs/plan/code-map/spec-to-implementation-map.md`, the `musa-compiler` lowering row. Four surface forms are recorded
  as refused at the node; three name the prompt that owns them (141ga, 141h, 142) and the `where` clause names none.
  This prompt is that name.

## Design

**It is admissible, and the specification is not ambiguous about it.** §1.4 writes the program, gives its elaboration —
"a `where` constraint to an extra parameter holding the dictionary", and "`x == y` inside `same` is `d.equal(x, y)` for
the `d` the caller supplied" — and §1.2 states the record case in a sentence of its own. `10-traits.md` §4's first
lookup step exists *for* this binder: a trait's context and an instance's context are both reached by projection out of
a dictionary the declaration already has, so a `where` on a free definition is the only thing step 1 can be about that
the other two positions do not already cover. The code refusing it is the gap.

**The constraint rides on the binder, because a definition is a value.** A derived method can keep its constraints in a
side record — `Derived::context` — because a method is *always* reached by name through `Classes`: §1.5 makes method
resolution exact-receiver and one-step, so there is no anonymous method value for a table to fail on. A definition has
no such restriction. `same` may be passed to a higher-order function, stored in a record field, or returned; at that use
site there is no name to look up and only the type is in hand. So the type has to say it.

It also has to say it *with the trait and its arguments*, and the domain cannot be inverted to recover them:
`Trait::dictionary` is the closed term `λp⃗. { … }`, so `Eq A` β-reduces to a record type and the head is gone by the
time anything looks. `dictionary::resolve` is keyed on a `Constraint`, and the binder is the only place that survives.

**The shape: a third plicity that carries its constraint.**

```rust
pub enum Plicity {
    Explicit,
    Implicit,
    /// Resolved at every use by `10-traits.md` §4, and never written.
    Constraint(Arc<Constraint>),
}
```

`Constraint` becomes `pub` with its fields still `pub(crate)` — an opaque public type, which is what it already is
everywhere but the `enum` above. `Plicity` loses `Copy` and keeps `Clone`; it has no user outside `musa-calculus`, so
the ripple is internal and mechanical.

**This adds no term to the calculus, which is §1.4's claim and has to stay true.** §1's "exactly one Π" is untouched:
there is one function type, and a constraint is a marking on its binder that **no core rule reads** — the same status
the `plicity` field already has and for the same stated reason. Conversion, evaluation, quotation, and the recheck are
unchanged; `well_typed` sees the Π it saw before. Only elaboration asks the question, at the two places it already asks
about plicity.

**The two elaboration rules, and they are the two halves that exist.**

- *Filling.* `inserted` gains a `Plicity::Constraint(c)` arm: instantiate `c`'s arguments in the current environment —
  `dictionary::instantiated`, which `method_at` already calls for exactly this — and apply `dictionary::resolve`. Not
  `fresh_meta`: a dictionary metavariable nothing solves is the unsolved-hole error, which is the wrong report for a
  constraint that was answerable. `resolve` already returns a postponed hole when the head is still a metavariable, so
  `same(x, y)` — where `A` is not yet known at the moment the constraint binder is reached — is §4's postponement and
  not a failure. `discharge` retries it.
- *Abstracting.* Checking a term against a constraint Π wraps it in the λ, exactly as §2 already wraps a term checked
  against an implicit Π, **and discharges the key into the scope** so the body's `x == y` finds the binder standing
  right there by §4 step 1 rather than a global instance. That second half is not optional and it is not new:
  `requirements` does both for a derived method and its doc says why. `Scope::discharging` stays `pub(crate)` — the
  discharge happens inside the core, at the moment the core writes the λ, so nothing outside needs the door.

**A constraint is written after the type parameters and before the value parameters.** `fn same<A>(…) where Eq<A>`
becomes `{A : Type} → [Eq A] → (x : A) → (y : A) → Bool`. That is `method_at`'s order — trait arguments, dictionary, own
parameters, own context — and it is the order that lets the constraint's arguments mention the parameters, which is what
`Constraint`'s doc means by "read under whatever binders the constraint was written under". The λ side writes nothing:
`items.rs`'s `function` already leaves implicit binders to the core on the ground that "§2 wraps a term checked against
an implicit Π in the λ it needs", and a constraint binder is the same sentence.

**A `record` and an `enum` are the same rule at a type former, and that is what §1.2's sentence means.** A declaration's
type parameter is explicit (`List<Nat>` writes its argument), so `record Cell<A> where Eq<A>` becomes
`(A : Type) → [Eq A] → Type`, and every place that writes the type `Cell<A>` resolves `Eq<A>` to fill the second
argument. That is "requires the constraint at every construction and carries it to every reader" exactly: a reader
cannot so much as name the type without the constraint being answerable where it stands. Nothing is stored. §1.2's "a
record is its fields" survives, because the dictionary is an argument the body ignores — `Cell Nat d` unfolds to
`{ at : Nat; value : Nat }` and is still the same type as a hand-written record with those fields. Say that as a law.

For `enum` the same binder has to live on a *family*, so `RawData` gains a `context: Vec<RawConstraint>` read after its
`params` — mirroring `RawTrait` and `RawImpl`, which is where a reader will look for it — and `family::Binder` gains a
`plicity`. The constraint binders **are** parameters, appended after the written ones, so every `Group::params()` count
and every `split_off(params)` in the file keeps counting the same thing and none of that arithmetic moves. This is the
expensive third of the prompt, and it is expensive in test surface rather than in new design: a constraint parameter is
a parameter, and the core already knows what a parameter is.

**`Storable` is unaffected, and that is worth a law rather than an assumption.** The generated instances ask two
questions of each *stored field*, and a constraint binder is a parameter and not a field. A dictionary contains Π, so a
family that stored one would stop being storable; a family that is *indexed by* one does not. Prove it on a declaration
that has both.

**A `where` on an `impl` method is misplaced, not unsupported, and today it is silently dropped.** The parser attaches a
`WhereClause` to every `FnDecl`, including the ones inside an `impl`, and `instance` builds a `RawDefinition` from the
name and the λ without ever looking at it. An impl method takes its type from the dictionary field it fills, so there is
nowhere for a constraint of its own to go and there never will be — that is the same reasoning
`Refusal::ConstrainedField` already gives for a *required* trait method. Refuse it under `Code::Misplaced` ("a statement
that cannot be where it is"), which is a permanent answer, rather than under `Code::UnsupportedLanguageStage`, which
promises a later prompt.

**Rejected: a name-keyed table of constrained definitions.** The obvious cheap route is `Derived`'s — record the
constraints beside the definition and have the use site consult them. `Cx::define` takes a type and a value and no name,
top-level definitions are not wired into a context until prompt 142, and a definition passed as an argument arrives
somewhere with no name at all. Building the table would mean inventing 142's name binding early *and* accepting that §4
stops working for a definition the moment it stops being called by its own name.

**Rejected: `data` gains a `where` too.** The grammar does not give it one — `where_clause` is called from
`record_decl`, `enum_decl`, `trait_decl`, `impl_decl`, and `fn_signature`, and `data_decl` is not among them — so
`nominal` has nothing to read and correctly reads nothing. Adding the clause to `data` is a surface change, and surface
changes are 142's.

## Target

- `crates/musa-calculus/src/term.rs`: `Plicity::Constraint(Arc<Constraint>)`, doc-commented with the sentence it
  implements and with the reason the constraint rides on the binder rather than in a table. `Copy` goes; `Clone` stays.
- `crates/musa-calculus/src/class.rs`: `Constraint` public, fields still `pub(crate)`.
- `crates/musa-calculus/src/raw.rs`: `Raw::constrained_pi(origin, constraint: RawConstraint, codomain: Self)`, and
  `RawData::context: Vec<RawConstraint>`. No `constrained_lam` — the core writes that λ.
- `crates/musa-calculus/src/elab/mod.rs`: the filling arm in `inserted` and the abstracting arm beside `lambda`, the
  second discharging as well as assuming. `function_type` takes the constraint's dictionary type as the domain and
  levels it the way it levels any other.
- `crates/musa-calculus/src/family/mod.rs`: `Binder::plicity`, and `declare` appending one constraint binder per
  `RawData::context` entry after the written parameters.
- `crates/musa-compiler/src/lower/items.rs`: `unconstrained` deleted; `function`, `structural`, and `enumeration`
  reading their `where` through the existing `written_constraints`; `instance` refusing a constraint on an impl method
  under `Code::Misplaced`.
- Laws in `crates/musa-calculus/tests/suite/`: `same` checks and its body's `==` resolves to the bound dictionary rather
  than a global instance; a call at a known head resolves; a call whose head is a metavariable postpones and is answered
  when the head is solved; a constraint no instance can answer is refused at the use site, naming the type and the
  trait; an inner `where` shadows an outer one (§4 step 1's "innermost first"); a constrained record type is convertible
  with the equal hand-written one; a constrained family's constructor and recursor carry the parameter and `Storable` is
  unaffected; the recheck accepts every term the new rules build.
- Laws in `crates/musa-compiler/src/lower/laws.rs`: `a_constraint_on_a_free_definition_is_refused_where_it_is_written`
  becomes the admission it was blocking — the same source, lowered and then declared by the core — plus a `record` and
  an `enum` each carrying one, plus the impl-method refusal.
- `docs/plan/code-map/spec-to-implementation-map.md`: the lowering row loses the `where` clause from its list of four
  refused forms, and the `musa-calculus` traits row records the constraint binder.
- `AGENTS.md`'s prompt count.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus -p musa-compiler
cargo clippy --all-targets -p musa-calculus -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Give a free definition its dictionary`.

## Stop

- No surface change and no `.musa` change. The grammar already parses every clause this prompt admits, `stdlib/` and
  `examples/` write none of them today, and the file that will is prompt 165's rewrite. `editors/tree-sitter-musa` is
  untouched for the same reason: the drift law binds it to the lexer, and the lexer does not move.
- No wiring. `mod lower` stays behind its dead-code expectation and prompt 142 is still the first caller, exactly as
  141g, 141ga, and 141h leave it. The laws beside these rules are their caller.
- No termination measure on a definition's `where`. §4 measures an *instance*, because an instance's context is what
  step 3 recurses into. A definition's context is discharged by its caller and descends into nothing.
- No new lookup rule, no backtracking, and no fourth step. `resolve` is used as it stands; a constraint binder is a new
  place to *call* §4 from, not a new §4.
- No `where` on `data`, and no change to `nominal`. That is a grammar change and grammar changes are 142's.
- No `Scope::discharging` made public, and no dictionary resolution moved into `musa-compiler`. If the compiler ends up
  needing either, the binder is in the wrong place and this prompt is wrong.
- If the family half proves to be its own feature rather than the last third of this one, split it out as `141ia` under
  execution rule 5 — repair the prompts first, `audit`, commit the repair, then implement. Do not land `fn` and `record`
  while leaving `enum` refused without that repair: §1.2 and §1.3 are siblings, and a language where one of them takes a
  constraint and the other does not is worse than one where neither does.
