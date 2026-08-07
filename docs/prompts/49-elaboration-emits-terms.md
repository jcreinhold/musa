---
id: 49
slug: elaboration-emits-terms
status: done
depends_on: [48]
phase: 3
---

# Elaboration Produces Terms, Not Values

## Task

Change elaboration's output from a `Timeline<ScoreFact>` to a `Term<ScoreFact>` that is evaluated at the boundary.
`repeat n { … }` becomes a `let` bound once and referenced `n` times instead of `n` elaborated copies; a motif
application becomes a reference to its elaborated body; a canon becomes one `let` and two `shift`s. The compiled meaning
is identical — T2 guarantees it — and the term is now what `musa kernel` prints, so the interchange file shows the
structure a composer wrote rather than its expansion.

Justified by two things and measured against both: elaboration cost on the large fixture, and what the kernel file looks
like.

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
- No lazy evaluation, no deferred observation — prompt 50, and only if measured.
- No caching of evaluated bindings across compilations.
- No new term form. If sharing wants one, the specification is repaired first and the evidence recorded.

## Repairs made while implementing

**The mark is wider than one step, and had to be.** The prompt (and `06-surface-elaboration.md`'s first draft) said a
reference's mark is "exactly one `step`" appended to each instantiated occurrence's origin. Three things broke that, all
discovered by the byte-identical-provenance requirement in *Stop*:

1. **Append is the wrong position.** Direct expansion puts `RepeatIteration(i)` *before* the steps of everything nested
   inside the body. Appending puts it after. The mark carries an **insertion depth**.
2. **A motif body cannot carry the path that led to the call**, because two call sites must reach the same body — so the
   body is elaborated with an *empty* path and the mark carries the whole prefix, spliced at depth zero. That makes the
   mark a step *list*, not a step.
3. **Nor can it carry the call site or the voice.** A motif body's `source_span` comes from the call and its `scope`
   from the voice. The body carries `u32::MAX` placeholders in both, and the mark says what to substitute — which is why
   `4294967295` is visible inside shared bindings in `examples/kernel/*.kernel`.

The mark is therefore `<depth>|<origin-span>|<scope>|<steps>`, and it is `ScoreFact`'s format, not the kernel's — the
kernel still only knows it is an opaque string it hands to `instantiate` (T6 unchanged). `docs/kernel/01-grammar.md` and
`06-surface-elaboration.md` are repaired to say so.

**Two kernel accessors were needed, and both are about ownership, not inspection.** `Term::into_literal` and taking the
term **by value** in `evaluate`/`evaluate_marked`. Both were added after measuring: the first draft cloned every
occurrence twice on the way to a value and cost **+29% allocations on the `large` workload**, which has no sharing to
offset it. `docs/kernel/09-performance.md` records the two corrections and the rule they illustrate. The `evaluate`
signature change is a repair to prompt 45's; callers that still need the term say `evaluate(term.clone())`.

**A third bench workload, not a bigger second one.** `tests/fixtures/large-score.musa` has no `repeat` and no `use`, so
it could not show what this prompt does. `tests/fixtures/shared-score.musa` is the same 1500 notes written as four
motifs repeated 100 times, generated from the same `LINES` table by the same generator, with a test asserting the two
agree on note count. Extending `large-score.musa` instead would have invalidated forty existing rows — the tension
prompt 45 recorded, resolved by adding a column rather than moving one.

**Coalescing adjacent literals was not in the prompt and is not optional.** Without it every note printed as its own
nested `timeline` inside a `sequence`, and the corpus grew ~30% while saying nothing new. With it, files with no reuse
are byte-identical to prompt 48's. `Term::seq` was deliberately *not* changed to coalesce: the printer prints the term
it is given (`01-grammar.md`), so the producer folds and the kernel does not.

**Four levels spend their sharing**, listed in `06-surface-elaboration.md`: a tie crossing an item boundary,
`retrograde`, `invert`, `stretch`, and a `use` with `with { … }` overrides. Each needs a *value*, so it evaluates its
reference — which instantiates the body exactly as direct expansion would have — and the binding it made is pruned when
the piece's term is closed. This is why `examples/kernel/variation.kernel` has one `let` for five `use`s.

**What did not change**: `project.rs`, the projected snapshot (byte-identical, as *Stop* required), `canonical_key`, any
semantic hash, and any golden outside `examples/kernel/`.
