//! What `quote { … }` means as a pattern (prompt 140).
//!
//! `docs/rules/language/11-quotation.md` §4 is the specification, and every law
//! here is a law about a program an *adapter author* writes: a probe adapter is
//! handed to the compiler the way any package adapter is, and what the compiler
//! says about the piece that imports it is the law's answer.
//!
//! **The observations are types and refusals, never printed text.** Whether an
//! arm was selected is observed by giving the arms differently typed answers
//! and annotating the binding the region expands into, so a pattern that
//! matched the wrong shape is a type error in the piece rather than a string
//! comparison here. Each law that could pass vacuously carries the program that
//! must fail beside the one that must not.
//!
//! Six laws:
//!
//! - **Match after build is the identity.** A pattern written like a quote
//!   binds what that quote spliced, in the order it spliced it.
//!   ([`match_after_build_binds_what_the_quote_spliced`])
//! - **Only splices bind.** Every other word is matched as written, and a name
//!   the arm then uses is a refusal rather than a silent wildcard.
//!   ([`a_literal_name_is_matched_and_not_bound`])
//! - **Shape, not provenance.** One pattern matches a node a composer wrote and
//!   a node a quote built. ([`a_derived_node_and_a_source_node_match_alike`])
//! - **Trivia is not shape.** A comment between two elements does not defeat a
//!   match. ([`a_comment_between_two_elements_does_not_defeat_a_match`])
//! - **Coverage is the ordinary coverage.** Shapes do not exhaust a type, and
//!   two of one shape are one arm.
//!   ([`a_match_of_shapes_still_needs_the_arm_that_says_what_this_reads`],
//!   [`two_arms_of_one_shape_are_one_arm`])
//! - **One spread per group**, because two would be a search.
//!   ([`two_spreads_in_one_group_are_refused`])
//!
//! §4's refusals follow the laws, one test each, and the last two tests are the
//! side-by-side programs that say which form is for which job.
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, ImportSources, Severity, SourceDocument, compile, resolve_import};

const PROBE: &str = "probe::adapter";

fn probe(body: &str) -> String {
    format!("library {{\n    let level = \"readable\";\n{body}\n}}\n")
}

/// An adapter whose answer is what `decide` makes of the region.
///
/// The fold is here for one reason: a quote needs a `NodePath` and the only
/// place one comes from is a traversal — an adapter cannot invent a place. What
/// these laws match on is the region itself, one node of a known shape, which
/// is the case §4 says a pattern is for.
fn deciding(decide: &str) -> String {
    probe(&format!(
        "{decide}
    let expand = fn (region) {{ Ok(built(region)) }};

    let built = fn (region: Syntax<TokenTree>) -> Syntax<Expr> {{
        syntax_fold_from_leaves(
            fn (here) {{ decide(here, region) }},
            fn (here, kind, text) {{ decide(here, region) }},
            fn (here, name) {{ decide(here, region) }},
            fn (here, delimiter, children) {{ decide(here, region) }},
            region,
        )
    }};
"
    ))
}

fn piece(declarations: &str, binding: &str, contents: &str) -> String {
    format!(
        "piece \"probe\" {{\n    import syntax {PROBE} as probe;\n\n{declarations}    let {binding} = syntax probe {{ \
         {contents} }};\n\n    score {{ part p {{ voice v {{ c4/1 }} }} }}\n}}\n"
    )
}

/// Every error compiling that piece against that adapter, as the whole small
/// document each one is — message, note, and help.
///
/// A module's own diagnostics reach here through `expand`'s level check, which
/// keeps the first one's **message** and replaces its note, help, and label
/// with the wrapper's own. So a law about what an adapter author is told is a
/// law about the message, and every distinction one of these tests observes is
/// written there rather than in a note.
fn errors(declarations: &str, binding: &str, contents: &str, module: &str) -> Vec<String> {
    let source = SourceDocument::new(piece(declarations, binding, contents), "probe.musa");
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
    .map(|diagnostic| {
        let mut lines = vec![diagnostic.message.clone()];
        if let Some(note) = diagnostic.note.as_deref() {
            lines.push(format!("note: {note}"));
        }
        if let Some(help) = diagnostic.help.as_deref() {
            lines.push(format!("help: {help}"));
        }
        lines.join("\n")
    })
    .collect()
}

fn plain(binding: &str, contents: &str, module: &str) -> Vec<String> {
    errors("", binding, contents, module)
}

/// Whether any error says `wanted`.
fn says(found: &[String], wanted: &str) -> bool {
    found.iter().any(|error| error.contains(wanted))
}

#[test]
fn match_after_build_binds_what_the_quote_spliced() {
    // The law that says the two forms are one relation read two ways: a
    // pattern written like the quote that built the value binds, hole for
    // hole, exactly what that quote spliced. `a` is the first operand and `b`
    // the second, so the pair the arm rebuilds is `(Nat, Text)` — and a
    // matcher that bound them the other way round, or bound the operator, or
    // bound the whole expression to one of them, is a different type here.
    //
    // Each quote and the pattern sit behind an annotated parameter because
    // both forms are checked rather than inferred: a quote is checked against
    // a category and a pattern is read at its scrutinee's, so neither can be
    // written where the type is still open.
    let module = deciding(
        r#"
    let one = fn (here: NodePath) -> Syntax<Expr> { quote at here { 1 } };
    let two = fn (here: NodePath) -> Syntax<Expr> { quote at here { "two" } };

    let sum = fn (here: NodePath, x: Syntax<Expr>, y: Syntax<Expr>) -> Syntax<Expr> {
        quote at here { $x + $y }
    };

    let split = fn (here: NodePath, built: Syntax<Expr>) -> Syntax<Expr> {
        match built {
            quote { $a + $b } -> quote at here { ($a, $b) },
            _ -> quote at here { "unmatched" },
        }
    };

    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {
        split(here, sum(here, one(here), two(here)))
    };
"#,
    );
    let found = plain("held: (Nat, Text)", "a", &module);
    assert!(
        found.is_empty(),
        "the pattern did not bind what the quote spliced: {found:?}"
    );
    // Swapped, so the half above is evidence rather than a program that would
    // have compiled either way.
    let refused = plain("held: (Text, Nat)", "a", &module);
    assert!(!refused.is_empty(), "the two holes were bound interchangeably");
}

#[test]
fn a_literal_name_is_matched_and_not_bound() {
    // §4's first rule. `thing` in the pattern is a word that has to be there,
    // not a wildcard: a region that writes it matches and a region that writes
    // something else does not. The alternative — a literal identifier binding
    // whatever it names — reads well in one example and turns every typo into
    // a pattern that matches everything.
    let module = deciding(
        r#"
    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {
        match region {
            quote { { thing } } -> quote at here { 1 },
            _ -> quote at here { "other" },
        }
    };
"#,
    );
    let matched = plain("held: Nat", "thing", &module);
    assert!(matched.is_empty(), "a literal name did not match itself: {matched:?}");
    let missed = plain("held: Text", "other", &module);
    assert!(
        missed.is_empty(),
        "a literal name matched a different name, so it was read as a wildcard: {missed:?}"
    );
}

#[test]
fn a_name_the_pattern_took_literally_is_reported_where_the_arm_uses_it() {
    // The other side of the same rule. Without this the arm reports `thing` as
    // a name nobody declared, which is true and says nothing about the `$`
    // that was left off — so the report names the identifier and the repair.
    let module = deciding(
        r#"
    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {
        match region {
            quote { { thing } } -> quote at here { ${ thing } },
            _ -> quote at here { "other" },
        }
    };
"#,
    );
    let found = plain("held: Nat", "thing", &module);
    assert!(
        says(&found, "the pattern matched `thing` literally rather than binding it"),
        "a literal name used as a binding was not reported: {found:?}"
    );
}

#[test]
fn a_derived_node_and_a_source_node_match_alike() {
    // §4's second rule, and the property the whole adapter design rests on: an
    // expansion's result is interchangeable with source. One pattern is asked
    // about a node the composer wrote and about a node a quote built, and it
    // answers the same — which is also why there is deliberately no way to ask
    // "was this written by a human" from inside a pattern.
    //
    // Literally one pattern: `braced` is the same text in all three modules,
    // and only the node handed to it differs.
    const BRACED: &str = r#"
    let braced = fn (here: NodePath, node: Syntax<TokenTree>) -> Syntax<Expr> {
        match node {
            quote { { $inside } } -> quote at here { 1 },
            _ -> quote at here { "unmatched" },
        }
    };
"#;
    let source = deciding(&format!(
        "{BRACED}
    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {{
        braced(here, region)
    }};
"
    ));
    let wrote = plain("held: Nat", "sole", &source);
    assert!(wrote.is_empty(), "a node the composer wrote did not match: {wrote:?}");

    let derived = deciding(&format!(
        "{BRACED}
    let made = fn (here: NodePath) -> Syntax<Expr> {{ quote at here {{ {{ 1 }} }} }};

    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {{
        braced(here, made(here))
    }};
"
    ));
    let quoted = plain("held: Nat", "sole", &derived);
    assert!(
        quoted.is_empty(),
        "a node a quote built did not match the pattern its source twin matched: {quoted:?}"
    );

    // And it is the shape being read on both sides rather than a pattern that
    // says yes to everything: a built node of a different shape reaches the
    // other arm.
    let other = deciding(&format!(
        "{BRACED}
    let made = fn (here: NodePath) -> Syntax<Expr> {{ quote at here {{ (1, 2) }} }};

    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {{
        braced(here, made(here))
    }};
"
    ));
    let missed = plain("held: Text", "sole", &other);
    assert!(
        missed.is_empty(),
        "a built node of another shape matched the braced pattern: {missed:?}"
    );
}

#[test]
fn a_comment_between_two_elements_does_not_defeat_a_match() {
    // "Trivia is not shape". A read region keeps every comment and space it
    // was written with, because its text has to come back; a template kept
    // none, because the printer re-spaces every expansion. So the significant
    // children are what is compared, and an adapter does not break when a
    // composer runs the formatter over the region it reads.
    let module = deciding(
        r#"
    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {
        match region {
            quote { { ($x, $y) } } -> quote at here { 1 },
            _ -> quote at here { "unmatched" },
        }
    };
"#,
    );
    let spaced = plain("held: Nat", "(a, b)", &module);
    assert!(spaced.is_empty(), "a plain product did not match: {spaced:?}");
    let commented = plain("held: Nat", "(a, /* why */ b)", &module);
    assert!(
        commented.is_empty(),
        "a comment between two elements defeated the match: {commented:?}"
    );
    let line = plain("held: Nat", "( a ,\n      b )", &module);
    assert!(
        line.is_empty(),
        "whitespace and a line break defeated the match: {line:?}"
    );
}

#[test]
fn a_spread_binds_the_run_it_stands_among() {
    // A spread in a pattern stands among a group's children and binds the run
    // of them — which is wider than construction, where a spread needs the
    // comma its position supplies. §4's own second example is a block, and a
    // block separates nothing.
    let module = deciding(
        r#"
    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {
        match region {
            quote { { $..items } } -> counted(here, items),
            _ -> quote at here { "unmatched" },
        }
    };

    let counted = fn (here: NodePath, items: List<Syntax<TokenTree>>) -> Syntax<Expr> {
        match items {
            [] -> quote at here { "none" },
            [first, ..others] -> quote at here { 1 },
        }
    };
"#,
    );
    let held = plain("held: Nat", "a", &module);
    assert!(held.is_empty(), "a spread bound no run at all: {held:?}");
    // The run is the group's own significant children, so an empty region
    // binds the empty run rather than failing to match.
    let empty = plain("held: Text", "", &module);
    assert!(empty.is_empty(), "an empty group did not bind an empty run: {empty:?}");
}

#[test]
fn a_match_of_shapes_still_needs_the_arm_that_says_what_this_reads() {
    // §4: "A quote pattern constrains and does not enumerate". The case tree
    // cannot know that a finite set of shapes exhausts the token trees, so the
    // refusal arm is the arm coverage requires — and it is where the adapter
    // says what it reads, which is why §4 calls it not defensive style.
    let module = deciding(
        r"
    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {
        match region {
            quote { { $..items } } -> quote at here { 1 },
        }
    };
",
    );
    let found = plain("held: Nat", "a", &module);
    assert!(
        says(&found, "this match leaves a possible value uncovered"),
        "a match made only of shapes was accepted as exhaustive: {found:?}"
    );
}

#[test]
fn two_arms_of_one_shape_are_one_arm() {
    // Unreachability, by the same check every other pattern goes through: one
    // shape is one coverage, so a second arm written with it can never be
    // selected. The names the holes bind do not enter into it — a shape is a
    // shape however its holes are spelled.
    let module = deciding(
        r"
    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {
        match region {
            quote { { $..items } } -> quote at here { 1 },
            quote { { $..other } } -> quote at here { 2 },
            _ -> quote at here { 3 },
        }
    };
",
    );
    let found = plain("held: Nat", "a", &module);
    assert!(
        says(&found, "this match arm can never be selected"),
        "a repeated shape was not reported unreachable: {found:?}"
    );
    // Two *different* shapes are two arms, so the check above is about the
    // shape and not about quote patterns in general.
    let distinct = deciding(
        r"
    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {
        match region {
            quote { { ($x, $y) } } -> quote at here { 1 },
            quote { { $..other } } -> quote at here { 2 },
            _ -> quote at here { 3 },
        }
    };
",
    );
    let two = plain("held: Nat", "(a, b)", &distinct);
    assert!(two.is_empty(), "two different shapes were read as one arm: {two:?}");
}

#[test]
fn two_spreads_in_one_group_are_refused() {
    // Two would make matching a search for the split point, and choosing the
    // split would be a guess about the author's intent. The refusal is at the
    // second one, where the repair is.
    let module = deciding(
        r#"
    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {
        match region {
            quote { f($..xs, $..ys) } -> quote at here { 1 },
            _ -> quote at here { "unmatched" },
        }
    };
"#,
    );
    let found = plain("held: Nat", "a", &module);
    assert!(
        says(&found, "two spreads stand in one position"),
        "two spreads in one group were accepted: {found:?}"
    );
}

#[test]
fn a_spread_where_one_node_stands_is_refused() {
    // The body of a pattern is one node, so there are no siblings for a spread
    // to be a run of. Same code as construction's refusal and a different
    // reason, and the *message* is where the difference is written:
    // construction has no separator to write, matching has no run to bind.
    let module = deciding(
        r#"
    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {
        match region {
            quote { $..everything } -> quote at here { 1 },
            _ -> quote at here { "unmatched" },
        }
    };
"#,
    );
    let found = plain("held: Nat", "a", &module);
    assert!(
        says(&found, "nothing here holds a run to bind"),
        "a spread standing alone was accepted, or was refused with construction's complaint: {found:?}"
    );
}

#[test]
fn a_braced_splice_in_a_pattern_is_refused() {
    // `${ e }` holds an expression to evaluate and a pattern has nothing to
    // evaluate it for. One splice production serves both directions, so the
    // form is refused where the direction is known rather than by a second
    // grammar that would have to be kept in step with the first.
    let module = deciding(
        r#"
    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {
        match region {
            quote { { ${ region } } } -> quote at here { 1 },
            _ -> quote at here { "unmatched" },
        }
    };
"#,
    );
    let found = plain("held: Nat", "a", &module);
    assert!(
        says(&found, "a pattern has nothing to evaluate"),
        "a braced splice was accepted in a pattern: {found:?}"
    );
}

#[test]
fn a_pattern_against_a_value_that_is_not_syntax_is_refused() {
    // A pattern is read at the scrutinee's category, so a value with no
    // category leaves it nothing to claim about what it binds. The report says
    // which type was matched rather than only that the pattern was wrong.
    let module = deciding(
        r#"
    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {
        match 1 {
            quote { { $a } } -> quote at here { 1 },
            _ -> quote at here { "unmatched" },
        }
    };
"#,
    );
    let found = plain("held: Nat", "a", &module);
    assert!(
        says(&found, "a quote pattern matches a syntax value, and this is a `Nat`"),
        "a quote pattern was accepted against a natural, or the report did not name the type: {found:?}"
    );
}

#[test]
fn a_pattern_decides_a_known_shape() {
    // **Which form is for which job, half one.** A pattern quote decides a
    // shape the adapter already knows how to write down: it destructures one
    // level, binds the pieces it names, and says nothing about anything
    // deeper. Reaching for the recursor here would mean writing a traversal to
    // ask a question about one node — see
    // [`the_recursor_traverses_an_unknown_shape`] for the case that is really
    // its own.
    let module = deciding(
        r#"
    let decide = fn (here: NodePath, region: Syntax<TokenTree>) -> Syntax<Expr> {
        match region {
            quote { { ($x, $y) } } -> quote at here { 1 },
            quote { { [$..voiced] } } -> quote at here { 2 },
            _ -> quote at here { "this adapter reads a pair or a bracketed run" },
        }
    };
"#,
    );
    // A pattern is written in Musa, so a bracketed run in one is a Musa list
    // and matches a comma-separated one. That is a real limit on what a pattern
    // can decide — much of the notation an adapter reads separates nothing —
    // and it is the other half of why the recursor stays.
    let pair = plain("held: Nat", "(a, b)", &module);
    assert!(pair.is_empty(), "the pair shape was not decided: {pair:?}");
    let bracketed = plain("held: Nat", "[a, b, c]", &module);
    assert!(
        bracketed.is_empty(),
        "the bracketed shape was not decided: {bracketed:?}"
    );
    let neither = plain("held: Text", "a b", &module);
    assert!(neither.is_empty(), "the refusal arm was not reached: {neither:?}");
}

#[test]
fn the_recursor_traverses_an_unknown_shape() {
    // **Which form is for which job, half two.** The recursor visits every
    // node of a region whose shape the adapter does not know and cannot write
    // down, handing each one its own path and its own children. A pattern
    // cannot do this: it would have to enumerate the shapes, and the region is
    // whatever the composer wrote. Nothing here is a shape decision, which is
    // why prompt 140 deletes neither form — see [`a_pattern_decides_a_known_shape`].
    let module = probe(
        r#"
    let expand = fn (region) { Ok(built(region)) };

    let one = fn (here: NodePath) -> Syntax<Expr> { quote at here { 1 } };

    let built = fn (region: Syntax<TokenTree>) -> Syntax<Expr> {
        syntax_fold_from_leaves(
            fn (here) { one(here) },
            fn (here, kind, text) { one(here) },
            fn (here, name) { one(here) },
            fn (here, delimiter, children) { quote at here { ("group", $..children) } },
            region,
        )
    };
"#,
    );
    // Whatever nests, the traversal reaches: the annotation counts what the
    // outermost group's children folded to, and the adapter never named a
    // shape to get there.
    let found = plain("held: (Text, Nat, Nat, Nat)", "1", &module);
    assert!(
        found.is_empty(),
        "the recursor did not reach the region's leaves: {found:?}"
    );
}
