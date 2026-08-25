//! Checked source controls reaching private prepared parameters.

#![allow(clippy::arithmetic_side_effects)]

use musa_calculus::SourceSchema;
use musa_compiler::{CompileOptions, SourceDocument, checked_source_value};

use super::audio_support::{options, parts, prepare, prepare_gestures, render_source};

const FRAMES: usize = 12_000;

fn profiled(level: &str, music: &str) -> String {
    format!(
        "piece \"expression\" {{ tempo 1/4 = 120; meter 4/4; key c major; \
         performance {{ profile shaped {{ dynamic p {{ amplitude = {level}; }} \
         dynamic f {{ amplitude = 1/1; }} mark accent {{ gate = 1/1; attack = 0 ms; }} }} }} \
         score {{ part lead {{ profile shaped; voice line {{ {music} }} }} }} }}"
    )
}

fn energy(samples: &[f32]) -> f32 {
    samples.iter().map(|sample| sample * sample).sum()
}

#[test]
fn source_expression_moves_gain_and_timbre_through_private_mappings() {
    let quiet = render_source(&profiled("1/4", "dynamic p; c4/1"), FRAMES);
    let full = render_source(&profiled("1/1", "dynamic p; c4/1"), FRAMES);
    let quiet_energy = energy(&quiet);
    let full_energy = energy(&full);
    assert!(
        quiet_energy < full_energy * 0.12,
        "the source mapping should make 1/4 expression audibly quieter: {quiet_energy} versus {full_energy}"
    );
    assert_ne!(quiet, full, "expression also reaches the declared timbre target");
}

#[test]
fn an_exact_hairpin_ramp_is_independent_of_host_block_partition() {
    let source = profiled("1/4", "dynamic p; crescendo to f { c4/4 c4/4 c4/4 c4/4 }");
    let mut whole = prepare(&source, FRAMES as u64);
    let mut partitioned = prepare(&source, FRAMES as u64);
    let mut expected = vec![0.0; FRAMES * 2];
    whole.render(&mut expected);
    let mut actual = Vec::with_capacity(expected.len());
    for frames in [1, 7, 64, 255, 1_024, 3_333] {
        if actual.len() == expected.len() {
            break;
        }
        let samples = (frames * 2).min(expected.len() - actual.len());
        let mut block = vec![0.0; samples];
        partitioned.render(&mut block);
        actual.extend(block);
    }
    while actual.len() < expected.len() {
        let samples = (137 * 2).min(expected.len() - actual.len());
        let mut block = vec![0.0; samples];
        partitioned.render(&mut block);
        actual.extend(block);
    }
    assert_eq!(
        actual.iter().map(|sample| sample.to_bits()).collect::<Vec<_>>(),
        expected.iter().map(|sample| sample.to_bits()).collect::<Vec<_>>()
    );
}

#[test]
fn same_source_and_options_reinstall_control_state_byte_exactly() {
    let source = profiled("1/3", "dynamic p; c4/4 accent c4/4 c4/4 c4/4");
    let first = render_source(&source, FRAMES);
    let second = render_source(&source, FRAMES);
    assert_eq!(
        first.iter().map(|sample| sample.to_bits()).collect::<Vec<_>>(),
        second.iter().map(|sample| sample.to_bits()).collect::<Vec<_>>()
    );
}

fn render_checked_controls(partial: &str, sustain: &str, connection: &str, frames: usize) -> Result<Vec<f32>, String> {
    let (score, studio) = parts(
        "piece \"custom\" { tempo 1/4 = 120; meter 4/4; key c major; \
         score { part lead { voice line { c4/8 } } } }",
    );
    let source = format!(
        r#"import std::sound::instrument;
let performance_interpretation: PerformanceInterpretationArtifact = PerformanceInterpretationArtifact {{
    schema_version = 1,
    results = [ProfileResult {{
        gesture = NoteGesture(
            GestureId {{ value = 0 }},
            c4,
            [
                some_control(expression, NormalizedValue(1/1)),
                some_control(brightness, NormalizedValue(1/2)),
                some_control(sustain, NormalizedValue({sustain})),
                some_control(phrase_relation, ConnectionValue({connection})),
                some_control(basic_sine_partial_ratio, ExactRatioValue({partial})),
            ],
            [],
        ),
        gate = 1/1,
        hold = 1/1
    }}]
}};
piece "Checked custom controls" {{ meter 4/4; key c major; score {{ part proof {{ voice v {{ rest/1 }} }} }} }}
"#
    );
    let artifact = checked_source_value(
        &SourceDocument::new(source, "checked-custom-controls.musa"),
        &CompileOptions::default(),
        "performance_interpretation",
        &SourceSchema::new(
            "std.performance.PerformanceInterpretationArtifact",
            "PerformanceInterpretationArtifact",
            1,
        ),
    )
    .map_err(|diagnostics| format!("custom source controls should check: {diagnostics:#?}"))?;
    let gestures = musa_score::lower_gestures_from_checked(&score, &[vec![artifact]])
        .map_err(|diagnostics| format!("checked controls should lower: {diagnostics:#?}"))?;
    let mut audio = prepare_gestures(&gestures, &studio, options(frames as u64)).map_err(|error| error.to_string())?;
    let mut output = vec![0.0; frames * 2];
    audio.render(&mut output);
    Ok(output)
}

#[test]
fn physical_custom_ratio_and_typed_grouping_reach_registered_parameters() -> Result<(), String> {
    let second_partial = render_checked_controls("2/1", "0/1", "Detached", 32_000)?;
    let third_partial = render_checked_controls("3/1", "0/1", "Detached", 32_000)?;
    assert_ne!(
        second_partial.iter().map(|sample| sample.to_bits()).collect::<Vec<_>>(),
        third_partial.iter().map(|sample| sample.to_bits()).collect::<Vec<_>>(),
        "the namespaced exact-ratio control must reach the oscillator bank"
    );

    let tail = 16_000 * 2..22_000 * 2;
    let detached_tail = energy(
        second_partial
            .get(tail.clone())
            .ok_or_else(|| "detached render ended before the measured tail".to_owned())?,
    );
    let legato = render_checked_controls("2/1", "0/1", "Legato", 32_000)?;
    let legato_tail = energy(
        legato
            .get(tail)
            .ok_or_else(|| "legato render ended before the measured tail".to_owned())?,
    );
    assert!(
        legato_tail > detached_tail,
        "the indexed phrase connection should select its declared release transfer: {legato_tail} versus {detached_tail}"
    );
    Ok(())
}
