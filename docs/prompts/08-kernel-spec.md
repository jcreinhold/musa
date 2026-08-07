---
id: 08
slug: kernel-spec
status: done
depends_on: [07]
phase: 1
---

# Temporal Kernel Specification

## Task

Write the authoritative specification for Musa's finite temporal kernel — the small, exact, backend-independent
semantics into which the surface language elaborates — as the `docs/kernel/` document set named by the course correction
§30 Step 1. Marked **candidate** until the law tests of prompt 10 and the falsification corpus of prompt 11 pass. No
code in this prompt; the spec is the deliverable that prompts 09–12 implement.

## Read

- `docs/course-correction.md` in full. The kernel sections (§§2–26, 29–35) are the primary source; this prompt
  transcribes and sharpens them, never contradicts them. Where the course correction and the original roadmap disagree,
  the course correction wins for kernel matters, and the disagreement is recorded in `08-open-questions.md`.
- Roadmap §6 (the current `ScoreSnapshot`, exact time, provenance) — the kernel spec must state precisely how
  `ScoreSnapshot` relates to the normalized kernel denotation (course correction §27).

## Design

Author nine documents, each with a "Status: candidate" banner until prompt 12:

- `docs/kernel/00-purpose.md` — the two-picture architecture (rich surface / small kernel); the question the kernel
  answers ("what musical facts exist, and where in musical time"); the governing rule of course correction §34 (semantic
  necessity, quoted verbatim, as the acceptance test for every future kernel proposal).
- `01-grammar.md` — the kernel interchange syntax of course correction §24 (timeline / sequence / overlay / composition
  references), plus a deliberately boring first-order payload-schema grammar; the file is an interchange language, not
  the musician-facing syntax. A parser for it is deferred (see `08-open-questions.md`); prompt 09 implements canonical
  serialization only.
- `02-static-semantics.md` — well-formedness: occurrence bounds `0 ≤ s ≤ e ≤ d`, positive scaling factors, reference
  resolution, acyclic composition references; payload schemas are checked by their own modules, not the kernel.
- `03-denotational-semantics.md` — the denotation `(d, E)` with `E` a finite multiset of `(s, e, a)`; definitions of
  sequence (§7), overlay (§8), ambient extension (§9), restriction with whole/visible span distinction (§17), payload
  mapping (§13), time scaling (§14), delay as derived (§15). Explicitly: no `Rest`/`Silence` object — uncovered regions
  are silent by absence (§2); not a semiring (§10); not assumed a monad (§16).
- `04-algebraic-laws.md` — every law of course correction §30 Step 3, stated formally and cross-referenced to the
  proptest names that prompt 10 must implement, including the synchronized interchange law (§11) with its
  duration-equality preconditions, and the explicit non-laws (distributivity, overlay idempotence) with counterexamples.
- `05-normalization.md` — the canonical normal form of §25: flat timeline, no sequence/overlay/references remaining,
  canonical occurrence ordering (define it: start, then end, then canonical payload serialization), semantic equality as
  equality of normal forms, semantic hashing implications.
- `06-surface-elaboration.md` — how the current surface constructs elaborate (course correction §30 Step 4): note →
  typed occurrence; rest → ambient extent with no occurrence; chord → simultaneous occurrences; voice/part → payload
  identity metadata (recording the §32 decision to keep voice identity out of the temporal primitives); motif/repeat →
  HIR structure evaluated into the kernel; transpose → payload map; key/meter → typed interval payloads (§21); tempo →
  performance-layer concern, not kernel (§22). Provenance rides in payload metadata, above the semantic quotient (§20).
- `07-backend-contract.md` — what downstream consumers may assume: normalized timelines only; notation decides rest
  glyphs for uncovered regions (§2); performance supplies the monotone tempo map `Beat → Second` (§22); audio stays a
  separate signal layer meeting the kernel at the instrument boundary (§23). Restate that the existing
  `ScoreSnapshot → NotationPlan → backend` boundary is preserved (§27–28).
- `08-open-questions.md` — the §32 list verbatim-ish (infinite/live patterns, aleatory, voice identity evidence,
  continuous controls, recursion), plus the deferred kernel-file parser and anything discovered while writing.

## Target

- `docs/kernel/` with the nine files above. No crate, no code, no example changes.

## Check

```sh
ls docs/kernel/*.md | wc -l   # 9
grep -L "Status: candidate" docs/kernel/*.md   # empty: every file carries the banner
grep -c "interchange" docs/kernel/01-grammar.md   # non-zero
```

Commit as `Specify the finite temporal kernel (candidate)`.

## Stop

- No implementation (prompt 09), no law tests (prompt 10), no elaboration code (prompt 11).
- No `Pattern`, aleatory choice, recursion, or DSP in the kernel spec beyond "producers of finite observations" (§18,
  §32) — these stay open questions.
- Do not redesign the surface language here (§31); `06-surface-elaboration.md` describes elaboration of the grammar as
  it exists.
