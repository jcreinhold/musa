---
id: 147
slug: adapter-freeze
status: pending
depends_on: [146]
phase: 3
---

# Freeze the Adapter Rules and Carry Them Through Hostile Review

## Task

Two adapters are built on the new language. Freeze the exact rules the adapter boundary now stands on, prove them, and
carry the proof through hostile review, repair, and re-review until the final verdict is correct under the stated
contracts with no fatal, high, or medium finding. This prompt absorbs 127dd, repaired for the interface that actually
exists: quotation and syntax patterns changed what has to be proved.

## Read

- The superseded prompt [127dd](127dd-adapter-trials.md), whose whole Task this prompt absorbs. Its nine proof
  obligations are the outline; what changed is that two of them now have different mechanisms underneath. Absorbing it
  was right because freezing rules about a phase API that prompts 138–140 then replaced would have frozen the wrong
  thing.
- `docs/notes/research/language-design-closure/26-language-design-decision.md` §§9–10 — the admission conditions, the
  nine proof obligations, and the promotion gate.
- `34-proof-review.md`, `35-proof-repair.md`, `36-final-proof-review.md`, and `37-final-blocker.md` — what the previous
  freeze got wrong, so this one does not repeat it. Read `37` twice: an operation that could not be both fresh and
  deterministic, found only at the last gate.
- `docs/rules/language/02-core-calculus.md` §5 as prompt 129 rewrote it, and §5.9 — the conservativity claim is now
  about a dependent core, so the argument is different even where the statement is the same.
- `docs/rules/language/11-quotation.md` and prompts [139](139-quotation.md) and [140](140-syntax-patterns.md) — the
  hygiene, derived-identity, substitution, and trivia-insensitivity laws that did not exist when 127dd was written.
- Prompts 145 and 146's measurements and the staff/studio asymmetry they recorded. The freeze describes what was built
  and what it cost, not what was hoped for.

## Design

**Freeze the exact rules, and prove each of these:**

- expansion termination, determinism, and hygiene — now including quotation hygiene, which is a stronger statement than
  the old one because a quote can introduce binders;
- unique path formation, and derived identity: same origin, same quotation, same path is the same node, and nothing else
  is;
- source attribution, including anchors, through both construction and matching;
- edit locality and the edit law;
- the print round-trip where a level claims it;
- match execution by the one evaluator — a syntax pattern goes through the same case-tree compiler as every other
  pattern, so this obligation is now partly discharged by prompt 135's coverage proof and the freeze says exactly which
  part;
- derivation coverage and associative derivation composition;
- the substitution law for quotation: splicing `quote { e }` into a hole equals writing `e` there; and
- what survives of the sealed-step traversal — sealed association, local structural decrease, repeatability of a
  captured step, and the reducibility cases for higher-order contexts, capture, duplication, delayed use, and nested
  traversal. Prompt 131 kept the recursor for unknown shape; the freeze proves it at every category of `Syntax<Cat>`,
  which is a generalization of what 127dcfaf proved and not a restatement of it.

**Conservativity is now over a dependent core**, which is the substantive change from 127dd. The phase-local calculus
must be a conservative extension of the source language `02-core-calculus.md` specifies, and the argument has to be made
against normalization-by-evaluation and indexed families rather than against rank-1 inference. **If the proof cannot
establish it, the finding is an amendment request under `docs/rules/README.md`, not a repair to make in passing**: stop,
publish the blocker beside the note, and hand the decision back.

**Every frozen rule maps to executable evidence.** A rule whose evidence is a paragraph is asserted, not frozen.
`scripts/check-syntax-adapter-conformance.sh` is that map, and it runs the evidence rather than describing it.

**Hostile review, repair, re-review, until it holds.** This prompt completes only when the final review says correct
under the stated contracts with no fatal, high, or medium finding. Record the freeze, each review, and each repair under
`docs/notes/research/language-design-closure/`, beside the notes that failed the last gate — including the ones that
failed, because a freeze whose failed attempts are invisible is a freeze nobody can audit.

## Target

- The frozen rules, one file, each rule numbered and each naming its evidence.
- The proofs of the obligations above, and the conservativity argument over the dependent core.
- `scripts/check-syntax-adapter-conformance.sh`, mapping each frozen rule to its executable evidence and running it.
- Hostile review, repairs, and a final correct-under-contracts verdict with no unresolved fatal, high, or medium
  finding, each recorded as its own note.
- Note 27 §9's five musical cases rewritten with no ellipses on the new language, showing elaborated types, expansion,
  evaluation, stage transitions, losses, added choices, and both notation-led and performance-led routes.

## Check

```sh
./scripts/check-syntax-adapter-conformance.sh
cargo build --workspace
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Freeze the adapter rules and carry them through hostile review`.

## Stop

- No new adapter operation and no new privilege. A trial that needed one was evidence against the boundary, and prompts
  145 and 146 have already reported each.
- No amendment to `docs/rules/`. A freeze that amends the rules it is freezing has not frozen anything; a blocker is
  published and handed back.
- No core metatheory. Prompt 148 owns the core's obligation matrix; this prompt owns the phase boundary over it.
- No adapter rewrite and no measurement change. 145 and 146 are fixed.
- No third adapter. Two were the evidence standard, and a third would be scope growth rather than proof.
