---
id: 145
slug: staff-rewrite
status: pending
depends_on: [144]
phase: 3
---

# Rewrite the Staff Adapter, and Find Out Whether Any of This Worked

## Task

Rewrite `stdlib/src/adapters/staff.musa` on the new language. This is the acceptance gate for the entire pass: the
adapter is the recorded failing program that prompt 128's amendment was granted on, and if the rewrite is not
dramatically shorter and more obviously correct than 2,404 lines, the design failed and this prompt is a repair of Phase
A rather than an implementation.

## Read

- `docs/notes/research/language-design-closure/42-dependent-core-decision.md` — the failing program and the measurements
  it recorded, which are the "before" side of this comparison.
- Prompt 132's trial and its **predicted** line and byte count. That number was written before any code existed and is
  the gate; discovering it was optimistic is a finding about the design, not a reason to restate the gate.
- `stdlib/src/adapters/staff.musa` as prompt 142 migrated it — the actual starting point, which is the old structure on
  the new checker.
- `docs/rules/language/11-quotation.md`, `10-traits.md`, and `01-surface.md` — the language as specified, since a
  rewrite that reaches for something unspecified is evidence, not a shortcut.
- Prompts [127dcd](127dcd-adapter-edit.md) and [127dce](127dce-adapter-print.md), and
  `crates/musa-compiler/tests/suite/staff_writing_laws.rs` — the edit and print laws, the three conformance levels, and
  the tests that already hold this adapter to them. **Those tests survive the rewrite unchanged.** They were written
  against behaviour, not against implementation, and if the rewrite needs them changed then either the rewrite changed
  the adapter's meaning or the tests were coupled to internals — and which one it is has to be established, not assumed.
- `docs/notes/research/language-design-closure/41-staff-on-the-repaired-interface.md` — the last time this file was
  rewritten and measured, and the method that measurement used.

## Design

**Five things must be gone, and each is checkable.** Zero `callN` helpers. Zero hand-allocated role integers. Zero
string dispatch on token kinds or delimiters. `Pending` as a record with named fields rather than an eight-field
destructure. And the reading algorithm running forwards, because prompt 141 gave it a way to accumulate. Write each as a
test or a grep in the Check, not as a claim in the commit message.

**Measure the same way twice.** Lines and bytes for the whole file, and separately for the four sections prompt 132
predicted: construction, dispatch, `Pending`, and `document_read`. A whole-file number can hide a section that got worse
under a section that got much better, and the section that got worse is the interesting one.

**"More obviously correct" is a claim that needs evidence.** The available evidence is: the same behavioural tests
passing unchanged, the anchors landing on the same nodes, the printer round-trip holding at the same conformance level,
and the rendered corpus for every staff example staying byte-identical. State each one. A rewrite that is shorter and
changes what a musician sees has not passed.

**Report honestly if the gate is missed.** If the rewrite lands materially above prompt 132's prediction, this prompt
does not quietly accept the number. It records what the language failed to remove, identifies which of prompts 129–131
made the wrong promise, and becomes a repair of that specification — which is the whole reason the prediction was
written down in advance. That is stop condition 5 territory for the prompt stack, and it hands the decision back rather
than absorbing it.

**Do not improve the notation coverage.** The rewrite reads the same music it read before, produces the same values, and
carries the same anchors. Adding a feature during the rewrite makes the measurement meaningless and would be the easiest
possible way to make the number look worse for a good reason and better for a bad one.

## Target

- `stdlib/src/adapters/staff.musa`, rewritten on the new language, with the five eliminations above.
- The measurement: whole file and per-section, before and after, against prompt 132's prediction, recorded in
  `docs/notes/research/language-design-closure/` and linked from the code map.
- `crates/musa-compiler/tests/suite/staff_writing_laws.rs` and every existing staff test passing **unchanged**.
- Mechanical checks in the Check section for the five eliminations.
- The rendered corpus for every staff example byte-identical.

## Check

```sh
cargo build --workspace
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- format --check stdlib/src/adapters/staff.musa
! grep -nE '\bcall[1-9]\b' stdlib/src/adapters/staff.musa
! grep -nE 'syntax_built[(][^,]*, *[0-9]' stdlib/src/adapters/staff.musa
! grep -nE 'text_equal[(][a-z_]*kind' stdlib/src/adapters/staff.musa
wc -lc stdlib/src/adapters/staff.musa
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

The three `grep` lines must find nothing; `wc` records the number the gate is judged on. The oracle stays fixed: a
rewrite of a library file has no business changing a semantic hash or a rendered corpus file.

Commit as `Rewrite the staff adapter on the new language`.

## Stop

- No new notation coverage, no new diagnostic, no changed anchor, no changed conformance level. The adapter does the
  same job.
- No change to `crates/` beyond what a library rewrite genuinely forces, and if it forces any, that is a finding worth
  reporting rather than a quiet commit — the whole claim is that this file is ordinary unprivileged Musa.
- No weakening or rewriting of an existing staff test to accommodate the rewrite.
- No adjustment of prompt 132's prediction. It is a fixed gate.
- No studio work. Prompt 146.
