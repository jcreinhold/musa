//! Laws for the exact one-step interpreter (`03-machine-calculus.md` §3).

// A law suite reports a violated law by failing, and its fixtures unwrap only
// the successful preparation the law itself asserts.
#![allow(clippy::expect_used)]

use musa_dsp::{MachineValue as Value, PrepareError, StepError, prepare_machine};
use musa_score::machine::{MachineSpec, PortSchema as Port, SpecForm, SpecNode, StepTag, descriptor};

fn ratio(numerator: i64, denominator: i64) -> Value {
    Value::ratio(numerator, denominator).expect("test denominator is nonzero")
}

fn stored_ratio(numerator: i64, denominator: i64) -> Vec<u8> {
    let mut bytes = vec![0];
    bytes.extend_from_slice(&numerator.to_be_bytes());
    bytes.extend_from_slice(&denominator.to_be_bytes());
    bytes
}

fn stored_nat(value: u64) -> Vec<u8> {
    let mut bytes = vec![3];
    bytes.extend_from_slice(&value.to_be_bytes());
    bytes
}

fn stored_pair(first: Vec<u8>, second: Vec<u8>) -> Vec<u8> {
    let mut bytes = vec![4];
    bytes.extend(first);
    bytes.extend(second);
    bytes
}

fn primitive(id: &str, version: u32, stored: Vec<u8>) -> SpecNode {
    SpecNode::primitive(descriptor(id, version).expect("test primitive is registered"), stored)
}

fn spec(input: Port, output: Port, nodes: Vec<SpecNode>) -> MachineSpec {
    MachineSpec::new(StepTag::AudioFrameStep, input, output, nodes)
}

#[test]
fn every_leaf_constructor_takes_its_exact_step() {
    let count = spec(Port::Unit, Port::Nat, vec![primitive("count", 1, stored_nat(7))]);
    let prepared = prepare_machine(&count).expect("count prepares");
    let mut running = prepared.start();
    assert_eq!(running.step(Value::Unit), Ok(Value::Nat(7)));
    assert_eq!(running.step(Value::Unit), Ok(Value::Nat(8)));

    let identity = spec(
        Port::Bool,
        Port::Bool,
        vec![SpecNode::wiring(SpecForm::Identity, vec![])],
    );
    assert_eq!(
        prepare_machine(&identity)
            .expect("identity prepares")
            .start()
            .step(Value::Bool(true)),
        Ok(Value::Bool(true))
    );

    let pair = Port::Pair(Box::new(Port::Bool), Box::new(Port::Nat));
    let copied = spec(
        Port::Bool,
        Port::Pair(Box::new(Port::Bool), Box::new(Port::Bool)),
        vec![SpecNode::wiring(SpecForm::Copy, vec![])],
    );
    assert_eq!(
        prepare_machine(&copied)
            .expect("copy prepares")
            .start()
            .step(Value::Bool(true)),
        Ok(Value::pair(Value::Bool(true), Value::Bool(true)))
    );

    let dropped = spec(Port::Nat, Port::Unit, vec![SpecNode::wiring(SpecForm::Drop, vec![])]);
    assert_eq!(
        prepare_machine(&dropped)
            .expect("drop prepares")
            .start()
            .step(Value::Nat(3)),
        Ok(Value::Unit)
    );

    let swapped = spec(
        pair,
        Port::Pair(Box::new(Port::Nat), Box::new(Port::Bool)),
        vec![SpecNode::wiring(SpecForm::Swap, vec![])],
    );
    assert_eq!(
        prepare_machine(&swapped)
            .expect("swap prepares")
            .start()
            .step(Value::pair(Value::Bool(true), Value::Nat(4))),
        Ok(Value::pair(Value::Nat(4), Value::Bool(true)))
    );
}

#[test]
fn connect_and_beside_follow_the_structural_equations() {
    let scale = descriptor("scale", 1).expect("registered");
    let connected = spec(
        Port::Ratio,
        Port::Ratio,
        vec![
            SpecNode::primitive(scale, stored_ratio(2, 1)),
            SpecNode::primitive(scale, stored_ratio(3, 1)),
            SpecNode::wiring(SpecForm::Connect, vec![0, 1]),
        ],
    );
    assert_eq!(
        prepare_machine(&connected)
            .expect("chain prepares")
            .start()
            .step(ratio(5, 1)),
        Ok(ratio(30, 1)),
        "the right child sees the left child's current-step output"
    );

    let beside = spec(
        Port::Pair(Box::new(Port::Ratio), Box::new(Port::Ratio)),
        Port::Pair(Box::new(Port::Ratio), Box::new(Port::Ratio)),
        vec![
            SpecNode::primitive(scale, stored_ratio(2, 1)),
            SpecNode::primitive(scale, stored_ratio(3, 1)),
            SpecNode::wiring(SpecForm::Beside, vec![0, 1]),
        ],
    );
    assert_eq!(
        prepare_machine(&beside)
            .expect("beside prepares")
            .start()
            .step(Value::pair(ratio(5, 1), ratio(7, 1))),
        Ok(Value::pair(ratio(10, 1), ratio(21, 1))),
        "side-by-side retains both outputs instead of mixing"
    );
}

#[test]
fn connect_reassociation_preserves_the_step_function() {
    let scale = descriptor("scale", 1).expect("registered");
    let left = spec(
        Port::Ratio,
        Port::Ratio,
        vec![
            SpecNode::primitive(scale, stored_ratio(2, 1)),
            SpecNode::primitive(scale, stored_ratio(3, 1)),
            SpecNode::wiring(SpecForm::Connect, vec![0, 1]),
            SpecNode::primitive(scale, stored_ratio(5, 1)),
            SpecNode::wiring(SpecForm::Connect, vec![2, 3]),
        ],
    );
    let right = spec(
        Port::Ratio,
        Port::Ratio,
        vec![
            SpecNode::primitive(scale, stored_ratio(2, 1)),
            SpecNode::primitive(scale, stored_ratio(3, 1)),
            SpecNode::primitive(scale, stored_ratio(5, 1)),
            SpecNode::wiring(SpecForm::Connect, vec![1, 2]),
            SpecNode::wiring(SpecForm::Connect, vec![0, 3]),
        ],
    );
    let input = ratio(7, 1);
    let left = prepare_machine(&left)
        .expect("left association")
        .start()
        .step(input.clone());
    let right = prepare_machine(&right).expect("right association").start().step(input);
    assert_eq!(left, right);
    assert_eq!(left, Ok(ratio(210, 1)));
}

#[test]
fn every_reference_primitive_runs_the_function_its_registration_names() {
    let scale_offset = spec(
        Port::Ratio,
        Port::Ratio,
        vec![primitive(
            "scale",
            2,
            stored_pair(stored_ratio(2, 1), stored_ratio(3, 1)),
        )],
    );
    assert_eq!(
        prepare_machine(&scale_offset)
            .expect("affine scale prepares")
            .start()
            .step(ratio(5, 1)),
        Ok(ratio(13, 1))
    );

    let ratios = Port::Pair(Box::new(Port::Ratio), Box::new(Port::Ratio));
    let mix = spec(
        ratios,
        Port::Ratio,
        vec![primitive("mix", 1, stored_pair(stored_ratio(2, 1), stored_ratio(3, 1)))],
    );
    assert_eq!(
        prepare_machine(&mix)
            .expect("mixer prepares")
            .start()
            .step(Value::pair(ratio(5, 1), ratio(7, 1))),
        Ok(ratio(31, 1))
    );

    let reached = spec(Port::Nat, Port::Bool, vec![primitive("reached", 1, stored_nat(4))]);
    let prepared = prepare_machine(&reached).expect("threshold prepares");
    assert_eq!(prepared.start().step(Value::Nat(3)), Ok(Value::Bool(false)));
    assert_eq!(prepared.start().step(Value::Nat(4)), Ok(Value::Bool(true)));
}

#[test]
fn feedback_has_a_first_output_and_commits_boolean_negation_afterward() {
    let feedback = spec(
        Port::Unit,
        Port::Bool,
        vec![
            primitive("delay_not", 1, vec![2, 1]),
            SpecNode::initialized(SpecForm::Feedback, vec![0], vec![2, 0]),
        ],
    );
    let prepared = prepare_machine(&feedback).expect("initialized feedback prepares");
    let mut running = prepared.start();
    assert_eq!(
        running.step(Value::Unit),
        Ok(Value::Bool(false)),
        "step zero reads the explicit initial value"
    );
    assert_eq!(running.step(Value::Unit), Ok(Value::Bool(true)));
    assert_eq!(running.step(Value::Unit), Ok(Value::Bool(false)));
}

#[test]
fn preparation_and_step_report_typed_port_failures() {
    let ill_typed = spec(
        Port::Bool,
        Port::Nat,
        vec![SpecNode::wiring(SpecForm::Identity, vec![])],
    );
    assert!(matches!(
        prepare_machine(&ill_typed),
        Err(PrepareError::PortMismatch { .. })
    ));

    let identity = spec(
        Port::Bool,
        Port::Bool,
        vec![SpecNode::wiring(SpecForm::Identity, vec![])],
    );
    let prepared = prepare_machine(&identity).expect("identity prepares");
    assert_eq!(
        prepared.start().step(Value::Nat(1)),
        Err(StepError::Input {
            expected: "Bool".to_owned(),
            actual: "Nat".to_owned()
        })
    );
}

#[test]
fn preparation_refuses_malformed_structure_and_stored_data() {
    let disconnected = spec(
        Port::Bool,
        Port::Bool,
        vec![
            SpecNode::wiring(SpecForm::Identity, vec![]),
            SpecNode::wiring(SpecForm::Identity, vec![]),
        ],
    );
    assert_eq!(
        prepare_machine(&disconnected).err(),
        Some(PrepareError::NotTree { node: 0 })
    );

    let wrong_arity = spec(
        Port::Bool,
        Port::Bool,
        vec![SpecNode::wiring(SpecForm::Connect, vec![])],
    );
    assert!(matches!(
        prepare_machine(&wrong_arity),
        Err(PrepareError::ChildCount { expected: 2, .. })
    ));

    let truncated = spec(Port::Ratio, Port::Ratio, vec![primitive("scale", 1, vec![0])]);
    assert!(matches!(
        prepare_machine(&truncated),
        Err(PrepareError::StoredValue { expected, .. }) if expected == "Ratio"
    ));
}

#[test]
fn one_description_is_deterministic_and_causal() {
    let feedback = spec(
        Port::Unit,
        Port::Bool,
        vec![
            primitive("delay_not", 1, vec![2, 1]),
            SpecNode::initialized(SpecForm::Feedback, vec![0], vec![2, 0]),
        ],
    );
    let prepared = prepare_machine(&feedback).expect("feedback prepares");
    let inputs = [Value::Unit, Value::Unit, Value::Unit, Value::Unit];
    let first = musa_dsp::testing::run_machine_offline(&prepared, inputs.clone()).expect("valid inputs");
    let second = musa_dsp::testing::run_machine_offline(&prepared, inputs).expect("valid inputs");
    assert_eq!(
        first, second,
        "one description and one input history have one output history"
    );
    assert_eq!(
        first,
        [
            Value::Bool(false),
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(true)
        ]
    );

    let scale = spec(
        Port::Ratio,
        Port::Ratio,
        vec![primitive("scale", 1, stored_ratio(2, 1))],
    );
    let prepared = prepare_machine(&scale).expect("scale prepares");
    let left = musa_dsp::testing::run_machine_offline(&prepared, [ratio(1, 1), ratio(2, 1), ratio(3, 1)])
        .expect("valid inputs");
    let right = musa_dsp::testing::run_machine_offline(&prepared, [ratio(1, 1), ratio(2, 1), ratio(9, 1)])
        .expect("valid inputs");
    assert_eq!(
        left.get(..2),
        right.get(..2),
        "a future input cannot change an earlier output prefix"
    );
    assert_ne!(
        left, right,
        "the suffix is allowed to depend on the input where it differs"
    );
}

#[test]
fn current_step_chain_defeats_the_old_whole_node_schedule() {
    let scale = descriptor("scale", 1).expect("registered");
    let chain = spec(
        Port::Ratio,
        Port::Ratio,
        vec![
            SpecNode::primitive(scale, stored_ratio(2, 1)),
            SpecNode::primitive(scale, stored_ratio(3, 1)),
            SpecNode::wiring(SpecForm::Connect, vec![0, 1]),
        ],
    );
    let output = prepare_machine(&chain)
        .expect("chain prepares")
        .start()
        .step(ratio(1, 1));
    assert_eq!(output, Ok(ratio(6, 1)));
    assert_ne!(
        output,
        Ok(ratio(0, 1)),
        "a right node never reads a stale whole-block buffer"
    );
}
