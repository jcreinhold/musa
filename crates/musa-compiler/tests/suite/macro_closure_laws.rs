//! The macro layer as one claim (prompt 160).
//!
//! Musa's macro layer is *one* thing: an adapter is an ordinary total function
//! from syntax to syntax, checked by the checker that checks everything else
//! and run by the evaluator that runs everything else, with typed quotation,
//! splicing, hygiene, and provenance. Idris2 needs two systems and roughly
//! 3,200 lines of Reify/Reflect to say something weaker, and musa needs neither
//! because it committed to a single theory.
//!
//! Eight properties carry that sentence, and every one of them was already
//! lawed when this file was written — in six files, three of them private to
//! the crate. That is the problem this file exists for. **Eight laws each true
//! about its own construct are not the same artifact as one claim you can
//! falsify.** A reader auditing "the macro layer is one thing" had nowhere to
//! start, and a rename could dissolve a row of the argument without failing
//! anything.
//!
//! So: [`CLAIMS`] is the claim, stated as eight properties rather than eight
//! mechanisms, each naming the law that carries it and the law that shows it
//! can fail. [`every_claim_is_carried_by_a_law_that_still_exists`] reads those
//! names out of the crate's own source, which is what keeps the table honest
//! when a law moves. And three of the eight had no control, so this file adds
//! them: expansion's single evaluation entry point, an adapter's inability to
//! read a source range, and the absence of general recursion.
//!
//! # What this file is not
//!
//! It does not re-test the eight. Copying a law next to its citation would give
//! two things to keep in step and prove nothing twice. Where a row cites a law,
//! that law is the evidence and this file is the index; where a row cites a
//! test *here*, the evidence is here because nothing carried it before.

// A law that does not hold is reported by panicking with what actually
// happened, which is more useful than an assertion message alone — and a law
// that reads the crate's own source cannot go on if a file it names is gone.
#![allow(clippy::panic, clippy::expect_used)]

use std::path::{Path, PathBuf};

use musa_compiler::{CompileOptions, ImportSources, SourceDocument, compile, resolve_import};
use musa_score::Severity;

/// One property of the macro layer, and where it is decided.
struct Claim {
    /// The property, in the terms `docs/rules/language/11-quotation.md` states
    /// it — never the mechanism that happens to implement it today.
    property: &'static str,
    /// The law that shows it holds, as `fn` name.
    holds: &'static str,
    /// The law that shows it can fail, as `fn` name. A property nobody has
    /// seen refused is an assertion.
    fails: &'static str,
}

/// The eight, and the whole of the claim.
///
/// The order is the order an adapter meets them: what an adapter *is*, what
/// runs it, what it may write, what it may build, what its names mean, what it
/// must not lose, why it stops, and what ordinary source may not borrow.
const CLAIMS: [Claim; 8] = [
    Claim {
        property: "an adapter is an ordinary declaration in an ordinary library, and nothing else is one",
        holds: "one_region_expands_to_one_ordinary_expression",
        fails: "a_declaration_an_import_or_a_module_is_not_an_expression_and_is_refused",
    },
    Claim {
        property: "expansion is evaluation: one checker, one evaluator, and no second route into either",
        holds: "expansion_reaches_the_core_through_one_pair_of_calls",
        fails: "expansion_reaches_the_core_through_one_pair_of_calls",
    },
    Claim {
        property: "a syntax value carries its category, and a position of one category refuses the other",
        holds: "a_token_tree_position_accepts_a_certified_expression",
        fails: "an_expression_position_refuses_a_tree_nobody_parsed",
    },
    Claim {
        property: "a spliced value arrives where the splice stood, at the category the position wants",
        holds: "a_spliced_value_arrives_where_the_splice_stood",
        fails: "a_splice_of_the_wrong_category_names_both_categories",
    },
    Claim {
        property: "a name a quote writes and a name spliced into it are different names",
        holds: "a_quoted_binder_does_not_capture_a_spliced_name",
        fails: "a_quote_may_not_write_a_name_its_own_hygiene_could_produce",
    },
    Claim {
        property: "a node the composer wrote keeps its own source information through an expansion",
        holds: "a_preserved_input_node_keeps_its_own_source_information",
        fails: "an_adapter_cannot_read_where_a_node_is",
    },
    Claim {
        property: "an adapter terminates, because it is checked by the kernel that checks every other function",
        holds: "expansion_terminates_because_the_bootstrap_is_adapter_free",
        fails: "an_adapter_that_calls_itself_is_refused",
    },
    Claim {
        property: "the phase's vocabulary is unspellable in ordinary source",
        holds: "bare_syntax_names_no_type",
        fails: "a_piece_cannot_write_a_quote_at_all",
    },
];

// ---- the gathering, made checkable ------------------------------------------

/// `crates/musa-compiler/`, from this file.
fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Every Rust line under `src/` and `tests/`, as one string.
///
/// Read as text for `boundary_laws.rs`'s reason: the laws named in [`CLAIMS`]
/// live in three private modules and five suite files, so no `use` reaches
/// them and a call would not compile. What is being checked is that the
/// argument still has eight rows, which is a question about the source.
fn sources() -> String {
    let mut held = String::new();
    for directory in ["src", "tests"] {
        gather(&crate_root().join(directory), &mut held);
    }
    held
}

fn gather(directory: &Path, into: &mut String) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        panic!("`{}` is readable", directory.display());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            gather(&path, into);
        } else if path.extension().is_some_and(|it| it == "rs") {
            let Ok(text) = std::fs::read_to_string(&path) else {
                panic!("`{}` is readable", path.display());
            };
            into.push_str(&text);
            into.push('\n');
        }
    }
}

/// Every row of [`CLAIMS`] names two laws that exist.
///
/// The one law about the table rather than about the language. A row whose
/// `holds` or `fails` has been renamed away is a row of the argument that
/// quietly stopped being evidence, and this is what makes that a failure
/// instead of a comment that aged.
#[test]
fn every_claim_is_carried_by_a_law_that_still_exists() {
    let held = sources();
    for claim in &CLAIMS {
        for name in [claim.holds, claim.fails] {
            assert!(
                held.contains(&format!("fn {name}(")),
                "`{}`: no law named `{name}` — the claim lost its evidence",
                claim.property
            );
        }
    }
}

/// Every claim is a different claim, and every law is cited once.
///
/// Two rows citing one law would make the table look like eight pieces of
/// evidence where there are seven.
#[test]
fn the_eight_claims_are_eight() {
    let mut properties: Vec<&str> = CLAIMS.iter().map(|claim| claim.property).collect();
    properties.sort_unstable();
    let before = properties.len();
    properties.dedup();
    assert_eq!(properties.len(), before, "two rows state one property");

    // `holds` may not repeat; `fails` may equal its own row's `holds` when the
    // law is a source-text law that fails in both directions at once, which is
    // row two and only row two.
    let mut positive: Vec<&str> = CLAIMS.iter().map(|claim| claim.holds).collect();
    positive.sort_unstable();
    let before = positive.len();
    positive.dedup();
    assert_eq!(positive.len(), before, "two rows cite one law");
}

// ---- the control row two is ------------------------------------------------

/// Expansion reaches the core through one pair of calls, and no other.
///
/// The claim `phase/mod.rs` states in prose — "the core this lowers into is
/// `musa-calculus`" — as something that can fail. And the shape it holds in is
/// stronger than the prose: **nothing under `phase/` or `expand/` calls the
/// checker or the evaluator at all.** An adapter module becomes a
/// `crate::document::Document` through `document::elaborate`, and the
/// transformer inside it is run by asking that `Document` for a term —
/// `Document::term_metered`, which is where `musa_calculus::infer_metered` and
/// `musa_calculus::normalize_metered` are called, and which is the same reader
/// an ordinary piece's score is read through. Not "the same algorithm": the
/// same function.
///
/// Stated over the source text rather than over behaviour because that is what
/// the property *is*. A second evaluator added to the phase tomorrow would make
/// no existing test fail, which is exactly why prompt 160 owed this one, and
/// `musa-calculus`'s `boundary_laws.rs` draws its own boundary the same way for
/// the same reason.
///
/// Test modules are read too. A law that let `phase/tests.rs` reach past the
/// boundary would be a boundary the crate's own tests do not believe in.
#[test]
fn expansion_reaches_the_core_through_one_pair_of_calls() {
    // `canonical` is not on this list on purpose: it reads a normal form the
    // caller already has and computes nothing, which is why it is the one
    // `musa_calculus::` verb the phase does name.
    const EVALUATING: [&str; 6] = [
        "musa_calculus::infer",
        "musa_calculus::check(",
        "musa_calculus::normalize",
        "musa_calculus::eval",
        "musa_calculus::recheck",
        "musa_calculus::convertible",
    ];

    for directory in ["src/phase", "src/expand"] {
        let mut held = String::new();
        gather(&crate_root().join(directory), &mut held);
        let reached: Vec<&str> = held
            .lines()
            .map(str::trim)
            .filter(|line| !line.starts_with("//"))
            .filter(|line| EVALUATING.iter().any(|verb| line.contains(verb)))
            .collect();
        assert!(
            reached.is_empty(),
            "`{directory}` reaches the core without going through a `Document`: {reached:?}"
        );
    }

    // The other half, and the one that makes the first half mean something: the
    // reader they all go through evaluates in exactly one place, so "the phase
    // uses the ordinary evaluator" is a fact about one function body.
    let document = std::fs::read_to_string(crate_root().join("src/document.rs")).expect("`document.rs` is readable");
    let normalizing = document
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .filter(|line| line.contains("musa_calculus::normalize"))
        .count();
    assert_eq!(
        normalizing, 2,
        "`Document`'s evaluation moved: `term_metered` and the readback beside it are the two"
    );
    assert!(
        document.contains("musa_calculus::normalize_metered(&self.cx, &ty, &term)"),
        "`Document::term_metered` no longer normalizes what it inferred"
    );
}

// ---- the two controls rows six and seven were missing -----------------------

/// Where the probe adapter is imported from, as the piece writes it.
const PROBE: &str = "probe::adapter";

/// A readable adapter module holding `body`.
fn probe(body: &str) -> String {
    format!("\n    let level = \"readable\";\n{body}\n\n")
}

/// Every error compiling a one-region piece against `module`, whole.
fn errors(module: &str) -> Vec<String> {
    let source = SourceDocument::new(
        format!(
            "piece \"probe\" {{\n    import syntax {PROBE} as probe;\n\n    let held = syntax probe {{ 1 }};\n\n    \
             score {{ part p {{ voice v {{ c4/1 }} }} }}\n}}\n"
        ),
        "probe.musa",
    );
    let mut imports = ImportSources::default();
    imports.insert(resolve_import("probe.musa", PROBE), module.to_owned());
    compile(
        &source,
        &CompileOptions {
            imports,
            ..CompileOptions::default()
        },
    )
    .diagnostics()
    .iter()
    .filter(|diagnostic| diagnostic.severity == Severity::Error)
    .flat_map(|diagnostic| {
        std::iter::once(diagnostic.message.clone()).chain(diagnostic.causes.iter().map(|cause| cause.message.clone()))
    })
    .collect()
}

/// An adapter cannot read where a node is, so it cannot fabricate provenance.
///
/// Row six's control, and it is a control about *vocabulary* rather than about
/// an answer: `SourceInfo` has no eliminator and the phase has no operation
/// taking a node to a range, so an adapter that tries to ask has written a
/// name. That is the whole mechanism behind "provenance survives expansion" —
/// a node's source information can only be carried, never read and never
/// written, so carrying it is the only thing an adapter can do with it.
///
/// Written as three spellings a reader might reasonably try. If any of them
/// ever resolves, the property is gone and the Origin view is reading a range
/// an adapter chose.
#[test]
fn an_adapter_cannot_read_where_a_node_is() {
    for spelling in ["syntax_span", "syntax_range", "syntax_source_info"] {
        let module = probe(&format!(
            "
    let expand = fn (region: Syntax(TokenTree)) -> Result(Syntax(TokenTree), Pair(Syntax(TokenTree), Text)) {{
        Err((region, {spelling}(region)))
    }};
"
        ));
        let found = errors(&module);
        assert!(
            found
                .iter()
                .any(|error| error.contains(&format!("cannot find `{spelling}`"))),
            "`{spelling}` resolved to something: an adapter can read a place {found:?}"
        );
    }
}

/// An adapter's `expand` cannot call itself.
///
/// Row seven's control, and the point of it is *which* check refuses this.
/// There is no termination check on adapters. There is one on definitions, an
/// adapter is a definition, and the sentence it gets back — "`expand` calls
/// itself on something this checker cannot see decrease" — is the one any
/// recursive function in the language gets. That is the whole claim: totality
/// is not something the phase arranges, it is something the phase inherits.
///
/// And it is why the phase can afford the ordinary evaluator instead of a
/// sandboxed one, so it is worth a law rather than a sentence.
#[test]
fn an_adapter_that_calls_itself_is_refused() {
    let module = probe(
        "
    fn expand(region: Syntax(TokenTree)) -> Result(Syntax(TokenTree), Pair(Syntax(TokenTree), Text)) {
        expand(region)
    }
",
    );
    let found = errors(&module);
    assert!(
        found
            .iter()
            .any(|error| error.contains("`expand` calls itself") && error.contains("`match` took")),
        "an adapter called itself and was not refused by the termination check: {found:?}"
    );
}
