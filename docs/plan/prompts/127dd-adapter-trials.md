---
id: 127dd
slug: adapter-trials
status: pending
depends_on: [127dc]
phase: 3
---

# Prove the Adapter Boundary with Two Complete Trials

## Task

Write the complete staff and studio adapters as unprivileged packages, give them the three declared operations, and
freeze the adapter rules and their proofs for hostile review. The phase is not proved by its own machinery; it is proved
by two adapters that carry real musical load without compiler privilege.

## Read

- `docs/notes/research/language-design-closure/27-adapter-trials.md` and `26-language-design-decision.md` §§4, 9, 10 —
  the three operations, the edit law, the print law, the conformance levels, the required coverage, and the promotion
  gate.
- `34-proof-review.md`, `35-proof-repair.md`, `36-final-proof-review.md`, and `37-final-blocker.md` — what the previous
  freeze got wrong, so this one does not repeat it.
- Prompts 127da, 127db, and 127dc, and their delivered modules.
- `crates/musa-project/src/` structured edit commands and `crates/musa-lsp` — an adapter edit reaches a musician through
  these, or it is a claim with no user.
- Open Music Theory `001`–`012` for the staff distinctions the trial must be able to make (spelling versus pitch class,
  written value versus exact span, meter versus hypermeter), and `docs/rules/style-guide.md` for spellings.

## Design

Separate three adapter operations and do not let one stand in for another:

- `expand` returns ordinary expression syntax or an error;
- `edit` returns focused text edits for a structured command; and
- optional `print` creates new source, or states what it cannot preserve.

The edit law is locality plus agreement: if `edit` returns a patch, applying it changes only ranges inside the region,
and expanding, checking, and evaluating the patched region gives the same adapter value that applying the command to the
original value gives, under the adapter's stated semantic equality. Bytes outside the returned ranges are unchanged —
comments, layout, names, and neighbouring definitions are not regenerated. **Reprinting an expanded value is not a safe
edit**, and a printer alone does not make a region editable.

The three conformance levels are readable, editable, and generative. The standard staff and studio adapters must be
generative; a third-party adapter may be read-only.

The staff trial covers notes, rests, chords, dots, exact durations, ties across bars, slurs, tuplets, grace notes,
pickups, repeats, alternate endings, meter changes, transposing instruments, diagnostics, and at least one structured
edit. Written rhythm and exact time stay different data even when they cover the same span: `c4/4.` asks for a dotted
quarter, `c4(3/8)` gives an exact duration and leaves the spelling to a named notation policy. No note inherits register
or duration from an earlier note.

The studio trial covers processors, named ports, connections, parameters, instrument bindings, graph inputs and outputs,
diagnostics, and at least one structured edit.

Neither adapter may call a private parser, receive an inferred type, or hold compiler state. Both use the public syntax
data, the path-aware fold, the builder facade, the expansion record, and the derivation graph the earlier prompts
deliver. If either trial needs a privilege, that is evidence against the boundary and repairs prompt 127da or 127dc
rather than being granted.

Then freeze the exact rules and prove: expansion termination, determinism, and hygiene; unique path formation; source
attribution; edit locality; print round-trip where it is claimed; match execution by the one evaluator; derivation
coverage; and associative derivation composition. The freeze covers the phase-local transformer calculus prompt 127da
introduced, which `docs/rules/` does not yet describe — that is what a freeze is for. It must also state, and prove,
that the calculus is conservative over the source core: `02-core-calculus.md` §5's closed type grammar, its "no syntax
value" sentence, and §5.8's four families are unchanged facts about ordinary source. If the proof cannot establish that,
the finding is an amendment request under `docs/rules/README.md`, not a repair to make in passing. Run hostile proof
review, repair, and re-review as many times as needed. This prompt completes only when the final review says correct
under the stated contracts with no fatal, high, or medium finding. Record the freeze, each review, and each repair under
`docs/notes/research/`, beside the notes that failed the last gate.

## Target

- The complete staff and studio adapters as unprivileged packages under `stdlib/`, both generative.
- All three operations for both, with the edit law and the print law stated and tested, and structured edits reaching a
  user through `musa-project` and `musa-lsp`.
- `scripts/check-syntax-adapter-conformance.sh`, mapping each frozen rule to its executable evidence.
- Frozen rules, proofs, hostile review, repairs, and a final correct-under-contracts verdict with no unresolved fatal,
  high, or medium finding.
- `examples/` fixtures for both trials, and the coverage lists above discharged item by item.

## Check

```sh
./scripts/check-syntax-adapter-conformance.sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Prove the adapter boundary with two complete trials`.

## Stop

- No compiler privilege for either adapter, and no private parser or checker access.
- No general macro system, type-directed expansion, adapter-generated declarations, or adapter recursion.
- No deletion of contextual `Music`, no notation migration of the corpus, and no surface cutover — prompt 127e owns all
  three.
- No decision-tree or join-point target without a measured need and its own complete semantics and simulation proof.
- No claim that expansion provenance alone proves musical derivation; the two records have different jobs.
- No green verdict while a fatal, high, or medium finding stands.
