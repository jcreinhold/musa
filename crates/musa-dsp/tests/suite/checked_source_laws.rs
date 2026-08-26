//! Checked source data crosses into DSP without a parallel studio language.

#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::fmt::Write as _;

use musa_compiler::{
    CompileOptions, SourceDocument, SourceSchema, checked_performance_interpretations, checked_source_value,
    checked_standard_instrument_machine, checked_standard_instruments, checked_standard_performance_vocabulary,
    checked_standard_studio_vocabulary, compile, standard_library_source,
};
use musa_dsp::{
    ExactQuantityError, ParameterValueKind, PortKindTag, SoundDimension, SoundUnit, StudioDeclarationKind,
    StudioDescriptionError, check_studio_vocabulary, decode_exact_quantity, decode_instrument_contracts,
    decode_studio_description, decode_studio_execution, decode_studio_vocabulary, exact_quantity_schema,
    studio_description_schema,
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
            "sound",
            "using",
            "assign",
            "modulate",
            "at",
            "instrument",
            "implementation",
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
fn the_production_studio_projection_retains_the_complete_checked_value() {
    let source = r#"piece "source studio" {
        meter 4/4;
        key c major;
        score { part lead { voice notes { c4/1 } } }
        studio {
            patch instrument {
                filter = oscillator(sine, ratio: 2) |> lowpass(cutoff: 1400 Hz);
                filter |> gain(-6 dB) |> output;
            }
            motion = oscillator(sine, frequency: 3/5 Hz) |> scale(250 Hz);
            assign lead -> instrument;
            route lead -> master;
            modulate motion -> instrument.filter.cutoff;
        }
    }"#;
    let compilation = compile(
        &SourceDocument::new(source, "production-studio-projection.musa"),
        &CompileOptions::default(),
    );
    let errors = compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_score::Severity::Error)
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "production studio should check: {errors:#?}");
    let artifact = compilation
        .studio_source()
        .expect("successful compilation has a studio artifact");
    let projection = decode_studio_execution(artifact).expect("the production schema decodes");
    assert_eq!(projection.exact_source(), artifact.exact_bytes());
    assert_eq!(projection.patches().len(), 1);
    assert_eq!(projection.signals().len(), 1);
    assert_eq!(projection.assignments().len(), 1);
    assert_eq!(projection.routes().len(), 1);
    assert_eq!(projection.modulations().len(), 1);
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
fn standard_instruments_are_checked_source_data_with_private_machines() {
    let artifact = checked_standard_instruments()
        .unwrap_or_else(|diagnostics| panic!("standard instruments should check: {diagnostics:#?}"));
    assert!(artifact.has_valid_framing());
    assert_eq!(
        artifact.schema().name(),
        "std.sound.instrument.InstrumentExecutionArtifact"
    );
    assert!(!artifact.exact_bytes().is_empty());
    let projected = decode_instrument_contracts(&artifact).expect("checked instrument schema projects");
    assert_eq!(projected.exact_source_bytes(), artifact.exact_bytes());
    let basic = projected
        .declaration("std.sound.basic_sine@1")
        .expect("edition-one basic instrument");
    assert_eq!((basic.name(), basic.channels()), ("note_instrument", 2));
    assert_eq!(basic.implementation_id(), basic.declaration_id());
    assert!(
        basic
            .controls()
            .iter()
            .any(|control| control.namespace() == "std.performance" && control.name() == "expression")
    );
    assert!(basic.controls().iter().any(|control| control.kind() == "ExactRatio"
        && control.namespace() == "std.sound.basic_sine"
        && control.name() == "partial_ratio"
        && control.default_ratio() == Some(Ratio::from_integer(2))));
    assert!(
        basic
            .mappings()
            .iter()
            .filter(|mapping| { mapping.namespace() == "std.performance" && mapping.name() == "expression" })
            .count()
            >= 2,
        "one source control may map to several private parameters"
    );
    assert!(
        basic.mappings().iter().any(|mapping| {
            mapping.namespace() == "std.sound.basic_sine"
                && mapping.name() == "partial_ratio"
                && mapping.parameter() == "ratio"
                && mapping.transfer().is_none()
        }),
        "the physical custom control crosses as an exact direct mapping"
    );
    assert!(basic.audition_bindings().iter().any(|binding| {
        binding.input() == musa_dsp::MidiAuditionInputKind::AttackVelocity
            && binding.scope() == musa_dsp::MidiAuditionScope::PerKey
            && binding.namespace() == "std.performance"
            && binding.name() == "emphasis"
    }));
    assert!(basic.audition_bindings().iter().any(|binding| {
        binding.input() == musa_dsp::MidiAuditionInputKind::SustainPedal
            && binding.scope() == musa_dsp::MidiAuditionScope::PerChannel
            && binding.switch_threshold() == Some(Ratio::from_integer(64))
    }));
    let machine = checked_standard_instrument_machine()
        .unwrap_or_else(|diagnostics| panic!("private instrument machine should check: {diagnostics:#?}"));
    assert_eq!(
        (machine.step(), machine.input(), machine.output()),
        ("AudioFrameStep", "Ratio", "Ratio")
    );
    musa_dsp::prepare_machine(&machine).expect("the source machine agrees with the runtime registry");
}

fn checked_instrument_module(source: &str) -> Result<musa_compiler::CheckedSource, Vec<musa_score::Diagnostic>> {
    checked_source_value(
        &SourceDocument::new(source, "musa-stdlib:/std/sound/instrument.musa"),
        &CompileOptions::default(),
        "standard_instruments",
        &SourceSchema::new(
            "std.sound.instrument.InstrumentExecutionArtifact",
            "InstrumentExecutionArtifact",
            2,
        ),
    )
}

#[test]
fn private_instrument_policy_changes_exact_execution_identity() {
    let source = standard_library_source("musa-stdlib:/std/sound/instrument.musa").expect("instrument source");
    let original = checked_instrument_module(source).expect("original module checks");
    let revised_source = source.replacen(
        "maps_normalized(brightness, \"voice\", \"blend\"",
        "maps_normalized(brightness, \"voice\", \"expression_timbre\"",
        1,
    );
    assert_ne!(revised_source, source, "the fixture must revise one private target");
    let revised = checked_instrument_module(&revised_source).expect("compatible private replacement checks");
    assert_ne!(original.exact_bytes(), revised.exact_bytes());
    let original = decode_instrument_contracts(&original).expect("original projects");
    let revised = decode_instrument_contracts(&revised).expect("replacement projects");
    let original = original
        .declaration("std.sound.basic_sine@1")
        .expect("original basic instrument");
    let revised = revised
        .declaration("std.sound.basic_sine@1")
        .expect("replacement basic instrument");
    assert_eq!(
        original.controls(),
        revised.controls(),
        "the public signature is unchanged"
    );
    assert_ne!(
        original.implementation_exact_bytes(),
        revised.implementation_exact_bytes(),
        "private mapping policy participates in exact preparation identity"
    );
}

#[test]
fn instrument_mapping_indices_use_the_general_unifier() {
    let source = standard_library_source("musa-stdlib:/std/sound/instrument.musa").expect("instrument source");
    let mismatched = source.replacen(
        "maps_normalized(brightness, \"voice\", \"blend\"",
        "maps_normalized(phrase_relation, \"voice\", \"blend\"",
        1,
    );
    assert_ne!(mismatched, source, "the fixture must revise the indexed key");
    assert!(
        checked_instrument_module(&mismatched).is_err(),
        "a phrase key cannot satisfy the normalized mapping index"
    );
}

#[test]
fn audition_binding_indices_use_the_general_unifier() {
    let source = standard_library_source("musa-stdlib:/std/sound/instrument.musa").expect("instrument source");
    let mismatched = source.replacen("control_key = emphasis", "control_key = phrase_relation", 1);
    assert_ne!(mismatched, source, "the fixture must revise the indexed key");
    assert!(
        checked_instrument_module(&mismatched).is_err(),
        "a phrase-relation key cannot satisfy a normalized audition binding"
    );
}

#[test]
fn audition_bindings_cannot_name_an_unimplemented_control() {
    let source = standard_library_source("musa-stdlib:/std/sound/instrument.musa").expect("instrument source");
    let unmapped = source.replacen(
        "        maps_normalized(emphasis, \"voice\", \"attack\", InverseTransfer(2/1000, 5/1000)),\n",
        "",
        1,
    );
    let artifact = checked_instrument_module(&unmapped).expect("ordinary source still checks");
    assert!(
        decode_instrument_contracts(&artifact).is_err(),
        "checked projection refuses a host-unreachable binding"
    );
}

#[test]
fn duplicate_source_audition_bindings_are_refused() {
    let source = standard_library_source("musa-stdlib:/std/sound/instrument.musa").expect("instrument source");
    let duplicate = source.replacen(
        "input = SustainPedal,\n            scope = PerChannel,\n            control_key = sustain",
        "input = AttackVelocity,\n            scope = PerKey,\n            control_key = emphasis",
        1,
    );
    assert_ne!(duplicate, source, "the fixture must duplicate one binding identity");
    let artifact = checked_instrument_module(&duplicate).expect("ordinary source still checks");
    assert!(decode_instrument_contracts(&artifact).is_err());
}

#[test]
fn a_private_instrument_body_is_not_a_client_address() {
    let compilation = compile(
        &SourceDocument::new(
            r#"import std::sound::instrument;
let forbidden = basic_sine_body;
piece "Private instrument" {
    meter 4/4;
    key c major;
    score { part proof { voice observed { rest/1 } } }
}
"#,
            "private-instrument-client.musa",
        ),
        &CompileOptions::default(),
    );
    assert!(
        compilation.has_errors(),
        "a client must not name a private implementation body"
    );
    assert!(
        compilation
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("private")),
        "the refusal should explain the visibility boundary: {:#?}",
        compilation.diagnostics()
    );
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
    let part = artifacts.first().expect("one part produces one artifact lane");
    assert_eq!(part.len(), 1);
    let artifact = part.first().expect("a small part produces one artifact");
    assert!(artifact.has_valid_framing());
    assert_eq!(
        artifact.schema().name(),
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

#[test]
fn performance_requests_are_checked_in_bounded_source_chunks() {
    let mut source = String::from(
        "piece \"Bounded performance bridge\" {\n    meter 4/4;\n    key c major;\n    score { part line { voice notes {\n",
    );
    for _ in 0..321 {
        writeln!(source, "        c4/4").expect("writing source cannot fail");
    }
    source.push_str("    } } }\n}\n");
    let compilation = compile(
        &SourceDocument::new(source, "bounded-performance-bridge.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:#?}", compilation.diagnostics());

    let score = compilation.snapshot().expect("score");
    let artifacts = checked_performance_interpretations(score)
        .unwrap_or_else(|diagnostics| panic!("bounded requests should check: {diagnostics:#?}"));
    assert_eq!(artifacts.len(), 1);
    let part = artifacts.first().expect("one part produces one artifact lane");
    assert_eq!(part.len(), 3, "321 requests use 128-item chunks");
    assert!(part.iter().all(musa_compiler::CheckedSource::has_valid_framing));

    let plan = musa_score::lower_gestures_from_checked(score, &artifacts)
        .expect("chunked checked results lower as one logical part lane");
    let lane = plan.lanes().first().expect("one source part produces one gesture lane");
    assert_eq!(
        lane.track().occurrences().len(),
        321,
        "chunking must preserve every requested gesture"
    );
}

#[test]
fn checked_source_results_drive_the_performed_span_and_projection() {
    let compilation = compile(
        &SourceDocument::new(
            r#"piece "Source-driven performance" {
                meter 4/4;
                key c major;
                score { part violin { voice line { c4/4 } } }
            }"#,
            "source-driven-performance.musa",
        ),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:#?}", compilation.diagnostics());
    let artifact = checked_source_value(
        &SourceDocument::new(
            r#"import std::performance;
let performance_interpretation: PerformanceInterpretationArtifact = PerformanceInterpretationArtifact {
    schema_version = 1,
    results = [
        ProfileResult {
            gesture = NoteGesture(
                GestureId { value = 0 },
                c4,
                [some_control(expression, NormalizedValue(1/4))],
                [],
            ),
            gate = 1/2,
            hold = 1/1
        },
    ]
};
piece "Checked source result" {
    meter 4/4;
    key c major;
    score { part proof { voice observed { rest/1 } } }
}
"#,
            "checked-source-performance-result.musa",
        ),
        &CompileOptions::default(),
        "performance_interpretation",
        &SourceSchema::new(
            "std.performance.PerformanceInterpretationArtifact",
            "PerformanceInterpretationArtifact",
            1,
        ),
    )
    .unwrap_or_else(|diagnostics| panic!("source result should check: {diagnostics:#?}"));
    let plan = musa_score::lower_gestures_from_checked(compilation.snapshot().expect("score"), &[vec![artifact]])
        .expect("checked source result lowers mechanically");
    let lane = plan.lanes().first().expect("one gesture lane");
    let occurrence = lane.track().occurrences().first().expect("one gesture");
    assert_eq!(occurrence.span().start().as_ratio(), Ratio::ZERO);
    assert_eq!(occurrence.span().end().as_ratio(), Ratio::new(1, 8));
    assert_eq!(occurrence.payload().amplitude(), Ratio::new(1, 4));
    assert!(!occurrence.payload().exact_source_bytes().is_empty());
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
fn a_control_index_blocked_by_a_lambda_is_settled_by_later_arguments() {
    let artifact = checked_performance_value(
        "postponed",
        r"fn transformed_control(
    {kind: ControlKind},
    transform: ControlValue(kind) -> ControlValue(kind),
    control_key: ControlKey(kind),
    value: ControlValue(kind),
) -> SomeControl { some_control(control_key, transform(value)) }
let postponed: SomeControl = transformed_control(
    fn (value) { value },
    expression,
    NormalizedValue(1/2),
);",
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
fn a_control_index_cannot_escape_the_lambda_scope_that_names_it() {
    let source = r#"import std::performance;
fn escaping_control_kind(
    {kind: ControlKind},
    choose: (local: ControlKind) -> ControlKey(kind),
) -> ControlKind { kind }
let escaped: ControlKind = escaping_control_kind(
    fn (local: ControlKind) -> ControlKey(local) { expression },
);
piece "Escaping control index" {
    meter 4/4;
    key c major;
    score { part proof { voice observed { rest/1 } } }
}
"#;
    assert!(
        checked_source_value(
            &SourceDocument::new(source, "escaping-control-index.musa"),
            &CompileOptions::default(),
            "escaped",
            &SourceSchema::new("test.performance.ControlKind", "ControlKind", 1),
        )
        .is_err(),
        "an index created outside `local` must not be solved to that inner binder"
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
fn phrase_group_identity_retains_membership_order_and_connection() {
    let legato = checked_performance_value(
        "group",
        "let group: Gesture = PhraseGroup(More(GestureId { value = 2 }, One(GestureId { value = 5 })), Legato);",
        "Gesture",
    );
    let detached = checked_performance_value(
        "group",
        "let group: Gesture = PhraseGroup(More(GestureId { value = 2 }, One(GestureId { value = 5 })), Detached);",
        "Gesture",
    );
    let reordered = checked_performance_value(
        "group",
        "let group: Gesture = PhraseGroup(More(GestureId { value = 5 }, One(GestureId { value = 2 })), Legato);",
        "Gesture",
    );
    assert_ne!(legato.exact_bytes(), detached.exact_bytes());
    assert_ne!(legato.exact_bytes(), reordered.exact_bytes());
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
