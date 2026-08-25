//! Exact checked source artifacts preserve the canonical-data contract.

use musa_calculus::{CheckedSourceError, Cx, Origin, Raw, SourceDatumKind, SourceSchema, checked_source};

use crate::family_laws::{binder, constructor, data, family, var};

const HERE: Origin = Origin::node(401);

fn box_context() -> Cx {
    let cx = crate::programs::cx();
    let declaration = data(
        Vec::new(),
        vec![family(
            "Box",
            vec![constructor("Box", vec![binder("value", var("Unit"))])],
        )],
    );
    let group = musa_calculus::declare(&cx, &declaration).expect("the record-shaped family declares");
    cx.declaring(&group)
}

fn boxed() -> Raw {
    Raw::call(HERE, Raw::var(HERE, "Box.Box"), [Raw::var(HERE, "Unit.Unit")])
}

#[test]
fn a_checked_record_is_one_canonical_constructor_not_a_parallel_record_shape() {
    let cx = box_context();
    let artifact = checked_source(&cx, &boxed(), &SourceSchema::new("test.box", "Box", 3), |_| None)
        .expect("the closed record is canonical source data");
    assert_eq!(
        artifact.root().kind(),
        Some(SourceDatumKind::Case { constructor: "Box.Box" })
    );
    let fields: Vec<_> = artifact
        .root()
        .fields()
        .expect("record fields")
        .map(|field| field.and_then(|field| field.kind()))
        .collect();
    assert_eq!(
        fields,
        vec![Some(SourceDatumKind::Case {
            constructor: "Unit.Unit"
        }),]
    );
}

#[test]
fn schema_identity_is_framed_into_exact_equality() {
    let cx = box_context();
    let freeze = |version| {
        checked_source(&cx, &boxed(), &SourceSchema::new("test.box", "Box", version), |_| None)
            .expect("the cell freezes")
    };
    assert_ne!(freeze(1).exact_bytes(), freeze(2).exact_bytes());
}

#[test]
fn a_checked_subvalue_has_schema_independent_exact_framing() {
    let cx = box_context();
    let first =
        checked_source(&cx, &boxed(), &SourceSchema::new("test.box-a", "Box", 1), |_| None).expect("first artifact");
    let second =
        checked_source(&cx, &boxed(), &SourceSchema::new("test.box-b", "Box", 9), |_| None).expect("second artifact");
    assert_ne!(first.exact_bytes(), second.exact_bytes());
    assert_eq!(first.root().exact_bytes(), second.root().exact_bytes());
}

#[test]
fn a_wrong_root_and_a_function_are_not_artifacts_of_the_claimed_schema() {
    let cx = box_context();
    let wrong = checked_source(&cx, &boxed(), &SourceSchema::new("test.wrong", "Unit", 1), |_| None);
    assert!(matches!(wrong, Err(CheckedSourceError::WrongRoot { .. })));

    let function = Raw::annotated_lam(HERE, "x", Raw::var(HERE, "Unit"), Raw::var(HERE, "x"));
    let not_data = checked_source(&cx, &function, &SourceSchema::new("test.function", "Unit", 1), |_| None);
    assert!(matches!(not_data, Err(CheckedSourceError::WrongRoot { .. })));
}
