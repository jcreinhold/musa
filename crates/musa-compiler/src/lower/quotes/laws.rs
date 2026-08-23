//! What a lowered quotation promises.
//!
//! Beside the lowering rather than in `tests/suite/`, and forced for the reason
//! [`crate::registry::traversal::laws`] gives at greater length: `expand_region`,
//! `read_region`, and [`Syntax`] are all private to this crate. A test outside it
//! links against `parse`/`compile`/`render_notation` and can reach none of them.
//!
//! # The comparison
//!
//! One quote, written once, read twice. The **whole** side runs it through
//! `expand_region` — an adapter module, elaborated and expanded exactly the way
//! a piece's own region reaches the phase; the **direct** side lowers the same
//! text to a [`Raw`] and hands it to `check`/`normalize`. Since prompt 142 both
//! sides end in the one core, which is what the law is worth having for: the
//! quotation counter, the anchor, the derived paths and the hygienic renaming
//! are all decided by the machinery *around* the lowering, and the direct side
//! has none of it. Both answer a whole [`Syntax`] value, so nothing is
//! summarized away: a disagreement about a derived path, a minted comma, a
//! hygienic renaming, or the order of a group's children fails the law.
//!
//! Both sides need a `NodePath` to anchor at, and an adapter's `expand` is handed
//! a region rather than a path — so both go through
//! `syntax_fold_from_leaves`, whose branches are the phase's own source of one.
//! The whole side writes the quote once, in a helper the four branches call,
//! because the quotation counter is per written quote (`11-quotation.md` §3) and
//! four written quotes would be four different constructions.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a law that cannot fail loudly is not a law"
)]

use musa_calculus::{Cx, Origin, Raw, Term};
use musa_syntax::SyntaxKind;

use super::super::{Lowering, Sites};
use crate::quote::{Cat, Delimiter, ExpansionPath, Syntax, read_region};
use crate::registry::{held, literal, owned, plain_type, syntax_type};
use crate::resolve::Resolver;
use musa_score::diagnose::Diagnostic;

/// Where a law's own terms are written, which is nowhere a composer can see.
const HERE: Origin = Origin::UNKNOWN;

/// The region both sides read.
///
/// The same one [`crate::registry::traversal::laws`] uses, and for the same
/// reason: identifiers, tokens, and groups nested two deep, so every branch of
/// the fold is reached.
const REGION: &str = "let melody = together(a, b)";

/// The body every law quotes, unless it says otherwise.
///
/// An application, so the body has a separated position for a spread and a
/// layout group around it — the two shapes `11-quotation.md` §2 distinguishes.
const BODY: &str = "together(a, b)";

fn expansion() -> ExpansionPath {
    ExpansionPath::at(vec![7])
}

/// The tree both sides read, exactly as the compiler reads it for an adapter.
fn subject() -> Syntax {
    read_region(&musa_syntax::parse(REGION).syntax(), expansion())
}

// ---- the two sides ----------------------------------------------------------

/// What the whole phase builds for `quoted`, at every node of the region.
///
/// The answer is the outermost group's, since that branch ignores the children
/// it was handed. The three leaf branches are there because the fold demands
/// four and every one of them has to have the same answer type, not because
/// their answers are read.
fn whole(quoted: &str) -> Syntax {
    let source = format!(
        "library {{
    let level = \"readable\";

    let quoting = fn (here: NodePath) -> Syntax<Expr> {{ {quoted} }};

    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> {{
        Ok(forget(syntax_fold_from_leaves(
            fn (here) {{ quoting(here) }},
            fn (here, kind, spelling) {{ quoting(here) }},
            fn (here, name) {{ quoting(here) }},
            fn (here, delimiter, children) {{ quoting(here) }},
            region,
        )))
    }};
}}"
    );
    crate::phase::expand_syntax(&source, crate::phase::PhaseImports::bundled(), &subject())
        .0
        .expect("the phase runs the transformer")
}

/// The same quote, lowered and handed to the core directly, through the same
/// fold.
fn direct(cx: &Cx, quoted: &str) -> Syntax {
    let body = lowered(quoted);
    let branch = |names: &[&str]| {
        names
            .iter()
            .rev()
            .fold(body.clone(), |built, name| Raw::lam(HERE, *name, built))
    };
    // `Answer` is an implicit parameter of the fold, so it is not written here
    // any more than it is in `whole`'s musa source — the checked result type
    // solves it. Writing it lands a `Type 0` at the `missing` branch's position.
    let program = apply(
        Raw::var(HERE, "syntax_fold_from_leaves"),
        [
            branch(&["here"]),
            branch(&["here", "kind", "spelling"]),
            branch(&["here", "name"]),
            branch(&["here", "delimiter", "children"]),
            Raw::lit(HERE, literal(syntax_type(Cat::TokenTree), subject())),
        ],
    );
    answer(cx, &program, Cat::Expr)
}

/// The [`Syntax`] a raw program reduces to, having checked and re-checked it.
///
/// Three steps that fail on three different mistakes: the program is refused if
/// it does not inhabit the category it claims, the normal form is refused if the
/// rules built something the declared result does not admit, and the re-check is
/// independent of the evaluator that produced it.
fn answer(cx: &Cx, program: &Raw, cat: Cat) -> Syntax {
    let ty = syntax_type(cat);
    let term = musa_calculus::check(cx, &ty, program).unwrap_or_else(|why| panic!("the program does not check: {why}"));
    let normal =
        musa_calculus::normalize(cx, &ty, &term).unwrap_or_else(|why| panic!("the program does not reduce: {why}"));
    let musa_calculus::Shape::Lit(musa_calculus::Constant::Payload(ref built)) = *normal.shape() else {
        panic!("the answer is not a literal: {normal:?}")
    };
    held::<Syntax>(built).expect("the answer is a syntax value").clone()
}

// ---- lowering one written expression ---------------------------------------

/// One expression, lowered where §5.9's vocabulary is readable, with a loud
/// failure when it complains.
fn lowered(written: &str) -> Raw {
    let (raw, complaints) = lowering(written);
    assert!(
        complaints.is_empty(),
        "`{written}` lowers without complaint: {complaints:?}"
    );
    raw.unwrap_or_else(|| panic!("`{written}` lowers"))
}

/// The same, keeping whatever it complained about.
fn lowering(written: &str) -> (Option<Raw>, Vec<Diagnostic>) {
    let source = format!("library {{ fn probe() -> Nat {{ {written} }} }}");
    let document = musa_syntax::parse(&source);
    assert!(
        document.errors().is_empty(),
        "the law's own source parses: {:?}",
        document.errors()
    );
    let node = document
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::BlockExpr)
        .expect("the function has a body");
    let mut resolver = Resolver::new();
    let mut sites = Sites::default();
    let raw = Lowering::phase(&mut resolver, &mut sites).expr(&node);
    (raw, resolver.diagnostics)
}

// ---- writing the bits of a raw program a law supplies -----------------------

fn apply(head: Raw, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    arguments
        .into_iter()
        .fold(head, |function, argument| Raw::app(HERE, function, argument))
}

/// `Syntax ⟨c⟩`, as a raw type.
fn category(cat: Cat) -> Raw {
    Raw::app(
        HERE,
        Raw::var(HERE, "Syntax"),
        Raw::lit(HERE, literal(plain_type("Cat"), cat)),
    )
}

/// One `Syntax ⟨expr⟩` value, read out of source of its own.
///
/// A whole region rather than a node assembled here, so that a spliced node is
/// one with real source information — which is what a splice is supposed to
/// carry through unchanged. The redundant layout wrapping a region comes in is
/// dropped, because `crate::quote::matched` drops it too: §4 matches on shape,
/// and a group of one child is not a shape a pattern can write.
fn spliceable(source: &str, at: u64) -> Syntax {
    let mut node = crate::quote::read_written(source, ExpansionPath::at(vec![u32::try_from(at).unwrap_or(u32::MAX)]));
    while let Syntax::Group {
        delimiter: Delimiter::Layout,
        ref children,
        ..
    } = node
        && let [only] = children.as_slice()
    {
        let inner = only.clone();
        node = inner;
    }
    node
}

/// A list of `Syntax ⟨expr⟩` values, annotated so the empty one is still a list
/// of something.
fn splices(members: Vec<Syntax>) -> Raw {
    let built = members
        .into_iter()
        .rev()
        .fold(Raw::var(HERE, "List.Empty"), |rest, member| {
            apply(
                Raw::var(HERE, "List.Cons"),
                [Raw::lit(HERE, literal(syntax_type(Cat::Expr), member)), rest],
            )
        });
    Raw::annot(HERE, built, Raw::app(HERE, Raw::var(HERE, "List"), category(Cat::Expr)))
}

/// The anchor every law that supplies one supplies: the region's own path.
fn anchor() -> Raw {
    let held = subject();
    let path = held.info().path().clone();
    Raw::lit(HERE, literal(plain_type("NodePath"), path))
}

// ---- the laws ---------------------------------------------------------------

/// A quote inhabits the category `11-quotation.md` §1 says it does.
///
/// `Expr` and not `TokenTree`, because the body went through the expression
/// production; that is the claim a quote can make, and the one the core is asked
/// to agree with here.
#[test]
fn a_quote_inhabits_the_expression_category_at_the_anchor_it_was_given() {
    let cx = owned().expect("the compiler's own context builds");
    let program = Raw::lam(HERE, "here", lowered(&format!("quote at here {{ {BODY} }}")));
    let ty = Term::pi(HERE, "here", plain_type("NodePath"), syntax_type(Cat::Expr));
    musa_calculus::check(&cx, &ty, &program).unwrap_or_else(|why| panic!("a quote does not check: {why}"));
}

/// The law prompt 141ga exists for: a quote handed straight to the core builds
/// the tree the whole phase builds.
///
/// Not "a tree of the same shape" — the same tree, so every derived path, every
/// minted comma, and every hygienic renaming has to agree. That is the whole
/// claim of moving quotation onto a δ-rule: `crate::quote::instantiate` is
/// reached a second way rather than reimplemented, and if it were not, identity
/// is exactly where the disagreement would show.
#[test]
fn a_lowered_quote_builds_what_the_whole_phase_builds() {
    let cx = owned().expect("the compiler's own context builds");
    for quoted in [
        "quote at here { together(a, b) }",
        "quote at here { 1 }",
        "quote at here { fn (x) { x } }",
        "quote at here { [a, b, c] }",
        "quote at here { match a { x -> x } }",
    ] {
        assert_eq!(
            direct(&cx, quoted),
            whole(quoted),
            "`{quoted}` builds two different trees"
        );
    }
}

/// §2's separator, at three lengths.
///
/// A spread's commas are the position's rather than the body's, so a run of any
/// length is right and the empty one leaves no trailing comma — which is the
/// case a template that kept its written commas would get wrong, and the reason
/// `Template::Group::separated` exists.
///
/// The printed text with its whitespace removed, because the claim is about the
/// separators and the printer's spacing is `crate::quote::print`'s own business
/// — a law that fixed it here would fail the next time the printer is tuned.
#[test]
fn a_spread_is_separated_by_the_commas_its_position_supplies() {
    let cx = owned().expect("the compiler's own context builds");
    let quoted = lowered("quote at here { together($..xs) }");
    for (members, expected) in [
        (vec![], "together()"),
        (vec![spliceable("one", 1)], "together(one)"),
        (
            vec![spliceable("one", 1), spliceable("two", 2), spliceable("three", 3)],
            "together(one,two,three)",
        ),
    ] {
        let program = Raw::bind(
            HERE,
            "here",
            anchor(),
            Raw::bind(HERE, "xs", splices(members), quoted.clone()),
        );
        let printed = crate::quote::print(&answer(&cx, &program, Cat::Expr));
        assert_eq!(
            printed.text.split_whitespace().collect::<String>(),
            expected,
            "a spread of this length is not separated the way §2 promises"
        );
    }
}

/// §4's inverse claim, stated once over the core's own reduction.
///
/// `quote_hole(instantiate_quote(a, t, s), t, i)` is `sᵢ`, and the whole
/// round trip is written in Musa: the quote builds, the pattern in the arm
/// beside it reads the same shape back, and the arm's body is the hole.
///
/// The law applies §1's forgetting itself, at the literal: `quote_hole` reads at
/// `⟨tokentree⟩` and the node that went in is a `⟨expr⟩`, so the two are compared
/// as the [`Syntax`] values they are. The core has no acceptance rule for that
/// yet, and prompt 142 is where one arrives.
#[test]
fn every_hole_comes_back_the_node_that_went_in() {
    let cx = owned().expect("the compiler's own context builds");
    let spliced = spliceable("one", 1);
    let built = answer(
        &cx,
        &Raw::bind(
            HERE,
            "here",
            anchor(),
            Raw::bind(
                HERE,
                "x",
                Raw::lit(HERE, literal(syntax_type(Cat::Expr), spliced.clone())),
                lowered("quote at here { together($x) }"),
            ),
        ),
        Cat::Expr,
    );
    // §1's forgetting, applied here because the core has no rule for it: the
    // value built at `⟨expr⟩` is handed back at `⟨tokentree⟩`, which is where
    // every phase operation reads. Prompt 142 is where the core learns to do
    // this itself, for this and for the fourteen builders that already need it.
    let forgotten = Raw::lit(HERE, literal(syntax_type(Cat::TokenTree), built));
    let read = answer(
        &cx,
        &Raw::bind(
            HERE,
            "subject",
            forgotten,
            Raw::bind(
                HERE,
                "missing",
                Raw::lit(HERE, literal(syntax_type(Cat::TokenTree), subject())),
                lowered(
                    "match subject {
                        quote { together($held) } -> held,
                        _ -> missing,
                    }",
                ),
            ),
        ),
        Cat::TokenTree,
    );
    assert_eq!(read, spliced, "the hole did not come back the node that went in");
}
