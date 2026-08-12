# Language design closure

**Status: blocked after the final review. Governs nothing.** This directory carries out
[`docs/plan/language-design-closure.md`](../../../plan/language-design-closure.md). Its purpose is to settle Musa's
source language, not to invent another temporal kernel or a package cache.

The five musical cases support the small call-by-value design. The repository is private and pre-release, so the later
repair rejects partial calls of compiler-owned operations instead of adding foreign function values to the core. The
final review found two false claims in the proof for retained programs: partial calls do not handle intervening defaults
correctly, and the proof does not relate a function's captured defaults to the surrounding declaration values. See
[17-final-proof-review.md](17-final-proof-review.md) and [18-final-blocker.md](18-final-blocker.md). The design was not
promoted.

## The question

> What is the smallest total language in which musicians can define their own musical concepts and elaborate them into
> Musa's existing temporal and audio stages?

Here, **total** means that every accepted source expression finishes with a value or a stated error. It does not mean
that a live performance finishes. A finite program may describe a processor or interaction that runs for as long as
musicians keep playing.

## Starting point

The following results have survived proof review and are not reopened here without a counterexample.

- A finite timeline has an exact rational length and a finite multiset of typed occurrences.
- Sequence adds lengths. Overlay takes the greater length. Overlay neither requires equal lengths nor inserts rests.
- The source expression language is pure, deterministic, and terminating.
- A valid finite audio graph advances by deterministic steps. Its possible input and output histories may be unbounded.
- Representations are connected by typed conversion records with explicit losses. They are not forced into one common
  value.

## Limits

This inquiry uses five musical cases, three language candidates, and at most two proof reviews. It does not design a
registry, version solver, stable compiled interface, or compiled-value cache. Exact Git source packages remain planned,
but they do not decide the source calculus.

The Karnatak, Balinese gamelan, and bomba cases are pressure tests based on limited published sources. They are not
package specifications. A package using one of those names needs review by a practitioner or specialist before it can
ship.

## Reading order

1. [01-current-language-audit.md](01-current-language-audit.md) says what Musa has and where each feature belongs.
2. [02-five-musical-cases.md](02-five-musical-cases.md) tests the language against five different kinds of musical work.
3. [03-language-comparison.md](03-language-comparison.md) tests three small language designs.
4. [04-source-calculus.md](04-source-calculus.md) states the selected language.
5. [04a-formal-rules.md](04a-formal-rules.md) gives the complete typing, module, and evaluation rules.
6. [05-stage-semantics.md](05-stage-semantics.md) states the path from source to notation, performance, and sound.
7. [06-paper-programs.md](06-paper-programs.md) tests the rules with complete programs.
8. [07-proof-prototype.md](07-proof-prototype.md) attacks the proof and compares two proof routes.
9. [08-proof-outline.md](08-proof-outline.md) fixes the theorem statements and proof order.
10. [09-metatheory.md](09-metatheory.md) gives the full proof under explicit compiler and stage contracts.
11. [10-proof-review.md](10-proof-review.md) records the failed first independent review.
12. [11-proof-repair.md](11-proof-repair.md) records the one permitted repair.
13. [12-proof-review.md](12-proof-review.md) gives the second and final proof review.
14. [13-final-blocker.md](13-final-blocker.md) records why promotion stopped and what survived.
15. [14-breaking-change-repair.md](14-breaking-change-repair.md) reopens the work under the explicit pre-release
    compatibility decision.
16. [15-proof-review-breaking-repair.md](15-proof-review-breaking-repair.md) attacks that decision and finds a false
    migration theorem.
17. [16-retained-translation-repair.md](16-retained-translation-repair.md) defines the exact retained call shapes and
    repairs the open-term proof.
18. [17-final-proof-review.md](17-final-proof-review.md) gives the final permitted review and finds two false proof
    claims.
19. [18-final-blocker.md](18-final-blocker.md) records why promotion stopped and the simplest design to test if the work
    is reopened.
