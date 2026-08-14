use musa_language::{BarSpacing, SyntaxElement, SyntaxKind, format, parse};

const EXPRESSIONS: &str = r#"piece "Expressions" {
    let fifth: Interval = P5;
    let paths: List<(Pitch, Option<Pitch>)> = [(c4, Some(e4)), (g4, None)];

    fn choose(value: Option<Pitch>, fallback: Pitch) -> Pitch { match value {
        None -> fallback,
        Some(found) -> found,
    } }

    fn transform(f: Pitch -> Pitch, root: Pitch) -> Pitch { f(root) }

    let raised: Pitch = transform(fn (from: Pitch) -> Pitch { choose(Some(from), c4) }, e4);

    fn reason(outcome: Result<Pitch, Text>) -> Text { match outcome {
        Ok(found) -> "",
        Err(said) -> said,
    } }

    let attempted: Result<Pitch, Text> = Ok(c4);
    let refused: Result<Pitch, Text> = Err("no such note");

    fn melody(root: Pitch) -> Music { music {
        root/4
        use answer(root);
    } }

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
        SyntaxKind::ResultType,
        SyntaxKind::ProductExpr,
        SyntaxKind::ListExpr,
        SyntaxKind::OptionExpr,
        SyntaxKind::ResultExpr,
        SyntaxKind::ApplyExpr,
        SyntaxKind::LambdaExpr,
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
                        | SyntaxKind::ResultType
                        | SyntaxKind::ProductExpr
                        | SyntaxKind::ListExpr
                        | SyntaxKind::OptionExpr
                        | SyntaxKind::ResultExpr
                        | SyntaxKind::ApplyExpr
                        | SyntaxKind::LambdaExpr
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
        "piece \"x\" { fn f(x: Nat) -> { x } }",
        "piece \"x\" { let x: Nat = f(1; }",
        "piece \"x\" { let x: Nat -> = 1; }",
        "piece \"x\" { let x: Option<Nat> = match x { None -> }; }",
        "piece \"x\" { let x: Nat = match x {}; }",
    ] {
        let document = parse(broken);
        assert!(!document.errors().is_empty(), "accepted: {broken}");
        assert_eq!(document.syntax().to_string(), broken);
    }
}

#[test]
fn a_note_line_and_a_general_expression_are_unambiguous_in_music() {
    let source = "piece \"x\" { fn p() -> Music { music { c4/4 use answer(c4); } } }";
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
        fn turn(root: Pitch, by: Interval) -> Music { music {
            (root up M2)/4
            ((root up by) down m2)/4
        } }
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

    let chained = parse("piece \"pitch\" { let x: Pitch = c4 up M2 down m2; }");
    assert!(
        !chained.errors().is_empty(),
        "unparenthesized pitch operators must not associate"
    );
}

#[test]
fn diminished_interval_spelling_does_not_steal_the_note_d4() {
    let parsed = parse("piece \"pitch\" { let sounded: Pitch = d4; let distance: Interval = dim4; }");
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
        "piece \"x\" { let copies: List<Nat> = repeat(1, 4); score { part p { voice v { repeat 2 { c4/4 } } } } }";
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

/// The ladder a conditional is written as survives being formatted, and the
/// formatting is a fixpoint.
///
/// `else if` is not a form of its own — the alternative of one conditional is
/// another conditional — so the thing under test is that the printer still
/// writes the nesting back as one ladder, with each rung opening at the same
/// indent, rather than stepping one level further in per rung the way the
/// nested `match` this replaces did.
#[test]
fn a_conditional_ladder_round_trips_and_formats_to_itself() {
    let source = "piece \"x\" { fn count(said: Text) -> Nat {\n\
         if text_equal(said, \"none\") { 0 } else if text_equal(said, \"one\") { 1 } \
         else if text_equal(said, \"two\") { 2 } else { 9 } } }";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let before = significant_shape(&parsed);

    let once = format(&parsed, BarSpacing::Compact).to_string();
    let reparsed = parse(&once);
    assert!(reparsed.errors().is_empty(), "{:?}\n{once}", reparsed.errors());
    assert_eq!(
        significant_shape(&reparsed),
        before,
        "formatting changed the tree:\n{once}"
    );
    assert_eq!(
        format(&reparsed, BarSpacing::Compact).to_string(),
        once,
        "formatting is not a fixpoint"
    );

    // Three rungs and a final `else`: three `IfExpr` nodes, nested through
    // their alternatives, and no `MatchExpr` — the elaboration to a boolean
    // match happens in the compiler, not in the tree the author edits.
    let root = reparsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::IfExpr)
            .count(),
        3
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::MatchExpr)
            .count(),
        0
    );

    // Each rung opens where the one before it closed.
    let rungs = once
        .lines()
        .filter(|line| line.trim_start().starts_with("} else"))
        .map(|line| line.len() - line.trim_start().len())
        .collect::<Vec<_>>();
    assert_eq!(rungs.len(), 3, "expected three `else` rungs in:\n{once}");
    assert!(
        rungs.windows(2).all(|pair| pair.first() == pair.last()),
        "rungs are indented apart in:\n{once}"
    );
}

/// `if` and `else` are keywords now, so the names are spoken for.
///
/// A language that let `if` be a variable would have to decide which reading
/// wins at every use, and the answer that costs nothing is that it cannot be a
/// variable. The check is that the parser refuses rather than quietly reading
/// the keyword as a name, and that it refuses in the words it already uses for
/// every other keyword — `match` and `data` are the same sentence, and a
/// message invented for these two would be one more thing to keep true.
#[test]
fn if_and_else_are_keywords_and_not_available_as_names() {
    for (taken, word) in [
        ("piece \"x\" { let if: Nat = 1; }", "if"),
        ("piece \"x\" { let else: Nat = 1; }", "else"),
        ("piece \"x\" { fn if(n: Nat) -> Nat { n } }", "if"),
        ("piece \"x\" { fn f(if: Nat) -> Nat { n } }", "if"),
    ] {
        let document = parse(taken);
        // The leading diagnostic, not merely one somewhere in the list: a
        // keyword used as a name derails the parse, and everything after the
        // first message is that derailment rather than the mistake.
        let leading = document.errors().first().map(|first| {
            let at: std::ops::Range<usize> = first.range().into();
            (
                first.message().ends_with(&format!("found `{word}`")),
                // At the keyword's own span, not the declaration around it.
                taken.get(at).unwrap_or_default().to_owned(),
            )
        });
        assert_eq!(
            leading,
            Some((true, word.to_owned())),
            "`{word}` must be refused by name and at its own span: {taken}\n{:?}",
            document.errors()
        );
        assert_eq!(document.syntax().to_string(), taken);
    }
}
