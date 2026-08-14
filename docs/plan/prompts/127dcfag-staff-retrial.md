---
id: 127dcfag
slug: staff-retrial
status: done
depends_on: [127dcfaf]
phase: 3
---

# Rewrite Staff Expansion on the Repaired API and Measure What Changed

## Task

The staff adapter is the evidence that failed the old interface. Four ergonomic repairs and one traversal replacement
have landed since it was written. This prompt rewrites `stdlib/src/adapters/staff.musa` against them and measures the
result, so the freeze in prompt 127dd stands on a program written for the boundary rather than around it.

The bar is preservation: the same values, the same diagnostics at the same spans, the same paths, and byte-identical
fixtures. The deliverable is the removal of the machinery the old interface forced — `data Pending`'s unclaimed-argument
slots and the `from_the_end` closure chain — and a measurement that says which repair removed what.

## Read

- Prompt [127dcfa](127dcfa-staff-expansion.md) — what the expansion produces, item by item. This prompt changes how it
  is written, not what it means.
- Prompts [127dcfaa](127dcfaa-list-fold-direction.md), [127dcfab](127dcfab-expression-if.md),
  [127dcfac](127dcfac-record-update.md), [127dcfad](127dcfad-result-question.md), and
  [127dcfaf](127dcfaf-syntax-step-recursor.md) — the five repairs, and the before-measurements each recorded. Those
  measurements exist so this prompt can attribute its improvements rather than claim them.
- Prompt [127dcfae](127dcfae-recursor-trial.md) and its research note — the simplified staff program written on paper.
  This prompt is that program at full size, and a divergence between them is a finding about the trial.
- `docs/notes/research/language-design-closure/39-totality-and-structural-abstraction.md` §5.4's staff paragraph and
  §12.1's falsifier — if the adapter must recover raw `Syntax`, forge a path, or rebuild the same `Pending`-plus-closure
  machine, the interface is wrong and this prompt reports that rather than working around it.
- `stdlib/src/adapters/staff.musa` as it stands: `data Pending` (line 274), the `placing` section (702), the `holding_*`
  setters (896–1003), `clef_named` (1155), `from_the_end` (1871), `read_body` (1941), and `document_read` (1988).
- Prompt [127dcec](127dcec-construction-charges.md) — the construction-charge measurement this prompt re-runs.

## Design

**Take `C` and `A` from the paper trial.** `C` is the inherited meter and open-form state; `A` is the reader's result.
The traversal no longer needs `A` to be a function of context, which is the specific claim being tested: if the rewrite
still ends up with `A = Context -> Result<…>`, the recursor did not do its job and the prompt says so.

**Retire what the old interface forced, and keep what the music needs.** `data Pending`'s seven slots exist because a
bottom-up fold could not carry a partially-read item. Slots that were only ever unclaimed arguments go; state that is
genuinely about music — a tie waiting for its partner, an open form waiting to close — stays, now as inherited context
or as an ordinary result rather than as a record threaded through every step. Which is which is the interesting finding
and must be written down, because it is the difference between the interface's cost and the domain's.

**`from_the_end` is already gone.** Prompt 127dcfaa deleted it. This prompt must not reintroduce a closure chain under
another name; if right-nested output still needs one, that is a finding.

**Measure per repair, not in aggregate.** For each of the five landed prompts, record what it removed from this file:
line count, frame depth at the worst site, `Pending` construction count, and constructed-node count on the smallest
region the adapter accepts. An aggregate improvement credited to the recursor when `?` and record update did the work
would corrupt the evidence prompt 127dd freezes, which is the whole reason the ergonomics landed first.

**Preservation is checked, not asserted.** Existing fixtures are compared byte for byte. A fixture that must change is a
behavior change, and this prompt has no license for one: it stops and reports.

## Target

- `stdlib/src/adapters/staff.musa` rewritten against the repaired API, with the same public surface and the same
  expansion results.
- The measurement table — per repair, what it removed — under `docs/notes/research/language-design-closure/`, with its
  entry in that directory's `README.md`, and a statement of which parts of `Pending` were interface cost and which were
  domain state.
- Any divergence from prompt 127dcfae's paper program, reported as a finding about the trial.
- Existing staff fixtures and snapshots unchanged, byte for byte.
- A re-run of prompt 127dcec's construction-charge measurement on the same region, so the cost claim is current.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Rewrite staff expansion on the repaired API and measure what changed`.

## Stop

- No behavior change. Same values, same diagnostics, same spans, same paths, byte-identical fixtures. A fixture that
  must change is a finding and a stop.
- No new adapter operation and no new privilege. An adapter that needs one is evidence against the boundary, and prompts
  127da–127dcfaf have already closed or reported each.
- No edit or print work. Prompt 127dcfb owns both and depends on this one.
- No studio adapter work. Prompt 127dcg owns it, and writing it here would fit the interface to staff twice.
- No language change of any kind. If the rewrite wants one, that is a finding for prompt 127dd's freeze to weigh, not a
  repair to make here.
- No new closure chain, continuation encoding, or state record standing in for the one this prompt removes.
