---
id: 127a
slug: core-calculus-governance
status: done
depends_on: [126, 127]
phase: 3
---

# Make the New Core Calculus the Design Target

## Task

Amend Musa's governing and directive documents before changing code. Replace the old account—contextual `Music` above a
temporal kernel and a separate process graph below it—with one total source language that can construct two finite core
values: exact event tracks and deterministic step machines. Keep notation and audio distinct, and make their checked
scheduler the explicit connection.

## Read

- `docs/notes/research/core-calculus/` in order, especially `05-selected-calculus.md`, `06-proof-outline.md`, and
  `17-final-review.md`.
- `docs/rules/{constitution,obligations}.md`, all of `docs/rules/kernel/`, `docs/rules/across-stages/`, and
  `docs/rules/language/`.
- `docs/plan/{roadmap.md,language-design-closure.md}`, `docs/plan/code-map/`, and prompts 92–127.
- The amendment procedure in `docs/rules/README.md`.

## Design

State the new foundation in plain English before notation:

- `EventTrack<C,A>` is a finite exact length and a finite multiset of half-open or point occurrences. It owns `empty`,
  `event`, `follow`, `together`, `map_events`, and `length`.
- `Machine<K,A,B>` is a finite description built from registered primitives, identity, chain connection, side-by-side
  connection, initialized one-step feedback, copy, drop, and swap.
- One audio step is one sample frame. A host block is only a proven or tested batching optimization over frame steps.
- `schedule(format, policy, time_map, track)` is the checked track-to-machine operation. It records all conversion
  decisions and never identifies notation, gesture, or sound.
- The source language is pure, strict, total, and Hindley–Milner inferred. It distinguishes ordinary values from exact
  storable data. It has complete calls, no general recursion, no contextual universal `Music`, no public `lift`, and no
  dependent types or CBPV.

Amend, do not layer. Remove governing claims that the studio has a separate calculus or that audio is outside the core
language. Keep the true part: an audio history is not a finite event track or source value. Preserve source authority,
named conversions, exact identity, derivation records, optional musical theories, unequal-length overlay, and the
real-time rules.

Rewrite the language, kernel, across-stage, roadmap, and code-map documents to use the same names and stage boundaries.
Delete superseded governing prose; keep its reasoning only under `docs/notes/research/`. Record an explicit clean-break
ledger naming source syntax, Rust APIs, fixtures, and serialized forms that later prompts will delete rather than
support through aliases.

## Target

- Deliberate constitutional amendment and fully reconciled governing specifications.
- Updated pipeline, crate ownership, terminology, equality rules, and proof obligations.
- A clean-break ledger for prompts 127b–127i, including prompt 127da, and repaired references in prompts 128–153.
- No implementation change.

## Check

```sh
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
/Users/jcreinhold/.cargo/bin/mdwright check docs/rules docs/plan
git diff --check
```

Commit as `Adopt the event-track and machine core`.

## Stop

- No Rust, TypeScript, grammar, fixture, or generated-file change.
- No compatibility promise, package-cache design, culture-specific theory claim, or new language feature beyond the
  reviewed calculus.
- No vague phrase such as “unified music object,” “world,” or “link” in place of an input, output, and rule.
