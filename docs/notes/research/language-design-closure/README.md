# Language design closure

**Status: research. Governs nothing.** This directory carries out
[`docs/plan/language-design-closure.md`](../../../plan/language-design-closure.md). Its purpose is to settle Musa's
source language, not to invent another temporal kernel or a package cache.

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
5. [05-stage-semantics.md](05-stage-semantics.md) states the path from source to notation, performance, and sound.
6. [06-paper-programs.md](06-paper-programs.md) tests the rules with complete programs.

Later files will prove the result and record the decision.
