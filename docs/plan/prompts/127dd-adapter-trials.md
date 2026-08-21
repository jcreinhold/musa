---
id: 127dd
slug: adapter-trials
status: superseded
depends_on: [127dcg]
phase: 3
---

# Freeze the Adapter Rules and Carry Them Through Hostile Review

> **Superseded by prompt [147](147-adapter-freeze.md).** This file is kept rather than deleted because completed prompts
> and research notes link to it. Its nine proof obligations are prompt 147's outline; what changed is that quotation and
> syntax patterns replaced the phase API these obligations were written against. Do not execute this prompt.

## Task

The two trials are built. Freeze the exact rules the adapter boundary now stands on, prove them, and carry the proof
through hostile review, repair, and re-review until the final verdict is correct under the stated contracts with no
fatal, high, or medium finding. The phase is not proved by its own machinery; it is proved by two adapters that carry
real musical load without compiler privilege, and this prompt is where that evidence becomes a frozen claim.

## Read

- `docs/notes/research/language-design-closure/26-language-design-decision.md` §§9, 10 — the admission conditions the
  trials had to meet, the nine proof obligations, and the promotion gate. §10's list is this prompt's outline.
- `docs/notes/research/language-design-closure/27-adapter-trials.md` §§4–6 — what the trials claimed to answer, and the
  five findings from review 24 they were required to close.
- `34-proof-review.md`, `35-proof-repair.md`, `36-final-proof-review.md`, and `37-final-blocker.md` — what the previous
  freeze got wrong, so this one does not repeat it. `37`'s blocker is the one to read twice: an operation that could not
  be both fresh and deterministic, found only at the last gate.
- `docs/rules/language/02-core-calculus.md` §5 and §5.8 — the closed type grammar, the "no syntax value" sentence, and
  the four builtin families. The freeze must state, and prove, that the phase-local transformer calculus is
  *conservative* over these: they are unchanged facts about ordinary source.
- `docs/notes/research/language-design-closure/39-totality-and-structural-abstraction.md` §5, and the paper trial from
  prompt [127dcfae](127dcfae-recursor-trial.md) — the sealed-step recursor's laws, its intrinsic equations, and the
  hostile nested-traversal program that decided its shape.
- Prompts 127da–127dcg and everything they delivered — the freeze describes what was built, not what was hoped for.
  Prompt [127da](127da-path-aware-syntax.md)'s "the fold is the only way into a syntax value" was superseded by prompt
  127dcfaf; the freeze states the rule that now holds, not the one that was first written.

## Design

Freeze the exact rules, and prove each of these:

- expansion termination, determinism, and hygiene;
- unique path formation;
- source attribution, including anchors;
- edit locality and the edit law;
- the print round-trip where a level claims it;
- match execution by the one evaluator;
- derivation coverage;
- associative derivation composition; and
- the sealed-step traversal prompt 127dcfaf installed: sealed association, local structural decrease, repeatability of a
  captured step, and the reducibility/fundamental-lemma cases for higher-order contexts/results, capture, duplication,
  delayed use, and nested traversal.

The last of these is new since this prompt was written, and it is the one to attack hardest. Prompt 127dcfae's paper
trial killed an earlier design in which a child and its descender were separate values: nested recursors could choose
the same context and result types, capture an outer child, and turn a purported structural call into self-descent, with
the types agreeing throughout. Sealing the child and its runner into one value closes reassociation; it does not by
itself prove strong normalization of arbitrary higher-order algebras. The freeze must prove both the association lemma
and the reducibility extension rather than restating either claim. Note 39 §5.3's eleven laws are the list, and each
already names its executable evidence.

The freeze covers the phase-local transformer calculus prompt 127da introduced, which `docs/rules/` does not yet
describe — that is what a freeze is for. It must also establish conservativity over the source core. **If the proof
cannot establish that, the finding is an amendment request under `docs/rules/README.md`, not a repair to make in
passing**: stop, publish the blocker beside the note, and hand the decision back.

Every frozen rule maps to executable evidence. A rule whose evidence is a paragraph is not frozen; it is asserted.
`scripts/check-syntax-adapter-conformance.sh` is that map, and it runs the evidence rather than describing it.

Run hostile proof review, repair, and re-review as many times as needed. This prompt completes only when the final
review says correct under the stated contracts with no fatal, high, or medium finding. Record the freeze, each review,
and each repair under `docs/notes/research/`, beside the notes that failed the last gate.

## Target

- The frozen rules, under `docs/notes/research/`, one file, each rule numbered and each naming its evidence.
- The proofs of the nine obligations above, and the conservativity argument.
- `scripts/check-syntax-adapter-conformance.sh`, mapping each frozen rule to its executable evidence and running it.
- Hostile review, repairs, and a final correct-under-contracts verdict with no unresolved fatal, high, or medium
  finding, each recorded as its own note.
- The five musical cases of §9 rewritten with no ellipses, showing inferred types, expansion, evaluation, stage
  transitions, losses, added choices, and both notation-led and performance-led routes.

## Check

```sh
./scripts/check-syntax-adapter-conformance.sh
cargo nextest run -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Freeze the adapter rules and carry them through hostile review`.

## Stop

- No new adapter operation, no new privilege, and no repair that grants one — a trial that needed a privilege was
  evidence against the boundary, and prompts 127da–127dcg have already closed or reported each.
- No general macro system, type-directed expansion, or adapter-generated declarations. No general recursion in adapter
  code — no `fix`, no recursive binding, no self-application. Sealed structural descent through prompt 127dcfaf's
  recursor is not that and is explicitly admitted: every step enters the proper child sealed into it, and the
  reducibility proof shows that higher-order capture and nested recursors preserve source termination. Local decrease
  alone is not accepted as the whole proof.
- No deletion of contextual `Music`, no notation migration of the corpus, and no surface cutover — prompt 127e owns all
  three.
- No decision-tree or join-point target without a measured need and its own complete semantics and simulation proof.
- No claim that expansion provenance alone proves musical derivation; the two records have different jobs.
- No green verdict while a fatal, high, or medium finding stands, and no amendment to `docs/rules/` made in passing.
