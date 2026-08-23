//! What the syntax index means, from outside the compiler (prompt 138).
//!
//! `docs/rules/language/11-quotation.md` §1 gives `Syntax` a category, and
//! every law about it is a law about what an *adapter* may write. So these
//! run the real thing: a probe adapter is handed to the compiler the way any
//! package adapter is, and what the compiler says about it is the law's
//! answer. The unit tests beside `crate::syntax` check the same rules against
//! the representation; these check that the checker, the evaluator, and the
//! expansion gate all agree with them on a program.
//!
//! Four laws, and they are different in kind:
//!
//! - **Forgetting is directional.** A `Syntax<TokenTree>` position accepts a
//!   certified expression, and a `Syntax<Expr>` position refuses a tree
//!   nobody parsed. The second half is the whole reason the index exists.
//! - **The checked parse is the parser.** `as_expression` answers `some`
//!   exactly when the tree prints as source the ordinary parser reads as an
//!   expression — measured against `musa_syntax::parse` itself rather than
//!   against a second opinion about what an expression is.
//! - **The phase can name every token kind the lexer produces.** Generated
//!   from `musa-syntax`'s own table, so a kind the lexer gains and the
//!   phase does not is a failure here rather than a silent gap in an
//!   adapter's dispatch.
//! - **A derived place is its three components.** Two construction sites
//!   reading one input node are two places; one construction site used twice
//!   is one place, and the gate says so.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
// A law that does not hold is reported by panicking with what actually
// happened, which is more useful than an assertion message alone.
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, ImportSources, SourceDocument, compile, resolve_import};

use musa_score::Severity;

/// Where the probe adapter is imported from, as the piece writes it.
const PROBE: &str = "probe::adapter";

/// The identity rebuild, as every adapter in the tree writes it: each node
/// rebuilt at its own derived place, the group's children preserved in order.
///
/// Shared by the probes so that what differs between them is only the law
/// under test, and so that a probe that fails for an unrelated reason fails
/// in every probe at once rather than in the one being read.
const REBUILT: &str = r#"
    let rebuilt = fn (region: Syntax<TokenTree>) -> Syntax<TokenTree> {
        syntax_fold_from_leaves(
            fn (here) { syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "") },
            fn (here, kind, text) { syntax_token(syntax_built(here, 1, 0), kind, text) },
            fn (here, name) { syntax_identifier(syntax_built(here, 2, 0), name) },
            fn (here, delimiter, children) { syntax_group(syntax_built(here, 3, 0), delimiter, children) },
            region,
        )
    };
"#;

/// A *readable* adapter module holding `body`, alongside the shared rebuild.
fn probe(body: &str) -> String {
    format!("library {{\n    let level = \"readable\";\n{REBUILT}\n{body}\n}}\n")
}

/// A piece whose one region is read by the probe adapter.
fn piece(contents: &str) -> String {
    format!(
        "piece \"probe\" {{\n    import syntax {PROBE} as probe;\n\n    let it = syntax probe {{ {contents} }};\n\n    \
         score {{ part p {{ voice v {{ c4/1 }} }} }}\n}}\n"
    )
}

/// Every error the compilation reported, as the whole small document it is,
/// and every **cause** with it.
///
/// All three parts, because a refusal puts the claim in the message, the
/// adapter's own words in the note, and the advice in the help — and a test
/// that reads only the first line tests less than it looks like it does. A
/// fault inside the adapter module arrives as a cause of the diagnostic about
/// the import, whole and one per fault, which is what lets these laws read the
/// note at all.
fn errors(contents: &str, module: &str) -> Vec<String> {
    let source = SourceDocument::new(piece(contents), "probe.musa");
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
        let mut found = vec![whole(
            &diagnostic.message,
            diagnostic.note.as_deref(),
            diagnostic.help.as_deref(),
        )];
        found.extend(
            diagnostic
                .causes
                .iter()
                .map(|cause| whole(&cause.message, cause.note.as_deref(), cause.help.as_deref())),
        );
        found
    })
    .collect()
}

/// One diagnostic, or one cause, as the small document it is.
fn whole(message: &str, note: Option<&str>, help: Option<&str>) -> String {
    let mut lines = vec![message.to_owned()];
    if let Some(note) = note {
        lines.push(format!("note: {note}"));
    }
    if let Some(help) = help {
        lines.push(format!("help: {help}"));
    }
    lines.join("\n")
}

#[test]
fn a_token_tree_position_accepts_a_certified_expression() {
    // §1's forgetting, in the direction it holds, and as prompt 159 makes it:
    // an operation the author writes. `as_expression` hands back a
    // `Syntax<Expr>`, and every position an adapter can put it in wants a
    // `Syntax<TokenTree>` — the answer of `expand` among them — so the program
    // says `forget` at each place a category is dropped, and the checker
    // inserts nothing behind its back.
    let module = probe(
        r"
    let kept = fn (node: Syntax<TokenTree>) -> Syntax<TokenTree> { node };

    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> {
        match as_expression(rebuilt(region)) {
            Some(node) -> Ok(kept(forget(node))),
            None -> Ok(rebuilt(region)),
        }
    };
",
    );
    let found = errors("c4", &module);
    assert!(
        found.is_empty(),
        "a certified expression was refused a tree's place: {found:?}"
    );
}

#[test]
fn an_expression_position_refuses_a_tree_nobody_parsed() {
    // The other direction, which is the reason the index is worth having. The
    // region is whatever the composer wrote between the braces; nothing has
    // read it as an expression, and the checker will not pretend otherwise.
    let module = probe(
        r"
    let demands = fn (node: Syntax<Expr>) -> Syntax<Expr> { node };

    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> { Ok(demands(region)) };
",
    );
    let found = errors("c4", &module);
    // The mismatch is reported at the smallest pair that disagrees: the
    // position's category index against the value's, which is the sentence the
    // author has to change one word of.
    assert!(
        found
            .iter()
            .any(|error| error.contains("`Expr`") && error.contains("`TokenTree`")),
        "an uncertified tree was accepted where an expression stands: {found:?}"
    );
}

#[test]
fn bare_syntax_names_no_type() {
    // `Syntax` takes a category the way `Duration` takes a coordinate. A
    // silent default would be the untyped `Syntax` back under the new
    // spelling, so it is a diagnostic that says which categories exist.
    let module = probe(
        r"
    let held = fn (node: Syntax) -> Syntax { node };

    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> { Ok(held(region)) };
",
    );
    let found = errors("c4", &module);
    assert!(
        found.iter().any(|error| {
            error.contains("`Syntax` takes an argument")
                && error.contains("Syntax<Expr>")
                && error.contains("Syntax<TokenTree>")
        }),
        "bare `Syntax` named a type: {found:?}"
    );
}

/// Regions whose contents do and do not parse as an expression.
///
/// Deliberately not a list of things known to be expressions: the law is that
/// `as_expression` agrees with the ordinary parser, and each entry's expected
/// answer is computed below by running that parser rather than written here.
const CORPUS: &[&str] = &["c4", "1", "3/4", "(1, 2)", "\"text\"", ";", "let", "meter 4/4", "= ="];

#[test]
fn the_checked_parse_answers_exactly_when_the_tree_parses_as_an_expression() {
    // §8's index-soundness obligation, discharged rather than asserted. The
    // probe refuses the region whenever `as_expression` says `none`, so
    // whether the piece compiles *is* the operation's answer, and the
    // expected answer comes from `musa_syntax::parse` — the same parser a
    // composer's own line is read by.
    let module = probe(
        r#"
    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> {
        match as_expression(rebuilt(region)) {
            Some(node) -> Ok(forget(node)),
            None -> Err((region, "this region does not parse as an expression")),
        }
    };
"#,
    );
    let mut answers = Vec::new();
    for contents in CORPUS {
        let parsed = musa_syntax::parse(&format!("piece \"reading\" {{\n    let it = {contents};\n}}\n"));
        let expected = parsed.errors().is_empty();
        let refused = errors(contents, &module)
            .iter()
            .any(|error| error.contains("does not parse as an expression"));
        assert_eq!(
            !refused, expected,
            "`{contents}` parses as an expression: {expected}, but the checked parse said {}",
            !refused
        );
        answers.push(expected);
    }
    // A corpus every entry of which parses would prove the operation says
    // `some`, not that it says `some` *exactly* then.
    assert!(
        answers.contains(&true) && answers.contains(&false),
        "the corpus does not straddle the answer: {answers:?}"
    );
}

#[test]
fn the_phase_can_name_every_token_kind_the_lexer_produces() {
    // The drift law, run through the checker rather than beside it. A kind
    // `musa-syntax` gains is a name an adapter may write the day it appears,
    // because the phase's case set is generated from that table and not
    // maintained next to it.
    let named: Vec<String> = musa_syntax::SyntaxKind::all()
        .filter(|kind| musa_syntax::TokenClass::of(*kind).is_some())
        .map(|kind| format!("    let {}_ = TokenKind.{kind:?};", format!("{kind:?}").to_lowercase()))
        .collect();
    let module = probe(&format!(
        "{}\n\n    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> {{ Ok(rebuilt(region)) }};\n",
        named.join("\n")
    ));
    let found = errors("c4", &module);
    assert!(
        found.is_empty(),
        "the phase cannot name a kind the lexer produces: {found:?}"
    );
}

#[test]
fn an_invented_token_kind_names_nothing() {
    // The other half of generation: the case set is exactly the lexer's, so a
    // name outside it is a name error rather than a kind no token ever has.
    let module = probe(
        r"
    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> { Ok(rebuilt(region)) };

    let invented = TokenKind.Zither;
",
    );
    let found = errors("c4", &module);
    assert!(!found.is_empty(), "`TokenKind.Zither` named a token kind");
}

/// A probe whose group branch builds a pair, with the comma at `role`.
///
/// Three tokens from one input node, so the only thing separating their
/// places is the construction site each was built at — which is exactly the
/// component the identity law is about.
fn pair_built_at(role: u32) -> String {
    probe(&format!(
        r#"
    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> {{
        Ok(syntax_fold_from_leaves(
            fn (here) {{ syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "") }},
            fn (here, kind, text) {{ syntax_token(syntax_built(here, 1, 0), kind, text) }},
            fn (here, name) {{ syntax_identifier(syntax_built(here, 2, 0), name) }},
            fn (here, delimiter, children) {{
                syntax_group(syntax_built(here, 3, 0), Delimiter.Parentheses, [
                    syntax_token(syntax_built(here, 4, 0), TokenKind.Integer, "1"),
                    syntax_token(syntax_built(here, {role}, 0), TokenKind.Comma, ","),
                    syntax_token(syntax_built(here, 6, 0), TokenKind.Integer, "2"),
                ])
            }},
            region,
        ))
    }};
"#
    ))
}

#[test]
fn two_construction_sites_reading_one_node_are_two_places() {
    // Injectivity in the quotation component, from the outside: three nodes
    // built from one input node at three sites are three places, and the
    // expansion gate — whose whole job is that a generated anchor names one
    // node — has nothing to complain about.
    let found = errors("c4", &pair_built_at(5));
    assert!(found.is_empty(), "three construction sites collided: {found:?}");
}

#[test]
fn one_construction_site_used_twice_is_one_place() {
    // And the converse, which is what makes the first test say something. The
    // same origin, quotation, and path is the same place however many nodes
    // are minted there, so the gate refuses rather than letting a later stage
    // guess which node an anchor named.
    let found = errors("c4", &pair_built_at(4));
    assert!(
        found.iter().any(|error| error.contains("not a well-formed expression")),
        "two nodes at one derived place went unreported: {found:?}"
    );
}
