//! Checked source data crosses into DSP without a parallel studio language.

#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{
    CompileOptions, SourceDocument, SourceSchema, checked_performance_interpretations, checked_source_value,
    checked_standard_performance_vocabulary, checked_standard_studio_vocabulary, compile,
};
use musa_dsp::{
    ExactQuantityError, ParameterValueKind, PortKindTag, SoundDimension, SoundUnit, StudioDeclarationKind,
    StudioDescriptionError, check_studio_vocabulary, decode_exact_quantity, decode_studio_description,
    decode_studio_vocabulary, exact_quantity_schema, studio_description_schema,
};
use num_rational::Ratio;

const TRIAL: &str = r"
input notes: note_events;
input expression: control;
instrument lead = poly_sine {
    voices = 16;
    attack = 3/100 s;
    release = 7/10 s;
};
processor expression_depth = scale { factor = 1/2; };
processor room = reverb {
    room = 4/5;
    damping = 11/20;
    mix = 3/10;
};
output main: audio(2);
connect notes.out -> lead.notes;
connect expression.out -> expression_depth.control_in;
connect expression_depth.control -> lead.expression;
connect lead.audio -> room.audio_in;
connect room.audio -> main.in;
bind clarinet -> lead;
";

fn graph_source(region: &str) -> String {
    format!(
        r#"piece "Checked source bridge" {{
import std::sound::graph;
import syntax std::adapters::graph as graph;
let description: StudioDescription = syntax graph {{
{region}
}};
meter 4/4;
key c major;
score {{ part proof {{ voice observed {{ rest/1 }} }} }}
}}
"#
    )
}

fn checked(region: &str, schema: &SourceSchema) -> musa_compiler::CheckedSource {
    let source = graph_source(region);
    checked_source_value(
        &SourceDocument::new(&source, "checked-source-laws.musa"),
        &CompileOptions::default(),
        "description",
        schema,
    )
    .unwrap_or_else(|diagnostics| panic!("source value should check: {diagnostics:#?}"))
}

fn quantity_source(expression: &str) -> String {
    format!(
        r#"import std::sound::quantity;
let quantity: ExactQuantityArtifact = quantity_artifact({expression});
piece "Exact source quantity" {{
    meter 4/4;
    key c major;
    score {{ part proof {{ voice observed {{ rest/1 }} }} }}
}}
"#
    )
}

fn checked_quantity(expression: &str) -> musa_compiler::CheckedSource {
    checked_source_value(
        &SourceDocument::new(quantity_source(expression), "checked-quantity-laws.musa"),
        &CompileOptions::default(),
        "quantity",
        &exact_quantity_schema(),
    )
    .unwrap_or_else(|diagnostics| panic!("source quantity should check: {diagnostics:#?}"))
}

#[test]
fn source_quantities_keep_exact_dimensions_until_dsp_preparation() {
    let cases = [
        (
            "hertz(440/1)",
            SoundDimension::Frequency,
            SoundUnit::Hertz,
            Ratio::from_integer(440),
        ),
        (
            "linear(1/10)",
            SoundDimension::LinearAmplitude,
            SoundUnit::Linear,
            Ratio::new(1, 10),
        ),
        (
            "decibels(ratio_sub(0/1, 6/1))",
            SoundDimension::Level,
            SoundUnit::Decibels,
            Ratio::from_integer(-6),
        ),
        (
            "seconds(3/100)",
            SoundDimension::Time,
            SoundUnit::Seconds,
            Ratio::new(3, 100),
        ),
    ];
    for (source, dimension, unit, magnitude) in cases {
        let artifact = checked_quantity(source);
        let checked = decode_exact_quantity(&artifact).expect("the checked source schema decodes");
        let quantity = checked.quantity();
        assert_eq!(quantity.dimension(), dimension);
        assert_eq!(quantity.unit(), unit);
        assert_eq!(quantity.magnitude(), &magnitude);
        assert_eq!(checked.exact_source_bytes(), artifact.exact_bytes());
    }
}

#[test]
fn standard_studio_vocabulary_is_checked_source_data() {
    let artifact = checked_standard_studio_vocabulary()
        .unwrap_or_else(|diagnostics| panic!("standard vocabulary should check: {diagnostics:#?}"));
    let vocabulary = decode_studio_vocabulary(&artifact).expect("the source vocabulary schema decodes");
    assert_eq!(vocabulary.schema_version(), 1);
    let processor_names = vocabulary
        .processors()
        .iter()
        .map(musa_dsp::ProcessorContract::name)
        .collect::<Vec<_>>();
    assert_eq!(
        processor_names,
        [
            "oscillator",
            "gain",
            "mix",
            "envelope",
            "lowpass",
            "highpass",
            "reverb",
            "delay",
            "chorus",
            "scale",
            "bias",
            "clamp",
            "smoothing",
        ],
        "every processor spelling recognized by the migration parser has one source declaration"
    );
    let term_names = vocabulary
        .terms()
        .iter()
        .map(musa_dsp::StudioTermContract::spelling)
        .collect::<Vec<_>>();
    assert_eq!(
        term_names,
        [
            "Audio",
            "Control",
            "NoteEvents",
            "Hz",
            "dB",
            "s",
            "ms",
            "Ratio",
            "adsr",
            "sine",
            "studio",
            "assign",
            "modulate",
            "at",
            "instrument",
            "patch",
            "signal",
            "bus",
            "send",
            "route",
            "master",
            "room",
            "output",
        ],
        "every standard parser/control/unit spelling has one source declaration"
    );
    for processor in vocabulary.processors() {
        assert!(!processor.summary().is_empty());
        assert!(!processor.note().is_empty());
        assert!(!processor.signature().is_empty());
        assert!(!processor.example().is_empty());
        assert!(!processor.ports().is_empty());
        assert!(processor.parameters().iter().all(|parameter| parameter.name() != "q"));
    }
    for term in vocabulary.terms() {
        assert!(!term.summary().is_empty());
        assert!(!term.note().is_empty());
        assert!(!term.signature().is_empty());
        assert!(!term.example().is_empty());
    }
    let lowpass = vocabulary.processor("lowpass").expect("source lowpass declaration");
    assert!(lowpass.note().contains("quality factor Q"));
    assert_eq!(
        lowpass.parameters().get(1).map(musa_dsp::StudioParameterContract::name),
        Some("resonance")
    );
    check_studio_vocabulary(&vocabulary).expect("source contracts agree with registered primitive support");
    assert_eq!(vocabulary.exact_source_bytes(), artifact.exact_bytes());
}

#[test]
fn standard_performance_vocabulary_is_checked_source_data() {
    let artifact = checked_standard_performance_vocabulary()
        .unwrap_or_else(|diagnostics| panic!("standard performance vocabulary should check: {diagnostics:#?}"));
    assert_eq!(
        artifact.schema().name(),
        "std.performance.PerformanceVocabularyArtifact"
    );
    assert!(artifact.has_valid_framing());
    assert!(!artifact.exact_bytes().is_empty());
}

#[test]
fn a_performance_request_batch_is_interpreted_as_checked_source_data() {
    let source = r#"piece "Checked performance" {
        meter 4/4;
        key c major;
        performance { profile strings {
            mark staccato { gate = 1/2; attack = 10 ms; }
            dynamic p { amplitude = 3/8; }
        } }
        score { part violin { profile strings; voice line { dynamic p; c4/4 staccato d4/4 } } }
    }"#;
    let compilation = compile(
        &SourceDocument::new(source, "checked-performance-bridge.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:#?}", compilation.diagnostics());
    let artifacts = checked_performance_interpretations(compilation.snapshot().expect("score"))
        .unwrap_or_else(|diagnostics| panic!("performance requests should check: {diagnostics:#?}"));
    assert_eq!(artifacts.len(), 1);
    assert!(artifacts[0].has_valid_framing());
    assert_eq!(
        artifacts[0].schema().name(),
        "std.performance.PerformanceInterpretationArtifact"
    );
    let plan = musa_compiler::lower_gestures(compilation.snapshot().expect("score"))
        .expect("checked source results lower mechanically");
    assert!(
        plan.lanes()
            .iter()
            .flat_map(|lane| lane.track().occurrences())
            .all(|occurrence| !occurrence.payload().exact_source_bytes().is_empty())
    );
}

fn checked_performance_value(binding: &str, declaration: &str, root_type: &str) -> musa_compiler::CheckedSource {
    let source = format!(
        r#"import std::performance;
{declaration}
piece "Performance value" {{
    meter 4/4;
    key c major;
    score {{ part proof {{ voice observed {{ rest/1 }} }} }}
}}
"#
    );
    checked_source_value(
        &SourceDocument::new(source, "performance-value-laws.musa"),
        &CompileOptions::default(),
        binding,
        &SourceSchema::new(format!("test.performance.{root_type}"), root_type, 1),
    )
    .unwrap_or_else(|diagnostics| panic!("performance value should check: {diagnostics:#?}"))
}

#[test]
fn a_control_kind_is_inferred_by_the_general_pattern_unifier() {
    let artifact = checked_performance_value(
        "inferred",
        "let inferred: SomeControl = some_control(expression, NormalizedValue(1/2));",
        "SomeControl",
    );
    assert!(artifact.has_valid_framing());
}

#[test]
fn disagreeing_control_indices_are_refused_without_a_sound_specific_fallback() {
    let source = r#"import std::performance;
let bad: SomeControl = some_control(expression, ConnectionValue(Legato));
piece "Bad control" {
    meter 4/4;
    key c major;
    score { part proof { voice observed { rest/1 } } }
}
"#;
    assert!(
        checked_source_value(
            &SourceDocument::new(source, "bad-control-index.musa"),
            &CompileOptions::default(),
            "bad",
            &SourceSchema::new("test.performance.SomeControl", "SomeControl", 1),
        )
        .is_err(),
        "a normalized key and phrase value must not acquire a guessed common kind"
    );
}

#[test]
fn an_unresolved_control_index_is_refused_instead_of_defaulted() {
    let source = r#"import std::performance;
fn unstated({kind: ControlKind}) -> ControlKind { kind }
let ambiguous: ControlKind = unstated();
piece "Unresolved control" {
    meter 4/4;
    key c major;
    score { part proof { voice observed { rest/1 } } }
}
"#;
    assert!(
        checked_source_value(
            &SourceDocument::new(source, "unresolved-control-index.musa"),
            &CompileOptions::default(),
            "ambiguous",
            &SourceSchema::new("test.performance.ControlKind", "ControlKind", 1),
        )
        .is_err(),
        "neither standard controls nor sound code may default an unsolved index"
    );
}

#[test]
fn the_source_hairpin_law_has_exact_endpoints_and_midpoint() {
    for (binding, expression, expected) in [
        ("start", "hairpin_expression(1/4, 3/4, 0/1)", "1/4"),
        ("middle", "hairpin_expression(1/4, 3/4, 1/2)", "1/2"),
        ("end", "hairpin_expression(1/4, 3/4, 1/1)", "3/4"),
    ] {
        let actual = checked_performance_value(binding, &format!("let {binding}: Ratio = {expression};"), "Ratio");
        let wanted = checked_performance_value("wanted", &format!("let wanted: Ratio = {expected};"), "Ratio");
        assert_eq!(actual.root().kind(), wanted.root().kind());
    }
}

#[test]
fn milliseconds_and_seconds_have_one_exact_checked_value() {
    let milliseconds = checked_quantity("milliseconds(30/1)");
    let ratio = checked_quantity("seconds(3/100)");
    assert_eq!(milliseconds.exact_bytes(), ratio.exact_bytes());
}

#[test]
fn a_graph_artifact_is_not_a_quantity_artifact() {
    let graph = checked("input control_in: control;", &studio_description_schema());
    assert_eq!(decode_exact_quantity(&graph), Err(ExactQuantityError::WrongSchema));
}

#[test]
fn source_indices_refuse_a_quantity_with_the_wrong_unit_witness() {
    let source = r#"import std::sound::quantity;
let quantity: ExactQuantityArtifact = quantity_artifact(
    Exact(Frequency, Written(Time, 1/1, Seconds))
);
piece "Ill-indexed source quantity" {
    meter 4/4;
    key c major;
    score { part proof { voice observed { rest/1 } } }
}
"#;
    let result = checked_source_value(
        &SourceDocument::new(source, "ill-indexed-quantity.musa"),
        &CompileOptions::default(),
        "quantity",
        &exact_quantity_schema(),
    );
    assert!(
        result.is_err(),
        "the ordinary dependent checker must refuse mismatched indices"
    );
}

#[test]
fn the_complete_trial_projects_every_source_field_exactly() {
    let artifact = checked(TRIAL, &studio_description_schema());
    let description = decode_studio_description(&artifact).expect("the trial schema decodes");
    assert_eq!(description.schema_version(), 1);
    let [
        input,
        expression,
        lead,
        expression_depth,
        room,
        output,
        connection,
        second_connection,
        third_connection,
        fourth_connection,
        fifth_connection,
        binding,
    ] = description.declarations()
    else {
        panic!("the complete trial has twelve declarations")
    };

    assert_eq!(input.kind(), StudioDeclarationKind::Input);
    assert_eq!(input.name(), Some("notes"));
    assert_eq!(input.port_kind().map(|kind| kind.tag()), Some(PortKindTag::NoteEvents));

    assert_eq!(expression.name(), Some("expression"));
    assert_eq!(lead.kind(), StudioDeclarationKind::Node);
    assert_eq!(lead.name(), Some("lead"));
    assert_eq!(lead.descriptor(), Some("poly_sine"));
    let parameters = lead.parameters().expect("node parameters");
    let [voices, attack, release] = parameters else {
        panic!("poly_sine retains its three parameters")
    };
    assert_eq!(voices.name(), "voices");
    assert_eq!(voices.value_kind(), ParameterValueKind::Count);
    assert_eq!(voices.count(), Some(16));
    assert_eq!(attack.ratio(), Some(&Ratio::new(3, 100)));
    assert_eq!(
        attack
            .exact_quantity()
            .map(|quantity| (quantity.dimension(), quantity.unit())),
        Some((SoundDimension::Time, SoundUnit::Seconds))
    );
    assert_eq!(release.ratio(), Some(&Ratio::new(7, 10)));
    assert_eq!(expression_depth.descriptor(), Some("scale"));
    assert_eq!(room.descriptor(), Some("reverb"));

    assert_eq!(output.kind(), StudioDeclarationKind::Output);
    assert_eq!(output.name(), Some("main"));
    let output_kind = output.port_kind().expect("output kind");
    assert_eq!(output_kind.tag(), PortKindTag::Audio);
    assert_eq!(output_kind.channels(), Some(2));

    assert_eq!(connection.kind(), StudioDeclarationKind::Connect);
    let source = connection.source_path().expect("source path");
    let target = connection.target_path().expect("target path");
    assert_eq!((source.node(), source.port()), ("notes", "out"));
    assert_eq!((target.node(), target.port()), ("lead", "notes"));
    for declaration in [second_connection, third_connection, fourth_connection, fifth_connection] {
        assert_eq!(declaration.kind(), StudioDeclarationKind::Connect);
    }

    assert_eq!(binding.kind(), StudioDeclarationKind::Bind);
    assert_eq!(binding.part_name(), Some("clarinet"));
    assert_eq!(binding.bound_node(), Some("lead"));
    assert!(artifact.exact_bytes().starts_with(b"musa-checked-source"));
    assert_eq!(description.exact_source_bytes(), artifact.exact_bytes());
}

#[test]
fn written_order_is_data_and_schema_version_is_identity() {
    let first = checked(
        "input one: control; input two: note_events;",
        &studio_description_schema(),
    );
    let reordered = checked(
        "input two: note_events; input one: control;",
        &studio_description_schema(),
    );
    assert!(decode_studio_description(&first).is_ok());
    assert!(decode_studio_description(&reordered).is_ok());
    assert_ne!(first.exact_bytes(), reordered.exact_bytes());

    let wrong_version = checked(
        "input one: control;",
        &SourceSchema::new("std.sound.graph.StudioDescription", "StudioDescription", 2),
    );
    assert_eq!(
        decode_studio_description(&wrong_version),
        Err(StudioDescriptionError::WrongSchema)
    );
}

#[test]
fn same_named_but_incomplete_data_is_refused_structurally() {
    let source = r#"
record StudioDescription { schema_version: Nat; }
let description: StudioDescription = StudioDescription { schema_version = 1 };
piece "Incomplete source value" {
    meter 4/4;
    key c major;
    score { part proof { voice observed { rest/1 } } }
}
"#;
    let artifact = checked_source_value(
        &SourceDocument::new(source, "incomplete-source-value.musa"),
        &CompileOptions::default(),
        "description",
        &studio_description_schema(),
    )
    .unwrap_or_else(|diagnostics| panic!("same-named source data checks: {diagnostics:#?}"));
    assert!(matches!(
        decode_studio_description(&artifact),
        Err(StudioDescriptionError::Malformed { .. })
    ));
}
