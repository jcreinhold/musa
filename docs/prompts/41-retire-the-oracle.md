---
id: 41
slug: retire-the-oracle
status: done
depends_on: [40]
phase: 3
---

# Delete the Direct Lowerer

## Task

Delete the prompt-05/06 CST→snapshot lowerer, the `Elaboration` switch that selects it, and the differential suite that
compared the two paths. It was retained at prompt 12 as the regression oracle for the kernel migration; that migration
finished at prompt 40, and the oracle has been unable to compile most of the language since prompt 27 — it rejects ties,
slurs, dynamics, tuplets, phrases, sections, and harmony by name. A frozen second implementation of a shrinking subset
is no longer a safety net; it is a second answer to "what does this piece mean" that has to be kept compiling.

Separate the shared resolution helpers out of `lower.rs` first, so the deletion removes an implementation and not the
parts of it the live path depends on.

## Read

- Course correction §29 ("do not grow the direct CST-to-score lowering architecture into the permanent semantic model"),
  §30 Step 6 (the switch, which prompt 12 performed).
- Prompt 12's Design section — it states the retention policy this prompt ends, and the reason (parity), which no longer
  applies.
- `crates/musa-compiler/src/lower.rs` — read the whole file and sort every item into *shared* or *oracle*. The shared
  set today is roughly: `Lowering`, `DeclInfo`, `DeclKey`, `MotifDef`, `BoundValue`, `ExpandCx`, `GroupInfo`,
  `GroupKind`, `span_of`, `trimmed_span`, `ordinal`, `token_text`, `declare`, `group`, `error`, `event_id`,
  `lower_header`, `lower_studio`, `register_motifs`, `merge_profiles`, `parse_profiles`, `part_metadata`,
  `tempo_reading`, `parse_ratio`, `parse_duration`, `resolve_pitch`, `resolve_duration`, `bind_argument`,
  `check_measure_sanity`. The oracle set is `lower`, `lower_score`, `lower_voice`, `lower_items`, `lower_use`. Verify
  the split against the compiler rather than trusting this list.
- `crates/musa-compiler/tests/elaboration.rs` (the differential suite, including
  `phase_two_constructs_are_kernel_only`), `crates/musa-compiler/tests/studio_laws.rs` (one test still selects
  `Elaboration::Direct`).
- PoSD ch. 16 (modifying existing code: leave the system with a better design than you found, not just a working one)
  and the red flag *conjoined methods* — a module that is both the shared vocabulary and one of two implementations is
  the reason this deletion needs a split first.

## Design

### Step 1 — split, and commit the split

`lower.rs` is two modules wearing one name. Separate them along the line of what each *hides*:

- **`resolve.rs`** — names, declarations, ordinals, motif registration, argument binding, pitch and duration resolution,
  header/studio/profile reading, source spans, and the diagnostic sink. This is the vocabulary both paths spoke, and it
  is what elaboration keeps. It hides the declaration table and the expansion context.
- what remains in `lower.rs` is the oracle, and only the oracle.

The split is mechanical and must change no behaviour: same functions, same signatures, new home. Commit it on its own so
the deletion that follows is reviewable as a deletion.

`Lowering` is a bad name for what survives — it names a pass that will not exist. Rename it to something that says what
it holds: it is the resolution context plus the diagnostic sink. If the honest name is two names, that is the module
telling you it is two things; split it. (PoSD ch. 14; the red flag is *hard to pick name*.)

### Step 2 — delete

Delete, in one commit: the oracle functions, `CompileOptions::elaboration`, `Elaboration`, `compile`'s match, the
differential suite, the `Elaboration::Direct` use in `studio_laws.rs`, and the oracle's rejection diagnostics for
phase-2 constructs.

`CompileOptions` becomes an empty struct again, and `compile(source, &CompileOptions::default())` is the only call
shape. Check whether `CompileOptions` still earns its existence: it is currently empty of everything except the switch
being deleted, and prompt 05's comment says defaults live inside. If nothing is left, delete the parameter too and let
`compile(source)` be the signature — a parameter every caller passes the same value to is a parameter that belongs
inside (PoSD ch. 4; the module-design rule "put defaults inside"). Update `musa-project`, `musa`, and the tests
accordingly. If prompt 36's work has put a real option in it, keep it and say so.

### What replaces the safety net

Nothing needs to. The regression net is now what it should have been: the `examples/*.musa` corpus with `insta` goldens
at every backend, the law suites (`transform_laws`, `notation_details_laws`, `annotation_laws`, `profile_laws`), the
kernel's own property tests, and the kernel normal form of every fixture. Confirm before deleting, by checking that each
behaviour the differential suite asserted has a home in that set — positions, durations, spelling, part/voice identity,
multiplicity, ordering, provenance. Anything that does not, gets a test in this commit **before** the suite is deleted.
List them in "Repairs made while implementing".

### The documents

- `docs/kernel/06-surface-elaboration.md`: the parity requirement and the "old lowerer is the regression oracle until
  prompt 12 and remains runnable permanently" sentence are now false. Repair them; state what the regression net is.
- `docs/prompts/README.md`: the paragraph under the sequence table says the direct lowering is "frozen as the regression
  oracle". Repair it — it should read as history: retained through prompt 40, deleted at 41.
- `crates/musa-compiler/src/lib.rs`: the pipeline doc comment loses the oracle sentence.

## Target

- `crates/musa-compiler/src/resolve.rs` (new, from `lower.rs`), `lower.rs` deleted entirely at step 2.
- `crates/musa-compiler/src/compile.rs`: `Elaboration` gone; `CompileOptions` gone if empty.
- `crates/musa-compiler/tests/elaboration.rs` deleted; `studio_laws.rs` updated; any coverage gap closed first.
- `crates/musa-project`, `crates/musa`: call-site updates.
- `docs/kernel/06-surface-elaboration.md`, `docs/prompts/README.md`, `musa-compiler` module docs.
- `docs/kernel/09-performance.md`: this prompt's row.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject   # no golden changes
for f in examples/*.musa; do cargo run -p musa -- check "$f"; done
grep -rn "Elaboration\|regression oracle\|lower_score\|lower_items" crates/ docs/kernel/ | wc -l   # 0
cargo bench -p musa-compiler
```

Commit the split as `Separate resolution from the frozen lowerer`, then the deletion as
`Delete the direct lowering path`.

## Stop

- Do not delete anything the elaboration path still uses. If a "shared" helper turns out to be oracle-shaped, that is a
  finding: fix elaboration to not need it, in its own commit.
- Do not change elaboration semantics. This prompt removes code; the output of `compile` is byte-identical.
- Do not take the opportunity to refactor `resolve.rs` beyond the rename. It has been moved; that is enough churn for
  one commit.
- Do not delete `examples/` fixtures or "consolidate" them. They are the net now.

## Repairs made while implementing

### The split (`Separate resolution from the frozen lowerer`)

- **`Lowering` became `Resolver`, and the module `resolve.rs`.** The struct holds the tables names resolve against, the
  counters that issue identities, the annotations resolved to those identities, and the diagnostics for what could not
  be resolved — one honest name, so it did not need splitting further.
- **`apply_intervals` is shared, not oracle-shaped.** The prompt's item list omits it; `resolve_pitch` calls it, so it
  moved with resolution. (`elaborate.rs` still has a second copy with the same semantics. That duplication predates this
  prompt and outlives it: removing it is a behaviour-preserving refactor of the *live* path, and the Stop section says
  not to take the opportunity.)

### The deletion (`Delete the direct lowering path`)

- **`CompileOptions` survives.** Prompt 36 put `imports` in it, so it is not empty and `compile(source, &options)` keeps
  its shape; three call sites lost a now-pointless `..CompileOptions::default()`.
- **`tests/elaboration.rs` was rewritten, not deleted.** Deleting the file would have taken
  `kernel_normal_forms_snapshot` and `repeat_unrolls_to_the_same_kernel` with it — the very goldens the prompt names as
  part of the replacement net — and renaming the file would have churned every `elaboration__*.snap`. The differential
  tests are gone; the rest stayed and the module doc now says what the file is.
- **Coverage checked before deleting, and two of the three "error fixtures" turned out to assert nothing.**
  `fixtures_have_errors_under_both_paths` compared three sources: `use nope()` (a real error, and the only one), a motif
  forward-reference in a piece with **no `score` block** (never expanded, so never an error — the real coverage is
  `compiler.rs::motifs_only_see_earlier_motifs`), and `css4` carried up two octaves and a minor second (`dss6`,
  perfectly spellable — the real coverage is `transform_laws.rs::a_mirror_image_the_language_cannot_write_is_reported`).
  All three are now stated as what they actually are. Everything else the suite asserted — positions, durations,
  spelling, identity, multiplicity, ordering, provenance — is pinned absolutely by the backend goldens, the law suites,
  and the kernel normal forms.
- **The generated corpus now checks itself against its own text**, since there is no second path to compare to: the
  strategies carry how many statements they wrote and how long those statements last, and the property is that the piece
  elaborates to exactly that. This immediately found a bug the differential suite had been hiding: its chord arm
  generated `chord (c4 c4)`, which is a **syntax error** — the real syntax is `chord [c4, c4]` — so a third of the
  generated corpus had been exercising the parser's error recovery rather than chords, and the comparison never noticed
  because both paths rejected it identically. That is the argument for the whole prompt in one line: two implementations
  agreeing proves nothing about either.
- **`phase_two_constructs_are_kernel_only` is gone with the boundary it pinned.** The kernel half of what it asserted —
  that slurs, dynamics and tuplets compile — is covered by `annotation_laws` and `notation_details_laws`.
- **The performance row is a non-measurement, deliberately recorded.** Every allocation count is identical to prompt
  40's in all four phases; the timings sit 5–8% higher because the machine was contended during the run. Recorded with
  that reasoning rather than silently, since the alternative is a table that looks like a regression nobody explained.
