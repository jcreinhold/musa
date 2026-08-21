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
19. [The vocabulary amendment](18-vocabulary-amendment.md) records the rename that followed: position split from
    duration, primitive split from builtin, and three smaller name repairs. It changes no decision, and files 1–18 are
    deliberately left in the old vocabulary, so read its §5 mapping before taking a name here as current.
20. [`Unit` has no surface value](19-unit-has-no-surface-value.md) records what prompt 127d's machine ports made
    explicit: `Unit` stays in the offered type vocabulary, because the compiler prints it, and stays valueless in the
    surface, because a port says what flows and nothing consumes the value. It refuses both a unit literal and dropping
    the name.

21. [The direction a list fold runs](20-the-direction-a-list-fold-runs.md) records prompt 127dcfaa's split of
    `list_fold` into `list_fold_from_start` and `list_fold_from_end`. `list` is the one inductive type here whose
    constructor nesting and element order disagree, so it is the one whose fold direction is observable and the one
    where a name has to say it; `nat` and `option` need one name each for reasons the note states. It refuses both
    flipping the name's meaning in place and leaving the closure chain in the standard library.

22. [Rebuilding a record by naming only what changed](21-record-update.md) records prompt 127dcfac's addition of
    `subject with { field = expr }`. It elaborates to a one-arm match and the declaration's own constructor, so it adds
    no core term and charges one construction; the binders it introduces are unspellable, which is what makes a
    right-hand side read the surrounding scope rather than the field. It records the measured `holding_*` reduction in
    `staff.musa`, refuses row polymorphism, generated setters, lenses, and nested-path update, and states the two places
    the implementation departs from the prompt's Design.

23. [Carrying a failure outward](22-carrying-a-failure-outward.md) records prompt 127dcfad's addition of postfix `?`. It
    elaborates to the exhaustive `Result` match with unspellable binders, so it adds no core term and costs what that
    match costs — proved by the meter rather than asserted. It records `document_read`'s measured collapse, states why
    the discharge point is the answer's *tail* in a language with no `return` and why that is not an annotation
    requirement, and refuses `Try`/`Monad`, a general `do`, error-type coercion, and `?` on `Option`.

24. [The events vocabulary amendment](23-events-vocabulary.md) records retiring the word *kernel*, which had come to
    name the crate, the governing directory, the quotation keyword, the interchange format, the CLI subcommand, and the
    value itself — six referents for a word the constitution never used, since it says **event track** throughout. It
    states the replacement rule, weighs `events` against `time`, `track`, and `core`, and answers the migration with a
    clean break: no deprecated spelling, because an accepted alias would be the defect the amendment removes.

Each rejected claim stays beside its refutation. No file here authorizes a compiler change or a change to `docs/rules/`.
