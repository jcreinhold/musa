//! `10-traits.md` §5 and §6 as syntax: what a trait, an instance, an operator,
//! a method call, and an index look like, and what the four spellings the
//! operators borrowed still mean where they were already spoken for.
//!
//! Stated over one program rather than a fragment per law, for the reason
//! `record_syntax_laws.rs` gives: a law that held only for the snippet it was
//! written beside would be a law about that snippet. The music statements are
//! the exception and are their own fixture, because what they claim is that
//! *nothing changed* — and a program that also declares traits could not say
//! so about the file that does not.

// A fixture that does not hold what the fixture says it holds is a defect in
// this file, not a property of any law — the same reason `editing_laws.rs`
// makes this allowance.
#![allow(clippy::panic)]

use musa_language::ast::{AstNode, FnDecl, ImplDecl, TraitDecl};
use musa_language::{BarSpacing, SyntaxElement, SyntaxKind, format, parse};

/// The program every trait and operator law here is stated over.
///
/// `Eq` and `Ord` are `10-traits.md` §5's own two traits, `Ord` carrying the
/// super-constraint that makes a `where` on a *trait* worth reading. `Held` is
/// the shape §4's measure is about, `Duration` is §6's inherent namespace, and
/// `same` is the constrained generic function whose body is the operator law:
/// `x == y` and `Eq.equal(x, y)` are the same term because the parser writes
/// the first as the second.
const TRAITS: &str = r#"piece "Traits" {
    trait Eq<A> {
        fn equal(x: A, y: A) -> Bool;
    }

    trait Ord<A> where Eq<A> {
        fn less(x: A, y: A) -> Bool;

        fn at_most(x: A, y: A) -> Bool { less(x, y) }
    }

    private trait Held<A> {
        fn holder(x: A) -> Nat;
    }

    impl Eq<Nat> {
        fn equal(x: Nat, y: Nat) -> Bool { nat_equal(x, y) }
    }

    private impl<A> Held<List<A>> where Held<A> {
        fn holder(x: List<A>) -> Nat { 1 }
    }

    impl Duration {
        fn of(r: Nat) -> Nat { r }
    }

    fn same<A>(x: A, y: A) -> Bool where Eq<A> { x == y }

    fn spread(a: Nat, b: Nat, c: Nat) -> Nat { a + b * c }

    fn narrow(a: Nat, b: Nat) -> Bool { a - b < c / d }

    fn first(xs: List<Nat>) -> Nat { xs[0] }

    fn tallied(xs: List<Nat>) -> Nat { counted(xs).holder() }
}
"#;

/// The significant tokens of a tree, which formatting may not change.
fn significant_shape(parsed: &musa_language::ParsedDocument) -> Vec<(SyntaxKind, String)> {
    parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| match element {
            SyntaxElement::Token(token) => (!matches!(
                token.kind(),
                SyntaxKind::Whitespace | SyntaxKind::LineComment | SyntaxKind::BlockComment
            ))
            .then(|| (token.kind(), token.text().to_owned())),
            SyntaxElement::Node(_) => None,
        })
        .collect()
}

fn count(parsed: &musa_language::ParsedDocument, kind: SyntaxKind) -> usize {
    parsed.syntax().descendants().filter(|node| node.kind() == kind).count()
}

/// Every new form parses, and the tree holds the node each one was written as.
#[test]
fn traits_and_operators_parse_into_their_own_nodes() {
    let parsed = parse(TRAITS);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    assert_eq!(count(&parsed, SyntaxKind::TraitDecl), 3, "Eq, Ord, and Held");
    assert_eq!(count(&parsed, SyntaxKind::ImplDecl), 3, "two instances and a namespace");
    assert_eq!(
        count(&parsed, SyntaxKind::WhereClause),
        3,
        "`Ord`, the `List` instance, and `same`"
    );
    assert_eq!(count(&parsed, SyntaxKind::Constraint), 3, "one constraint apiece");
    // `x == y`, `a + b * c`, `a - b < c / d` — one, two, and three.
    assert_eq!(count(&parsed, SyntaxKind::BinaryExpr), 6);
    assert_eq!(count(&parsed, SyntaxKind::IndexExpr), 1, "`xs[0]`");
    assert_eq!(count(&parsed, SyntaxKind::MethodCallExpr), 1, "`counted(xs).holder()`");
}

/// The typed wrappers say what a declaration is, so no consumer counts tokens.
///
/// A required method is one whose body is absent, which is the whole of the
/// difference between a field an instance must fill and one it may replace
/// (`10-traits.md` §1). Reading it as "no body" rather than as a flag is what
/// keeps a trait's methods the same node as every other function.
#[test]
fn a_required_method_is_one_with_no_body() {
    let parsed = parse(TRAITS);
    let traits: Vec<TraitDecl> = parsed.syntax().descendants().filter_map(TraitDecl::cast).collect();

    let named: Vec<(String, Vec<(String, bool)>)> = traits
        .iter()
        .filter_map(|declaration| {
            Some((
                declaration.name()?,
                declaration
                    .methods()
                    .iter()
                    .filter_map(|method| Some((method.name()?, method.body().is_some())))
                    .collect(),
            ))
        })
        .collect();
    assert_eq!(
        named,
        vec![
            ("Eq".to_owned(), vec![("equal".to_owned(), false)]),
            (
                "Ord".to_owned(),
                vec![("less".to_owned(), false), ("at_most".to_owned(), true)]
            ),
            ("Held".to_owned(), vec![("holder".to_owned(), false)]),
        ]
    );

    // The parameters and the super-constraint, read the same way.
    let ord = traits
        .iter()
        .find(|declaration| declaration.name().as_deref() == Some("Ord"))
        .unwrap_or_else(|| panic!("`Ord` is declared in the fixture"));
    assert_eq!(ord.parameters(), vec!["A".to_owned()]);
    let written: Vec<String> = ord
        .constraints()
        .iter()
        .filter_map(|constraint| Some(constraint.ty()?.text().to_string().trim().to_owned()))
        .collect();
    assert_eq!(written, vec!["Eq<A>".to_owned()]);

    // An impl reads as its head, its parameters, and its constraints, and the
    // wrapper declines to say which of §6's two forms it is.
    let instances: Vec<ImplDecl> = parsed.syntax().descendants().filter_map(ImplDecl::cast).collect();
    let heads: Vec<String> = instances
        .iter()
        .filter_map(|instance| Some(instance.head()?.text().to_string().trim().to_owned()))
        .collect();
    assert_eq!(
        heads,
        vec!["Eq<Nat>".to_owned(), "Held<List<A>>".to_owned(), "Duration".to_owned()]
    );
    let Some(held) = instances.get(1) else {
        panic!("the fixture declares three impls");
    };
    assert_eq!(held.parameters(), vec!["A".to_owned()]);
    assert_eq!(held.constraints().len(), 1);
    assert!(held.is_private(), "the marker hides the name, never the instance");
    assert!(
        instances.first().is_some_and(|first| !first.is_private()),
        "and an unmarked impl is public, as every declaration is"
    );
}

/// A constrained function carries its constraint, and its body is the term the
/// operator stood for.
#[test]
fn a_where_clause_is_read_where_it_is_written() {
    let parsed = parse(TRAITS);
    let same = parsed
        .syntax()
        .descendants()
        .filter_map(FnDecl::cast)
        .find(|declaration| declaration.name().as_deref() == Some("same"))
        .unwrap_or_else(|| panic!("`same` is declared in the fixture"));
    let written: Vec<String> = same
        .constraints()
        .iter()
        .filter_map(|constraint| Some(constraint.ty()?.text().to_string().trim().to_owned()))
        .collect();
    assert_eq!(written, vec!["Eq<A>".to_owned()]);
    assert!(same.body().is_some(), "a function outside a trait has a body");
}

/// The precedence table, read off the tree rather than asserted about it.
///
/// `01-surface.md` §1 fixes six levels, and the two that matter here are that
/// `*` binds tighter than `+` and that both bind tighter than `<`. A tree that
/// nests the other way would still round-trip and still format, so the shape
/// is what has to be checked.
#[test]
fn the_operator_table_is_the_one_section_one_fixes() {
    // `a + b * c` is `a + (b * c)`: the outer operator is `+`, and its right
    // operand is the whole product.
    let sum = operator_tree("a + b * c");
    assert_eq!(sum, "+(a, *(b, c))");

    // `a - b < c / d` is `(a - b) < (c / d)`: comparison is the loosest level
    // and takes both arithmetic expressions whole.
    assert_eq!(operator_tree("a - b < c / d"), "<(-(a, b), /(c, d))");

    // Left-associative within a level, so a chain reads left to right.
    assert_eq!(operator_tree("a - b - c"), "-(-(a, b), c)");

    // A comparison is non-associative: the second `==` is left unread, which
    // the parser reports rather than folding into a comparison against a
    // `Bool`.
    let chained = parse(&program("a == b == c"));
    assert!(!chained.errors().is_empty(), "`a == b == c` is refused");
}

/// Arithmetic binds tighter than the written-pitch operators, which is the
/// reading a musician wants: `c4 up M3 + P5` transposes by the sum.
#[test]
fn arithmetic_binds_tighter_than_up_and_down() {
    let parsed = parse(&program("c4 up M3 + P5"));
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let pitch = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::PitchExpr)
        .unwrap_or_else(|| panic!("`up` builds a PitchExpr"));
    assert!(
        pitch.children().any(|child| child.kind() == SyntaxKind::BinaryExpr),
        "the sum is the operand of `up`, not the other way round:\n{pitch:#?}"
    );
}

/// The four places the borrowed spellings were already spoken for.
///
/// `-`, `/`, `<`, and `>` were in the lexer before this prompt and were read
/// only by music statements and type positions. Putting them in the expression
/// grammar is safe exactly to the extent that these still parse the way they
/// did, so each is a fixture rather than an argument. The load-bearing line is
/// the transposed note: its pitch is an *expression* and its duration is a
/// `/`, so a note whose pitch ran to the end of the arithmetic grammar would
/// have swallowed the duration.
#[test]
fn the_statements_that_own_the_operator_spellings_still_parse() {
    let parsed = parse(MUSIC);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    // Durations: one per note, including the transposed one.
    assert_eq!(count(&parsed, SyntaxKind::Duration), 5);
    assert_eq!(count(&parsed, SyntaxKind::NoteStmt), 5);
    // The transposed note keeps its duration.
    assert_eq!(count(&parsed, SyntaxKind::PitchExpr), 1);
    // Type arguments: `<` and `>` around a parameter, never a comparison.
    assert_eq!(count(&parsed, SyntaxKind::ListType), 1);
    // The accent mark, which is `>` and is not `Ord`.
    assert_eq!(count(&parsed, SyntaxKind::ArticulationList), 1);
    // A hairpin, spelled in words rather than in the `<` the page draws.
    assert_eq!(count(&parsed, SyntaxKind::HairpinStmt), 1);
    // The negative rational of a send level.
    assert_eq!(count(&parsed, SyntaxKind::SendStmt), 1);
    // And nothing here is an operator.
    assert_eq!(count(&parsed, SyntaxKind::BinaryExpr), 0);
}

/// A method call on a *name* is a name and a call, because the parser cannot
/// know whether the name is a module or a value.
///
/// `low.rise()` reaches into an aliased module and `x.equal(y)` calls a method,
/// and the two are the same three tokens. Which was written is decided by what
/// the first word denotes, which is a question about declarations —
/// `10-traits.md` §6's exact-receiver lookup answers it where the answer is
/// known. `MethodCallExpr` is for the receivers no name can spell.
#[test]
fn a_method_call_is_a_node_only_where_a_name_could_not_have_been_written() {
    let named = parse(&program("low.rise()"));
    assert!(named.errors().is_empty(), "{:?}", named.errors());
    assert_eq!(count(&named, SyntaxKind::MethodCallExpr), 0);
    assert_eq!(count(&named, SyntaxKind::NameExpr), 1, "`low.rise` is one name");

    for source in ["f(x).m(y)", "xs[0].m(y)", "(p).m(y)"] {
        let parsed = parse(&program(source));
        assert!(parsed.errors().is_empty(), "{source}: {:?}", parsed.errors());
        assert_eq!(
            count(&parsed, SyntaxKind::MethodCallExpr),
            1,
            "{source} has a computed receiver"
        );
    }
}

/// The whole program round-trips losslessly and formats to a fixpoint.
#[test]
fn traits_and_operators_round_trip_and_format_to_a_fixpoint() {
    for source in [TRAITS, MUSIC] {
        let parsed = parse(source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
        let before = significant_shape(&parsed);
        assert_eq!(parsed.syntax().text().to_string(), source, "parsing is lossless");

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
            "formatting is not a fixpoint:\n{once}"
        );
    }
}

/// The layout the formatter decided, stated as the lines it must write.
///
/// An operator is a word between two operands and takes a space either side,
/// including the three spellings — `/`, `<`, `-` — that close up everywhere
/// else. An index closes up to what it indexes, the way a call does.
#[test]
fn an_operator_is_spaced_and_an_index_is_not() {
    let once = format(&parse(TRAITS), BarSpacing::Compact).to_string();
    for wanted in [
        "x == y",
        "a + b * c",
        "a - b < c / d",
        "xs[0]",
        "counted(xs).holder()",
        "trait Eq<A> {",
        "trait Ord<A> where Eq<A> {",
        "private impl<A> Held<List<A>> where Held<A> {",
        "fn same<A>(x: A, y: A) -> Bool where Eq<A> {",
        "fn equal(x: A, y: A) -> Bool;",
    ] {
        assert!(once.contains(wanted), "expected `{wanted}`:\n{once}");
    }
}

/// What the grammar refuses, and that it says something about each one.
#[test]
fn the_grammar_refuses_what_sections_five_and_six_refuse() {
    // §9: a trait with no parameter has nothing to dispatch on.
    let headless = parse("piece \"x\" {\n    trait Nothing {\n    }\n}\n");
    assert!(!headless.errors().is_empty(), "a trait dispatches on a parameter");

    // A `;` where a body belongs means *required*, and only a trait requires
    // anything. Elsewhere it is the missing body it has always been.
    let bodiless = parse("piece \"x\" {\n    fn f(x: Nat) -> Nat;\n}\n");
    assert!(!bodiless.errors().is_empty(), "a function outside a trait has a body");

    // There is no user-defined operator, so a symbol the table does not hold
    // is what it was before: not an expression.
    let invented = parse(&program("a @ b"));
    assert!(!invented.errors().is_empty(), "the operator table is closed");

    // §5's table has no `>`; `>` after a note is the accent mark, and in an
    // expression it is nothing.
    let greater = parse(&program("a > b"));
    assert!(!greater.errors().is_empty(), "`Ord` has one method, and it is `<`");
}

/// A one-expression function body, as the smallest program that holds one.
fn program(body: &str) -> String {
    format!("piece \"x\" {{\n    fn f() -> Nat {{ {body} }}\n}}\n")
}

/// The operator tree of `source`, written prefix.
///
/// Prefix rather than a node dump, because what is being claimed is the
/// *shape* — which operator is outermost and what its operands are — and a
/// dump would bury that under every token the expression is made of.
fn operator_tree(source: &str) -> String {
    let parsed = parse(&program(source));
    assert!(parsed.errors().is_empty(), "{source}: {:?}", parsed.errors());
    let outermost = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::BinaryExpr)
        .unwrap_or_else(|| panic!("an operator was written"));
    written(&outermost)
}

fn written(node: &musa_language::SyntaxNode) -> String {
    if node.kind() != SyntaxKind::BinaryExpr {
        return node.text().to_string().trim().to_owned();
    }
    let operator = node
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|token| !token.kind().is_trivia())
        .map(|token| token.text().to_owned())
        .unwrap_or_default();
    let operands: Vec<String> = node.children().map(|child| written(&child)).collect();
    format!("{operator}({})", operands.join(", "))
}

/// The music every "nothing changed" law is stated over.
///
/// One of each: a duration written after a pitch and after a rational, a
/// transposed note whose duration follows the transposition, an accent mark, a
/// hairpin, a type argument, and the negative rational of a send level.
const MUSIC: &str = r#"piece "Music" {
    tempo 1/4 = 96;
    meter 7/8;
    key c major;

    let sizes: List<Nat> = counted(4);

    score {
        part p {
            voice v {
                c5/4 d5/4. e5/8>
                (c5) up M3 /4
                crescendo to f {
                    g5/4
                }
            }
        }
    }

    studio {
        patch pad {
            saw();
        }

        bus hall {
            reverb(room: 0.82, damping: 0.55);
        }

        assign p -> pad;
        send p -> hall at -18 dB;
        route hall -> master;
    }
}
"#;
