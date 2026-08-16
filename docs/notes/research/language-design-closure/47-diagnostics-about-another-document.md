# 47 — Diagnostics about another document

**Status: research. Governs nothing.** Written while implementing prompt 141a. It records what a `Cause` turned out to
be, and names the one place the same shape is already waiting and was deliberately left alone.

## 1. What was wrong

`import syntax std::adapters::staff as staff;` names a module the composer did not write. When that module does not
check, the compiler has a set of diagnostics about *it* — each with its own code, its own span inside that file, its own
note and its own help — and exactly one place to put them: a `LevelFault`, which is a message, a help, and a code.

`LevelFault` has no span for a good reason, written in its own doc comment: the fault gets restated at whichever of the
caller's spans is right, and the caller knows which. That reasoning is correct for the fault *this module does not
promise what it claims*. It is wrong for the fault *this module does not check*, which is not one sentence about the
import at all. `level_of`'s `Broken` arm therefore kept `diagnostics.first().message` and dropped everything else: the
rest of the first diagnostic, and all of every later one.

The visible cost was six diagnostic codes — `UnspreadSequence`, `AmbiguousSpread`, `PatternCategory`,
`QuotedLiteralName`, `QuotedCapture`, `SpliceCategory` — that can only be raised inside an adapter module, every one of
them written with a note and a help that no human had ever read. The invisible cost was worse, and it is why this is a
research note rather than a bug report: the constraint had started to shape the diagnostics themselves. `quote_splice`
carried a comment instructing future authors to write distinctions into the *message*, because a distinction in a note
was a distinction nobody would ever see. A delivery defect had become a wording rule.

## 2. What a cause is

A `Cause` is a diagnostic about a document other than the one being compiled: a document key, a code, a message, labels,
a note, and a help. `Diagnostic` gains `causes: Vec<Cause>`, and the `Broken` arm attaches every diagnostic the module
read produced, whole, under the resolved import key. Nothing is spliced into the wrapper's message, because the causes
say it in full.

Three properties hold, and each has a reason rather than a convention:

- **Spans in a cause's `labels` are spans in `cause.document` and in no other file.** This is what keeps `remap_spans`
  correct without a runtime check: the source map moves the composer's own text, and a cause is not in it. The law is
  that remapping a diagnostic leaves its causes byte-identical.
- **A cause has no fixes.** `remap_spans` already argues this for generated text — "an edit offered against generated
  text would silently rewrite a file the composer cannot see" — and a file the composer did not write is the same
  argument with a shorter path.
- **A cause holds no causes.** One level, because the only producer is a module read and a module cannot import.

Every renderer already had the mechanism and only lacked something to render. miette's `related()` carries its own
`source_code()`, so the CLI gets a real caret in the adapter's file. The LSP's `DiagnosticRelatedInformation` already
carries a `Location` with a URI; the only reason it was always this document's URI is that nothing upstream could say
otherwise. The desktop lists causes under their diagnostic, with the document's name where the location goes.

## 3. What made it small

The session already holds the text. `ImportSources` keys the text of every transitively imported file by resolved path,
and `crate::imports::closure` collects `import syntax` statements along with ordinary ones — so
`Diagnostic::from_compiler` can resolve a cause's line and column out of the map it already has, and the layering rule
survives untouched: the compiler measures bytes, the session counts lines, and a `Position` is still never derived by
the frontend.

The alternative designs were both worse in a way worth recording. Embedding the adapter's text in the serialized cause
would ship an 80 KB module to the UI through the serde shape, per diagnostic. Having the CLI re-read the file from disk
would duplicate work the session already did, and would fail outright for a bundled `std::` module, which has no file.

## 4. The obvious next application, and why it is not here

`Code::Import`'s `` `{path}` does not compile `` ([`imports.rs:392`](../../../../crates/musa-compiler/src/imports.rs))
has exactly the shape a cause is for. It reports one parse error out of a library, folded into a note as
`it says: {message}`, and sends the reader to `musa check {path}` — which is to say it tells them to run a second
compilation to see what this one already knew. `core.rs:251` raises the same code with the same summary for a library
that parses and does not check.

Causes would fit it without any new machinery. It is left alone anyway, and the reason is a boundary rather than a
budget: a library is resolved on a different path (`imports::closure`, walking a graph and detecting cycles) with a
different failure type, and folding both into one prompt would make the adapter contract wait on the import contract.
Two things a later prompt would have to decide, which the adapter case did not raise:

- **A library can import.** The one-level property above holds because a module cannot; a library graph can nest
  arbitrarily, so a prompt that carries library faults has to decide whether causes nest, flatten with the path spelled
  out, or stop at one hop.
- **A library's diagnostics are the same compiler's, at the same stage.** An adapter module is read by a separate entry
  point that hands its diagnostics back rather than reporting them. Library resolution reports as it goes, so the fix
  there is about *where* a diagnostic is routed rather than about what a fault can carry.

Neither is hard. Both are decisions, and they belong to a prompt whose Task is the import contract.
