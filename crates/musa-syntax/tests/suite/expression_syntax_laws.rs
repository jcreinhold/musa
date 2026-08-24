use musa_syntax::{BarSpacing, SyntaxElement, SyntaxKind, format, parse};

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

    fn melody(root: Pitch) -> EventTrack<WrittenTime> { music {
        root/4
        use answer(root);
    } }

    score { part piano { voice one { c4/1 } } }
}
"#;

fn significant_shape(document: &musa_syntax::ParsedDocument) -> Vec<SyntaxKind> {
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
    let source = "piece \"x\" { fn p() -> EventTrack<WrittenTime> { music { c4/4 use answer(c4); } } }";
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
        fn turn(root: Pitch, by: Interval) -> EventTrack<WrittenTime> { music {
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

/// `repeat` opens a repeated passage and nothing else.
///
/// It was a statement keyword *and* a value operation until prompt 142 wrote
/// the eight collection eliminators as library code and found that one of the
/// eight had no name available: `fn repeat` does not parse, so the operation is
/// `repeated` in `std::list` and the keyword keeps the word. The law is the same
/// shape it always was — one call and one repeated passage in one piece — and
/// what it now says is that the call is an ordinary identifier's.
#[test]
fn repeat_is_a_statement_keyword_and_the_list_operation_is_a_name() {
    let source =
        "piece \"x\" { let copies: List<Nat> = repeated(1, 4); score { part p { voice v { repeat 2 { c4/4 } } } } }";
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

/// A record update round-trips and formats to a fixpoint, with one field and
/// with many, and a chain of updates stays a chain.
///
/// The many-field case is the one that has to be looked at: a field list is
/// laid out the way a `data` declaration's is, one field to a line once it no
/// longer fits, rather than being run together the way a call's arguments are.
/// The chain matters because `p with { … } with { … }` is two updates and not
/// a longer field list — the second updates the first's result — so the tree
/// has to keep them nested and the printer has to write them back that way.
#[test]
fn a_record_update_round_trips_with_one_field_and_with_many() {
    let source = "piece \"x\" {\n\
         data Pending {\n\
             Pending(read: Nat, length: Nat, dots: Nat, tying: Nat, numbers: Nat, taken: Nat)\n\
         }\n\
         fn one(held: Pending) -> Pending { held with { dots = 1 } }\n\
         fn many(held: Pending) -> Pending { held with { read = 1, length = 2, dots = 3, tying = 4, numbers = 5, taken = 6 } }\n\
         fn chained(held: Pending) -> Pending { held with { dots = 1 } with { tying = 2 } }\n\
     }";
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

    let root = reparsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::RecordUpdateExpr)
            .count(),
        4,
        "one, many, and two chained updates:\n{once}"
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::FieldUpdate)
            .count(),
        9,
        "one field, six fields, and one each in the chain:\n{once}"
    );

    // The one-field update stays on its line; the six-field one does not.
    assert!(
        once.lines().any(|line| line.contains("held with { dots = 1 } }")),
        "a one-field update was broken open:\n{once}"
    );
    assert!(
        once.lines().any(|line| line.trim() == "read = 1,"),
        "a field list that does not fit must lay out one field to a line:\n{once}"
    );
}

/// `with` after a `use` value belongs to the statement, and `with` after any
/// other expression is an update.
///
/// The two forms were spelled with the same word before this prompt existed —
/// `use theme() with { note 3 = a5; }` is the occurrence override, and
/// `examples/variation.musa` writes it — so the reading a reader already has is
/// the one the statement keeps. Only the *top level* of a `use` value stands
/// down: an update written inside an argument is an ordinary update, which is
/// what the third case here pins.
#[test]
fn a_use_statement_keeps_its_with_and_an_expression_does_not() {
    for (source, updates, overrides) in [
        (
            "piece \"x\" { score { part p { voice v { use theme() with { note 3 = a5; } } } } }",
            0,
            1,
        ),
        (
            "piece \"x\" { data P { P(a: Nat) } fn f(p: P) -> P { p with { a = 1 } } }",
            1,
            0,
        ),
        (
            "piece \"x\" { data P { P(a: Nat) } fn t(p: P) -> EventTrack<WrittenTime> { music { c4/4 } } \
             score { part p { voice v { use t(held with { a = 1 }); } } } }",
            1,
            0,
        ),
    ] {
        let parsed = parse(source);
        assert!(parsed.errors().is_empty(), "{source}\n{:?}", parsed.errors());
        let root = parsed.syntax();
        let counted = |kind| root.descendants().filter(|node| node.kind() == kind).count();
        assert_eq!(
            (counted(SyntaxKind::RecordUpdateExpr), counted(SyntaxKind::WithClause)),
            (updates, overrides),
            "`with` was claimed by the wrong form in: {source}"
        );
    }
}

/// `?` binds tighter than anything it can be written after, and the formatter
/// keeps it against its subject.
///
/// Postfix operators are where a grammar quietly acquires ambiguity, so the
/// cases here are the three that could have gone wrong: a question asked of a
/// call, a question asked of a question, and a question asked of a record
/// update, each of which has to take the *whole* preceding expression as its
/// subject rather than its last fragment. Formatting round-trips because a
/// space before `?` would read as a stray token, and the formatter's idempotence
/// law is what makes that stay true.
#[test]
fn a_question_takes_the_whole_expression_before_it() {
    for (source, questions) in [
        (
            "piece \"x\" { fn f(r: Result<Nat, Text>) -> Result<Nat, Text> { Ok(r?) } }",
            1,
        ),
        (
            "piece \"x\" { fn f(r: Result<Result<Nat, Text>, Text>) -> Result<Nat, Text> { Ok(r??) } }",
            2,
        ),
        (
            "piece \"x\" { data P { P(a: Result<Nat, Text>) } \
             fn f(p: P, r: Result<Nat, Text>) -> Result<Nat, Text> { Ok(g(p with { a = r })?) } \
             fn g(p: P) -> Result<Nat, Text> { match p { P(a) -> a } } }",
            1,
        ),
    ] {
        let parsed = parse(source);
        assert!(parsed.errors().is_empty(), "{source}\n{:?}", parsed.errors());
        let root = parsed.syntax();
        assert_eq!(
            root.descendants()
                .filter(|node| node.kind() == SyntaxKind::QuestionExpr)
                .count(),
            questions,
            "the wrong number of questions was parsed in: {source}"
        );

        let once = format(&parsed, BarSpacing::Compact).to_string();
        assert!(
            !once.contains(" ?"),
            "the formatter loosened `?` from its subject:\n{once}"
        );
        let reparsed = parse(&once);
        assert!(
            reparsed.errors().is_empty(),
            "the formatted text no longer parses:\n{once}"
        );
        assert_eq!(
            format(&reparsed, BarSpacing::Compact).to_string(),
            once,
            "formatting `?` is not idempotent:\n{once}"
        );
    }
}

/// A sub-position holds another pattern, and the tree says so.
///
/// `docs/rules/language/01-surface.md` §1's **Patterns nest** paragraph is the
/// rule; the CST is where it becomes readable. Each sub-position is a
/// `Pattern` node rather than a bare binder token, so the lowering walks the
/// same shape at every depth instead of switching to token-reading one level
/// down. The formatter round-trip is here for the same reason it is on every
/// other form: a node the printer does not know is a node that loses text.
#[test]
fn a_sub_position_of_a_pattern_holds_another_pattern() {
    let source = r"library {
    data Inner { Quiet, Loud(count: Nat) }
    data Outer { Wrap(held: Inner) }
    record Marking { written: Inner; }

    fn wrapped(o: Outer) -> Nat { match o { Wrap(Loud(count)) -> count, Wrap(Quiet) -> 0 } }

    fn listed(l: List<Inner>) -> Nat {
        match l { [] -> 0, [Loud(count), .. others] -> count, [Quiet, .. others] -> 0 }
    }

    fn marked(m: Marking) -> Nat {
        match m { Marking { written = Loud(count) } -> count, Marking { written = Quiet } -> 0 }
    }

    fn held(h: Option<Inner>) -> Nat {
        match h { Some(Loud(count)) -> count, Some(Quiet) -> 0, None -> 0 }
    }

    fn paired(p: (Inner, Nat)) -> Nat { match p { (Loud(count), after) -> count, (Quiet, after) -> after } }
}
";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let nested = parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::Pattern)
        .filter(|node| node.parent().is_some_and(|held| held.kind() == SyntaxKind::Pattern))
        .count();
    assert_eq!(
        nested, 17,
        "3 in `wrapped`, 5 in `listed`, 1 in `marked`, 3 in `held`, 5 in `paired` — \
         a record's sub-position hangs under its `FieldPattern` and so is not counted here"
    );

    let once = format(&parsed, BarSpacing::Compact).to_string();
    let reparsed = parse(&once);
    assert!(
        reparsed.errors().is_empty(),
        "the formatted text no longer parses:\n{once}"
    );
    assert_eq!(
        format(&reparsed, BarSpacing::Compact).to_string(),
        once,
        "formatting a nested pattern is not idempotent:\n{once}"
    );
}

/// `Bass | Tenor` is one pattern written as several.
///
/// `docs/rules/language/01-surface.md` §1 admits alternation at every position
/// a pattern stands. The tree says which nodes are the alternatives — the `|`
/// is the parent's own token and each alternative is a `Pattern` child — and a
/// pattern with no `|` in it is the node it always was, which the counts below
/// are what check.
#[test]
fn an_alternation_is_one_pattern_holding_its_alternatives() {
    let source = r"library {
    data Clef { Treble, Bass, Alto, Tenor }
    data Inner { Quiet, Loud(count: Nat) }
    data Outer { Wrap(held: Inner), Hold(held: Inner) }

    fn low(written: Clef) -> Bool { match written { Bass | Tenor -> true, Treble | Alto -> false } }

    fn plain(o: Outer) -> Nat { match o { Wrap(Loud(_) | Quiet) -> 0, Hold(_) -> 1 } }
}
";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let alternations = parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::Pattern)
        .filter(|node| {
            node.children_with_tokens()
                .filter_map(|held| held.into_token())
                .any(|token| token.kind() == SyntaxKind::Pipe)
        })
        .count();
    assert_eq!(alternations, 3, "two in `low`, one inside `Wrap` in `plain`");

    let once = format(&parsed, BarSpacing::Compact).to_string();
    assert!(
        once.contains("Bass | Tenor"),
        "the formatter lost the spacing around `|`:\n{once}"
    );
    let reparsed = parse(&once);
    assert!(
        reparsed.errors().is_empty(),
        "the formatted text no longer parses:\n{once}"
    );
    assert_eq!(
        format(&reparsed, BarSpacing::Compact).to_string(),
        once,
        "formatting an alternation is not idempotent:\n{once}"
    );
}

/// `let name = value; body` is one expression, wherever an expression stands.
///
/// `docs/rules/language/01-surface.md` §1 admits it in a block, and a block is
/// not the only place an expression is read: a match arm's result and a
/// `quote at here { … }`'s interior are the same production. What the tree
/// says is that several bindings are several nodes nested rightward — the
/// second `LetExpr` is a *child* of the first and not a sibling — which is
/// what makes the block still hold exactly one expression.
#[test]
fn a_binding_is_one_expression_holding_its_body() {
    let source = r#"library {
    fn stacked(x: Text) -> Text {
        let one: Text = text_join([x, "a"]);
        let two = text_join([one, "b"]);
        text_join([one, two])
    }

    fn in_an_arm(written: Bool) -> Text { match written { True -> let word = "yes"; word, False -> "no" } }

    fn in_a_quote(x: Syntax<TokenTree>) -> Syntax<TokenTree> { quote at here { let held = $x; held } }
}
"#;
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let bindings: Vec<_> = parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::LetExpr)
        .collect();
    assert_eq!(bindings.len(), 4, "two in `stacked`, one in the arm, one in the quote");

    // Rightward nesting, as a fact about the tree rather than about the text:
    // `stacked`'s second binding hangs under its first, so the block below
    // them holds one expression and not three.
    let nested = bindings
        .iter()
        .filter(|node| node.parent().is_some_and(|owner| owner.kind() == SyntaxKind::LetExpr))
        .count();
    assert_eq!(nested, 1, "`let two` is a child of `let one`");

    let once = format(&parsed, BarSpacing::Compact).to_string();
    let reparsed = parse(&once);
    assert!(
        reparsed.errors().is_empty(),
        "the formatted text no longer parses:\n{once}"
    );
    assert_eq!(
        format(&reparsed, BarSpacing::Compact).to_string(),
        once,
        "formatting a binding is not idempotent:\n{once}"
    );
}
