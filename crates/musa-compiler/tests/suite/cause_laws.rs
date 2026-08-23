//! What a **cause** is, from outside the compiler (prompt 141a).
//!
//! A diagnostic the checker raises while reading an adapter module is about a
//! file the composer did not write and cannot open. It arrives as a `Cause` of
//! the diagnostic about the `import syntax` statement, and the three laws here
//! are the three properties that make that safe to carry:
//!
//! - **Remapping does not touch a cause.** Expansion rewrites the composer's
//!   text and every span in the composer's file moves with it. A cause's spans
//!   are in another document, which is not in that map, so they stay exactly
//!   where they were.
//!   ([`remapping_a_diagnostic_leaves_its_causes_where_they_were`])
//! - **A cause offers no fix**, because its spans index a file this compiler
//!   will not let anyone edit — the same argument `remap_spans` already makes
//!   about generated text, with a shorter path.
//!   ([`a_cause_offers_no_fix_and_its_labels_index_its_own_document`])
//! - **Every fault arrives.** A module whose check produces two diagnostics
//!   reaches its author as two, at their own places, with both notes.
//!   ([`every_diagnostic_of_a_module_with_two_faults_arrives`])
//!
//! The observations are spans cut out of real text: a label is only in the
//! right document if slicing that document at it yields the words the label is
//! about.

// A law that does not hold is reported by panicking with what actually
// happened, which is more useful than an assertion message alone.
#![allow(clippy::panic)]

use musa_compiler::{Compilation, CompileOptions, ImportSources, SourceDocument, compile, resolve_import};

use musa_score::{Cause, Diagnostic, Severity, SourceSpan};

/// The module with the fault in it, as the piece writes the import.
const BROKEN: &str = "probe::broken";
/// A module that checks, so that a region can actually expand.
const SOUND: &str = "probe::sound";

/// An adapter module that folds every node of its region to `emit`'s answer.
///
/// The fold is here for the same reason it is in the quotation laws: a quote
/// needs a `NodePath`, and the only place one comes from is a traversal.
fn module(emit: &str) -> String {
    format!(
        "library {{
    let level = \"readable\";
{emit}
    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> {{ Ok(forget(built(region))) }};

    let built = fn (region: Syntax<TokenTree>) -> Syntax<Expr> {{
        syntax_fold_from_leaves(
            fn (here) {{ emit(here, []) }},
            fn (here, kind, text) {{ emit(here, []) }},
            fn (here, name) {{ emit(here, []) }},
            fn (here, delimiter, children) {{ emit(here, children) }},
            region,
        )
    }};
}}
"
    )
}

/// `$..kids` in a position that holds one node — §7's spread refusal, and the
/// one fault the two single-fault laws below are about.
fn unspread() -> String {
    module(
        r"
    let emit = fn (here: NodePath, kids: List<Syntax<Expr>>) -> Syntax<Expr> {
        quote at here { $..kids }
    };
",
    )
}

/// An adapter that checks and answers `1` for any region.
fn sound() -> String {
    module(
        r"
    let emit = fn (here: NodePath, kids: List<Syntax<Expr>>) -> Syntax<Expr> {
        quote at here { 1 }
    };
",
    )
}

/// A piece importing both modules, with `body` between the imports and the
/// score.
fn piece(body: &str) -> String {
    format!(
        "piece \"probe\" {{\n    import syntax {BROKEN} as broken;\n    import syntax {SOUND} as sound;\n\n{body}    \
         score {{ part p {{ voice v {{ c4/1 }} }} }}\n}}\n"
    )
}

/// That piece compiled against the broken module and the sound one.
fn compiled(text: &str, broken: &str) -> Compilation {
    let source = SourceDocument::new(text.to_owned(), "probe.musa");
    let mut imports = ImportSources::default();
    imports.insert(resolve_import("probe.musa", BROKEN), broken.to_owned());
    imports.insert(resolve_import("probe.musa", SOUND), sound());
    compile(
        &source,
        &CompileOptions {
            imports,
            ..CompileOptions::default()
        },
    )
}

/// Every error's causes, in the order they were reported.
fn causes(compilation: &Compilation) -> Vec<Cause> {
    errors(compilation)
        .flat_map(|diagnostic| diagnostic.causes.iter().cloned())
        .collect()
}

fn errors(compilation: &Compilation) -> impl Iterator<Item = &Diagnostic> {
    compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
}

/// What `span` selects from `text`, or `None` when it does not fit — which is
/// itself an answer: a span from another document need not land inside this
/// one at all.
fn cut(text: &str, span: SourceSpan) -> Option<&str> {
    let start = usize::try_from(span.start).ok()?;
    let end = usize::try_from(span.end).ok()?;
    text.get(start..end)
}

#[test]
fn remapping_a_diagnostic_leaves_its_causes_where_they_were() {
    // The same broken import, read twice: once in a piece with no region, so
    // the source map is the identity and nothing is remapped, and once in a
    // piece whose region expands, so every span in the composer's file moves.
    // The causes must be byte-identical across the two, because the document
    // they are spans in is not the document the map is about.
    let quiet = compiled(&piece(""), &unspread());
    let region = piece("    let held: Nat = syntax sound { x };\n    let after: Nat = nowhere;\n");
    let expanded = compiled(&region, &unspread());

    let untouched = causes(&quiet);
    assert!(!untouched.is_empty(), "the broken module reported nothing at all");
    assert_eq!(
        untouched,
        causes(&expanded),
        "an expansion moved the spans of a fault in another file"
    );

    // And the map was live, so the equality above is evidence rather than two
    // identity maps agreeing: `nowhere` stands after a region that expands to
    // one character, so a span left unmapped would cut the wrong text.
    assert!(
        errors(&expanded).any(|diagnostic| diagnostic
            .labels
            .iter()
            .any(|label| cut(&region, label.span) == Some("nowhere"))),
        "nothing in the composer's own file was reported at its own place, so the map was never exercised"
    );
}

#[test]
fn a_cause_offers_no_fix_and_its_labels_index_its_own_document() {
    // A cause has no `fixes` field, and this is the reason: its spans are
    // spans in the adapter module. Cutting the module's own text at one yields
    // the words the label is about; cutting the piece at the same numbers
    // yields something else entirely. An edit offered against those numbers
    // would rewrite whichever file happened to be open.
    let broken = unspread();
    let text = piece("");
    let compilation = compiled(&text, &broken);

    let carrying: Vec<&Diagnostic> = errors(&compilation)
        .filter(|diagnostic| !diagnostic.causes.is_empty())
        .collect();
    assert!(!carrying.is_empty(), "no diagnostic carried the module's own faults");
    for diagnostic in carrying {
        assert!(
            diagnostic.fixes.is_empty(),
            "a diagnostic about a fault in another file offered an edit: {:?}",
            diagnostic.fixes
        );
        for cause in &diagnostic.causes {
            assert_eq!(
                cause.document,
                resolve_import("probe.musa", BROKEN),
                "a cause named a document other than the module that was read"
            );
            for label in &cause.labels {
                assert!(
                    cut(&broken, label.span).is_some(),
                    "a cause's label is not a span in the module it is about: {label:?}"
                );
            }
        }
    }

    // And the spans are the module's own numbers, not numbers that happen to
    // fit: the refusal is at the spread, and the same numbers read against the
    // piece do not select it.
    let found = causes(&compilation);
    let spread = found
        .iter()
        .find(|cause| cause.message.contains("spreads a sequence"))
        .unwrap_or_else(|| panic!("the module's own refusal did not arrive: {found:?}"));
    let at = spread
        .labels
        .first()
        .unwrap_or_else(|| panic!("the refusal arrived without its place: {spread:?}"));
    assert_eq!(
        cut(&broken, at.span),
        Some("$..kids"),
        "a cause's label does not select what it is about in the module"
    );
    assert_ne!(
        cut(&text, at.span),
        Some("$..kids"),
        "the two documents agree at that span, so this proves nothing"
    );
}

#[test]
fn every_diagnostic_of_a_module_with_two_faults_arrives() {
    // The observation the flattening made impossible. Two unrelated faults in
    // one module — a spread where one node stands, and a quoted name the
    // printer's own hygiene could produce — reach the author as two causes, at
    // two places, each with the note that says why.
    let broken = module(
        r"
    let emit = fn (here: NodePath, kids: List<Syntax<Expr>>) -> Syntax<Expr> {
        quote at here { $..kids }
    };

    let also = fn (here: NodePath) -> Syntax<Expr> {
        quote at here { (fn (held: Nat) { held })(held_g0) }
    };
",
    );
    let found = causes(&compiled(&piece(""), &broken));

    let spread = found
        .iter()
        .find(|cause| cause.message.contains("spreads a sequence"))
        .unwrap_or_else(|| panic!("the spread refusal did not arrive: {found:?}"));
    let hygiene = found
        .iter()
        .find(|cause| cause.message.contains("could generate"))
        .unwrap_or_else(|| panic!("the hygiene refusal did not arrive: {found:?}"));

    assert!(
        spread.note.is_some() && hygiene.note.is_some(),
        "a cause arrived without the note that says why: {found:?}"
    );
    assert_ne!(
        spread.labels.first().map(|label| label.span),
        hygiene.labels.first().map(|label| label.span),
        "two faults were reported at one place"
    );
}
