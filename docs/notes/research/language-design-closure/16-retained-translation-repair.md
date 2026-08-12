# Retained-translation repair

**Purpose:** repair the migration theorem without adding partial compiler operations to the new language.

## Decision

The refined language keeps ordinary functions as values. It accepts a partial call only when the supplied arguments form
the first part of an ordinary function's parameter list. It rejects both of these forms:

- a compiler operation with any missing argument; and
- an ordinary call that supplies a later parameter while skipping an earlier one.

For example, this ordinary prefix call remains valid:

```musa
fn transposer(interval: Interval, subject: Music) -> Music {
    transpose(interval, subject)
}

let answer: Music -> Music = transposer(runtime_interval);
```

The call to `transposer` supplies its first argument and returns an ordinary function waiting for `music`. The complete
call to `transpose` occurs only inside that function.

This current spelling is no longer valid:

```musa
choose(second: true)
```

It supplies the second parameter while skipping the first. Musa can express the same behavior with an explicit parameter
order:

```musa
fn choose_second(second: Bool, first: Nat) -> Nat {
    choose(first, second)
}

let waiting: Nat -> Nat = choose_second(true);
```

This is a source simplification, not a loss of expressive power. Parameter order now says which values a partial call
captures.

## Exact call rule

After resolving argument names, the elaborator classifies an ordinary call in one of two ways.

1. A **complete call** supplies every required parameter. The elaborator puts supplied arguments in parameter order and
   fills omitted defaults. A default may refer only to earlier parameters. Fresh `let` bindings ensure that every
   supplied argument and default is evaluated once, in parameter order.
2. A **prefix call** supplies exactly parameters 1 through `k`, for some `k < n`, and supplies no later parameter. The
   elaborator applies the nested unary function to those `k` arguments. It inserts no default after the first missing
   parameter.

Every other partial call is an error. A compiler operation has only the complete-call case.

## Correct theorem domain

The old proof defined its domain by one informal condition and then claimed an exhaustive translation table. That left
accepted old call shapes with no translation. The repair defines the domain by a translation judgment:

```text
Gamma |- old_term translates_to new_term : A
```

The judgment has one rule for each retained old form. Its ordinary-call rules are exactly the complete and prefix rules
above. Its compiler-operation rule requires every argument. A current term belongs to the retained fragment exactly when
this judgment produces a new term.

This definition is not a trick. The rules give a finite algorithm. They also state every source break at the point where
the compiler can report it.

## Correct proof shape

The final-result proof must cover open function bodies. Define two environments to agree at `Gamma` when they bind each
name to agreeing values of its declared type. Then prove:

> If an old term translates to a new term at type `A`, agreeing environments close the two terms, and old evaluation
> succeeds, new evaluation ends at an agreeing value.

The proof is by induction on the translation derivation. The function case extends both environments with agreeing
arguments and applies the induction hypothesis to the body. The closed-program theorem follows by using empty
environments.

## Audit corrections

The first breaking-change audit missed two facts.

- The governing semantics uses `transpose(i) : Music -> Music` for an arbitrary `i`. The generic `transposer` function
  above replaces it without an anonymous function or a compiler-owned closure.
- Current Musa accepts non-prefix named partial calls of ordinary functions. The refined language rejects them. A small
  wrapper can put captured parameters first, as `choose_second` does above.

The fixed uses of `transpose`, `stretch`, `retrograde`, and `invert` still have the wrappers listed in
`14-breaking-change-repair.md`. Before implementation, an AST-based migration check must find both rejected compiler
calls and rejected non-prefix ordinary calls. Text search is useful evidence, but it is not the migration gate.

## Gate

The proof still does not govern Musa. The repaired translation and theorem need one final independent review. Tasks 7–8
remain blocked until that review reports no fatal, high, or medium issue.
