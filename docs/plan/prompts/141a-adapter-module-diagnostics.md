---
id: 141a
slug: adapter-module-diagnostics
status: done
depends_on: [140]
phase: 3
---

# Carry an Adapter Module's Own Diagnostics to Its Author

## Task

A diagnostic the checker produces while reading an adapter module is currently flattened to one string. Give
`Diagnostic` a channel for diagnostics that are about *another document*, carry every one of the module's own through it
whole — code, message, labels, note, help — and show them in all three renderers. Then delete the workarounds the
flattening forced: the message-only law suites at 139 and 140, and the comment in `core.rs` that tells a future author
to write distinctions into messages because notes do not survive.

## Read

- [`crates/musa-compiler/src/expand/mod.rs`](../../../crates/musa-compiler/src/expand/mod.rs) — `LevelFault` (the type
  with three fields and a stated reason for having no span), `level_of`, and its three callers: the import check in
  `expand`, `adapter_edit`, and `adapter_print`. The `ModuleFault::Broken` arm keeps `diagnostics.first().message` and
  substitutes its own help; the rest of the first diagnostic and all of every later one are dropped there.
- [`crates/musa-compiler/src/phase/mod.rs`](../../../crates/musa-compiler/src/phase/mod.rs) — `read_adapter_module` and
  its doc comment, which is the constraint this prompt has to keep: "the diagnostics come back rather than being
  reported: they are about the adapter package's own document, and publishing a span inside it as a span in the
  composer's file is exactly what the source map exists to prevent." Also `read_adapter_module_metered`, which returns
  `resolver. diagnostics` — a vector, routinely longer than one.
- The six codes that can only be raised inside an adapter module, because `Syntax<Cat>`, `quote at here { … }`, and
  quote patterns appear nowhere else: `UnspreadSequence`, `AmbiguousSpread`, `PatternCategory`, `QuotedLiteralName`,
  `QuotedCapture`, `SpliceCategory`. Every one of them was written with a note and a help. Today no one has ever read
  them.
- [`crates/musa-score/src/diagnose.rs`](../../../crates/musa-score/src/diagnose.rs) — `Diagnostic`, its seven fields,
  and `remap_spans`. **There is no related-diagnostics channel**; `labels` is the only place a second span can go, and
  every span in it is a span in the composer's file.
- [`crates/musa-project/src/diagnostic.rs`](../../../crates/musa-project/src/diagnostic.rs) and
  [`position.rs`](../../../crates/musa-project/src/position.rs) — the restatement, and the layering rule this prompt
  keeps: the compiler measures bytes, the session counts lines, and a `Position` is "never derived by the frontend".
  `Diagnostic::from_compiler` is handed one `Lines` over the open document.
- [`crates/musa-project/src/session.rs`](../../../crates/musa-project/src/session.rs) — the session already holds
  `imports: musa_compiler::ImportSources`, the text of every transitively imported file keyed by resolved path, and
  `crate::imports::closure` collects `import syntax` statements along with ordinary ones. **The session already has the
  adapter's text.** That is what makes this prompt small.
- [`crates/musa/src/main.rs`](../../../crates/musa/src/main.rs) `CliDiagnostic` and `report`, and
  `musa_project::explain`. `explain` answers about a *code*, not a diagnostic, so it is not the channel — a cause needs
  to arrive with the diagnostic, not be looked up after it. miette's `related()` is the mechanism, and a related report
  carries its own `source_code()`, which is how a second document gets a real caret in the terminal.
- [`crates/musa-lsp/src/features/diagnostics.rs`](../../../crates/musa-lsp/src/features/diagnostics.rs) — secondary
  labels already become `DiagnosticRelatedInformation`, and a `Location` already carries a URI. The only reason it is
  always this document's URI is that nothing upstream can say otherwise.
- [`docs/rules/desktop/05-states.md`](../../rules/desktop/05-states.md) §5 — the governing account of what a diagnostic
  is on screen, "at most four lines", location in `--t-value` `--ink-muted`, and diagnostics shown in the compiler's own
  words. It does not cover a diagnostic about a document the composer did not open, and this prompt repairs it.
  `docs/rules/README.md`: pages outside the constitution are "amendable in the ordinary way — a prompt that repairs
  them, committed before the code changes."
- [`crates/musa-compiler/tests/suite/quotation_laws.rs`](../../../crates/musa-compiler/tests/suite/quotation_laws.rs)
  and [`syntax_pattern_laws.rs`](../../../crates/musa-compiler/tests/suite/syntax_pattern_laws.rs) — the two `errors`
  helpers and, in the second, the doc comment that states the workaround outright: "a law about what an adapter author
  is told is a law about the message, and every distinction one of these tests observes is written there rather than in
  a note."

## Design

**The defect is a contract, not a bug.** `level_of` is doing exactly what the code around it says to do: a `LevelFault`
is a message, a help, and a code, because the fault gets restated at whichever of the *caller's* spans is right. That is
correct for the fault "this module does not promise what it claims". It is wrong for the fault "this module does not
check", which is not one sentence about the import — it is a whole set of diagnostics about a different file, each with
its own place in it. So the fix is to give the compiler somewhere to put them, not to make the string longer.

**A `Cause` is a diagnostic about another document.**

```rust
/// A diagnostic about a document other than the one being compiled.
///
/// Spans in `labels` are spans in `document` and in no other file. That is
/// what keeps [`Diagnostic::remap_spans`] correct without a runtime check:
/// the source map moves the composer's own text, and a cause is not in it.
pub struct Cause {
    /// The document, by the key the import resolved to.
    pub document: String,
    pub code: Code,
    pub message: String,
    /// Where, in `document`. Primary first, as in a diagnostic.
    pub labels: Vec<Label>,
    pub help: Option<String>,
    pub note: Option<String>,
}
```

`Diagnostic` gains `pub causes: Vec<Cause>` and a `caused_by` builder. Three properties, each with a reason and each
tested:

- **A cause has no fixes.** `remap_spans` already argues it: "an edit offered against generated text would silently
  rewrite a file the composer cannot see, which is worse than offering no fix at all." A file the composer did not write
  is the same argument with a shorter path.
- **`remap_spans` does not descend into `causes`.** A cause's spans are already in the document they belong to. The law
  is that remapping a diagnostic leaves its causes byte-identical.
- **A cause holds no causes.** One level, because the only producer is a module read, and a module cannot import.

**`level_of` stops summarizing.** The `Broken` arm keeps the wrapper's own sentence — the module is named, the import is
where the caret goes, the help still says what an adapter module is — and attaches every diagnostic
`read_adapter_module` returned as a cause under the resolved import key. Nothing is spliced into the message, because
the causes now say it in full and saying it twice is what §5 of the desktop specification forbids of a label. The
`Stopped` arm is unchanged: a read that ran out of budget said nothing about the module, so there is nothing to carry.

`LevelFault` grows `causes: Vec<Cause>` and its doc comment gains the second half of its argument — no span, because
which of the caller's spans is right is the caller's question; *and* causes, because which document the module's own
faults are in is not a question at all. All three callers attach them: the import check in `expand`, and the `refusal`
and `broken` paths in `adapter_edit` and `adapter_print`.

**The session resolves the line and column, because it is the one that holds the text.** `Diagnostic::from_compiler`
takes the import sources beside the open document's `Lines`, and a cause's labels arrive already carrying `Position`s in
*their* document. The frontend still derives nothing. A cause whose document is not in the map — possible only if a
compilation was handed sources the session was not — carries its label texts with no positions rather than being
dropped.

The session-side type is deliberately not `Label`: a `musa_project::Label` documents its `span` as "a byte range in the
current source text", and a byte range into a file the frontend does not hold is a footgun with no use. A `CauseLabel`
carries `at` and `to` as `Position`s and no span at all, so there is nothing to misuse.

**Three renderers, and each already has the mechanism.**

- **CLI** — each cause becomes a miette `related()` report with its own `NamedSource` over the adapter's text, so the
  terminal shows the caret in the adapter file under the diagnostic about the import. Its note and fixes footer follow
  the same shape `report` already uses for the primary.
- **LSP** — each cause's labels become `DiagnosticRelatedInformation` at the cause's document URI when the resolved key
  is a real path, with the cause's message, note, and help folded into the related message the way help is already
  folded into the primary's. A key with no file behind it — a `std::` virtual URI — folds into the primary message
  instead of being dropped, because a composer who cannot click through still has to be told.
- **Desktop** — causes list under their diagnostic, indented, each with the document's name where the location goes and
  the position within it beside it. They are not clickable and offer no fix: the composer cannot edit that file, which
  `08-elaboration.md` already establishes as the shape for text a composer does not own.

**The desktop specification is repaired first, in the same commit.** `05-states.md` §5's "at most four lines" is a rule
about one diagnostic; a diagnostic with causes is that plus one entry per cause. Write the rule where the rest of §5's
rules are, including that a cause is not a navigation target and carries no fix control, and why: the compiler's own
words about a file the interface will not let them edit.

**Then the workarounds go.** Both `errors` helpers read the whole document each diagnostic is, causes included, so a law
can observe a note again. `syntax_pattern_laws.rs`'s helper doc comment loses its explanation of the constraint and
gains a sentence about what it now reads. The comment in `core.rs`'s `quote_splice` loses its claim that a distinction
in a note is a distinction no adapter author reads — the two `UnspreadSequence` messages stay distinct, because they are
two different faults and a message should say which, but they stay for that reason and not for this one.

**One new law in each suite, because relaxing an assertion is not evidence.** A module whose check produces two
diagnostics reaches its author as two, at their own places, with both notes — the observation the `.first()` call made
impossible.

## Target

- `musa-compiler`: `Cause`, `Diagnostic::causes`, `caused_by`, the `remap_spans` law, and `LevelFault::causes` carried
  through all three `level_of` callers. `Cause` is exported beside `Diagnostic` from the crate root.
- `crates/musa-compiler/tests/suite/`: the remapping law, the no-fixes law, and a case proving every diagnostic of a
  multi-fault module arrives.
- `musa-project`: `Cause`, `CauseLabel`, `Diagnostic::causes`, and `from_compiler` reading positions out of the import
  sources; serde shape for the UI.
- `musa`: `related()` on `CliDiagnostic`, with the adapter's source attached, and the note/fix footer per cause.
- `musa-lsp`: related information at the cause's URI, and the fold-into-message fallback for a document with no path.
- `apps/musa-desktop/ui`: causes under their diagnostic in the problems list, per the repaired §5.
- `docs/rules/desktop/05-states.md` §5 — the rule for a diagnostic about another document.
- `crates/musa-compiler/tests/suite/quotation_laws.rs` and `syntax_pattern_laws.rs`: both `errors` helpers read causes,
  both helper doc comments repaired, and one new law per suite on a two-diagnostic module.
- `crates/musa-compiler/src/phase/mod.rs`: the `quote_splice` comment repaired.
- `docs/plan/code-map/` rows for `musa-compiler`, `musa-project`, `musa-lsp`, and the desktop.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-compiler -p musa-project -p musa-lsp -p musa
cargo nextest run --workspace
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa-lsp -p musa -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
PATH=/Users/jcreinhold/.cargo/bin:$PATH make lint-ui
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

A behavior check the suite cannot make: compile a piece against an adapter module with a `$..xs` in a leaf position and
read the terminal. The note and the help that prompt 140 wrote must be on screen, under a caret in the adapter's file.

Commit as `Carry an adapter module's own diagnostics to its author`.

## Stop

- **No second span space in `labels`.** A label on a `Diagnostic` is a place in the composer's file, exactly as today.
  Causes are the whole of the multi-document story, and a diagnostic that wants to point at two files uses them.
- No navigation into another document from the desktop, no read-only package viewer, and no editing of a file the
  session does not own. A cause says where in words.
- No fix, quick-fix, or code action on a cause, in any renderer.
- No change to the wording of any diagnostic the checker raises, beyond `quote_splice`'s comment. Wording is prompt
  165's; this prompt is delivery.
- No `ModuleFault::Stopped` change, and no attempt to report partial results from a read that crossed a limit.
- No use of `causes` for ordinary imports. `Code::Import`'s "`{path}` does not compile" has the same shape and would
  benefit, but a library is resolved on a different path with a different failure type, and folding both into one prompt
  would make the adapter contract wait on the import contract. Name it in `docs/notes/` as the obvious next application
  and leave it.
- No constitution, obligations, event track, or `docs/rules/language/` change. This changes how a compiler failure is
  delivered, not what the language admits.
