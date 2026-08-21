# 51. The terseness audit: what the correction got right, and three missteps

A decision record. Governs nothing. It audits note 50's correction against the goal note 50 was serving, names three
decisions that work against that goal, and states the replacement. Note 52 is its other half: what the musical domain
actually asks the language to say.

## 1. The test note 50 has to pass, and the one it ran

The directive note 50 records has four clauses:

> the smallest practical language that gives Musa **excellent metaprogramming**, **ordinary ergonomic programming**,
> **useful lightweight dependency**, and **exact resource checking**

Note 50's audit rule — *every mechanism names the committed Musa program that requires it* — is a good rule, and it
tested exactly one word: **smallest**. It has no test for *practical*, none for *ergonomic*, and none for *useful*. A
rule that can only subtract will subtract until the language is minimal, and nothing in the procedure stops it at
usable.

The arithmetic shows the direction of travel. Roughly 35,000 lines have been deleted so far, and the one number that
measures the goal has not moved: `stdlib/src/adapters/staff.musa` was 2,404 lines when prompt 128's amendment was
granted on it, and it is **2,515 lines today**. The constitution names that number as the falsifier in its own amendment
record:

> It is answerable to a measurement: the staff adapter is rewritten on the surviving language, and if the number does
> not move, the whole pass was wrong.

Phase 5 has not run, so this is not yet a verdict. But the correction is **unvindicated**, and three specific decisions
are why it will stay that way if phase 5 runs as written.

## 2. The methodological flaw

*Every mechanism names the committed Musa program that requires it.* Against speculative generality — "standard in
Lean", "useful later" — this is exactly right and should stay.

Applied to a standard library **we wrote ourselves, in a language that did not have the feature**, it proves nothing.
Absence of use is not evidence of absence of demand; it is evidence of the workaround. The audit found "0 user-declared
indexed or parameterized `data`" and concluded no index mechanism is needed. The question it did not ask is the one that
has a measurable answer:

> **What is the corpus doing instead?**

§3 answers it: 17 compiler builtins, a `fallback` parameter in a public signature, and a domain restricted to one
modulus. The rule needs one clause, and stays falsifiable with it:

> A mechanism is admitted when a committed program requires it, **or** when the corpus contains a named, counted
> workaround that the mechanism deletes. The workaround must be exhibited and its cost measured — a claim that something
> "would be nicer" is not a workaround.

## 3. Misstep one: indices were deleted, and the corpus pays for them in the builtin registry

### The count

`stdlib/src/post_tonal/` needs pitch classes and twelve-tone rows. With no way to index a type by a number, the modulus
is baked into seventeen compiler-owned builtins:

```
pc12_of  pc12_number  pc12_transposed  pc12_inverted  pc12_forget  pc12_spelled
row12_of  row12_pcs  row12_head  row12_transposed  row12_inverted  row12_retrograde
row12_matrix  row12_forms  row12_symmetries  row12_repeats  row12_missing
```

That is **17 of the registry's 139 entries — 12% — spent on one value of one index**, and prompt 163's builtin collapse
was superseded before it could notice. The domain demands other values: `068-equal-divisions-of-the-octave.md` is
*about* dividing the octave into n parts, `106-collections.md` generates collections from interval patterns that must
sum to the octave, and quarter-tone and 19-EDO repertoire needs `Row(24)` and `Row(19)`. Under the current design each
new modulus is another seventeen builtins. Under an index it is none: `Pc(n)` and `Row(n)`, one set of operations.

### The impossible case in a public signature

`stdlib/src/transformational.musa`:

```musa
fn major_triad_on(root: NoteName, fallback: Triad) -> Triad {
    chord_triad(chord_on(chord c major, root)).fold_from_end(
        fallback,
        fn (refined, impossible) { itself(refined) },
    )
}
```

A major triad is always a triad. `fallback` exists only because the type cannot say so — the file's own comment says
"returned only if a major triad were not a triad" — and because it is in a public signature, **every caller invents a
dummy triad it will never see**. A five-line body, a spare top-level `itself` to serve as the fold's present case, and a
parameter at every call site, in place of:

```musa
fn major_triad_on(root: NoteName) -> Triad { triad(chord_on(chord c major, root)) }
```

This is PoSD ch. 8 inverted. "It is more important for a module to have a simple interface than a simple implementation
… most modules have more users than developers, so it is better for the developers to suffer than the users." Deleting
the index made the *checker* simpler and pushed the cost up into every signature that needed a refinement — the exact
trade ch. 8 says to refuse.

### The third case, which nobody has written because it cannot be written

A bar's contents sum to its meter. It is the most common error in written music, it is a linear equation over exact
rationals, and Musa checks it at runtime.

## 4. What to admit instead: a stratified index domain, not inductive families

The mechanism note 50 deleted was **general indexed inductive families with index unification**, and deleting it was
right. No committed program unifies an index; the machinery was 1,820 lines; and index unification *inside the
conversion checker* is precisely where the complexity lived.

The mechanism the corpus demands is a different and much smaller one:

> A type may be indexed by a value of a **decidable arithmetic domain**, and index equality is decided by **arithmetic,
> not by unification**.

That is Xi and Pfenning's **Dependent ML**, and it is what "useful lightweight dependency" names in the literature:

- indices are drawn from a **separate index language** with its own decision procedure — here ℕ and exact ℚ under linear
  arithmetic, plus finite literal enums, all decidable;
- indices are **erased**: no proof terms, no dependent pattern matching on an index, nothing to construct or eliminate;
- conversion on an index calls the **solver**, never the unifier;
- elaboration stays bidirectional and the term core is untouched.

**This is the decomplecting move, and deletion was not.** The design note 50 inherited braided two questions into one
procedure: *are these terms equal* and *are these indices equal* were both decided by `unify.rs`. Deleting one of the
two questions is subtraction, not simplification — Hickey's point is that the braid is the defect, not the strand count.
Stratifying gives two independent deciders, neither of which knows the other exists, and the term core keeps the
spine-free assignment rule it has now. The checker does not grow; a solver module appears beside it.

### The language already has this, and gives it only to itself

`02-core-calculus.md` §1.3, in the sentence that refuses it:

> **No indexed families.** … the one indexed-looking type in the tooling, `Syntax<Cat>`, is a base type with a literal
> index.

`Syntax<Cat>` and `EventTrack<time>` are compiler-owned base types indexed by literals, and the compiler's own macro
system is built on the first. AGENTS.md's standing rule:

> **No sublanguage by subtraction.** Any language we hand a user … is built by *enriching* a core, never by removing
> modules … every convenience it drops is paid by every author who writes in it rather than once by us (ch. 8).

The compiler wrote itself the feature and handed users the language minus it. That is the rule's own definition of the
mistake.

### What it buys, program by program

| Program | Today | With a stratified index |
| --- | --- | --- |
| pitch classes and rows | 17 builtins at one modulus | `Pc(n)`, `Row(n)`, one set of operations |
| `major_triad_on` | `fallback` in a public signature | no parameter, no fold, no `itself` |
| a bar in 7/8 | runtime check | `Bar(m)`: the contents sum to `m`, statically |
| SATB voice leading | `List<Voice>` plus a length check | `Voicing(4)`, and the rules of `022-chords-in-satb-style.md` typed |
| interval-class vector | fixed at six entries | `Icv(n)`, derived from the collection |
| Messiaen's modes of limited transposition | a prose comment | the orbit's size, checked |

Every row names a committed program or a chapter of the corpus the language is built for. None of them needs an identity
type, a universe, a measure, or a dependent motive — which is why this is an amendment to note 50 and not a reversal of
it.

## 5. Misstep two: higher-order APIs are hard to write, and the cheap fix is already in the file

Checking `λx. e` against `(x : A) → B` pushes a binder whose type may still be a hole
(`crates/musa-calculus/src/elab/check.rs`). With postponement deleted, nothing rescues a body that needs to know that
type, so **argument order became semantically significant for un-annotated lambdas**. The standard library is shaped
around the constraint: `xs.fold_from_start(seed, combine)` puts the seed first so the accumulator type is solved before
the lambda is reached. That ordering is inference weakness showing up in the API, and per ch. 8 it is paid by every
author who writes a higher-order signature.

Idris2 solves this without a constraint queue, and Musa already has both of the predicates it uses.
`TTImp/Elab/App.idr`:

> In theory we can check the arguments in any order. But it turns out that it's sometimes better to do the rightmost
> arguments first … So we do that if the target type is unknown, or if we see that the raw term is otherwise worth
> delaying.

`checkRtoL` makes a metavariable for the argument whose expected type is still a hole, checks *the rest of the spine* —
which solves the hole — then comes back and checks the argument against the solved type. Falls back to left-to-right if
that fails. `needsDelayExpr` returns true for `ILam`, `ICase`, `ILocal`, `IUpdate`, `IAlternative` — which is
`Raw::checks_only`'s list (`Lam | Match | Rec | Record`) almost exactly.

Musa has `mentions_unsolved(&domain)` and `checks_only()` in `elab/spine.rs` and uses them to choose *infer versus
check*. Using them to **reorder** is the whole change. It is bounded to one application spine, deterministic, and
terminating — each argument is checked exactly once — so none of postponement's cost returns: no constraint queue, no
wakeup discipline, no fixpoint, no order-independence obligation. It is the middle ground the deletion overshot.

## 6. Misstep three: the domain's central abstraction is a partial application, and §1.3 forbids it

`02-core-calculus.md` §1.3:

> **A call must be complete**: an application supplies every declared parameter, and an under-applied call is a type
> error rather than a value. Partial application would make an ordinary argument list a place where a function silently
> becomes a function-valued result, which is exactly the value that may not be stored (§1.2).

The library author hit this and wrote it down, in `stdlib/src/post_tonal/pcset.musa`:

> `T_n`: transposition by n semitones … **It is not a function of the index** — a call supplies every parameter, and
> `transposed_by(3)` supplies one of two.

T₃ is not a convenience. In Lewin's transformational theory it is *the object of study*: transformations are composed,
inverted, and applied, and the group they form is the analysis.
`101-pitch-class-sets-normal-order-and-transformations.md` and `072-neo-riemannian-triadic-progressions.md` are both
written in terms of naming a transformation and then doing algebra with it. A language for this domain that cannot name
T₃ is not expressive enough for its first chapter.

The stated reason does not survive reading. Storability is a **structural fact the checker computes** (§1.2) — it does
not need application completeness to enforce it, and a function value is perfectly fine everywhere except a machine
port, which §1.2 already checks directly. Banning partial application to protect §1.2 braids two unrelated concerns:
*where function values may be stored* and *how many arguments a call site writes*.

The real content of the rule is about **silence** — `f(x)` quietly yielding a function when `f(x, y)` was meant — and
that is fixable without the ban. Require the section to be *written*:

```musa
transposed_by(3, _)        % T₃, explicit
retrograde_of(_)           % R
transposed_by(3, _) . invert_about(0, _)
```

Every slot is still named at every call, so the diagnostic argument is fully preserved; an under-applied call with no
`_` stays a type error. What changes is that the domain's operations become values.

## 7. What the correction got right, and stays deleted

None of this reopens the rest of note 50's audit, and the evidence for each of these is unchanged:

- `Id`, `refl`, `J`, K — no program proves an equality theorem. Stays deleted.
- universe polymorphism and level metavariables — two fixed levels are enough. Stays deleted.
- **postponed constraints with a retry loop** — replaced by §5's bounded reordering, which is strictly weaker and
  strictly cheaper. Stays deleted.
- general termination measures — structural descent covers all 13 recursive functions. Stays deleted.
- constraint-based traits with super-constraints and recursive resolution — the corpus is flat. Stays deleted.
- `Storable` as a trait — a structural predicate. Stays deleted; §4 does not reintroduce it, and an index is not a
  constraint.
- `recheck.rs`, the old `core.rs` checker — gone, and the 32,482 lines the compiler shed are the correction's real win.

The pattern-fragment machinery stays deleted too, and §4 does not bring it back: indices are compared by the solver, so
no index ever reaches the unifier.

## 8. The six requirements

`docs/rules/README.md` binds a change to `constitution.md` §9 — whose refusal of refinement types is currently narrowed
only to "a result type may mention an earlier explicit argument" — to six things. §4 is such a change; §§5–6 are
ordinary repairs to the candidate `language/` pages.

1. **A concrete musical or engineering reason.** §3: 17 builtins at one modulus, a `fallback` parameter in a public
   signature, and a bar whose contents cannot be summed at compile time. §6: T₃ cannot be named.
2. **Which current examples no longer work.** None. Indices are additive — every existing declaration is an index-free
   one, and no committed program spells a section today. `examples/` and `stdlib/` compile unchanged; the migration in
   §4 is a rewrite we choose, not one the change forces.
3. **The replacement rule in plain language.** A type may carry index arguments drawn from a fixed arithmetic domain (ℕ,
   exact ℚ, finite literal enums). Indices are erased before evaluation, never matched on, and two indexed types are the
   same type when the solver proves their indices equal. Nothing else about the calculus changes.
4. **The formal specification and the code map.** `02-core-calculus.md` gains the index stratum and loses §1.3's "no
   indexed families" and "a call must be complete"; §2.1 is restated as what the code does; `10-traits.md` gains the
   action laws of note 52; `03-musical-domains.md` gains the indexed domains. The code map follows the implementation.
5. **Migration of stored files and public APIs.** No stored format changes: indices are erased, so a compiled term, an
   event track, and the `.musa.events` interchange format are byte-identical. The public API change is the seventeen
   builtins collapsing to six, which is a clean break under `docs/plan/clean-break-ledger.md` and touches no file a user
   has written.
6. **This record.** Note 50 stands unedited beside it, as `18-vocabulary-amendment.md` established.

## 9. What would falsify this note

The same measurement note 50 accepted, and one more:

- `stdlib/src/adapters/staff.musa` rewritten on the language of §§4–6 is **not** dramatically shorter than 2,404 lines —
  then the diagnosis here is wrong, whatever the line counts of the checker say.
- `stdlib/src/post_tonal/` does not lose the eleven builtins §3 predicts, or loses them and gets longer.
- The index solver is not decidable in practice — a constraint the corpus generates that linear arithmetic cannot
  settle. The answer is to shrink the index domain, not to reach for unification.
