use musa_language::{BarSpacing, SyntaxElement, SyntaxKind, format, parse};

const EXPRESSIONS: &str = r#"piece "Expressions" {
    let fifth: interval = P5;
    let paths: list[(pitch, option[pitch])] = [(c4, some(e4)), (g4, none)];

    fn choose(value: option[pitch], fallback: pitch = c4) -> pitch = match value {
        none -> fallback,
        some(found) -> found,
    };

    fn transform(f: pitch -> pitch, root: pitch) -> pitch = f(root);

    fn melody(root: pitch) -> music = music {
        root/4
        use answer(root);
    };

    score { part piano { voice one { c4/1 } } }
}
"#;

fn significant_shape(document: &musa_language::ParsedDocument) -> Vec<SyntaxKind> {
    let root = document.syntax();
    root.descendants_with_tokens()
        .filter_map(|element| match element {
            SyntaxElement::Node(node) => Some(node.kind()),
            SyntaxElement::Token(token) if token.kind() != SyntaxKind::Whitespace => Some(token.kind()),
            SyntaxElement::Token(_) => None,
        })
        .collect()
}

#[test]
fn expression_cst_has_one_role_for_each_surface_form() {
    let document = parse(EXPRESSIONS);
    assert!(document.errors().is_empty(), "{:?}", document.errors());
    let root = document.syntax();
    let kinds: Vec<SyntaxKind> = root.descendants().map(|node| node.kind()).collect();
    for expected in [
        SyntaxKind::LetDecl,
        SyntaxKind::FnDecl,
        SyntaxKind::FunctionType,
        SyntaxKind::ProductType,
        SyntaxKind::ListType,
        SyntaxKind::OptionType,
        SyntaxKind::ProductExpr,
        SyntaxKind::ListExpr,
        SyntaxKind::OptionExpr,
        SyntaxKind::ApplyExpr,
        SyntaxKind::MatchExpr,
        SyntaxKind::MatchArm,
        SyntaxKind::MusicExpr,
        SyntaxKind::UseStmt,
        SyntaxKind::NoteStmt,
    ] {
        assert!(kinds.contains(&expected), "missing {expected:?}");
    }
    insta::assert_debug_snapshot!(
        "expression_node_kinds",
        kinds
            .into_iter()
            .filter(|kind| {
                matches!(
                    kind,
                    SyntaxKind::LetDecl
                        | SyntaxKind::FnDecl
                        | SyntaxKind::FunctionType
                        | SyntaxKind::ProductType
                        | SyntaxKind::ListType
                        | SyntaxKind::OptionType
                        | SyntaxKind::ProductExpr
                        | SyntaxKind::ListExpr
                        | SyntaxKind::OptionExpr
                        | SyntaxKind::ApplyExpr
                        | SyntaxKind::MatchExpr
                        | SyntaxKind::MatchArm
                        | SyntaxKind::MusicExpr
                        | SyntaxKind::UseStmt
                        | SyntaxKind::NoteStmt
                )
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn formatting_round_trips_is_idempotent_and_keeps_comments() {
    let messy = EXPRESSIONS.replace("let fifth", "// theory value\n let   fifth");
    let parsed = parse(&messy);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let before = significant_shape(&parsed);
    let once = format(&parsed, BarSpacing::Compact).to_string();
    let reparsed = parse(&once);
    assert!(reparsed.errors().is_empty(), "{:?}\n{once}", reparsed.errors());
    assert_eq!(significant_shape(&reparsed), before);
    assert!(once.contains("// theory value"));
    assert_eq!(format(&reparsed, BarSpacing::Compact).to_string(), once);
}

#[test]
fn incomplete_expressions_recover_without_losing_source() {
    for broken in [
        "piece \"x\" { fn f(x: nat) -> = x; }",
        "piece \"x\" { let x: nat = f(1; }",
        "piece \"x\" { let x: nat -> = 1; }",
        "piece \"x\" { let x: option[nat] = match x { none -> }; }",
        "piece \"x\" { let x: nat = match x {}; }",
    ] {
        let document = parse(broken);
        assert!(!document.errors().is_empty(), "accepted: {broken}");
        assert_eq!(document.syntax().to_string(), broken);
    }
}

#[test]
fn a_note_line_and_a_general_expression_are_unambiguous_in_music() {
    let source = "piece \"x\" { fn p() -> music = music { c4/4 use answer(c4); }; }";
    let document = parse(source);
    assert!(document.errors().is_empty(), "{:?}", document.errors());
    let root = document.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::NoteStmt)
            .count(),
        1
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::ApplyExpr)
            .count(),
        1
    );
}

#[test]
fn pitch_translation_is_a_single_non_associative_expression_layer() {
    let source = r#"piece "pitch" {
        fn turn(root: pitch, by: interval) -> music = music {
            (root up M2)/4
            ((root up by) down m2)/4
        };
    }"#;
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::PitchExpr)
            .count(),
        3
    );
    let formatted = format(&parsed, BarSpacing::Compact).to_string();
    let reparsed = parse(&formatted);
    assert!(reparsed.errors().is_empty(), "{:?}\n{formatted}", reparsed.errors());

    let chained = parse("piece \"pitch\" { let x: pitch = c4 up M2 down m2; }");
    assert!(
        !chained.errors().is_empty(),
        "unparenthesized pitch operators must not associate"
    );
}

#[test]
fn diminished_interval_spelling_does_not_steal_the_note_d4() {
    let parsed = parse("piece \"pitch\" { let sounded: pitch = d4; let distance: interval = dim4; }");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let tokens = parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| matches!(token.kind(), SyntaxKind::PitchLiteral | SyntaxKind::IntervalLiteral))
        .map(|token| (token.kind(), token.text().to_owned()))
        .collect::<Vec<_>>();
    assert_eq!(
        tokens,
        [
            (SyntaxKind::PitchLiteral, "d4".to_owned()),
            (SyntaxKind::IntervalLiteral, "dim4".to_owned()),
        ]
    );
}

#[test]
fn repeat_is_a_statement_keyword_and_a_finite_value_operation() {
    let source =
        "piece \"x\" { let copies: list[nat] = repeat(1, 4); score { part p { voice v { repeat 2 { c4/4 } } } } }";
    let document = parse(source);
    assert!(document.errors().is_empty(), "{:?}", document.errors());
    let root = document.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::ApplyExpr)
            .count(),
        1
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::RepeatStmt)
            .count(),
        1
    );
}
