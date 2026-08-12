# Final blocker: defaults in partial calls

## Purpose

This note records why the proposed source language was not promoted and what to try if the work is reopened.

## Decision

Stop this design round. The final proof review found two High and two Medium errors. The plan requires a clean final
review before the research may change governing rules or implementation prompts. That condition was not met.

The small call-by-value language remains a useful candidate. It is not yet Musa's governing language.

## What failed

### Partial calls and defaults disagree about the result type

Consider this function:

```musa
fn choose(first: Nat, middle: Bool = true, last: Nat) -> Nat {
    first
}
```

Current Musa accepts `choose(1)` as a function of type `Nat -> Nat`. It fills `middle` with `true` at once and leaves
only `last` open.

The proposed translation turns the declaration into a curried function:

```text
Nat -> Bool -> Nat -> Nat
```

It translates `choose(1)` without filling `middle`, so the result has type `Bool -> Nat -> Nat`. The translation does
not preserve the type of this accepted source program.

### The proof loses the values captured by a default

A default may refer to a value that was in scope when the function was defined:

```musa
let captured: Nat = 1;

fn choose(value: Nat = captured) -> Nat {
    value
}
```

Current Musa stores the default with the function. The proposed translation copies the default expression into a call.
Its proof compares the function and the surrounding bindings one by one, but it never states that the copied binding is
the same value that the old function captured. The main open-term lemma is therefore false as written.

### Two smaller gaps remain

- A music recipe may store a pitch-mapping function. The value relation says how to compare functions and how to compare
  recipe graphs, but not how to compare a function stored inside a graph node.
- One summary still promises compatibility for more current programs than the final theorem accepts.

The full arguments and counterexamples are in [17-final-proof-review.md](17-final-proof-review.md).

## What survived

The review did not find a reason to add dependent types, call-by-push-value, worlds, or a second temporal kernel.

These parts remain strong candidates:

- a total call-by-value source language;
- finite user-defined data and exhaustive matching;
- ordinary first-class functions;
- compiler-owned musical operations that must be called completely;
- an explicit `Music` context that closes to a finite temporal term or an error;
- exact derivation records between notation, analysis, performance, and sound;
- the existing finite timeline and running-process semantics.

The proof of the new calculus itself also survived the attack. The failed part is the claim that a selected set of
current calling forms translates into it without changing results.

## Simplest design to test next

If the work is reopened, treat a default as shorthand for a **complete direct call**, not as part of a function value.

Under that rule:

- `f()` may fill defaults only when the call can finish;
- a partial call must supply an initial run of parameters;
- the result accepts every remaining parameter, even one whose declaration supplied a default; and
- an indirect function call has one ordinary argument and never applies a declaration default.

Then `choose(1)` above has type `Bool -> Nat -> Nat`. A caller that wants the default writes a complete direct call,
such as `choose(1, last: 2)`, and the compiler inserts `middle = true` there.

This rule is smaller than current Musa's rule. It removes defaults from closure values, avoids the captured-default
proof problem, and makes the curried translation literal. It is a source break, which is acceptable before release but
must have a repository-wide audit and clear diagnostics.

The alternative is to preserve today's partial-call behavior by storing defaults and their captured values in refined
closures. That adds another closure rule and a stronger proof relation for little musical benefit. Do not choose it
without a concrete program that needs it.

Any new round should also define pitch-mapping callback agreement inside music recipes and replace the stale broad
compatibility claim. It needs a new proof budget and another independent review; this round is closed.
