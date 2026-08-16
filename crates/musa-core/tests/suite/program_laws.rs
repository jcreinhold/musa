//! §2.4's last paragraph: a document's definitions, declared together.
//!
//! "Top-level signatures are collected before bodies are elaborated, so a later
//! declaration may be referenced. From each body the checker records an edge to
//! every free named declaration. **Non-recursive** declarations must form an
//! acyclic graph, exactly as before; a *recursive* declaration is the case §2.4
//! admits, and it is admitted through the measure rather than through the
//! graph."
//!
//! Four sentences, and the laws below are one per sentence plus the two the
//! arrangement owes on its own: a definition is filtered by 136a's visibility
//! rule like any other declaration, and a *use* of one is a reference rather
//! than a copy — which is the property that keeps `stdlib/`'s two thousand
//! definitions from being quadratic in the corpus.

use musa_core::{
    Cx, Definitions, ModuleId, Raw, RawArm, RawPattern, RawProgram, RawTopLevel, Refusal, Shape, Term, Visibility,
};

use crate::coverage_laws::nat_vec_context;
use crate::family_laws::{apply, var};
use crate::programs::{WRITTEN, refusal};

/// The module the private definitions below are written in.
const INSIDE: ModuleId = ModuleId::new(1);

/// A module that is not [`INSIDE`].
const OUTSIDE: ModuleId = ModuleId::new(2);

fn arrow(domain: Raw, codomain: Raw) -> Raw {
    Raw::pi(WRITTEN, "_", domain, codomain)
}

/// `name : ty = value`, public and in no module.
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

/// The same, `private` and written in `module`.
fn private_in(module: ModuleId, name: &str, ty: Option<Raw>, value: Raw) -> RawTopLevel {
    RawTopLevel {
        origin: WRITTEN,
        name: name.into(),
        visibility: Visibility::Private,
        module: Some(module),
        ty,
        value,
    }
}

fn program(definitions: Vec<RawTopLevel>) -> RawProgram {
    RawProgram { definitions }
}

fn number(count: u32) -> Raw {
    (0..count).fold(var("Nat.Zero"), |built, _| apply(var("Nat.Succ"), [built]))
}

/// The type `Nat`, as a core term.
///
/// # Panics
///
/// If `Nat` is not in scope, which is a defect in the fixture rather than in
/// any law.
fn nat(cx: &Cx) -> Term {
    musa_core::infer(cx, &var("Nat")).expect("Nat is declared").0
}

/// The group, declared, or a panic naming what refused it.
///
/// # Panics
///
/// When elaboration refuses a program a law says it accepts.
fn declared(name: &str, cx: &Cx, program: &RawProgram) -> std::sync::Arc<Definitions> {
    musa_core::declare_program(cx, program).unwrap_or_else(|error| panic!("{name}: {error}"))
}

/// The refusal a group answers with, or a panic naming what it did instead.
///
/// # Panics
///
/// When elaboration accepts a program a law says it refuses, or fails for a
/// reason that is not a refusal at all (§4).
fn refused(name: &str, cx: &Cx, program: &RawProgram) -> Refusal {
    let Err(error) = musa_core::declare_program(cx, program) else {
        panic!("{name}: the group was declared, and this law says it cannot be");
    };
    refusal(name, error)
}

/// How many nodes a term holds.
///
/// The measure the size law is stated in. A use of a definition is one node
/// whatever the definition is, and that is what this counts.
fn size(term: &Term) -> u32 {
    let inner = match term.shape() {
        Shape::Var(_)
        | Shape::Const(_)
        | Shape::Def(_)
        | Shape::Base(_)
        | Shape::Builtin(_)
        | Shape::Lit(_)
        | Shape::Universe(_)
        | Shape::Meta(_) => 0,
        Shape::Pi { domain, codomain, .. } => size(domain).saturating_add(size(codomain)),
        Shape::Lam { body, .. } => size(body),
        Shape::App { function, argument } => size(function).saturating_add(size(argument)),
        Shape::RecordType(fields) | Shape::Record(fields) => fields
            .iter()
            .fold(0_u32, |total, field| total.saturating_add(size(&field.term))),
        Shape::Project { record, .. } => size(record),
        Shape::Id { ty, left, right } => size(ty).saturating_add(size(left)).saturating_add(size(right)),
        Shape::Refl(witness) => size(witness),
        Shape::J {
            ty,
            from,
            motive,
            base,
            to,
            proof,
        } => [from, motive, base, to, proof]
            .iter()
            .fold(size(ty), |total, part| total.saturating_add(size(part))),
        Shape::Let { ty, value, body, .. } => size(ty).saturating_add(size(value)).saturating_add(size(body)),
    };
    inner.saturating_add(1)
}

/// §2.4: a body may name a declaration written later.
///
/// The whole reason this door exists. `earlier` is written first and calls
/// `later`, which the document has not reached yet — the shape
/// `examples/neo-riemannian.musa:72` has when it calls the `compose_close`
/// declared at `:151`, and the shape a nested `let` chain could never have.
#[test]
fn a_body_may_name_a_definition_written_after_it() {
    let cx = nat_vec_context();
    let group = program(vec![
        definition("earlier", Some(var("Nat")), apply(var("later"), [number(2)])),
        definition(
            "later",
            Some(arrow(var("Nat"), var("Nat"))),
            Raw::lam(WRITTEN, "n", apply(var("Nat.Succ"), [var("n")])),
        ),
    ]);
    let definitions = declared("a forward reference", &cx, &group);
    let inside = cx.defining(&definitions);
    let ty = nat(&inside);
    let term = musa_core::check(&inside, &ty, &var("earlier")).expect("the definition is in scope after the group");
    let normal = musa_core::normalize(&inside, &ty, &term).expect("and it computes");
    let expected = musa_core::check(&inside, &ty, &number(3)).expect("three is a natural number");
    assert!(
        musa_core::convertible(&inside, &ty, &normal, &expected).expect("conversion answers"),
        "the forward call ran: `later 2` is 3"
    );
}

/// The same, for a definition that wrote no type.
///
/// The order is *computed*, so an unannotated definition is referable from
/// anywhere too — it is finished before anything that names it, exactly as an
/// annotated one is. A rule that made an unannotated definition referable only
/// from after itself would refuse this program, and §2.4 admits it.
#[test]
fn a_body_may_name_an_unannotated_definition_written_after_it() {
    let cx = nat_vec_context();
    let group = program(vec![
        definition("earlier", Some(var("Nat")), apply(var("Nat.Succ"), [var("later")])),
        definition("later", None, number(2)),
    ]);
    let definitions = declared("a forward reference to an inferred definition", &cx, &group);
    let inside = cx.defining(&definitions);
    let ty = nat(&inside);
    let term = musa_core::check(&inside, &ty, &var("earlier")).expect("the definition is in scope");
    let normal = musa_core::normalize(&inside, &ty, &term).expect("and it computes");
    let expected = musa_core::check(&inside, &ty, &number(3)).expect("three is a natural number");
    assert!(
        musa_core::convertible(&inside, &ty, &normal, &expected).expect("conversion answers"),
        "the inferred definition was elaborated before the one that names it"
    );
}

/// §2.4: non-recursive declarations must form an acyclic graph.
///
/// Two definitions naming each other, which the measure cannot reach: a
/// hypothesis is minted by a `match` inside *one* body, so a mutually recursive
/// pair has no hypothesis to become. The refusal names both, because a cycle
/// with one name in it tells a reader half of where to look.
#[test]
fn two_definitions_that_name_each_other_are_refused_by_name() {
    let cx = nat_vec_context();
    let group = program(vec![
        definition("ping", Some(var("Nat")), var("pong")),
        definition("pong", Some(var("Nat")), var("ping")),
    ]);
    let refusal = refused("a cycle of two", &cx, &group);
    let Refusal::DefinitionCycle { names, .. } = &refusal else {
        panic!("refused, but as `{refusal}`");
    };
    for expected in ["ping", "pong"] {
        assert!(
            names.iter().any(|name| **name == *expected),
            "the cycle names `{expected}`: {refusal}"
        );
    }
}

/// §2.4: a *recursive* declaration is admitted through the measure.
///
/// A self-edge is not a cycle. `add` recurses on a field of its own `match`,
/// which is exactly what `rec.rs` admits — and the group builds the `rec` form
/// itself, so the surface never has to decide whether a definition is
/// recursive. The sums are what make this a law about the rewrite rather than
/// about willingness: `add` is gone from the accepted term, replaced by the
/// induction hypothesis its `Succ` branch was handed.
#[test]
fn a_self_recursive_definition_goes_to_the_measure() {
    let cx = nat_vec_context();
    let group = program(vec![definition(
        "add",
        Some(arrow(var("Nat"), arrow(var("Nat"), var("Nat")))),
        Raw::lam(
            WRITTEN,
            "a",
            Raw::lam(
                WRITTEN,
                "b",
                Raw::match_on(
                    WRITTEN,
                    [var("a")],
                    vec![
                        RawArm {
                            patterns: vec![RawPattern::constructor(WRITTEN, "Nat.Zero", [])],
                            body: var("b"),
                        },
                        RawArm {
                            patterns: vec![RawPattern::constructor(
                                WRITTEN,
                                "Nat.Succ",
                                [RawPattern::bind(WRITTEN, "k")],
                            )],
                            body: apply(var("Nat.Succ"), [apply(var("add"), [var("k"), var("b")])]),
                        },
                    ],
                ),
            ),
        ),
    )]);
    let definitions = declared("self-recursive addition", &cx, &group);
    let inside = cx.defining(&definitions);
    let ty = nat(&inside);
    for (left, right) in [(0, 0), (0, 3), (2, 0), (2, 3)] {
        let written = apply(var("add"), [number(left), number(right)]);
        let sum = musa_core::check(&inside, &ty, &written).expect("addition applies");
        let normal = musa_core::normalize(&inside, &ty, &sum).expect("and computes");
        let expected = musa_core::check(&inside, &ty, &number(left + right)).expect("so does the answer");
        assert!(
            musa_core::convertible(&inside, &ty, &normal, &expected).expect("conversion answers"),
            "{left} + {right} is {}",
            left + right
        );
    }
}

/// A definition that names itself and wrote no type is refused.
///
/// Not a weaker case of the law above: the measure is checked against the type
/// the author wrote, and inference has nothing to infer from a body whose
/// meaning is the definition being inferred.
#[test]
fn a_self_recursive_definition_has_to_write_its_type() {
    let cx = nat_vec_context();
    let group = program(vec![definition("loop", None, apply(var("Nat.Succ"), [var("loop")]))]);
    let refusal = refused("self-recursion with no signature", &cx, &group);
    assert!(
        matches!(&refusal, Refusal::UntypedRecursion { name, .. } if **name == *"loop"),
        "refused, but as `{refusal}`"
    );
}

/// 136a's visibility rule, unchanged, for a definition.
///
/// The same mechanism a private constructor is filtered by, and the same
/// refusal: `private` is not `unknown-name`, because the two send a reader to
/// different places.
#[test]
fn a_private_definition_is_invisible_from_another_module() {
    let cx = nat_vec_context();
    let group = program(vec![private_in(INSIDE, "secret", Some(var("Nat")), number(1))]);
    let definitions = declared("a private definition", &cx, &group);
    let ty = nat(&cx);

    let at_home = cx.defining(&definitions).in_module(INSIDE);
    musa_core::check(&at_home, &ty, &var("secret")).expect("its own module may name it");

    let elsewhere = cx.defining(&definitions).in_module(OUTSIDE);
    let Err(error) = musa_core::check(&elsewhere, &ty, &var("secret")) else {
        panic!("another module named a private definition");
    };
    let refusal = refusal("a private definition", error);
    assert!(
        matches!(&refusal, Refusal::Private { module, .. } if *module == INSIDE),
        "refused, but as `{refusal}`"
    );
}

/// A use is a reference, not a copy.
///
/// The term a use elaborates to is one node whatever the definition holds, and
/// this states it as a *comparison* rather than as a constant: two definitions
/// of very different size are named, and the two uses are the same size. An
/// implementation that inlined the body would pass a one-node assertion only by
/// accident and fail this one immediately — and would make `stdlib/` quadratic
/// in the corpus, put the whole standard library inside every normal form, and
/// make a semantic hash a function of what a definition happens to be written
/// as.
#[test]
fn a_use_of_a_definition_does_not_grow_with_its_body() {
    let cx = nat_vec_context();
    let group = program(vec![
        definition("small", Some(var("Nat")), number(1)),
        definition("large", Some(var("Nat")), number(64)),
    ]);
    let definitions = declared("two definitions of different size", &cx, &group);
    let inside = cx.defining(&definitions);
    let ty = nat(&inside);
    let small = musa_core::check(&inside, &ty, &var("small")).expect("`small` is in scope");
    let large = musa_core::check(&inside, &ty, &var("large")).expect("`large` is in scope");
    assert_eq!(
        size(&small),
        size(&large),
        "a use costs the same whatever it names, and both are one node"
    );
    assert_eq!(size(&small), 1, "one node");
}

/// The two refusals this door owns, for `elaboration_laws.rs`'s coverage gate.
///
/// Beside the laws rather than inside them because the gate asks a different
/// question — *every* refusal has a program that reaches it — and a refusal
/// reachable only through a law nobody remembered to list is the gap it exists
/// to close.
pub(crate) fn refused_groups() -> Vec<(&'static str, RawProgram)> {
    vec![
        (
            "definitions that name each other",
            program(vec![
                definition("ping", Some(var("Nat")), var("pong")),
                definition("pong", Some(var("Nat")), var("ping")),
            ]),
        ),
        (
            "a definition that names itself with no written type",
            program(vec![definition("loop", None, apply(var("Nat.Succ"), [var("loop")]))]),
        ),
    ]
}
