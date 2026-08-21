# How the core boundary was decided

**Status: research record. Governs nothing.** The decision it reached is `docs/rules/constitution.md` §7 and its §4
extension; the rules that implement it are `docs/rules/events/12-payload-admission.md`,
`docs/rules/events/07-backend-contract.md`, `docs/rules/events/10-term-calculus.md`, and
`docs/rules/events/04-algebraic-laws.md` X3. This file keeps the evidence and the argument, because the argument is what
makes the decision hard to re-open by accident.

The question, from prompt 126: Musa's core is a calculus — but a calculus *of what*? Peyton Jones (1987) §3 states the
criterion applied throughout: the translation into the core *is* the language's semantics, so a surface construct that
translates into nothing has no semantics beyond whatever its compiler pass happens to do that week.

## The census

Every keyword the lexer accepts was listed with what it elaborates to. The row list was driven from
`crates/musa-syntax/src/keywords.rs` — the table prompt 84 made exhaustive by construction, since `keyword_doc` is a
wildcard-free match over `SyntaxKind` and the workspace forbids wildcard arms, so a keyword the lexer accepts and the
table omits is a compile error. At the time of the decision both sides held 88 spellings.

The census is not reproduced here. It was evidence with an expiry date: its "elaborates to" column was pinned to line
numbers in `elaborate.rs` and `term.rs` and said of itself that those were "orientation, not a contract". What it
*found* is what mattered, and that is below. `docs/rules/events/06-surface-elaboration.md` is the maintained version of
the same information.

The finding: the keywords fell into three groups. Group 1 (notation) and group 2 (values and modules) each elaborated
into an existing calculus. Group 3 did not. Group 3a was performance — `profile`, `performance`, `groove`, and the
realization half of `dynamic`, `crescendo`, `diminuendo`, `mark`, `slur`, and `senza`. Group 3b was the studio.

## The three candidates

- **A — score-only core, the status quo.** The kernel is a calculus of notated occurrences. Performance, instruments,
  and sound are compiler pipelines with no calculus, and group 3's `none` rows are permanent.
- **B — one kernel, several payloads.** The same finite temporal calculus, instantiated at notation facts, at a gesture
  payload, and at whatever the instrument boundary requires. Combination, normalization, equality, and hashing are
  reused rather than reimplemented per layer.
- **C — one dependently typed core.** Payloads *and* the indices that constrain them — part, voice, metre, tuning,
  transposition — become types in a single dependently typed calculus.

## Why A was rejected

Those nine group-3a spellings have a product that is an exactly timed object over a rational extent. Under A that object
still exists — the performance prompt has to build it either way — but it is defined by no calculus, so its ordering,
its equality, its identity, and its behaviour under sequencing, overlay, and scaling are whatever the pass does. Applied
literally, Peyton Jones's criterion says those nine constructs would have no semantics. That is a census row A cannot
serve, not a preference.

Worth recording: A was not *catastrophic*. Going from A to B later is cheap and mechanical — the gesture object exists
either way, and reversing means changing what type it is and re-deriving ordering and equality by hand. But "cheap to
fix later" is not an argument for shipping nine constructs with no semantics.

## Why C was rejected, and why that reason does not expire

This is the load-bearing argument, and it is about reversibility rather than about cost.

B → C later is the normal cost of adding a type theory to a language with a settled core: large, but ordinary, and
exactly the cost C has today. B does not make C harder.

C → anything later is different. **You cannot un-index a payload.** Once indices are load-bearing, every term, every
law, every proof, and every downstream consumer is stated in terms of them. A timeline of gestures indexed by part and
voice does not become an unindexed timeline of gestures by deletion: the operations that were type-correct only under
the index have no meaning without it, and the callers that relied on the index to establish an invariant have to
re-establish it some other way. Reversal is a rewrite of the kernel crate, the compiler, every kernel document, and the
law suite.

Musa has exactly one falsifiable reason to want C — that a wrong part-and-voice pairing should be a type error rather
than a diagnostic — and that reason is served today by the resolver at a cost of one diagnostic each. The decision to
pay an irreversible rewrite for it is not one this project has the evidence to make.

## The decision

**Candidate B — and it always was.** Notation facts are one payload; a gesture payload is a second. The temporal algebra
and the generic carrier do not change; what changes is the compiler's answer to "what is a performance?" — from a list
of scheduled events built by a pass, to a timeline that is normalized, semantically equal or not, semantically hashed,
and obeying the existing laws without a new line of proof.

One thing was withdrawn rather than confirmed: the old identity wording claimed both that a key was injective on values
and that it quotiented fields away, which is contradictory, and the display-text hashing then in use was not uniquely
framed. `docs/rules/events/12-payload-admission.md` is the repair.

The decision also turned out to be one the code had already half-made. The kernel's term, timeline, and occurrence types
were already generic in their payload, and the laws were already proved at a non-musical payload type. What was missing
was the statement that this was the design rather than an accident.

## What was read

Peyton Jones, *The Implementation of Functional Programming Languages* (1987), §3 and §3.1. The kernel documents,
particularly the open questions and the term calculus. The language specification's semantics and core calculus.
