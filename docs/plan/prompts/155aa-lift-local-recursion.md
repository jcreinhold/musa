---
id: 155aa
slug: lift-local-recursion
status: pending
depends_on: [155]
phase: 3
---

# Lift a Local `rec` to a Definition

## Task

A `rec f : A = e` in **term position** closes over the binders around it. `musa-compiler/src/prelude.rs`'s
`List.fold_from_start` is one: `rec walk : List A → B → B = λxs. λbuilt. match xs { … }`, written inside a term whose
enclosing binders supply `A`, `B`, `zero`, and `step`. Nothing in the core can hold such a thing as a *name*: a
definition is a global name (§1, and `kernel/program.rs`'s module doc argues at length for why), so a local recursion
has no name to reduce by, and §1.3 refuses the fixed point that would let one stand without one.

Lift it. A term-position `rec` becomes an **auxiliary top-level definition** abstracted over the binders it captures,
and the expression it stood in becomes that definition applied to them. Peyton Jones ch. 13 and ch. 14 — ch. 14 is the
one that matters here, because §14.0 is explicit that its subject is obtaining "a set of mutually recursive
supercombinator definitions **without using Y**", and Y is exactly what §1.3 refuses.

**This prompt relocates; it does not change how a recursion is compiled.** `elaboration/rec.rs`'s `#ih` rewrite is
untouched and still runs, now at a definition's top and nowhere else. That is the whole point: prompt 155a replaces that
compilation with a tree body and a descent check, and it can only do so if every recursion in the language is at a place
a definition body can be.

**Why the stack needed this, measured while preparing 155a.** 155a retires the `#ih` rewrite in favour of a definition
that names itself and a check over its tree. A definition that names itself is a `Definition::Compiled` in the globals
table. A term-position `rec` cannot be one — its body mentions `A`, `B`, `zero`, and `step`, which are binders and not
names — so with 155a's rewrite retired and nothing put in its place, `prelude.rs`'s two folds and the `rec` laws in
`collection_laws.rs`, `nesting_laws.rs`, and `termination_laws.rs` have no compilation at all. There were two ways out
and only one of them is a language musa has: a term-level fixed point, which §1.3 refuses, or this.

## Read

- `~/Code/papers/logic-and-computation/software-engineering/implementation-of-functional-programming-languages/13-supercombinators-and-lambda-lifting.md`
  and `14-recursive-supercombinators.md`. Ch. 13 is the transformation; ch. 14 §14.0(iii) and §14.1 are the recursive
  case and the reason it is a separate chapter. Take the *analysis* and not the machinery: musa has no graph reduction,
  so the sharing and cyclic-graph half of ch. 14 describes a machine this core has no use for, and what transfers is
  which binders a lifted definition must abstract over and why the recursive occurrence may stay a name.
- `crates/musa-compiler/src/prelude.rs`, `list_from_start` and `list_from_end` — the two term-position `rec`s in the
  repository, and the measurement above. `list_from_start`'s captures include `zero` and `step`, which is why "lift it
  and abstract nothing" is not an option.
- `crates/musa-calculus/src/elaboration/rec.rs` — the `#ih` rewrite, unchanged by this prompt, and its module doc's
  argument for rewriting *raw* syntax. Read it for the technique, which this prompt reuses if it needs a rewrite.
- `crates/musa-calculus/src/kernel/program.rs` — a definition is a global name, and the module doc's paragraph on why a
  `let` chain and a de Bruijn binder both fail to close the circle. That paragraph is this prompt's premise.
- `crates/musa-calculus/src/kernel/meta.rs` and `Cx::types` — a metavariable already abstracts over its whole context
  and instantiates itself by applying the binders back. A lifted definition is the same abstraction with a body instead
  of an unknown, and the two should share the walk rather than grow a second copy of it.
- `crates/musa-calculus/src/elaboration/declare_program.rs`, `elaborate` — where a `Defined` is built, and where a
  lifted one has to arrive.
- `crates/musa-calculus/src/elaboration/elab/check.rs`, `RawShape::Rec` — the one arm that reaches
  [`rec::define`](../../../crates/musa-calculus/src/elaboration/rec.rs), and the arm this prompt splits in two.

## Design

**A lifted definition abstracts the whole context, not the free variables of the body.** The narrower rule is the
textbook one and it costs a free-variable analysis over raw syntax that would have to agree with the elaborator's own
scoping — a second resolver, which `Globals::definition`'s doc refuses for the same reason. Abstracting the context is
what a metavariable already does, it is decided by the scope rather than by a walk, and the extra arguments are
variables applied at the one use site the lift creates. **Ch. 15's fully lazy lifting is not adopted**: it exists to
avoid recomputing a free expression under a graph reducer, and this core evaluates by need in a semantic domain with no
graph to share.

**Nothing needs shifting, and that is why this is small.** The body is elaborated in the scope the `rec` was written in,
so it is already a term under exactly those binders; `λ` over them, outermost first, *is* the definition's value. The
type is the same telescope of Πs over the written type. De Bruijn indices are relative, so no term is rewritten at a new
depth — which is the property that makes this expressible in a crate with no substitution on terms (§3).

**The recursive occurrence stays inside the lifted body and is still the `#ih` rewrite's problem.** After the lift the
body is the body of a top-level `rec`, which is the shape `rec::define` already handles. This prompt therefore changes
no refusal: every call `rec.rs` admits today it still admits, and every call it refuses it still refuses, at the same
diagnostic. `termination_laws.rs` is the control — it must pass unchanged, and if a law there has to be edited the lift
changed meaning and the prompt is wrong.

**The name is unwritable and the definition is private.** `<enclosing>#<name>`, with `#` for `rec.rs`'s reason: no
source identifier holds one, so no program can name a lifted definition, shadow one, or collide with one. Its
[`Visibility`] is private to the module its parent was written in.

**Where a lifted definition is installed is decided by the Check, not by taste.** Collecting them on the elaborator and
installing them in `declare_program` before the parent's body is evaluated and re-checked is the smaller change. If a
conversion *inside* the parent's own elaboration has to unfold one, that is not enough and it must be installed at the
moment it is lifted; the suite says which, and the answer belongs in a doc comment either way.

**No mutual recursion between lifted definitions.** Ch. 14's subject is a *set* of them; musa's `rec` binds one name,
§2.4 admits one recursive definition at a time, and a group of more than one is the cycle `declare_program` already
refuses. One lift, one definition.

## Target

- `crates/musa-calculus/src/elaboration/rec.rs`: `lift` — a term-position `rec` abstracted over its context, as an
  auxiliary `Defined`, with the application that replaces it. `define` stays and is now reached only at a definition's
  top.
- `crates/musa-calculus/src/elaboration/elab/check.rs`: the `RawShape::Rec` arm chooses between the two by where it
  stands.
- `crates/musa-calculus/src/elaboration/elab/mod.rs`: the elaborator collects what it lifted.
- `crates/musa-calculus/src/elaboration/declare_program.rs`: a lifted definition joins the `Program`, in scope before
  its parent's body is evaluated and re-checked.
- `crates/musa-calculus/src/kernel/program.rs`: whatever `Defined` needs so that a lifted member is distinguishable from
  a written one — a reader of a `Program` must not have to parse a name to tell.
- `crates/musa-calculus/tests/suite/`: a law that a term-position `rec` capturing enclosing binders elaborates and
  evaluates to what it did before, a law that the lifted definition is in the program and is private, a law that no
  source program can name one, and `termination_laws.rs` passing **unedited**.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
git diff --stat crates/musa-calculus/tests/suite/termination_laws.rs   # must be empty
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

**The re-checker's obligation for this prompt.** A lifted definition is an ordinary definition and its use is an
ordinary application, so the existing arms cover both — but the suite must contain a re-checked fixture whose program
holds a definition *nothing in the source wrote*, because that is a member the re-checker has never walked before.

Commit as `Lift a local rec to a definition`.

## Stop

- No change to how a recursion is checked or compiled. `#ih` is untouched; 155a is where it goes.
- No fully lazy lifting, no sharing analysis, no supercombinator machinery beyond the abstraction itself.
- No mutual recursion, and no `letrec`.
- No change to what an author writes. `rec` reads and elaborates as it does today; only where its body ends up changes.
- No change to `docs/rules/`.
