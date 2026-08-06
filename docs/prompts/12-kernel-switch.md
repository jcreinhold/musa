---
id: 12
slug: kernel-switch
status: pending
depends_on: [11]
phase: 1
---

# Switch the Canonical Semantic Path to the Kernel

## Task

Make kernel elaboration the canonical semantics of `musa-compiler::compile` (course correction §30 Step 6), demote the
prompt-05/06 direct lowerer to a retained regression oracle, and lift the "Status: candidate" banner from the kernel
spec. After this prompt the temporal kernel — not the surface grammar — defines Musa's ontology (§35).

## Read

- Course correction §30 Step 6 (switch only after parity), §26 (compiler architecture with the kernel at the center),
  §29 ("do not grow the direct CST-to-score lowering architecture into the permanent semantic model"), §34–35.
- Prompt 11's elaboration module and its differential suite (must be fully green before starting).

## Design

- Flip the default in `CompileOptions` so `compile` elaborates through the kernel. The old lowerer remains compiled in
  behind the options switch, used only by the differential regression tests — it is frozen: bugs found in it after this
  prompt are fixed in the elaboration path unless the differential suite itself is at fault (record the policy in the
  module docs).
- Provenance must be observably unchanged: origin spans and expansion paths in emitted snapshots are byte-identical to
  the pre-switch behavior for every fixture (the differential suite already asserts this; keep it running against the
  oracle permanently).
- Spec graduation: remove the "Status: candidate" banner from `docs/kernel/*.md` and record the falsification status
  (§33: which examples are proven, which remain open and why) in `docs/kernel/08-open-questions.md`.
- `docs/initial-design-roadmap.md` gets one pointer section (where it describes the lowering architecture) forwarding
  to `docs/kernel/` and the course correction — the roadmap remains the source for everything else.
- Update the module-level docs of `musa-compiler` to state the pipeline as: CST → (expansion-aware) elaboration →
  temporal kernel → `ScoreSnapshot` adapter, with the old lowerer named as oracle.

## Target

- `crates/musa-compiler/`: default path switch, doc updates, frozen-oracle policy.
- `docs/kernel/*.md`: candidate banners lifted; falsification status recorded.
- `docs/initial-design-roadmap.md`: forwarding pointer only — no other roadmap edits.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
for f in examples/*.musa; do cargo run -p musa-cli -- check "$f"; done
cargo run -p musa-cli -- render examples/counterpoint.musa --to plan   # unchanged output
grep -L "Status: candidate" docs/kernel/*.md | wc -l   # 9: banners gone
```

Commit as `Make the temporal kernel the canonical semantics`.

## Stop

- Do not delete the old lowerer or its tests.
- No surface-language redesign, no new constructs (§35.1: semantic grammar growth stays frozen).
- No changes to `NotationPlan` or any backend (§35.2).
- Do not start prompt 13+ features "since the kernel is done" — one prompt, one commit.
