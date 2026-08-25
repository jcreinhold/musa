//! Baseline for the reference semantic step, separate from preparation.

#![allow(clippy::expect_used)]

use musa_dsp::{MachineValue, PreparedMachine, prepare_machine};
use musa_score::machine::{MachineSpec, PortSchema, SpecNode, StepTag, descriptor};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn counter() -> PreparedMachine {
    let mut stored = vec![3];
    stored.extend_from_slice(&0_u64.to_be_bytes());
    let spec = MachineSpec::new(
        StepTag::AudioFrameStep,
        PortSchema::Unit,
        PortSchema::Nat,
        vec![SpecNode::primitive(
            descriptor("count", 1).expect("reference primitive is registered"),
            stored,
        )],
    );
    prepare_machine(&spec).expect("reference counter prepares")
}

#[divan::bench(sample_count = 100)]
fn reference_counter_step(bencher: divan::Bencher<'_, '_>) {
    bencher.with_inputs(|| counter().start()).bench_values(|mut running| {
        for _ in 0..4_096 {
            divan::black_box(running.step(MachineValue::Unit).expect("input has the checked schema"));
        }
    });
}

fn main() {
    divan::main();
}
