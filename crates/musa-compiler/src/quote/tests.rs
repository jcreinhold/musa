//! The syntax calculus' laws.
//!
//! A law suite reports a violated law by failing, and the helpers below take
//! apart a value whose existence the law has already asserted: a panic is the
//! report, not an accident.
#![allow(clippy::expect_used, clippy::panic)]

use super::category::TOKEN_KINDS;
use super::path::Scope;
use super::*;
use crate::phase::{ExpansionFailure, expand_region};

const REGION: &str = "let melody = together(a, b)";

/// The expansion these laws run under. Any coordinate does; what matters is
/// that it is the compiler's to supply and never a transformer's to invent.
fn expansion() -> ExpansionPath {
    ExpansionPath::at(vec![7])
}

fn read() -> Syntax {
    read_region(&musa_syntax::parse(REGION).syntax(), expansion())
}

fn run(transformer: &str) -> Syntax {
    match expand_region(transformer, REGION, expansion()) {
        Ok(produced) => produced,
        Err(failure) => panic!("the transformer did not run: {failure:?}"),
    }
}

/// Every generated node in a value, in the order they were written.
fn generated(node: &Syntax, into: &mut Vec<NodePath>) {
    if let SourceInfo::Generated(path) = node.info() {
        into.push(path.clone());
    }
    if let Syntax::Group { children, .. } = node {
        for child in children {
            generated(child, into);
        }
    }
}

fn identifiers<'a>(node: &'a Syntax, into: &mut Vec<&'a Syntax>) {
    if matches!(node, Syntax::Identifier { .. }) {
        into.push(node);
    }
    if let Syntax::Group { children, .. } = node {
        for child in children {
            identifiers(child, into);
        }
    }
}

fn count(node: &Syntax) -> usize {
    match node {
        Syntax::Missing(_) | Syntax::Token { .. } | Syntax::Identifier { .. } => 1,
        Syntax::Group { children, .. } => children.iter().map(count).sum::<usize>().saturating_add(1),
    }
}

/// Write a transformer whose four cases are `missing`, `token`,
/// `identifier`, and `group`, in that order.
///
/// The fold's answer is wrapped in `Ok`, because a transformer answers
/// `Result<Syntax, (Syntax, Text)>` and every law here is about the half
/// that accepts. The refusing half is `crate::expand`'s to exercise, where
/// there is a diagnostic to read it out of.
fn transformer(missing: &str, token: &str, identifier: &str, group: &str) -> String {
    format!(
        "fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> {{ Ok(syntax_fold_from_leaves(fn (here) {{ {missing} }}, fn (here, kind, text) {{ {token} }}, \
         fn (here, name) {{ {identifier} }}, fn (here, delimiter, children) {{ {group} }}, region)) }}"
    )
}

/// A transformer that rebuilds the region node for node, marking each case
/// with its own builder role.
fn rebuild() -> String {
    transformer(
        r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
        r"syntax_token(syntax_built(here, 1, 0), kind, text)",
        r"syntax_identifier(syntax_built(here, 2, 0), name)",
        r"syntax_group(syntax_built(here, 3, 0), delimiter, children)",
    )
}

#[test]
fn a_transformer_is_a_program_rather_than_a_picture_of_one() {
    assert_eq!(
        count(&run(&rebuild())),
        count(&read()),
        "rebuilding the region node for node gives a value of the same shape"
    );
}

#[test]
fn every_node_the_fold_visits_has_its_own_path() {
    let produced = run(&rebuild());
    let mut paths = Vec::new();
    generated(&produced, &mut paths);
    let unique: std::collections::BTreeSet<_> = paths.iter().cloned().collect();
    assert_eq!(
        unique.len(),
        paths.len(),
        "the fold handed two nodes one path, so an output anchor would be ambiguous"
    );
    assert_eq!(paths.len(), count(&read()), "the fold visited each input node once");
}

#[test]
fn a_derived_output_path_never_addresses_an_input_node() {
    // Reading descends by `Child` and building extends by `Built`, so a
    // derived path and a structural one cannot be the same path.
    let mut paths = Vec::new();
    generated(&run(&rebuild()), &mut paths);
    let read = read();
    for path in &paths {
        assert!(read.at(path).is_none(), "a derived output path addressed an input node");
    }
}

#[test]
fn folding_is_total_and_deterministic() {
    assert_eq!(
        run(&rebuild()),
        run(&rebuild()),
        "two runs of one transformer disagreed"
    );
    for region in ["", "let", "let x = (", "piece \"p\" { }"] {
        assert!(
            expand_region(&rebuild(), region, expansion()).is_ok(),
            "the fold declined to answer on `{region}`"
        );
    }
}

#[test]
fn a_builder_is_a_function_of_its_displayed_arguments() {
    // Two calls written with the same arguments give one value, so they
    // give one *path*, and the gate refuses the pair. A builder that minted
    // a fresh id would hand back two paths and the pair would be accepted,
    // which is exactly the operation `34-proof-review.md` found could not
    // be both fresh and deterministic.
    let twice = r#"syntax_group(syntax_built(here, 3, 0), delimiter,
        [syntax_token(syntax_built(here, 4, 0), TokenKind.Integer, "1"),
         syntax_token(syntax_built(here, 4, 0), TokenKind.Integer, "1")])"#;
    assert_eq!(
        expand_region(
            &transformer(
                r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
                r"syntax_token(syntax_built(here, 1, 0), kind, text)",
                r"syntax_identifier(syntax_built(here, 2, 0), name)",
                twice,
            ),
            REGION,
            expansion(),
        ),
        Err(ExpansionFailure::NotAnExpression(NotAnExpression::DuplicatePath)),
        "one builder written twice with one argument list gave two different paths"
    );
}

#[test]
fn one_binding_path_denotes_one_name() {
    // The binder and the reference each ask for `syntax_binding(here, 5)`,
    // and share a scope because asking twice gives the same binding.
    let bound = r#"syntax_group(syntax_built(here, 3, 0), delimiter,
        [syntax_binder(syntax_binding(here, 5), "voice"),
         syntax_reference(syntax_built(here, 6, 0), syntax_binding(here, 5), "voice")])"#;
    let produced = run(&transformer(
        r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
        r"syntax_token(syntax_built(here, 1, 0), kind, text)",
        r"syntax_identifier(syntax_built(here, 2, 0), name)",
        bound,
    ));
    let mut names = Vec::new();
    identifiers(&produced, &mut names);
    let scoped: Vec<&Vec<Scope>> = names
        .iter()
        .filter_map(|node| match node {
            Syntax::Identifier { scopes, .. } if !scopes.is_empty() => Some(scopes),
            Syntax::Missing(_) | Syntax::Token { .. } | Syntax::Identifier { .. } | Syntax::Group { .. } => None,
        })
        .collect();
    assert!(scoped.len() >= 2, "the transformer bound nothing");
    assert_eq!(
        scoped.first(),
        scoped.get(1),
        "a binder and a reference to it carry one scope"
    );
}

#[test]
fn two_binding_paths_are_two_names() {
    let two = r#"syntax_group(syntax_built(here, 3, 0), delimiter,
        [syntax_binder(syntax_binding(here, 5), "voice"),
         syntax_binder(syntax_binding(here, 6), "voice")])"#;
    let produced = run(&transformer(
        r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
        r"syntax_token(syntax_built(here, 1, 0), kind, text)",
        r"syntax_identifier(syntax_built(here, 2, 0), name)",
        two,
    ));
    let mut names = Vec::new();
    identifiers(&produced, &mut names);
    let scopes: Vec<Scope> = names
        .iter()
        .filter_map(|node| match node {
            Syntax::Identifier { scopes, .. } => scopes.first().cloned(),
            Syntax::Missing(_) | Syntax::Token { .. } | Syntax::Group { .. } => None,
        })
        .collect();
    assert!(scopes.len() >= 2, "the transformer bound nothing");
    let unique: std::collections::BTreeSet<_> = scopes.iter().cloned().collect();
    assert_eq!(
        unique.len(),
        scopes.len(),
        "two binding paths gave one name, so two binders would capture each other"
    );
}

#[test]
fn a_preserved_input_node_keeps_its_own_source_information() {
    // `syntax_at` is how a transformer carries input through: it turns a
    // path the fold revealed back into the node, unchanged.
    let carry = r#"syntax_group(syntax_built(here, 3, 0), delimiter,
        [syntax_at(region, here).fold_from_end(
             syntax_token(syntax_built(here, 7, 0), TokenKind.Error, ""),
             fn (node, unused) { node })])"#;
    let produced = run(&transformer(
        r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
        r"syntax_token(syntax_built(here, 1, 0), kind, text)",
        r"syntax_identifier(syntax_built(here, 2, 0), name)",
        carry,
    ));
    let Syntax::Group { children, .. } = &produced else {
        panic!("expected a group");
    };
    assert_eq!(
        children.first(),
        Some(&read()),
        "carrying an input node through changed it"
    );
}

#[test]
fn a_conflicting_binder_is_refused_by_name() {
    let conflict = r#"syntax_group(syntax_built(here, 3, 0), delimiter,
        [syntax_binder(syntax_binding(here, 5), "x"),
         syntax_group(syntax_built(here, 8, 0), Delimiter.Parentheses,
             [syntax_binder(syntax_binding(here, 5), "x")])])"#;
    assert_eq!(
        expand_region(
            &transformer(
                r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
                r"syntax_token(syntax_built(here, 1, 0), kind, text)",
                r"syntax_identifier(syntax_built(here, 2, 0), name)",
                conflict,
            ),
            REGION,
            expansion(),
        ),
        Err(ExpansionFailure::NotAnExpression(NotAnExpression::ConflictingBinder)),
        "one binding path declared two binders"
    );
}

/// A fused group whose children spell one lexeme is admitted, and the three
/// kinds are the three.
///
/// Built here rather than read, because a read region still hands its literals
/// over whole (prompt 162hb) and the claim is about what a *transformer* may
/// assemble. `as_expression`'s own reader is asked afterwards, so the law says
/// both halves: the gate admits it, and the parser reads back the pitch that
/// was written.
#[test]
fn a_fused_group_that_spells_one_lexeme_is_one_lexeme() {
    for (written, parts) in [
        (
            "c#5",
            vec![("PitchLetter", "c"), ("PitchAccidental", "#"), ("PitchOctave", "5")],
        ),
        ("M3", vec![("IntervalQuality", "M"), ("IntervalSize", "3")]),
        (
            "3/8",
            vec![("RationalNumerator", "3"), ("Slash", "/"), ("RationalDenominator", "8")],
        ),
    ] {
        let root = NodePath::root(expansion());
        let children = parts
            .into_iter()
            .enumerate()
            .map(|(index, (kind, text))| {
                let at = root.child(u32::try_from(index).unwrap_or(u32::MAX));
                let kind = token_kind_named(kind).unwrap_or_else(|| panic!("`{kind}` names a token kind"));
                token(at, kind, text.to_owned())
            })
            .collect();
        let built = group(root, Delimiter::Fused, children);
        assert_eq!(check_expression(&built), Ok(()), "`{written}` was refused");
        assert_eq!(
            print(&built).text,
            written,
            "`{written}` did not print back as one word"
        );
        assert!(
            parses_as_expression(&built),
            "`{written}` does not stand where an expression stands"
        );
    }
}

/// A fused group that is not one lexeme is refused, and the message says what
/// the reader made of it.
///
/// The second case is `11-quotation.md` §2's splice rule, restated as a test.
/// A fused group of `a` and `bc` assembles `abc`, which *does* lex as exactly
/// one token — and its kind is `Identifier`, which is not one of the three, so
/// there is still no way to build a name out of pieces.
#[test]
fn a_fused_group_that_is_not_one_lexeme_is_refused() {
    for (parts, text, lexed) in [
        (
            r##"syntax_token(syntax_built(here, 4, 0), TokenKind.PitchLetter, "c"),
               syntax_token(syntax_built(here, 5, 0), TokenKind.PitchAccidental, "#"),
               syntax_token(syntax_built(here, 6, 0), TokenKind.PitchOctave, "x5")"##,
            "c#x5",
            "3 tokens",
        ),
        (
            r#"syntax_identifier(syntax_built(here, 4, 0), "a"),
               syntax_token(syntax_built(here, 5, 0), TokenKind.Identifier, "bc")"#,
            "abc",
            "one `Identifier`",
        ),
    ] {
        let written = expand_region(
            &transformer(
                r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
                r"syntax_token(syntax_built(here, 1, 0), kind, text)",
                r"syntax_identifier(syntax_built(here, 2, 0), name)",
                &format!("syntax_group(syntax_built(here, 3, 0), Delimiter.Fused, [{parts}])"),
            ),
            REGION,
            expansion(),
        );
        assert_eq!(
            written,
            Err(ExpansionFailure::NotAnExpression(NotAnExpression::NotOneLexeme {
                text: text.to_owned(),
                lexed: lexed.to_owned(),
            })),
            "`{text}` was not refused as the lexeme it is not"
        );
    }
}

#[test]
fn a_group_that_names_no_real_delimiter_does_not_check() {
    // The question moved. It used to be asked of a finished expansion, by
    // the gate, because a transformer named a delimiter as text; now
    // `Delimiter` is a type whose only values are the four, so a name that
    // is not one of them is refused where it is *written* — and the whole
    // expansion never runs.
    let written = expand_region(
        &transformer(
            r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
            r"syntax_token(syntax_built(here, 1, 0), kind, text)",
            r"syntax_identifier(syntax_built(here, 2, 0), name)",
            r"syntax_group(syntax_built(here, 3, 0), Delimiter.Curly, children)",
        ),
        REGION,
        expansion(),
    );
    assert!(
        matches!(written, Err(ExpansionFailure::NotATransformer(_))),
        "a delimiter the fixed grouper does not have was accepted: {written:?}"
    );
}

#[test]
fn a_token_kind_the_lexer_does_not_have_does_not_check() {
    let written = expand_region(
        &transformer(
            r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Zither, "")"#,
            r"syntax_token(syntax_built(here, 1, 0), kind, text)",
            r"syntax_identifier(syntax_built(here, 2, 0), name)",
            r"syntax_group(syntax_built(here, 3, 0), delimiter, children)",
        ),
        REGION,
        expansion(),
    );
    assert!(
        matches!(written, Err(ExpansionFailure::NotATransformer(_))),
        "a token kind the lexer never produces was accepted: {written:?}"
    );
}

#[test]
fn every_token_kind_the_lexer_produces_is_nameable() {
    // The drift law. `TokenKind` *is* the lexer's own kind, so the case set
    // cannot disagree about what a kind means; what a hand-written list can
    // still do is fall behind, and a kind the lexer produces that the phase
    // cannot name would be a silent gap in an adapter's dispatch.
    //
    // The partition has three parts and not two. A kind the lexer emits is
    // nameable; a kind the parser mints from a composite literal's spelling is
    // nameable too, because an adapter that can reach a numerator and cannot
    // say `TokenKind.RationalNumerator` about it has been handed half an
    // operation; and a parser *node* kind is not, because no token ever has it.
    for kind in musa_syntax::SyntaxKind::all() {
        let named = TOKEN_KINDS.iter().any(|(_, candidate)| *candidate == kind);
        let is_token = musa_syntax::TokenClass::of(kind).is_some();
        assert_eq!(
            named,
            is_token || kind.is_literal_part(),
            "`{kind:?}` is a kind a token can have that the phase cannot name, or a node kind it can"
        );
        assert!(
            !(is_token && kind.is_literal_part()),
            "`{kind:?}` claims to be both a lexer token and a part the parser mints"
        );
    }
}

#[test]
fn a_named_token_kind_is_the_kind_it_names() {
    for (name, kind) in TOKEN_KINDS {
        assert_eq!(token_kind_named(name), Some(*kind), "`{name}` named another kind");
    }
    assert_eq!(token_kind_named("Zither"), None, "an invented name found a kind");
    for delimiter in Delimiter::ALL {
        assert_eq!(
            Delimiter::named(delimiter.name()),
            Some(delimiter),
            "`{}` named another delimiter",
            delimiter.name()
        );
    }
    assert_eq!(Delimiter::named("Curly"), None, "an invented name found a delimiter");
}

#[test]
fn a_derivation_is_its_three_components_and_nothing_else() {
    // The identity law, minted directly rather than through an expansion,
    // so that what is under test is the representation and not one
    // transformer's use of it.
    let origin = NodePath::root(expansion());
    let derived = Derived {
        origin: origin.clone(),
        quotation: 3,
        path: vec![0, 1],
    };
    assert_eq!(derived.path(), derived.path(), "one derivation gave two paths");
    for other in [
        Derived {
            origin: origin.child(0),
            ..derived.clone()
        },
        Derived {
            quotation: 4,
            ..derived.clone()
        },
        Derived {
            path: vec![0, 2],
            ..derived.clone()
        },
    ] {
        assert_ne!(
            other.path(),
            derived.path(),
            "two derivations that differ in one component gave one path"
        );
    }
    // And a derived path is never an input node's path, whatever the
    // components are: the steps are disjoint by construction.
    assert!(
        read().at(&derived.path()).is_none(),
        "a derived path addressed an input node"
    );
}

#[test]
fn the_gate_is_reachable_from_inside_a_transformer() {
    // `checked_expression` answers with a value either way, so a
    // transformer can ask the question of a fragment it is still building
    // rather than only having it asked of its finished result. The answer
    // is a `Result`, so it is taken apart by the one match evaluator every
    // other sum in the language is taken apart by.
    let gated = r#"syntax_group(syntax_built(here, 3, 0), delimiter,
        [match checked_expression(syntax_token(syntax_built(here, 4, 0), TokenKind.Integer, "1")) {
            Ok(node) -> forget(node),
            Err(message) -> syntax_token(syntax_built(here, 5, 0), TokenKind.Error, message),
         }])"#;
    let produced = run(&transformer(
        r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
        r"syntax_token(syntax_built(here, 1, 0), kind, text)",
        r"syntax_identifier(syntax_built(here, 2, 0), name)",
        gated,
    ));
    let Syntax::Group { children, .. } = &produced else {
        panic!("expected a group");
    };
    let Some(Syntax::Token { kind, .. }) = children.first() else {
        panic!("the gate answered with nothing");
    };
    assert_eq!(
        *kind,
        musa_syntax::SyntaxKind::Integer,
        "the gate refused a fragment it should have accepted"
    );
}
