//! What lowering the surface promises.
//!
//! Beside the lowering rather than in `tests/suite/`, and for the reason
//! [`crate::registry::laws`] gives for the δ agreement law: the answer is a
//! [`musa_calculus::Raw`], the context is [`crate::registry::owned`], and both are
//! private to this crate. A test outside it could observe neither.
//!
//! # The one property this module can have
//!
//! **What lowering writes, the core accepts.** `musa-calculus` decides what a type
//! is and what a term inhabits it, so agreeing with it is the whole of what a
//! reading can be right about — a lowering that produced a plausible-looking
//! term the core refused would be worse than one that refused at the node, and a
//! law that checked the shape of the [`Raw`] instead would be this module
//! marking its own homework. So every law below lowers something a composer
//! could write and hands it to [`musa_calculus::check`].
//!
//! The corollary is what these laws *do not* test: nothing here asserts that an
//! unknown name is refused, because the core refuses it, and nothing here
//! asserts an arity. Where a form is refused *at the node*, the law is that the
//! complaint is at the node, which is the part the core cannot do.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a law that cannot fail loudly is not a law"
)]

use musa_calculus::{Cx, Raw, RawData, RawPattern, RawShape, Sort, Term};
use musa_syntax::{SyntaxKind, SyntaxNode};

use super::items::{Declared, Definition, Item};
use super::{Lowering, Sites, is_type_node};
use crate::resolve::Resolver;
use musa_score::diagnose::{Code, Diagnostic};

// ---- reading a written program back out of a parse ----

/// The tree `source` parses to, with a loud failure when it does not parse.
///
/// Every law below writes real `.musa` and reads the node it wants out of the
/// tree, rather than assembling a [`SyntaxNode`] or a [`Raw`] by hand. That is
/// deliberate: the shape of a `Pattern` under a `MatchArm` is the parser's, and
/// a grammar change that moved it is exactly what these should notice.
fn parsed(source: &str) -> SyntaxNode {
    let document = musa_syntax::parse(source);
    assert!(
        document.errors().is_empty(),
        "the law's own source parses: {:?}",
        document.errors()
    );
    document.syntax()
}

/// The first descendant of `root` whose kind `wanted` admits.
fn first(root: &SyntaxNode, wanted: SyntaxKind) -> SyntaxNode {
    root.descendants()
        .find(|node| node.kind() == wanted)
        .unwrap_or_else(|| panic!("the tree holds a {wanted:?}"))
}

/// The written type of `fn probe(x: …)`, lowered in ordinary source.
fn lowered_type(written: &str) -> (Option<Raw>, Vec<Diagnostic>) {
    lowered_type_in(written, false)
}

/// The same, in whichever of the two scopes `02-core-calculus.md` §5.9
/// distinguishes.
fn lowered_type_in(written: &str, in_phase: bool) -> (Option<Raw>, Vec<Diagnostic>) {
    let root = parsed(&format!("library {{ fn probe(x: {written}) -> Nat {{ 0 }} }}"));
    let node = first(&root, SyntaxKind::Param)
        .children()
        .find(|child| is_type_node(child.kind()))
        .expect("the parameter writes a type");
    let mut resolver = Resolver::new();
    let mut sites = Sites::default();
    let raw = {
        let mut lowering = if in_phase {
            Lowering::phase(&mut resolver, &mut sites)
        } else {
            Lowering::new(&mut resolver, &mut sites)
        };
        lowering.ty(&node)
    };
    (raw, resolver.diagnostics)
}

/// The body of `fn probe() -> Nat { … }`, lowered.
///
/// The body is a `BlockExpr` and [`Lowering::expr`] is what the declaration
/// lowering will hand it to, so the law goes through the same door a real
/// program will.
fn lowered_expr(written: &str) -> (Option<Raw>, Vec<Diagnostic>) {
    let root = parsed(&format!("library {{ fn probe() -> Nat {{ {written} }} }}"));
    let node = first(&root, SyntaxKind::BlockExpr);
    let mut resolver = Resolver::new();
    let mut sites = Sites::default();
    let raw = Lowering::new(&mut resolver, &mut sites).expr(&node);
    (raw, resolver.diagnostics)
}

/// The first arm's pattern in `match x { … }`, lowered.
fn lowered_pattern(written: &str) -> Option<RawPattern> {
    let root = parsed(&format!(
        "library {{ fn probe(x: Nat) -> Nat {{ match x {{ {written} -> 0, }} }} }}"
    ));
    let node = first(&root, SyntaxKind::Pattern);
    let mut resolver = Resolver::new();
    let mut sites = Sites::default();
    Lowering::new(&mut resolver, &mut sites).pattern(&node)
}

// ---- the context the core answers in ----

/// The compiler's own context.
fn host() -> Cx {
    crate::registry::owned().expect("the compiler's own context builds")
}

/// `Type 0`, which every type a signature writes lands in.
fn type0() -> Term {
    Term::universe(musa_calculus::Origin::UNKNOWN, Sort::ZERO)
}

/// The type one declared name denotes.
fn declared(cx: &Cx, name: &str) -> Term {
    crate::prelude::constant(cx, name).unwrap_or_else(|failure| panic!("`{name}` is declared: {failure:?}"))
}

/// `f a`, for building the expected type of a law's subject.
fn at(function: Term, argument: Term) -> Term {
    Term::app(musa_calculus::Origin::UNKNOWN, function, argument)
}

// ---- types ----

#[test]
fn every_written_base_type_lowers_to_a_type_the_core_accepts() {
    let cx = host();
    for written in [
        "Bool",
        "Nat",
        "Ratio",
        "Text",
        "Pitch",
        "NoteName",
        "Interval",
        "Scale",
        "Key",
        "Degree",
        "Frame",
        "ChordClass",
        "Triad",
        "Roman",
        "Voicing",
        "Pc12",
        "PcSet12",
        "Row12",
        "Duration<WrittenTime>",
        "Position<PhysicalTime>",
        // The type a fragment inhabits, written the way its duration is. This
        // is the ledger's replacement for `Music`, and it is a *written* type
        // rather than a contextual one: a track of beats and a track of seconds
        // are different types, and neither depends on where it is used.
        "EventTrack<WrittenTime>",
    ] {
        let (raw, complaints) = lowered_type(written);
        assert!(complaints.is_empty(), "`{written}` lowers without complaint");
        let raw = raw.unwrap_or_else(|| panic!("`{written}` lowers"));
        musa_calculus::check(&cx, &type0(), &raw).unwrap_or_else(|failure| {
            panic!("`{written}` lowers to a type the core accepts, not {failure:?}");
        });
    }
}

#[test]
fn a_written_constructor_lowers_to_the_family_applied() {
    let cx = host();
    for written in ["Option<Nat>", "List<Text>", "Result<Ratio, Text>", "List<List<Pitch>>"] {
        let (raw, complaints) = lowered_type(written);
        assert!(complaints.is_empty(), "`{written}` lowers without complaint");
        let raw = raw.unwrap_or_else(|| panic!("`{written}` lowers"));
        musa_calculus::check(&cx, &type0(), &raw).unwrap_or_else(|failure| {
            panic!("`{written}` lowers to a type the core accepts, not {failure:?}");
        });
    }
}

#[test]
fn a_written_arrow_lowers_to_a_pi_nothing_refers_through() {
    let cx = host();
    let (raw, complaints) = lowered_type("Nat -> Text");
    assert!(complaints.is_empty(), "an arrow lowers without complaint");
    let raw = raw.expect("an arrow lowers");
    let term = musa_calculus::check(&cx, &type0(), &raw).expect("the arrow is a type");
    let expected = Term::pi(
        musa_calculus::Origin::UNKNOWN,
        "argument",
        declared(&cx, "Nat"),
        crate::registry::plain_type("Text"),
    );
    assert!(
        musa_calculus::convertible_types(&cx, &term, &expected).expect("both are types"),
        "`Nat -> Text` is the Π whose codomain does not mention its binder"
    );
}

/// The rule the deleted type scope used to enforce a second time.
#[test]
fn a_name_the_compiler_does_not_own_is_written_through_for_the_core_to_resolve() {
    let cx = host();
    let (raw, complaints) = lowered_type("List<A>");
    assert!(
        complaints.is_empty(),
        "lowering holds no context, so it has nothing to complain with"
    );
    let raw = raw.expect("the application lowers");
    assert!(
        format!("{raw:?}").contains('A'),
        "the parameter reaches the core as a name for it to resolve"
    );
    assert!(
        musa_calculus::check(&cx, &type0(), &raw).is_err(),
        "and in a context that does not bind `A`, the core is what refuses it"
    );
}

#[test]
fn a_bare_indexed_type_is_refused_where_it_is_written() {
    let (raw, complaints) = lowered_type("Duration");
    assert!(raw.is_none(), "`Duration` alone names no type");
    assert_eq!(complaints.len(), 1, "one complaint, at the word");
    let complaint = complaints.first().expect("one complaint").message.as_str();
    assert!(
        complaint.contains("takes an argument"),
        "the complaint is about the missing coordinate, not about an unknown name: {complaint}"
    );
}

/// An index written in a type reaches the core as one.
///
/// `02-core-calculus.md` §1.5 lets a type carry an index and the grammar spells
/// it `T(i)`, so the whole of the path — parser to `IndexedType`, [`Lowering::ty`]
/// to `Raw::indexed`, elaborator to a formed type — has to hold. Nothing tested
/// that it did, and it did not: [`is_type_node`] was a copy of
/// `musa_syntax::ast::is_type` made before `IndexedType` was a node kind, so
/// every reader of an annotation looked straight past `Nat(12)`; the parameter
/// lowered as a λ with no domain, and the author was told their parameter could
/// not be given a type on its own.
///
/// The *refusal* is what the law ends at, and that is prompt 142f's doing. `Nat`
/// is the sort an index is drawn from and declares none of its own, so an index
/// written on it is refused — and refused for taking no index, which is a
/// refusal only a lowering that reached the core as `Raw::indexed` could have
/// earned. A lowering that dropped the parentheses would have accepted `Nat`.
#[test]
fn an_index_written_in_a_type_reaches_the_core_as_the_type_it_forms() {
    let cx = host();
    // `Nat(12)`, whose index the core holds itself as a numeral, and
    // `Ratio(3/4)`, whose index only the compiler that put the fraction in can
    // read back. Neither head declares an index, so both end at the same
    // refusal — which is the evidence that both arrived as indexed types.
    for written in ["Nat(12)", "Ratio(3/4)"] {
        let (raw, complaints) = lowered_type(written);
        assert!(complaints.is_empty(), "`{written}` lowers without complaint");
        let raw = raw.unwrap_or_else(|| panic!("`{written}` lowers"));
        assert!(
            matches!(*raw.shape(), RawShape::Indexed { .. }),
            "`{written}` lowers to the indexed type it writes, not to its head alone"
        );
        let Err(musa_calculus::ElabError::Refused(refusal)) = musa_calculus::check(&cx, &type0(), &raw) else {
            panic!("`{written}` was formed, and neither head declares an index");
        };
        assert!(
            matches!(refusal, musa_calculus::Refusal::NotIndexed { .. }),
            "`{written}`: refused as {refusal:?} rather than for its index"
        );
    }
}

/// A base type's literals reach §1.5's reader at the base type that registered
/// how to read them.
///
/// [`Base`](musa_calculus::Base) compares by *name*, so a second `Ratio` built
/// without [`measuring`](musa_calculus::Base::measuring) is accepted everywhere
/// the registered one is and then answers differently when asked whether a
/// literal of it is a number. That is what `crate::registry::registered` exists
/// to prevent, and reading the measure back off a lowered literal's own type is
/// how the prevention is checked rather than assumed.
#[test]
fn a_literal_carries_the_base_type_that_registered_how_to_read_it() {
    let (raw, complaints) = lowered_expr("3/4");
    assert!(complaints.is_empty(), "`3/4` lowers without complaint");
    let raw = raw.expect("`3/4` lowers");
    let RawShape::Lit(literal) = raw.shape() else {
        panic!("`3/4` lowers to a literal, not to {:?}", raw.shape());
    };
    let musa_calculus::Shape::Base(base) = literal.ty().shape() else {
        panic!("a `Ratio` literal stands at a base type");
    };
    assert!(
        base.reads_an_index(),
        "`Ratio` registers how to read its literals as index values, and this literal stands at a `Ratio` that does not"
    );
}

#[test]
fn a_phase_type_is_readable_only_where_an_adapter_is_read() {
    let cx = host();
    let (raw, complaints) = lowered_type_in("Syntax<Expr>", true);
    assert!(complaints.is_empty(), "`Syntax<Expr>` lowers inside the phase");
    let raw = raw.expect("`Syntax<Expr>` lowers inside the phase");
    musa_calculus::check(&cx, &type0(), &raw).expect("`Syntax ⟨expr⟩` is a type");

    let (raw, complaints) = lowered_type("Syntax<Expr>");
    assert!(
        complaints.is_empty(),
        "outside the phase the word is written through, not complained about here"
    );
    let raw = raw.expect("outside the phase it still lowers, as a name");
    assert!(
        musa_calculus::check(&cx, &type0(), &raw).is_err(),
        "and the core is what refuses it, since `Syntax` is not a name ordinary source may resolve"
    );
}

// ---- expressions ----

#[test]
fn every_written_literal_lowers_to_a_term_the_core_checks_at_its_own_domain() {
    let cx = host();
    let base = |name: &'static str| crate::registry::plain_type(name);
    let cases: Vec<(&str, Term)> = vec![
        ("3", declared(&cx, "Nat")),
        ("true", declared(&cx, "Bool")),
        ("false", declared(&cx, "Bool")),
        ("\"a text\"", base("Text")),
        ("3/4", base("Ratio")),
        ("c5", base("Pitch")),
        ("g#4", base("Pitch")),
        ("M3", base("Interval")),
        ("chord c# minor", base("ChordClass")),
        ("scale c dorian", base("Scale")),
        ("key a minor", base("Key")),
    ];
    for (written, expected) in cases {
        let (raw, complaints) = lowered_expr(written);
        assert!(complaints.is_empty(), "`{written}` lowers without complaint");
        let raw = raw.unwrap_or_else(|| panic!("`{written}` lowers"));
        musa_calculus::check(&cx, &expected, &raw).unwrap_or_else(|failure| {
            panic!("`{written}` checks at its own domain, not {failure:?}");
        });
    }
}

#[test]
fn the_written_container_forms_lower_to_the_constructors_they_stand_for() {
    let cx = host();
    let nat = declared(&cx, "Nat");
    let text = crate::registry::plain_type("Text");
    let cases: Vec<(&str, Term)> = vec![
        ("[1, 2, 3]", at(declared(&cx, "List"), nat.clone())),
        ("[]", at(declared(&cx, "List"), nat.clone())),
        ("Some(2)", at(declared(&cx, "Option"), nat.clone())),
        ("None", at(declared(&cx, "Option"), nat.clone())),
        ("Ok(2)", at(at(declared(&cx, "Result"), nat.clone()), text.clone())),
        ("Err(\"no\")", at(at(declared(&cx, "Result"), nat), text)),
    ];
    for (written, expected) in cases {
        let (raw, complaints) = lowered_expr(written);
        assert!(complaints.is_empty(), "`{written}` lowers without complaint");
        let raw = raw.unwrap_or_else(|| panic!("`{written}` lowers"));
        musa_calculus::check(&cx, &expected, &raw).unwrap_or_else(|failure| {
            panic!("`{written}` checks at the type its constructors build, not {failure:?}");
        });
    }
}

/// `if` is a `match` on `Bool`, so the law is that the core accepts it as one.
#[test]
fn a_conditional_lowers_to_the_two_armed_boolean_match() {
    let cx = host();
    let (raw, complaints) = lowered_expr("if true { 1 } else { 2 }");
    assert!(complaints.is_empty(), "a conditional lowers without complaint");
    let raw = raw.expect("a conditional lowers");
    musa_calculus::check(&cx, &declared(&cx, "Nat"), &raw).expect("and the core checks it at the arms' type");
}

/// A `?` propagates to the position that delimits the answer, and the subject is
/// evaluated once because it becomes the scrutinee.
#[test]
fn a_question_lowers_to_a_match_that_evaluates_its_subject_once() {
    let cx = host();
    let root = parsed("library { fn probe(r: Result<Nat, Text>) -> Result<Nat, Text> { Ok(r?) } }");
    let node = first(&root, SyntaxKind::BlockExpr);
    let mut resolver = Resolver::new();
    let mut sites = Sites::default();
    let raw = Lowering::new(&mut resolver, &mut sites)
        .expr(&node)
        .expect("`Ok(r?)` lowers");
    assert!(resolver.diagnostics.is_empty(), "and does so without complaint");
    let shown = format!("{raw:?}");
    assert_eq!(
        shown.matches("\"r\"").count(),
        1,
        "the subject is written into the term once, as the scrutinee: {shown}"
    );

    // Checkable only where `r` stands, and standing it up is what the
    // declaration lowering does; here the law is the shape plus the core's
    // acceptance of the same term under a binder for `r`. `Ok(…)` is written
    // because a `?` has no early return to desugar to: the `Err` arm answers
    // the whole expression, so the answer has to be a `Result` — which is the
    // rule the surface always had, now enforced by the core rather than by a
    // state machine beside it.
    let bound = Raw::annotated_lam(musa_calculus::Origin::UNKNOWN, "r", result_of_nat_text_raw(), raw);
    let expected = Term::pi(
        musa_calculus::Origin::UNKNOWN,
        "r",
        result_of_nat_text(&cx),
        result_of_nat_text(&cx),
    );
    musa_calculus::check(&cx, &expected, &bound).expect("`\\r -> Ok(r?)` is the identity on a `Result`");
}

/// `Result Nat Text`, as a raw term.
fn result_of_nat_text_raw() -> Raw {
    let here = musa_calculus::Origin::UNKNOWN;
    Raw::app(
        here,
        Raw::app(here, Raw::var(here, "Result"), Raw::var(here, "Nat")),
        Raw::var(here, "Text"),
    )
}

/// `Result Nat Text`, as a core term.
fn result_of_nat_text(cx: &Cx) -> Term {
    at(
        at(declared(cx, "Result"), declared(cx, "Nat")),
        crate::registry::plain_type("Text"),
    )
}

// ---- patterns ----

#[test]
fn a_pattern_column_reaches_match_in_the_order_the_arm_wrote_it() {
    let bound = lowered_pattern("Some(head)").expect("`Some(head)` lowers");
    let shown = format!("{bound:?}");
    assert!(
        shown.contains("Option.Some") && shown.contains("head"),
        "a constructor pattern carries its constructor and its field binders: {shown}"
    );

    // `[head, ..tail]` is `List`'s cons cell, not a list of known length: the
    // grammar admits the empty bracket and that one shape, so the reading has
    // two cases rather than a chain.
    let cons = lowered_pattern("[first, ..others]").expect("a list pattern lowers");
    let shown = format!("{cons:?}");
    let (before, after) = shown.split_once("others").expect("both binders are written");
    assert!(
        before.contains("List.Cons") && before.contains("first") && !after.contains("first"),
        "and the head is bound before the tail, as the arm wrote them: {shown}"
    );
    let empty = lowered_pattern("[]").expect("the empty list pattern lowers");
    assert!(
        format!("{empty:?}").contains("List.Empty"),
        "and the empty bracket is the other constructor"
    );
}

/// `Nat` and `Bool` are declared families, so their literal patterns are
/// constructor patterns and need no literal case at all.
#[test]
fn a_whole_number_pattern_is_the_constructor_chain_it_counts_to() {
    let two = lowered_pattern("2").expect("`2` lowers as a pattern");
    let shown = format!("{two:?}");
    assert_eq!(
        shown.matches("Nat.Succ").count(),
        2,
        "`2` is two successors over zero: {shown}"
    );
    assert_eq!(shown.matches("Nat.Zero").count(), 1, "and one zero: {shown}");
}

// ---- declarations ----

/// The first declaration of `written` in a library, lowered.
///
/// The kind is named rather than searched for, because a `record` holds a
/// `FieldDecl` and a `trait` holds an `FnDecl`: "the first declaration" is not a
/// question the tree answers on its own.
fn lowered_item(written: &str, wanted: SyntaxKind) -> (Option<Item>, Vec<Diagnostic>) {
    let root = parsed(&format!("library {{ {written} }}"));
    let node = first(&root, wanted);
    let mut resolver = Resolver::new();
    let mut sites = Sites::default();
    let item = match Lowering::new(&mut resolver, &mut sites).item(&node) {
        Declared::Item(item) => Some(item),
        Declared::Refused | Declared::Elsewhere => None,
    };
    (item, resolver.diagnostics)
}

/// The item `written` lowers to, with a loud failure when it complains.
fn item(written: &str, wanted: SyntaxKind) -> Item {
    let (item, complaints) = lowered_item(written, wanted);
    assert!(
        complaints.is_empty(),
        "`{written}` lowers without complaint: {complaints:?}"
    );
    item.unwrap_or_else(|| panic!("`{written}` lowers"))
}

/// The declaration group `written` lowers to.
fn declaration(written: &str, wanted: SyntaxKind) -> RawData {
    let Item::Data(data) = item(written, wanted) else {
        panic!("`{written}` is a declaration group");
    };
    data
}

/// The definition `written` lowers to.
fn definition(written: &str, wanted: SyntaxKind) -> Definition {
    let Item::Definition(defined) = item(written, wanted) else {
        panic!("`{written}` is a definition");
    };
    defined
}

/// The names of a family's constructors, and of each one's fields.
fn cases(data: &RawData) -> Vec<(String, Vec<String>)> {
    data.families
        .first()
        .expect("one family")
        .constructors
        .iter()
        .map(|case| {
            (
                case.name.to_string(),
                case.fields.iter().map(|field| field.name.to_string()).collect(),
            )
        })
        .collect()
}

/// A definition's value, checked at the type its declaration wrote.
///
/// The whole property a declaration lowering can have, and the same one the
/// expression laws above assert: the core is what decides, so agreeing with it
/// is what "read correctly" means.
fn inhabits_its_written_type(cx: &Cx, written: &str, defined: &Definition) {
    let declared = defined
        .ty
        .as_ref()
        .unwrap_or_else(|| panic!("`{written}` wrote its type"));
    let (declared, _) = musa_calculus::infer(cx, declared)
        .unwrap_or_else(|failure| panic!("`{written}`'s written type is a type, not {failure:?}"));
    musa_calculus::check(cx, &declared, &defined.value)
        .unwrap_or_else(|failure| panic!("`{written}`'s value inhabits it, not {failure:?}"));
}

#[test]
fn a_data_declaration_lowers_to_a_family_the_core_declares() {
    let written = "data Motive { Silence, Sounded(sung: Pitch, held: Duration<WrittenTime>) }";
    let data = declaration(written, SyntaxKind::DataDecl);
    assert_eq!(
        cases(&data),
        vec![
            ("Silence".to_owned(), Vec::new()),
            ("Sounded".to_owned(), vec!["sung".to_owned(), "held".to_owned()]),
        ],
        "a `data` names every field of every constructor"
    );
    musa_calculus::declare(&host(), &data).expect("the core declares what the surface wrote");
}

/// `01-surface.md` §1.3's two case forms, which differ only in whether the
/// fields have names worth writing.
#[test]
fn an_enum_lowers_a_positional_case_by_position_and_a_named_one_by_name() {
    let written = "enum Reading<A> { Done(A), Refused { place: Text; why: Text; } }";
    let data = declaration(written, SyntaxKind::EnumDecl);
    assert_eq!(
        cases(&data),
        vec![
            ("Done".to_owned(), vec!["_0".to_owned()]),
            ("Refused".to_owned(), vec!["place".to_owned(), "why".to_owned()]),
        ],
        "a positional case's fields are named by their positions"
    );
    assert_eq!(data.params.len(), 1, "and the declaration's parameter is the group's");
    musa_calculus::declare(&host(), &data).expect("the core declares what the surface wrote");
}

/// A `record` is its fields (`01-surface.md` §1.2), so it is a *definition* of a
/// type and not a family: a `RawData` here would have made it nominal.
#[test]
fn a_record_lowers_to_a_definition_of_the_type_its_fields_are() {
    let cx = host();
    let written = "record Pending { read: Nat; dots: Nat; }";
    let defined = definition(written, SyntaxKind::RecordDecl);
    assert_eq!(&*defined.name, "Pending", "the declared name is what diagnostics say");
    inhabits_its_written_type(&cx, written, &defined);
}

#[test]
fn a_parameterized_record_lowers_to_a_function_to_a_type() {
    let cx = host();
    let written = "record Cell<A> { index: Nat; value: A; }";
    let defined = definition(written, SyntaxKind::RecordDecl);
    inhabits_its_written_type(&cx, written, &defined);
    let expected = Term::pi(
        musa_calculus::Origin::UNKNOWN,
        "A",
        type0(),
        Term::universe(musa_calculus::Origin::UNKNOWN, Sort::ZERO),
    );
    let (declared, _) = musa_calculus::infer(&cx, defined.ty.as_ref().expect("a written type")).expect("it is a type");
    assert!(
        musa_calculus::convertible_types(&cx, &declared, &expected).expect("both are types"),
        "a record's parameter is explicit, because a written `Cell<Nat>` is `Cell Nat`"
    );
}

/// A `fn` whose signature is complete carries it as a Π, and the value binds
/// without repeating it.
#[test]
fn a_written_signature_becomes_the_pi_and_the_lambda_binds_without_repeating_it() {
    let cx = host();
    let written = "fn tied(x: Nat, y: Nat) -> Bool { true }";
    let defined = definition(written, SyntaxKind::FnDecl);
    inhabits_its_written_type(&cx, written, &defined);
    assert!(
        matches!(defined.value.shape(), musa_calculus::RawShape::Lam { domain: None, .. }),
        "the λ binds bare: the domain is in the Π, and writing it twice would convert it against itself"
    );
}

/// `01-surface.md`'s `param := IDENT (":" type)?`, and what follows from it.
#[test]
fn a_signature_the_author_did_not_finish_writing_has_no_pi_to_hand_forward() {
    let written = "fn tied(x, y: Nat) -> Bool { true }";
    let defined = definition(written, SyntaxKind::FnDecl);
    assert!(
        defined.ty.is_none(),
        "there is no raw term for a type nobody wrote, and a hole is the core's to mint"
    );
    assert!(
        matches!(defined.value.shape(), musa_calculus::RawShape::Lam { domain: None, .. }),
        "and the value keeps what was written, so nothing is lost"
    );
}

#[test]
fn a_binding_carries_the_type_it_wrote_and_no_other() {
    let cx = host();
    let written = "let held: Nat = 3;";
    let defined = definition(written, SyntaxKind::LetDecl);
    inhabits_its_written_type(&cx, written, &defined);
    let bare = definition("let held = 3;", SyntaxKind::LetDecl);
    assert!(bare.ty.is_none(), "an unannotated binding has no written type");
    musa_calculus::infer(&cx, &bare.value).expect("and its value infers, which is why it needs none");
}

/// An `impl` block, read out of source and admitted: every `fn` in it is an
/// ordinary definition in the type's namespace.
///
/// §1.5's whole mechanism, seen at the lowering. There is no class, no instance,
/// and no table — `impl Same { fn same(…) }` is the definition `Same.same`, and
/// what the core admits is a definition like any other.
#[test]
fn an_impl_block_lowers_to_definitions_in_the_types_namespace() {
    let cx = host();
    let Item::Namespace(members) = item(
        "impl Same { fn same(x: Nat, y: Nat) -> Bool { true } }",
        SyntaxKind::ImplDecl,
    ) else {
        panic!("an `impl` declares a namespace");
    };
    assert_eq!(members.len(), 1, "one member, because one `fn` was written");
    let (_, defined) = members.first().expect("one member");
    assert_eq!(&*defined.name, "Same.same", "filed under the head, as a dotted name");
    inhabits_its_written_type(&cx, "fn same(x: Nat, y: Nat) -> Bool { true }", defined);
}

/// Two `fn`s in one block are two definitions, and the block itself is nothing.
///
/// The negative half of the law above: an `impl` is not a declaration with
/// members, it is a *spelling* for several declarations. Nothing about the block
/// survives into the core, which is why a second block at the same head is not a
/// duplicate of anything.
#[test]
fn a_second_block_at_one_head_adds_to_the_same_namespace() {
    let Item::Namespace(members) = item(
        "impl Same { fn same(x: Nat, y: Nat) -> Bool { true } fn apart(x: Nat, y: Nat) -> Bool { false } }",
        SyntaxKind::ImplDecl,
    ) else {
        panic!("an `impl` declares a namespace");
    };
    let named: Vec<&str> = members.iter().map(|(_, defined)| &*defined.name).collect();
    assert_eq!(named, vec!["Same.same", "Same.apart"], "in the order they were written");
}

// ---- refusals ----

/// The subject used to be `music { … }`, and then the event track quote. Prompt 141k
/// gave the notated block a core shape and [`crate::lower::events`] gave the
/// quote one, so the form left standing here is the **quote pattern** — the
/// inverse of `quote at here { … }`, which `11-quotation.md` §5 gives a
/// `match_quote` and `quote_hole` to read with and which prompt 142's Target
/// still owns the reading of.
#[test]
fn a_form_with_no_core_shape_is_refused_at_the_node_with_its_prompt_named() {
    let root = parsed("library { fn read(node: Syntax<TokenTree>) -> Nat { match node { quote { a } -> 1, } } }");
    let node = first(&root, SyntaxKind::Pattern);
    let mut resolver = Resolver::new();
    let mut sites = Sites::default();
    let raw = Lowering::new(&mut resolver, &mut sites).pattern(&node);
    assert!(
        raw.is_none(),
        "a quote pattern has no core *spelling* until 142 gives it one"
    );
    let complaints = resolver.diagnostics;
    assert_eq!(complaints.len(), 1, "one complaint, at the form");
    let complaint = complaints.first().expect("one complaint").message.as_str();
    assert!(
        complaint.contains("core spelling"),
        "and it says what is missing: {complaint}"
    );
}

/// A declaration is numbered like everything else, which is what lets a refusal
/// about one be pointed at the line that wrote it rather than at its body.
#[test]
fn a_definition_is_numbered_at_the_declaration_that_wrote_it() {
    let source = "library { let held: Nat = 3; }";
    let root = parsed(source);
    let node = first(&root, SyntaxKind::LetDecl);
    let mut resolver = Resolver::new();
    let mut sites = Sites::default();
    let lowered = Lowering::new(&mut resolver, &mut sites).item(&node);
    let Declared::Item(Item::Definition(defined)) = lowered else {
        panic!("a `let` is a definition");
    };
    let span = sites.span(defined.origin).expect("the definition was numbered");
    assert_eq!(
        source
            .get(span.start as usize..span.end as usize)
            .expect("the span is within the source"),
        "let held: Nat = 3;",
        "the whole declaration, not the value it binds"
    );
}

/// A context holding every `fn` of one `impl` block, in the head's namespace.
///
/// The whole of what a namespace needs, and it is the ordinary definition door:
/// an `impl` lowers to definitions, and a context that has them is a context
/// that has definitions.
fn namespacing(written: &str) -> Cx {
    let Item::Namespace(members) = item(written, SyntaxKind::ImplDecl) else {
        panic!("an `impl` declares a namespace");
    };
    let definitions = members
        .into_iter()
        .map(|(_, defined)| musa_calculus::RawTopLevel {
            origin: defined.origin,
            name: defined.name,
            visibility: musa_calculus::Visibility::Public,
            module: None,
            ty: defined.ty,
            value: defined.value,
        })
        .collect();
    let cx = host();
    let declared = musa_calculus::declare_program(&cx, &musa_calculus::RawProgram { definitions })
        .expect("a namespace member is a definition");
    cx.defining(&declared)
}

/// A context in which `Nat.same` is defined.
///
/// The smallest thing §1.5's member rule can be *about*: a head, and a member
/// spelled in its namespace that a use site can reach. Every law below writes
/// its call against this one.
fn with_same() -> Cx {
    namespacing("impl Nat { fn same(x: Nat, y: Nat) -> Bool { true } }")
}

/// The other half of [`Sites`]: a refusal the core raised, pointed back at the
/// text that caused it.
#[test]
fn a_core_refusal_is_restated_at_the_span_that_caused_it() {
    let cx = host();
    let source = "library { fn probe() -> Nat { misspelt } }";
    let root = parsed(source);
    let node = first(&root, SyntaxKind::BlockExpr);
    let mut resolver = Resolver::new();
    let mut sites = Sites::default();
    let raw = Lowering::new(&mut resolver, &mut sites)
        .expr(&node)
        .expect("an unknown name is written through rather than refused here");
    let failure = musa_calculus::check(&cx, &declared(&cx, "Nat"), &raw).expect_err("the core is what refuses it");
    let complaint = super::refusals::restate(&sites, &failure);
    assert_eq!(
        complaint.code,
        musa_score::diagnose::Code::UnknownName,
        "filed under the refusal's own family"
    );
    let span = complaint.labels.first().expect("pointed somewhere").span;
    assert_eq!(
        source
            .get(span.start as usize..span.end as usize)
            .expect("the span is within the source"),
        "misspelt",
        "at the word, which is the whole point of the site table"
    );
}

/// A refusal about a *registered* signature has nowhere of its own to point, and
/// says so rather than pointing at node one.
#[test]
fn a_refusal_carrying_no_written_origin_points_nowhere() {
    let sites = Sites::default();
    let complaint = super::refusals::restate(
        &sites,
        &musa_calculus::Refusal::UnknownName {
            at: musa_calculus::Origin::UNKNOWN,
            name: "nowhere".into(),
            candidates: Vec::new(),
        }
        .into(),
    );
    assert!(
        complaint.labels.is_empty(),
        "a signature nobody wrote has no span, and an empty label list is the honest answer"
    );
}

// ---- the site table ----

/// A site's number is what a refusal is pointed back at.
#[test]
fn a_site_answers_the_span_it_was_numbered_for() {
    let mut sites = Sites::default();
    let first = sites.at(musa_score::origin::SourceSpan::new(3, 9));
    let second = sites.at(musa_score::origin::SourceSpan::new(11, 14));
    assert_eq!(sites.span(first), Some(musa_score::origin::SourceSpan::new(3, 9)));
    assert_eq!(sites.span(second), Some(musa_score::origin::SourceSpan::new(11, 14)));
    assert_eq!(
        sites.span(musa_calculus::Origin::UNKNOWN),
        None,
        "a term nobody wrote has nowhere to point, and says so"
    );
}

// ---- the qualified path ----

/// The three ways one definition is reached, and the law that they are one
/// definition.
///
/// §1.5's whole claim about access. `Nat.same` is defined once by [`with_same`];
/// the path writes its name out, method syntax finds it by the head of the
/// receiver's type, and neither is a different program from the other.
#[test]
fn a_member_is_reached_by_the_path_and_by_the_receiver_alike() {
    let cx = with_same();
    for written in [
        "fn probe() -> Bool { Nat::same(1, 2) }",
        "fn probe() -> Bool { 1.same(2) }",
    ] {
        inhabits_its_written_type(&cx, written, &definition(written, SyntaxKind::FnDecl));
    }
}

/// §1.5's own reading, on §1.5's own example shape: the modules are dropped and
/// the type and its item are the name.
#[test]
fn a_module_prefix_reads_to_the_name_it_qualifies() {
    let named = |written: &str| -> String {
        let (built, complaints) = lowered_expr(written);
        assert!(
            complaints.is_empty(),
            "`{written}` is read, not refused: {complaints:?}"
        );
        match built.unwrap_or_else(|| panic!("`{written}` lowers")).shape() {
            RawShape::Var(name) | RawShape::Hosted(name) => name.to_string(),
            other => panic!("a path is a name for the core to resolve, not {other:?}"),
        }
    };
    assert_eq!(
        named("std::tonal::Same::same"),
        named("Same::same"),
        "`std::tonal::` says where the name lives and nothing about which name it is"
    );
}

/// A case named in its type's namespace, in both positions that admit one.
///
/// The pattern half is the same defect one position over: `pattern` read a flat
/// run of tokens, so `Tying::Untied` bound `Untied` and matched a constructor
/// named `Tying`. Both now go through [`Lowering::qualified`], which is what
/// makes them the same name rather than two readings that agree by accident.
#[test]
fn an_enum_case_is_reached_by_its_type_in_an_expression_and_in_a_pattern() {
    let cx = host();
    let tying = musa_calculus::declare(&cx, &declaration("enum Tying { Untied, Tied }", SyntaxKind::EnumDecl))
        .expect("the core declares the family");
    let cx = cx.declaring(&tying);

    let (built, complaints) = lowered_expr("Tying::Untied");
    assert!(complaints.is_empty(), "the path is read: {complaints:?}");
    musa_calculus::check(&cx, &declared(&cx, "Tying"), &built.expect("the path lowers"))
        .expect("and the case it names inhabits its own type");

    let matched = lowered_pattern("Tying::Untied").expect("the pattern lowers");
    let RawPattern::Constructor { name, fields, .. } = &matched else {
        panic!("a case named in a namespace is a constructor pattern, not {matched:?}");
    };
    assert_eq!(&**name, "Tying.Untied", "the whole path is the constructor's name");
    assert!(
        fields.is_empty(),
        "and `Untied` is the case rather than a field it binds"
    );
}

/// §5's operator table, and the rule §1.5 states about it: an operator is
/// method syntax on its left operand.
///
/// The repair prompt 146 made. `x == y` used to lower to `Eq::equal(x, y)` for
/// one reason, stated in the old lowering's own comment: an operator under a
/// `where` had to resolve against the supplied dictionary. There is no `where`,
/// so there is no second path — the operator writes `equal` on the left operand
/// and the receiver's head picks the definition. The two spellings below are one
/// program, which is what makes the table a *sugar* rather than a mechanism.
///
/// `Nat.equal` is written here rather than shipped: the prelude gives the five
/// base types an `equal` and gives `Nat` none, on the stated ground that `Nat`
/// is a declared family and ι already answers. What a law needs is something for
/// `==` to resolve to while it checks that it resolves at all.
#[test]
fn an_operator_is_method_syntax_on_its_left_operand() {
    let cx = namespacing("impl Nat { fn equal(x: Nat, y: Nat) -> Bool { true } }");
    for written in ["fn probe() -> Bool { 1.equal(2) }", "fn probe() -> Bool { 1 == 2 }"] {
        inhabits_its_written_type(&cx, written, &definition(written, SyntaxKind::FnDecl));
    }
}

/// The one refusal a *reading* can raise about a path.
///
/// "Exactly one segment follows it" is a claim about the written text, so it is
/// answerable without resolving anything — which is what keeps it here rather
/// than in the core, where every other thing that can be wrong with a path is
/// answered.
#[test]
fn a_path_naming_two_items_in_one_namespace_is_refused_at_the_node() {
    let (built, complaints) = lowered_expr("TokenKind::PitchLiteral::spelling");
    assert!(built.is_none(), "a path with no reading has no term");
    let complaint = complaints.first().expect("one complaint, at the path");
    assert_eq!(complaint.code, Code::QualifiedPath, "filed under its own code");
    assert!(
        complaint.message.contains("one item"),
        "and it says what a namespace holds: {}",
        complaint.message
    );
}

/// §2's `play`, as `stdlib/src/voicing.musa:57` writes it: `fn sound_for(chosen:
/// Voicing, held: Duration<WrittenTime>) -> … { play(chosen, held) }`.
///
/// Two claims in one line. `play` answers a track and not a `Result` — prompt
/// 141m's refusal channel — and a composer writes two arguments where the
/// registered operation takes four, because §5.7's origin and scope are the
/// reading's to supply and a composer has neither. Either one missing and the
/// line does not elaborate, which is why the law hands the whole function to the
/// core rather than looking at the application's shape.
///
/// The two Πs are built rather than written. `-> EventTrack[WrittenTime,
/// ScoreFact]` is not a spelling the grammar has — today a track type is written
/// `Music`, and replacing that word is prompt 142's — and the binders are the
/// whole difference between this and §2's own line.
///
/// §2 writes the length as the literal `1/2`, and that does *not* check: a
/// written `1/2` is a `Ratio` and `play` reads a `Duration ⟨written⟩`, which is a
/// different type and the core says so. A finding recorded in this prompt's file
/// rather than an edit, because what a bare literal may mean at a coordinate type
/// is the literal domains' question and not the refusal channel's.
#[test]
fn a_written_play_supplies_the_origin_and_the_scope_no_composer_has() {
    let cx = host();
    let (built, complaints) = lowered_expr("play(chosen, held)");
    assert!(
        complaints.is_empty(),
        "the stdlib's own body lowers without complaint: {complaints:?}"
    );
    let built = built.expect("the body lowers");
    let sound_for = Raw::lam(
        musa_calculus::Origin::UNKNOWN,
        "chosen",
        Raw::lam(musa_calculus::Origin::UNKNOWN, "held", built),
    );
    let sounds = Term::pi(
        musa_calculus::Origin::UNKNOWN,
        "chosen",
        crate::registry::plain_type("Voicing"),
        Term::pi(
            musa_calculus::Origin::UNKNOWN,
            "held",
            crate::registry::tagged_type("Duration", crate::phase::Coordinate::WrittenTime),
            crate::registry::tagged_type("EventTrack", crate::phase::Coordinate::WrittenTime),
        ),
    );
    musa_calculus::check(&cx, &sounds, &sound_for)
        .unwrap_or_else(|failure| panic!("a voicing played for a length sounds a track, not {failure:?}"));
}
