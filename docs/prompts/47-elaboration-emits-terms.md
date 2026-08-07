---
id: 47
slug: elaboration-emits-terms
status: pending
depends_on: [46]
phase: 3
---

# Elaboration Produces Terms, Not Values

## Task

Change elaboration's output from a `Timeline<ScoreFact>` to a `Term<ScoreFact>` that is evaluated at the boundary.
`repeat n { … }` becomes a `let` bound once and referenced `n` times instead of `n` elaborated copies; a motif
application becomes a reference to its elaborated body; a canon becomes one `let` and two `shift`s. The compiled meaning
is identical — T2 guarantees it — and the term is now what `musa kernel` prints, so the interchange file shows the
structure a composer wrote rather than its expansion.

Justified by two things and measured against both: elaboration cost on the large fixture, and what the kernel file
looks like.

## Read

- `docs/kernel/10-term-calculus.md` T2 (`let` is transparent) and T3 (evaluation is normalization) — the theorems that
  make this a representation change and not a semantic one.
- Course correction §19 (normalization is a semantic boundary, **not** the internal representation of every compiler
  pass — "nothing requires duplicating thousands of nodes merely to obey the normalized model"). This prompt is that
  sentence finally implemented; quote it in the module docs.
- §20 (provenance lives above the kernel) — the hard part below.
- `crates/musa-compiler/src/elaborate.rs` as prompts 39–41 left it: `elaborate_items`, `elaborate_item`'s `Repeat` and
  `Use` arms, `elaborate_use`.
- `docs/kernel/09-performance.md` P2 (elaboration) — the number this prompt moves.

## Design

### Provenance is what makes this hard, and it must be faced first

Today every occurrence of the third repetition carries `RepeatIteration(2)` in its `Origin`, and the Origin view depends
on it. If the body is elaborated once and shared, the occurrences inside the `let` cannot each carry a different
iteration — that is the whole saving.

Resolve it before writing code, and record the resolution in `06-surface-elaboration.md`:

- **Option A — provenance at the reference.** A `let` reference carries the expansion step that distinguishes this use
  (`RepeatIteration(i)`, `MotifApplication`), and evaluation *appends* it to each occurrence's `Origin` as the body is
  instantiated. Provenance stays exact; the saving is in elaboration, not in evaluation, and the evaluated timeline is
  byte-identical to today's.
- **Option B — provenance stays inside.** Sharing only where the expansion path would be identical, which for `repeat`
  is never. Cheap, honest, and buys almost nothing.

Take **A** unless implementation shows it cannot preserve provenance byte-for-byte. It requires the evaluator to thread
a payload-transforming context — which is `map` re-entering by the back door, and must therefore be specified narrowly:
evaluation may append an `ExpansionStep` to a payload's origin at a reference site, and may do nothing else to payloads.
That is a repair to `10-term-calculus.md` (a fifth theorem: instantiation preserves the denotation up to provenance) and
to `45`'s `evaluate` signature, committed **before** the implementation, per AGENTS.md.

If A cannot be made to work, take B, record why, and expect the measurement below to show little — then say so rather
than shipping churn.

### What shares, and what does not

- `repeat n { body }` → `let` + n references. Yes.
- `use motif(args)` → a `let` per (motif, argument tuple), referenced at each call site. Yes; the binding is keyed by
  arguments because different arguments are different bodies.
- `transpose` / `invert` / `stretch` bodies → no. These are payload maps and time scaling applied during elaboration;
  `scale` has a term, the payload maps do not, and inventing one would breach the absent list.
- Voices and parts → `over`; voice items → `seq`. Structural, and they make the printed file legible.

### The measurement decides how far this goes

Run P1–P4 on both fixtures before and after. The expected shape: P2 (elaboration) improves markedly on any fixture with
repeats or motifs, P1 improves less because evaluation still materializes everything, and P3/P4 are unchanged. If P2
does **not** improve on the large fixture, the fixture has no reuse to exploit — extend it with a repeat-heavy section
and measure again, because a change justified by sharing must be measured on material that shares.

Also record the qualitative result: the byte size and the readability of `examples/kernel/canon.kernel` before and
after. A canon that prints as one `let` and two `shift`s is the visible payoff, and the golden diff is the evidence.

### The boundary stays where it is

`compile` still returns a `Compilation` whose snapshot is projected from an evaluated timeline. Terms are internal plus
the `musa kernel` path. Nothing downstream of the compiler learns that terms exist.

## Target

- `docs/kernel/10-term-calculus.md`: the instantiation theorem, if Option A is taken (committed first).
- `crates/musa-kernel/src/term.rs`: reference-site provenance hook, narrowly specified.
- `crates/musa-compiler/src/elaborate.rs`: emits `Term<ScoreFact>`; `repeat` and `use` become bindings; evaluation at
  the boundary.
- `crates/musa-compiler/src/project.rs`: unchanged — it projects an evaluated timeline exactly as before.
- `examples/kernel/*.kernel`: goldens re-recorded (these change; nothing else may).
- `docs/kernel/06-surface-elaboration.md`: the elaboration table gains the term column and the provenance resolution.
- `docs/kernel/09-performance.md`: before/after rows, plus the fixture note if the large fixture was extended.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject   # only examples/kernel/ goldens change
for f in examples/*.musa; do cargo run -p musa-cli -- check "$f"; done
cargo run -p musa-cli -- kernel examples/canon.musa   # a let and two shifts, not a thousand occurrences
cargo bench -p musa-compiler
```

Commit as `Elaborate into kernel terms`.

## Stop

- Provenance must be byte-identical in the projected snapshot. Not "equivalent", not "reordered". If it is not, stop.
- No lazy evaluation, no deferred observation — prompt 48, and only if measured.
- No caching of evaluated bindings across compilations.
- No new term form. If sharing wants one, the specification is repaired first and the evidence recorded.
