//! Checked source data crosses into DSP without a parallel studio language.

#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, SourceSchema, checked_source_value};
use musa_dsp::{
    ParameterValueKind, PortKindTag, StudioDeclarationKind, StudioDescriptionError, decode_studio_description,
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
