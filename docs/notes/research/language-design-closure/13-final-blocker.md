# Final proof gate

**Purpose:** record why this design was not promoted and what a later attempt must repair.

## Decision

Do not promote the proposed source language. Do not revise the governing language rules, architecture map, roadmap, or
implementation prompts from this draft.

The domain study supports the design: five different musical cases fit a small, pure, terminating language with finite
data, `Text`, `Result`, hidden constructors, and explicit stage boundaries. The final proof review did not reject that
direction. It found two exact flaws in the formal calculus. The closure plan says either flaw is enough to stop.

## What failed

First, the proposed calculus cannot express every program that current Musa accepts. Current Musa permits a built-in
music transform to be used as a function:

```musa
let octave_answer: Music -> Music = transpose(P8);
```

The proposed rules allow `transpose` only after all its arguments are present. They have no value for the partly applied
call above. The theorem claiming that every current expression embeds in the new calculus is therefore false. This is a
High finding because the counterexample is a governing, shipped, tested program.

Second, the formal rules for `map` and `filter` can take two different next steps. Their entry rules may run before the
list argument becomes a value, even though the evaluation-context rule says to evaluate that argument first. The same
problem appears while a pending callback is still reducing. This makes the stated evaluation relation nondeterministic
and breaks the proof of repeatable resource charges. This is a Medium finding.

The exact arguments and counterexamples are in [12-proof-review.md](12-proof-review.md).

## What survived

The final review found no remaining problem with:

- decidable checking over one finite resolved import graph;
- substitution, preservation, and progress;
- termination of the intended source language;
- finite non-recursive user data and its rank order;
- hidden constructors and abstract data members;
- the finite `Music` recipe and its explicit musical context;
- closing successful `Music` recipes into finite typed temporal terms;
- exact-anchor composition of stage records;
- the five paper programs under their stated adapter contracts; or
- keeping source evaluation, temporal normalization, process steps, and an unbounded audio run distinct.

The case studies also found no need for dependent types, call-by-push-value, worlds, links, general recursion, source
effects, or a common value that is at once notation and sound.

## How to reopen the work

A later attempt should make two small changes before writing another proof.

1. Treat the fixed set of built-in music transforms as ordinary curried function values. Supplying some arguments
   returns a closed value that remembers them. Supplying the last argument constructs the finite `Music` recipe. This
   keeps current behavior without opening a registry of arbitrary higher-order compiler operations. A complete
   type-directed translation into source lambdas would also work, but it must cover every current partial call.
2. Require values in the `map` and `filter` entry and callback-completion rules. Then prove that each non-value term has
   one next evaluation position and that callbacks and resource charges occur in source order.

After those changes, rewrite the compatibility, determinism, and metering proofs and commission a new review under a new
plan. The two reviews allowed by this plan are spent. Starting a third review here would hide the failed gate rather
than respect it.

## Work deliberately not done

Because the proof gate failed, this effort did not:

- make the research calculus govern Musa;
- move current musical types out of the compiler;
- add implementation prompts for `Text`, `Result`, user data, or sealing;
- alter studio, gesture, or audio dependencies;
- repair the pinned-package prompt; or
- claim that the proposed paper programs compile today.

Those are consequences of a passing language decision, not evidence that can make a failed proof pass.
