# The core calculus

**Status: research. These notes do not set Musa's rules.**

This line of work asks one question:

> What is the smallest typed language that can build written music, connect it to instruments and effects, and account
> for the sound that runs from the result?

The answer must be small enough to explain without slogans. It must also survive real examples. A neat notation is not
enough.

## Reading order

1. [What the calculus must explain](00-what-the-calculus-must-explain.md) states the tests and records the earlier false
   start.
2. [Draft A: make everything a flow](01-draft-a-one-flow.md) tries to use one kind of time-varying object for notation
   and audio.
3. [Review of Draft A](02-review-a.md) gives the counterexamples that reject it.
4. [Draft B: tracks and machines](03-draft-b-tracks-and-machines.md) separates finite placed events from things that run
   one step at a time, but keeps both in one total language.
5. [Review of Draft B](04-review-b.md) attacks its unsafe lifting, loose step boundary, and feedback rule.
6. [The selected calculus](05-selected-calculus.md) repairs those defects and gives the full typing and execution rules.
7. [Proof outline](06-proof-outline.md) states the main claims and why they should hold.
8. [Review of the selected calculus](07-review-of-selected-calculus.md) finds two false theorems and two missing
   premises.
9. [Repairs after the first review](08-repairs-after-review.md) closes those findings.
10. [Second review](09-second-review.md) finds a polymorphic escape from the first-order boundary and an imprecise error
    state.
11. [Second repairs](10-second-repairs.md) add the smallest static distinction and type the error boundary.
12. [Third design audit](11-third-design-audit.md) finds an unbounded scheduler counter, hidden policy, handle
    collision, and two overclaims.
13. [Third repairs](12-third-repairs.md) bound the source state, make policy explicit, keep merged handles apart, and
    state the machine algebra exactly.
14. [Fourth design audit](13-fourth-design-audit.md) finds a time-reversal gap, an unstated end-frame rule, and a
    missing success premise.
15. [Fourth repairs](14-fourth-repairs.md) require ordered time, use half-open spans, and compare only successful
    schedules.
16. [Fifth design audit](15-fifth-design-audit.md) finds an ambiguous primitive registry and an overstatement about
    testing batch code.
17. [Fifth repairs](16-fifth-repairs.md) make primitive lookup functional and leave batch correctness as an explicit
    contract.
18. [Final review](17-final-review.md) checks the frozen calculus against its proofs, musical cases, old
    counterexamples, and current code. Its verdict is correct under the stated contracts.

Each rejected claim stays beside its refutation. No file here authorizes a compiler change or a change to `docs/rules/`.
