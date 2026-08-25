//! Whether a structure can carry its laws
//! (`docs/rules/language/05-verification.md` §4.0).
//!
//! Prompt 156 made `Equal` declarable and prompt 157 made a structure an
//! ordinary record, so a field may be an equation and a record literal that
//! violates one is refused where it is written. Prompt 163 asked what follows:
//! do `Group`, `Action` and `Torsor` gain law fields?
//!
//! They do not, and the answer is a measurement rather than a policy. **A
//! δ-rule computes on canonical data, so it does not step under a
//! constructor.** `nat_add` is a builtin, `x + 0` is stuck for an abstract `x`,
//! splitting `x` leaves `nat_add (Succ i) 0` stuck in the same way, and every
//! carrier in `stdlib/` reaches that arithmetic before it reaches a normal
//! form. A quantified law over any of them has no inhabitant, so a field typed
//! by one could never be filled.
//!
//! This file exists so that the measurement cannot go stale quietly. Each test
//! below pins one half of it: what works, what is refused, and what the refusal
//! says. If a δ-rule ever unfolds on a neutral, the refusals here go green in a
//! test that expects red, and the laws over the affected carrier should be
//! re-asked as fields.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_score::{Code, Severity};

/// The imports every fixture below is written against.
const PRELUDE: &str = r"
    import std::cyclic;
    import std::indexed;
    import std::post_tonal::pcset;

    meter 4/4;
";

/// A piece carrying `bindings` and sounding nothing in particular.
fn probe(bindings: &str) -> String {
    format!("piece \"Law\" {{\n{PRELUDE}\n{bindings}\n    score {{ part p {{ voice v {{ c4/1 }} }} }}\n}}\n")
}

fn errors_of(source: &str) -> Vec<(Code, String)> {
    compile(&SourceDocument::new(source, "test.musa"), &CompileOptions::default())
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
        .collect()
}

/// The bindings elaborate, which is what makes each `Equal` a proof.
fn holds(bindings: &str) {
    let errors = errors_of(&probe(bindings));
    assert!(errors.is_empty(), "expected a clean compile, got {errors:?}");
}

/// The bindings are refused, and the message says what stayed stuck.
fn stuck_on(bindings: &str, needle: &str) {
    let errors = errors_of(&probe(bindings));
    assert!(
        errors
            .iter()
            .any(|(code, message)| *code == Code::ConversionMismatch && message.contains(needle)),
        "expected a conversion mismatch naming `{needle}`, got {errors:?}"
    );
}

#[test]
fn a_record_field_may_be_an_equation_and_a_false_one_is_refused_at_the_literal() {
    holds(
        "    record Doubler { twice: Nat -> Nat; fixes_zero: Equal(Nat, twice(Zero), Zero); }
    fn itself_nat(n: Nat) -> Nat { n }
    let honest: Doubler = Doubler { twice = itself_nat, fixes_zero = Refl(Zero) };",
    );
    stuck_on(
        "    record Doubler { twice: Nat -> Nat; fixes_zero: Equal(Nat, twice(Zero), Zero); }
    fn one_more(n: Nat) -> Nat { Succ(n) }
    let lying: Doubler = Doubler { twice = one_more, fixes_zero = Refl(Zero) };",
        "expected",
    );
}

#[test]
fn induction_proves_a_law_when_the_operation_is_written_by_matching() {
    holds(
        "    fn cong({A: Type}, {B: Type}, f: A -> B, x: A, y: A, same: Equal(A, x, y)) -> Equal(B, f(x), f(y)) {
        match same { Refl(only) -> Refl(f(only)), }
    }
    fn plus(left: Nat, right: Nat) -> Nat {
        match left { Zero -> right, Succ(less) -> Succ(plus(less, right)), }
    }
    fn one_more(n: Nat) -> Nat { Succ(n) }
    fn right_unit(x: Nat) -> Equal(Nat, plus(x, Zero), x) {
        match x {
            Zero -> Refl(Zero),
            Succ(less) -> cong(one_more, plus(less, Zero), less, right_unit(less)),
        }
    }",
    );
}

#[test]
fn a_delta_rule_does_not_step_under_a_constructor() {
    stuck_on("    fn add_zero(x: Nat) -> Equal(Nat, x + 0, x) { Refl(x) }", "nat_add");
    stuck_on(
        "    fn add_zero(x: Nat) -> Equal(Nat, x + 0, x) {
        match x { Zero -> Refl(0), Succ(less) -> Refl(Succ(less)), }
    }",
        "nat_add",
    );
}

#[test]
fn no_law_over_the_musical_carriers_has_an_inhabitant() {
    stuck_on(
        "    fn act_unit(n: Nat, place: Cyclic(n))
        -> Equal(Cyclic(n), moved(n, place, Transpose(0)), place) { Refl(place) }",
        "number_of",
    );
    stuck_on(
        "    fn ti_left_unit(n: Nat, cycle: Cycle(n), by: Ti)
        -> Equal(Ti, ti_compose(n, cycle, Transpose(0), by), by)
    {
        match by { Transpose(steps) -> Refl(Transpose(steps)), Invert(about) -> Refl(Invert(about)), }
    }",
        "folded",
    );
    stuck_on(
        "    fn pc_act_unit(n: Nat, cycle: Cycle(n), member: Pc(n))
        -> Equal(Pc(n), class_moved(n, cycle, member, Transpose(0)), member) { Refl(member) }",
        "place_in",
    );
}

#[test]
fn the_closed_form_of_each_law_still_computes() {
    holds(
        "    let composed: Equal(Ti, ti_compose(12, chromatic, Transpose(0), Transpose(3)), Transpose(3)) =
        Refl(Transpose(3));
    let acted: Equal(Nat,
        class_number(12, class_moved(12, chromatic, pc(12, chromatic, 5), Transpose(0))),
        5,
    ) = Refl(5);",
    );
}

#[test]
fn a_parameter_in_a_function_type_cannot_be_named() {
    let errors = errors_of(&probe(
        "    record Quantified { op: Nat -> Nat; left_unit: (x: Nat) -> Equal(Nat, op(x), x); }",
    ));
    assert!(
        errors.iter().any(|(code, _)| *code == Code::Syntax),
        "a named parameter in a function type is a parse error: {errors:?}"
    );
}
