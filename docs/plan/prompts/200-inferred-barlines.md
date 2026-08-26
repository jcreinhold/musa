---
id: 200
slug: inferred-barlines
status: pending
depends_on: [90, 193]
phase: 3
---

# Insert Proven Bar Lines

## Task

Give an unbarred, measured passage an explicit source action that inserts the `|` bar syntax wherever the compiler can
prove a measure boundary. Make it available as `musa format --insert-bars`, as an LSP refactor, and from the desktop
command palette. Ordinary formatting remains lossless and never adds a significant token; the opt-in action is a
semantic source rewrite whose added bars assert facts the compiler has already established.

## Read

- [`docs/plan/roadmap.md`](../roadmap.md) §7.2, especially “Bars are written down, and checked”; §10.6–10.7 on the
  compiler pipeline and incrementality; §11 on syntax-aware source edits and canonical source; §14.6 on score commands;
  and §17.3 on the ordinary formatter laws.
- [`docs/rules/events/03-denotational-semantics.md`](../../rules/events/03-denotational-semantics.md) and
  [`06-surface-elaboration.md`](../../rules/events/06-surface-elaboration.md): sequential duration is exact ambient
  written time, while a surface bar is an assertion and not a second event-track form.
- [`docs/rules/language/05-verification.md`](../../rules/language/05-verification.md): bar claims are discharged only
  after elaboration has settled their exact contents and the meter in force.
- Prompts 61, 74, 75, 89, and 90: the one `BarLines` answer, deliberately unmeasured passages, polymetric scope, `|` as
  `BarStmt`, and the syntax-only formatter boundary. Prompt 25 owns transactional structured source edits and prompt 77
  owns the LSP shell.
- Open Music Theory, chapters 010–012: a measure is one complete metrical group; simple and compound division alter
  internal grouping, not the criterion for the next bar line; tuplets can cross visible subdivisions without ceasing to
  have exact duration. Take that distinction, not a restriction to common-practice meters.

## Design

### An explicit semantic rewrite, not a new default formatter

`musa format` continues to rewrite layout only and keeps its significant-token preservation law. The opt-in
`--insert-bars` mode first asks the project session for a proved source rewrite and then runs the existing formatter on
the resulting CST. The LSP advertises the same operation as `refactor.rewrite.insertBarlines`; the desktop command
palette calls the same session operation. None of the three shells derives durations or edits independently.

This distinction is visible because adding `|` adds a `BarStmt` and therefore a checked claim. It is safe when proved,
but it is not merely whitespace. The operation is undoable and transactional like every structured edit; preview queries
never mutate session history.

### Reuse the duration and barline answers already computed

Add one compiler-owned projection for source rewriting: for each direct item in a bar-admitting source sequence, expose
its source insertion boundary, exact written-time start and end, and the scoped `BarLines` decision already used by
verification and notation. It is a projection of a successful checked document, not a second public musical model and
not a CST duration evaluator. The project layer joins those facts back to the lossless CST and produces text edits.

For each maximal run of direct, authored items not already inside a `BarStmt`, insert `|` before the first item and at
each later item whose exact onset is a measure boundary. A run is admissible only when all of these hold:

- a measured meter is in force in that item’s part/voice scope;
- the run begins and ends on that scope’s bar lines;
- every interior bar line coincides with a direct source-item boundary;
- each proposed bar contains exactly one measure after the edit; and
- the insertion point belongs to the authored sequence itself, not to the expansion of a `use`, transformation, adapter
  region, or other generated value.

An item whose checked duration is exactly one or more measures is admissible only when every required boundary is
already a source boundary. The action never opens an item or copies expanded notes back into source. Existing anonymous
and named bars are fixed boundaries and remain byte-for-byte the assertions the author wrote; eligible loose runs on
either side may still be barred.

### Refusal is one result, not partial guessing

The whole requested scope either has a certain edit or has none. Return structured blockers naming the first relevant
source span and reason: unmeasured scope; run begins or ends between bar lines; an item crosses a required boundary;
generated material hides a boundary; or the document is not currently valid. `meter none` is intentionally unbarred, not
an error, and is skipped when other measured scopes can be rewritten. If no measured loose run needs an edit, the result
is “already barred” rather than an empty successful transaction.

Do not invent pickups. Musa has no source construct for an uncounted measure, and turning a short opening run into bar 1
would silently renumber everything after it. Do not split a note or synthesize ties: that is a separate notation choice,
not evidence about where the author intended an assertion.

### One edit plan at every surface

The project facade owns a query returning the proposed edits plus a short musical summary, and a command that applies
that exact plan against the revision it was computed from. Stale revision means recompute or refuse, never apply old
byte offsets. The CLI, LSP, and desktop translate this result only:

- `musa format --insert-bars [--check|--diff]` composes insertion with canonical formatting while preserving the
  existing meanings of `--check` and `--diff`;
- the LSP offers one `CodeActionKind::REFACTOR_REWRITE` action for a valid document only when a certain edit exists,
  returning workspace edits without touching its session history; and
- the desktop palette item previews the count (“Insert 12 bar lines in 12 measures”), applies one undoable command, and
  reports blockers in the existing result surface.

## Target

- `crates/musa-compiler`: a narrow checked-document projection of direct source-item extents against the already
  computed scoped bar lines; no new duration evaluator and no public pass type.
- `crates/musa-project`: `BarlineRewrite`/blocker facts, preview and transactional apply through one session facade,
  stale-revision protection, and generated DTO coverage.
- `crates/musa`: `musa format --insert-bars`, including `--check` and `--diff` behavior and CLI diagnostic snapshots.
- `crates/musa-lsp`: the `refactor.rewrite.insertBarlines` code action backed only by the project preview.
- `apps/musa-desktop/ui`: one command-map/palette entry backed by the generated session command; no local inference.
- Tests covering literal notes, rests and chords; a whole-bar `use`; a boundary hidden inside a multi-bar expansion;
  tuplets; meter changes; polymeter; mixed existing/loose bars; `meter none`; an incomplete opening and closing run;
  idempotence; stale revisions; undo; UTF-16 LSP ranges; and equality of CLI, LSP, and project-produced text.
- One handbook example showing an unbarred passage before the action, the inserted `|` form after it, and a refusal
  whose message explains the boundary-crossing item.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa -p musa-lsp
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check
make docs-check
cd apps/musa-desktop/ui && pnpm check && pnpm test:unit
```

Behavior checks prove that applying the action and applying its returned edits are the same transaction; a second
application is “already barred”; removing the inserted pipes returns the same event-track semantics; every inserted bar
passes the existing bar-duration verifier; ordinary `musa format` preserves the original significant-token sequence; and
no shell contains duration accumulation or meter inference.

Commit as `Add proved barline insertion`.

## Stop

- No automatic bar insertion in ordinary formatting, on save, or while typing.
- No pickup/anacrusis syntax or implicit-measure numbering; that needs its own governing design.
- No note splitting, synthesized rests, ties, or duration respelling to manufacture a boundary.
- No editing inside motif definitions or generated expansions merely because an occurrence crosses a bar line.
- No unmeasured-to-measured conversion and no guessed default meter.
- No cross-voice alignment rewrite: each part/voice uses the `BarLines` scope already governing it, including polymeter.
