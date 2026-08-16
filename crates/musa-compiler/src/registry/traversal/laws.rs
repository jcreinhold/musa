//! What has to be true of the two traversals.
//!
//! Unit tests inside the crate rather than a file in `tests/suite/`, and forced
//! for the reason [`crate::registry::laws`] gives at greater length: the answer
//! being agreed *with* is the old evaluator's, and `expand_region`,
//! `read_region`, and `Syntax` are all private to this crate. A test outside it
//! links against `parse`/`compile`/`render_notation` and can reach none of them.
//!
//! # What "the same answer" is measured on
//!
//! One region, read once, handed to both. The old side runs a `.musa`
//! transformer through `expand_region`, which is the whole of `eval_syntax`
//! reached the way an adapter reaches it; the new side runs the same algebra as
//! a core program against the registry. Both answer a whole [`Syntax`] value, so
//! the comparison loses nothing — paths, kinds, delimiters, and children all have
//! to agree, not just a summary of them.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a law that cannot fail loudly is not a law"
)]

use musa_core::{Cx, Raw, RawArm, RawPattern, Term};

use crate::core::expand_region;
use crate::registry::rules::Kind;
use crate::registry::{HERE, held, literal, owned, plain_type, syntax_type};
use crate::syntax::{Cat, Delimiter, ExpansionPath, Syntax, read_region};

/// The region both sides read.
///
/// Three shapes in one tree, which is what the law needs: identifiers
/// (`melody`, `together`, `a`, `b`), tokens (`let`, `=`, `,`), and groups nested
/// two deep (the layout group holding the parenthesis group).
const REGION: &str = "let melody = together(a, b)";

fn expansion() -> ExpansionPath {
    ExpansionPath::at(vec![7])
}

/// The tree, read once, exactly as the compiler reads it for an adapter.
fn subject() -> Syntax {
    read_region(&musa_language::parse(REGION).syntax(), expansion())
}

/// What the old evaluator answers for `transformer` on the region.
fn old(transformer: &str) -> Syntax {
    let written = format!("fn (region) {{ Ok({transformer}) }}");
    expand_region(&written, REGION, expansion()).expect("the old evaluator runs the transformer")
}

/// What the core answers for `program`, which must be a `Syntax ⟨token-tree⟩`.
fn new(cx: &Cx, program: &Raw) -> Syntax {
    let ty = syntax_type(Cat::TokenTree);
    let term = musa_core::check(cx, &ty, program).unwrap_or_else(|why| panic!("the program does not check: {why}"));
    let normal =
        musa_core::normalize(cx, &ty, &term).unwrap_or_else(|why| panic!("the program does not reduce: {why}"));
    musa_core::well_typed(cx, &ty, &normal).unwrap_or_else(|why| panic!("the answer does not re-check: {why}"));
    let musa_core::Shape::Lit(ref answer) = *normal.shape() else {
        panic!("the answer is not a literal: {normal:?}")
    };
    held::<Syntax>(answer).expect("the answer is a syntax value").clone()
}

// ---- writing the core side --------------------------------------------------

fn var(name: &str) -> Raw {
    Raw::var(HERE, name)
}

fn apply(head: Raw, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    arguments
        .into_iter()
        .fold(head, |function, argument| Raw::app(HERE, function, argument))
}

fn lambda(names: &[&str], body: Raw) -> Raw {
    names
        .iter()
        .rev()
        .fold(body, |built, name| Raw::lam(HERE, *name, built))
}

/// `Syntax ⟨token-tree⟩`, as a raw type.
///
/// Written through the registered base name and a `Cat` literal rather than
/// spliced as a term, because a raw program is what a source program is and this
/// law is about what one of those means.
fn tree() -> Raw {
    Raw::app(
        HERE,
        var("Syntax"),
        Raw::lit(HERE, literal(plain_type("Cat"), Cat::TokenTree)),
    )
}

fn text(spelling: &str) -> Raw {
    Raw::lit(HERE, literal(plain_type("Text"), spelling.to_owned()))
}

/// A `Nat`, one `Succ` at a time, which is the representation the prelude chose.
fn whole(value: usize) -> Raw {
    (0..value).fold(var("Nat.Zero"), |built, _| apply(var("Nat.Succ"), [built]))
}

/// `syntax_built(here, role, 0)` — a derived output path, at the role the old
/// side writes for the same case.
fn built(role: usize) -> Raw {
    apply(var("syntax_built"), [var("here"), whole(role), whole(0)])
}

/// `syntax_identifier(syntax_built(here, 2, 0), "leaf")`, the answer both sides
/// give at a node with no children.
fn leaf() -> Raw {
    apply(var("syntax_identifier"), [built(2), text("leaf")])
}

// ---- the agreement laws -----------------------------------------------------

/// `syntax_fold_from_leaves` rebuilds the region node for node, and the core and
/// the old evaluator build the same tree.
///
/// The identity fold is the strongest cheap comparison available: every case is
/// reached — the region has tokens, identifiers, and groups nested two deep — and
/// the answer is a whole `Syntax`, so a disagreement about a delimiter, a token
/// kind, a derived path, or the *order* of a group's children would fail it.
///
/// It also happens to be the shape a real adapter writes:
/// `stdlib/src/adapters/doubled.musa` rebuilds its region with these four
/// branches and these four roles.
#[test]
fn the_fold_rebuilds_the_region_the_way_the_old_evaluator_does() {
    let cx = owned().expect("the compiler's own context builds");
    let program = apply(
        var("syntax_fold_from_leaves"),
        [
            tree(),
            lambda(&["here"], leaf()),
            lambda(
                &["here", "kind", "spelling"],
                apply(var("syntax_token"), [built(1), var("kind"), var("spelling")]),
            ),
            lambda(
                &["here", "name"],
                apply(var("syntax_identifier"), [built(2), var("name")]),
            ),
            lambda(
                &["here", "delimiter", "children"],
                apply(var("syntax_group"), [built(3), var("delimiter"), var("children")]),
            ),
            Raw::lit(HERE, literal(syntax_type(Cat::TokenTree), subject())),
        ],
    );
    let expected = old(r#"syntax_fold_from_leaves(
            fn (here) { syntax_identifier(syntax_built(here, 2, 0), "leaf") },
            fn (here, kind, text) { syntax_token(syntax_built(here, 1, 0), kind, text) },
            fn (here, name) { syntax_identifier(syntax_built(here, 2, 0), name) },
            fn (here, delimiter, children) { syntax_group(syntax_built(here, 3, 0), delimiter, children) },
            region,
        )"#);
    assert_eq!(new(&cx, &program), expected, "the two folds built different trees");
}

/// `recurse_syntax` descends the leftmost spine, and the core and the old
/// evaluator descend it to the same depth.
///
/// The observation is a chain of parenthesis groups as deep as the region's
/// leftmost spine, with a leaf at the bottom. It is not the identity rebuild the
/// fold gets, and the reason is a real limitation rather than a shortcut: a group
/// branch is handed a `List (SyntaxStep C A)` and rebuilding needs to map over
/// it, which the *source* language cannot do — `list_fold_from_end` can consume a
/// list and nothing in the phase can build one, which is the missing constructor
/// `stdlib/src/adapters/staff.musa`'s header records. Both sides can take the
/// first child's answer, so that is what both sides take.
///
/// What it still establishes is everything this prompt added: a step is minted
/// per child, sealed with the algebra and the child it names, carried into a
/// branch as an ordinary list member, and run — under a context the branch
/// supplies rather than the one the traversal started with.
#[test]
fn recursing_descends_the_leftmost_spine_the_way_the_old_evaluator_does() {
    let cx = owned().expect("the compiler's own context builds");
    let step = apply(var("SyntaxStep"), [Raw::var(HERE, "Text"), tree()]);
    // `List.elim`'s `Cons` method ignores its induction hypothesis, so the answer
    // is the *first* member's — which is `list_fold_from_end`'s answer for a step
    // function that ignores its accumulator, and is why the two sides agree.
    let first = apply(
        var("List.elim"),
        [
            step,
            lambda(&["_"], tree()),
            leaf(),
            lambda(
                &["first", "rest", "done"],
                Raw::match_on(
                    HERE,
                    [var("first")],
                    vec![RawArm {
                        patterns: vec![RawPattern::constructor(
                            HERE,
                            "SyntaxStep.Step",
                            [RawPattern::bind(HERE, "run")],
                        )],
                        body: apply(var("run"), [var("context")]),
                    }],
                ),
            ),
            var("children"),
        ],
    );
    let program = apply(
        var("recurse_syntax"),
        [
            Raw::var(HERE, "Text"),
            tree(),
            lambda(&["context", "here"], leaf()),
            lambda(&["context", "here", "kind", "spelling"], leaf()),
            lambda(&["context", "here", "name"], leaf()),
            lambda(
                &["context", "here", "delimiter", "children"],
                apply(
                    var("syntax_group"),
                    [
                        built(3),
                        Raw::lit(HERE, literal(plain_type("Delimiter"), Delimiter::Parentheses)),
                        apply(var("List.Cons"), [tree(), first, apply(var("List.Empty"), [tree()])]),
                    ],
                ),
            ),
            text(""),
            Raw::lit(HERE, literal(syntax_type(Cat::TokenTree), subject())),
        ],
    );
    let expected = old(r#"recurse_syntax(
            fn (context, here) { syntax_identifier(syntax_built(here, 2, 0), "leaf") },
            fn (context, here, kind, text) { syntax_identifier(syntax_built(here, 2, 0), "leaf") },
            fn (context, here, name) { syntax_identifier(syntax_built(here, 2, 0), "leaf") },
            fn (context, here, delimiter, children) {
                syntax_group(syntax_built(here, 3, 0), Delimiter.Parentheses, [
                    list_fold_from_end(
                        syntax_identifier(syntax_built(here, 2, 0), "leaf"),
                        fn (child, found) { run_syntax_step(context, child) },
                        children,
                    )
                ])
            },
            "",
            region,
        )"#);
    assert_eq!(
        new(&cx, &program),
        expected,
        "the two descents reached different depths"
    );
}

/// A branch the node's kind does not select is never entered.
///
/// `Kind` is imported for the token literal the missing branch would need if it
/// were reached, and the point of the law is that it is not: the region has no
/// missing node, so a branch that refuses everything is passed through the whole
/// traversal unevaluated. A rewrite that eagerly applied all four branches — or
/// that built its answer by evaluating them and choosing afterwards — would fire
/// it.
#[test]
fn a_branch_the_node_does_not_select_is_never_evaluated() {
    let cx = owned().expect("the compiler's own context builds");
    // A missing branch whose body is a stuck builtin: `syntax_number` at a
    // non-numeric token answers `Option.None`, but `token_kind_equal` applied to
    // one argument is a partial spine that cannot reduce, so evaluating this
    // branch's *body* would leave a term the answer's type refuses.
    let refusing = lambda(
        &["here"],
        apply(
            var("syntax_token"),
            [
                built(0),
                Raw::lit(
                    HERE,
                    literal(plain_type("TokenKind"), Kind(musa_language::SyntaxKind::Error)),
                ),
                text(""),
            ],
        ),
    );
    let program = apply(
        var("syntax_fold_from_leaves"),
        [
            tree(),
            refusing,
            lambda(
                &["here", "kind", "spelling"],
                apply(var("syntax_token"), [built(1), var("kind"), var("spelling")]),
            ),
            lambda(
                &["here", "name"],
                apply(var("syntax_identifier"), [built(2), var("name")]),
            ),
            lambda(
                &["here", "delimiter", "children"],
                apply(var("syntax_group"), [built(3), var("delimiter"), var("children")]),
            ),
            Raw::lit(HERE, literal(syntax_type(Cat::TokenTree), subject())),
        ],
    );
    let answer = new(&cx, &program);
    let mut kinds = Vec::new();
    collect_kinds(&answer, &mut kinds);
    assert!(
        !kinds.contains(&musa_language::SyntaxKind::Error),
        "the missing branch was evaluated on a region with no missing node"
    );
}

fn collect_kinds(node: &Syntax, into: &mut Vec<musa_language::SyntaxKind>) {
    match *node {
        Syntax::Missing(_) | Syntax::Identifier { .. } => {}
        Syntax::Token { kind, .. } => into.push(kind),
        Syntax::Group { ref children, .. } => {
            for child in children {
                collect_kinds(child, into);
            }
        }
    }
}

/// A subject that is not a literal leaves an ordinary blocked spine.
///
/// The structural arm fires on a literal and on nothing else, so a traversal
/// standing at a variable is neutral rather than stuck — which is what lets a
/// traversal appear inside a function nobody has applied yet, and is the
/// difference between "not yet" and [`musa_core::Malformed::BuiltinStuck`].
#[test]
fn a_traversal_at_a_variable_stays_neutral() {
    let cx = owned().expect("the compiler's own context builds");
    let ty = Term::pi(
        HERE,
        "subject",
        syntax_type(Cat::TokenTree),
        syntax_type(Cat::TokenTree),
    );
    for spelling in super::SPELLINGS {
        let branches: Vec<Raw> = if spelling == super::SPELLINGS[0] {
            vec![
                Raw::var(HERE, "Text"),
                tree(),
                lambda(&["context", "here"], leaf()),
                lambda(&["context", "here", "kind", "spelling"], leaf()),
                lambda(&["context", "here", "name"], leaf()),
                lambda(&["context", "here", "delimiter", "children"], leaf()),
                text(""),
            ]
        } else {
            vec![
                tree(),
                lambda(&["here"], leaf()),
                lambda(&["here", "kind", "spelling"], leaf()),
                lambda(&["here", "name"], leaf()),
                lambda(&["here", "delimiter", "children"], leaf()),
            ]
        };
        let blocked = Raw::lam(
            HERE,
            "subject",
            apply(apply(var(spelling), branches), [Raw::var(HERE, "subject")]),
        );
        let term = musa_core::check(&cx, &ty, &blocked)
            .unwrap_or_else(|why| panic!("`{spelling}` at a variable does not check: {why}"));
        let normal = musa_core::normalize(&cx, &ty, &term)
            .unwrap_or_else(|why| panic!("`{spelling}` at a variable does not reduce: {why}"));
        musa_core::well_typed(&cx, &ty, &normal)
            .unwrap_or_else(|why| panic!("`{spelling}` at a variable does not re-check: {why}"));
    }
}

/// Every self-application a rewrite writes stands at a strictly smaller node.
///
/// §5.8's structural obligation, sampled rather than assumed. The rewrite is
/// called directly on a group literal and its answer is searched for the syntax
/// literals it wrote; each has to be a proper child, and a proper child of a
/// finite tree has strictly fewer nodes. A rewrite that passed its own target
/// down — the way an accidental `subject` in place of a `child` would — puts a
/// literal of the same size in the answer and fails here rather than running
/// until the meter stops it.
#[test]
fn every_self_application_stands_at_a_smaller_node() {
    let cx = owned().expect("the compiler's own context builds");
    let all = super::eliminators(&cx).expect("the traversals register");
    let subject = subject();
    let target = literal(syntax_type(Cat::TokenTree), subject.clone());
    let mut sampled = 0_usize;
    let rewrites: [musa_core::Rewrite; 2] = [super::rewrite_recurse, super::rewrite_fold];
    for (builtin, rewrite) in all.iter().zip(rewrites) {
        let answer = rewrite(builtin, &target).expect("the rewrite answers at a group");
        let mut written = Vec::new();
        trees(&answer, &mut written);
        assert!(!written.is_empty(), "`{}` wrote no child at all", builtin.name());
        for node in &written {
            assert!(
                nodes(node) < nodes(&subject),
                "`{}` applied itself at a node no smaller than its target",
                builtin.name()
            );
        }
        sampled = sampled.saturating_add(1);
    }
    assert_eq!(sampled, super::SPELLINGS.len(), "both traversals were sampled");
}

/// Every syntax literal a term holds, in the order it holds them.
fn trees(term: &Term, into: &mut Vec<Syntax>) {
    match *term.shape() {
        musa_core::Shape::Lit(ref value) => {
            if let Some(node) = held::<Syntax>(value) {
                into.push(node.clone());
            }
        }
        musa_core::Shape::App {
            ref function,
            ref argument,
        } => {
            trees(function, into);
            trees(argument, into);
        }
        musa_core::Shape::Lam { ref body, .. } => trees(body, into),
        // Spelled out rather than wildcarded, for the reason
        // `crate::registry::phase_type` gives: a variant added to `Shape` that
        // could hold a literal should stop here and be decided.
        musa_core::Shape::Var(_)
        | musa_core::Shape::Const(_)
        | musa_core::Shape::Def(_)
        | musa_core::Shape::Base(_)
        | musa_core::Shape::Builtin(_)
        | musa_core::Shape::Universe(_)
        | musa_core::Shape::Pi { .. }
        | musa_core::Shape::RecordType(_)
        | musa_core::Shape::Record(_)
        | musa_core::Shape::Project { .. }
        | musa_core::Shape::Id { .. }
        | musa_core::Shape::Refl(_)
        | musa_core::Shape::J { .. }
        | musa_core::Shape::Meta(_)
        | musa_core::Shape::Let { .. } => {}
    }
}

/// How many nodes a tree has, which is the measure the decrease is stated in.
fn nodes(node: &Syntax) -> usize {
    match *node {
        Syntax::Missing(_) | Syntax::Token { .. } | Syntax::Identifier { .. } => 1,
        Syntax::Group { ref children, .. } => children.iter().map(nodes).sum::<usize>().saturating_add(1),
    }
}
