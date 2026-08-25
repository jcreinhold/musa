//! The device-free callback harness uses the same semantic step as offline.

// A law suite reports a violated law by failing, and its fixtures unwrap only
// the successful preparation the law itself asserts.
#![allow(clippy::expect_used)]

use musa_dsp::{MachineValue as Value, prepare_machine};
use musa_playback::testing::MachineCallbackHarness;
use musa_score::machine::{MachineSpec, PortSchema as Port, SpecForm, SpecNode, StepTag, descriptor};

#[test]
fn engine_and_offline_harnesses_share_one_step() {
    let mut stored = vec![3];
    stored.extend_from_slice(&12_u64.to_be_bytes());
    let spec = MachineSpec::new(
        StepTag::AudioFrameStep,
        Port::Unit,
        Port::Nat,
        vec![SpecNode::primitive(descriptor("count", 1).expect("registered"), stored)],
    );
    let prepared = prepare_machine(&spec).expect("counter prepares");
    let offline = musa_dsp::testing::run_machine_offline(&prepared, [Value::Unit, Value::Unit]).expect("valid inputs");
    let mut callback = MachineCallbackHarness::new(&prepared);
    let live = [callback.frame(Value::Unit), callback.frame(Value::Unit)]
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("valid inputs");
    assert_eq!(live, offline);

    let identity = MachineSpec::new(
        StepTag::AudioFrameStep,
        Port::Bool,
        Port::Bool,
        vec![SpecNode::wiring(SpecForm::Identity, vec![])],
    );
    let prepared = prepare_machine(&identity).expect("identity prepares");
    assert_eq!(
        MachineCallbackHarness::new(&prepared).frame(Value::Bool(true)),
        Ok(Value::Bool(true))
    );
}
