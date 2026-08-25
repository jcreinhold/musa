//! Glued evaluation: a definition use stays folded behind a `Head::Def` that
//! carries the value, δ runs on demand, and conversion tries the folded
//! comparison first.
//!
//! One test per property the prompt names: a body is evaluated once per
//! definition however many uses it has; a folded comparison spends what
//! comparing two *references* costs, not what comparing their unfoldings
//! costs; the retry on a folded disagreement keeps conversion complete; and a
//! refusal quotes the name the author wrote rather than the unfolding the
//! elaborator matched against.

use musa_calculus::{
    Raw, RawProgram, RawTopLevel, Refusal, Shape, Visibility, check, convertible, convertible_metered, declare_program,
    declare_program_metered,
};

use crate::family_laws::{apply, core_constant, nat_context, type0, var};
use crate::programs::WRITTEN;

/// `name : ty = value`, public and in no module — `program_laws`' shape.
fn definition(name: &str, ty: Option<Raw>, value: Raw) -> RawTopLevel {
    RawTopLevel {
        origin: WRITTEN,
        name: name.into(),
        visibility: Visibility::Public,
        module: None,
        ty,
        value,
    }
}

fn program(definitions: Vec<RawTopLevel>) -> RawProgram {
    RawProgram {
        families: Vec::new(),
        definitions,
    }
}

fn arrow(domain: Raw, codomain: Raw) -> Raw {
    Raw::pi(WRITTEN, "_", domain, codomain)
}

/// `let a0: Nat = Nat.Zero in let a1: Nat = Nat.Succ(a0) in … in a<steps>`.
///
/// A body that is cheap to *declare* and expensive to *reduce*: each level is
/// a handful of steps, so `steps` levels cost orders of magnitude more than a
/// reference to the result does.
fn deep(steps: u32) -> Raw {
    let mut body = var(&format!("a{steps}"));
    for i in (1..=steps).rev() {
        body = Raw::annotated_bind(
            WRITTEN,
            format!("a{i}"),
            var("Nat"),
            apply(var("Nat.Succ"), [var(&format!("a{}", i.saturating_sub(1)))]),
            body,
        );
    }
    Raw::annotated_bind(WRITTEN, "a0", var("Nat"), var("Nat.Zero"), body)
}

/// The sharing law. A use of a definition is a reference to the value the
/// declaration computed once; if folding re-evaluated the body at a use, the
/// spend of eight uses would carry the body's cost eight times.
#[test]
fn a_definitions_body_is_evaluated_once_however_many_uses_it_has() {
    let (cx, _) = nat_context();
    let one = program(vec![
        definition("count", Some(var("Nat")), deep(120)),
        definition("u1", Some(var("Nat")), var("count")),
    ]);
    let many = program(vec![
        definition("count", Some(var("Nat")), deep(120)),
        definition("u1", Some(var("Nat")), var("count")),
        definition("u2", Some(var("Nat")), var("count")),
        definition("u3", Some(var("Nat")), var("count")),
        definition("u4", Some(var("Nat")), var("count")),
        definition("u5", Some(var("Nat")), var("count")),
        definition("u6", Some(var("Nat")), var("count")),
        definition("u7", Some(var("Nat")), var("count")),
        definition("u8", Some(var("Nat")), var("count")),
    ]);
    let (_, one_spend) = declare_program_metered(&cx, &one).expect("the one-use program declares");
    let (_, many_spend) = declare_program_metered(&cx, &many).expect("the eight-use program declares");
    assert!(
        one_spend.steps >= 1_000,
        "the body is the cost being shared: {}",
        one_spend.steps
    );
    assert!(
        many_spend.steps - one_spend.steps <= 7 * 64,
        "seven more uses cost references, not re-evaluations: {} + {}",
        one_spend.steps,
        many_spend.steps - one_spend.steps
    );
    assert!(
        many_spend.constructed_nodes - one_spend.constructed_nodes <= 7 * 16,
        "seven more names construct references, not the named value again: {} + {} nodes",
        one_spend.constructed_nodes,
        many_spend.constructed_nodes - one_spend.constructed_nodes
    );
    assert!(
        many_spend.logical_bytes - one_spend.logical_bytes <= 7 * 32,
        "seven more names wire references, not the named value again: {} + {} bytes",
        one_spend.logical_bytes,
        many_spend.logical_bytes - one_spend.logical_bytes
    );
}

/// The folded comparison, stated as a spend law: two uses of one definition
/// compare as *references*, so the spend does not move when the definition's
/// normal form does. `f` and `g` below differ only in how deep their bodies
/// nest; comparing a use of either to itself costs the same handful of steps.
/// And the other half: two *different* heads cannot share, so asking `f z ≡ g z`
/// pays for the unfolding — the spend says the unfold is real.
#[test]
fn a_folded_comparison_does_not_pay_for_the_unfolding() {
    let (cx, _) = nat_context();
    let body = |steps| Raw::lam(WRITTEN, "x", deep(steps));
    let raw = program(vec![
        definition("f", Some(arrow(var("Nat"), var("Nat"))), body(60)),
        definition("g", Some(arrow(var("Nat"), var("Nat"))), body(120)),
        // `f`'s body written out again under another name: the same value, a
        // different head.
        definition("h", Some(arrow(var("Nat"), var("Nat"))), body(60)),
        definition("z", Some(var("Nat")), var("Nat.Zero")),
    ]);
    let declared = declare_program(&cx, &raw).expect("the program declares");
    let full = cx.defining(&declared);
    let nat_ty = core_constant(&cx, "Nat");
    let fz = check(&full, &nat_ty, &apply(var("f"), [var("z")])).expect("f z checks");
    let gz = check(&full, &nat_ty, &apply(var("g"), [var("z")])).expect("g z checks");
    let hz = check(&full, &nat_ty, &apply(var("h"), [var("z")])).expect("h z checks");

    let (answer, folded_f) = convertible_metered(&full, &nat_ty, &fz, &fz).expect("decided");
    assert!(answer);
    let (answer, folded_g) = convertible_metered(&full, &nat_ty, &gz, &gz).expect("decided");
    assert!(answer);
    assert_eq!(
        folded_f.steps, folded_g.steps,
        "the spend does not measure the unfolding: {} steps either way",
        folded_f.steps
    );
    assert!(
        folded_f.steps <= 64,
        "two references compare as references: {} steps",
        folded_f.steps
    );
    let (answer, opened) = convertible_metered(&full, &nat_ty, &fz, &hz).expect("decided");
    assert!(answer, "the bodies agree: `h` is `f`'s body under another name");
    assert!(
        opened.steps >= 10 * folded_f.steps,
        "different heads unfold, and the unfolding is what is paid for: {} vs {}",
        opened.steps,
        folded_f.steps
    );
}

/// The retry is what keeps the folded comparison *complete*: a spine
/// disagreement between two uses of one definition is not a mismatch, because
/// the definition may not use its argument. Unfolding decides.
#[test]
fn a_folded_disagreement_still_decides_by_unfolding() {
    let (cx, _) = nat_context();
    let raw = program(vec![
        definition(
            "constantly",
            Some(arrow(var("Nat"), var("Nat"))),
            Raw::lam(WRITTEN, "_", var("Nat.Zero")),
        ),
        definition("z", Some(var("Nat")), var("Nat.Zero")),
        definition("w", Some(var("Nat")), apply(var("Nat.Succ"), [var("Nat.Zero")])),
    ]);
    let declared = declare_program(&cx, &raw).expect("the program declares");
    let full = cx.defining(&declared);
    let nat_ty = core_constant(&cx, "Nat");
    let at_z = check(&full, &nat_ty, &apply(var("constantly"), [var("z")])).expect("checks");
    let at_w = check(&full, &nat_ty, &apply(var("constantly"), [var("w")])).expect("checks");
    assert_eq!(
        convertible(&full, &nat_ty, &at_z, &at_w),
        Ok(true),
        "the spines disagree and the uses agree: unfolding decided"
    );
}

/// The diagnostic law. Applying a value whose *type* the author wrote as a
/// definition is refused by quoting that type — and the quotation keeps the
/// name, because the name is what the author wrote. The elaborator matched
/// against the unfolding; the message is not built from it.
#[test]
fn a_refusal_quotes_the_name_the_author_wrote() {
    let (cx, _) = nat_context();
    let raw = program(vec![
        definition("MyNat", Some(type0()), var("Nat")),
        definition("n", Some(var("MyNat")), var("Nat.Zero")),
        definition("bad", Some(var("Nat")), apply(var("n"), [var("Nat.Zero")])),
    ]);
    let error = declare_program(&cx, &raw).expect_err("applying a Nat is refused");
    let musa_calculus::ElabError::Refused(Refusal::NotAFunction { ty, .. }) = error else {
        panic!("applying a Nat is NotAFunction, got {error}");
    };
    let Shape::Named {
        name,
        role: musa_calculus::Role::Defined,
        ..
    } = ty.shape()
    else {
        panic!("the refusal names the author's definition, got {ty:?}");
    };
    assert_eq!(name.to_string(), "MyNat");
}
