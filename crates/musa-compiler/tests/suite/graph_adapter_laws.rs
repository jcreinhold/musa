//! The finite studio graph adapter and its ordinary validation package.
//!
//! Each reading feature gets its own law. Validation errors are values, so the
//! four diagnostic laws turn their retained anchors into exact event onsets;
//! that makes the source location observable through the public compiler
//! surface without granting the adapter a diagnostic callback.

#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, adapter_edits, adapter_print, compile};
use musa_score::{MusicalTime, ScoreEventKind};

const IMPORTS: &str = r"
import std::sound::graph;
import syntax std::adapters::graph as graph;

fn nat_ratio(value: Nat) -> Ratio {
    match value {
        Zero -> 0/1,
        Succ(fewer) -> nat_ratio(fewer) + 1/1,
    }
}

fn anchored_note(anchor: Nat) -> EventTrack<WrittenTime> {
    shift(
        position_between(position_of(0/1), position_of(nat_ratio(anchor))),
        music { c5/4 },
    )
}

fn anchors_at(anchors: List<Nat>) -> EventTrack<WrittenTime> {
    anchors.fold_from_start(
        music { rest/1 },
        fn (heard: EventTrack<WrittenTime>, anchor: Nat) -> EventTrack<WrittenTime> {
            together(heard, anchored_note(anchor))
        },
    )
}
";

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

fn source(region: &str, observation: &str) -> String {
    format!(
        "piece \"Graph adapter laws\" {{\n{IMPORTS}\n\
         let description: StudioDescription = syntax graph {{\n{region}\n}};\n\
         let heard: EventTrack<WrittenTime> = {observation};\n\
         meter 4/4;\nkey c major;\n\
         score {{ part proof {{ voice observed {{ use heard; }} }} }}\n\
         }}\n"
    )
}

fn notes(region: &str, observation: &str) -> Vec<MusicalTime> {
    let text = source(region, observation);
    let compilation = compile(
        &SourceDocument::new(&text, "graph-adapter-laws.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    compilation
        .snapshot()
        .expect("a score")
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events().to_vec())
        .filter(|event| !matches!(event.kind, ScoreEventKind::Rest))
        .map(|event| event.onset)
        .collect()
}

fn valid(region: &str) {
    assert_eq!(
        notes(
            region,
            "match validate(description) { Ok(reached) -> music { c5/4 }, Err(problem) -> music { rest/4 } }",
        )
        .len(),
        1,
        "the focused graph validates"
    );
}

#[test]
fn processors_are_finite_named_declarations() {
    valid("processor room = reverb { mix = 3/10; };");
}

#[test]
fn named_ports_resolve_on_both_ends() {
    valid(
        "input expression: control; processor depth = scale { factor = 1/2; }; \
         connect expression.out -> depth.control_in;",
    );
}

#[test]
fn connections_preserve_their_written_direction() {
    valid("input notes: note_events; instrument lead = poly_sine {}; connect notes.out -> lead.notes;");
}

#[test]
fn parameters_preserve_counts_plain_ratios_and_seconds() {
    valid(
        "instrument lead = poly_sine { voices = 16; attack = 3/100 s; release = 7/10 s; }; \
         processor depth = scale { factor = 1/2; }; \
         processor room = reverb { room = 4/5; damping = 11/20; mix = 3/10; };",
    );
}

#[test]
fn instrument_bindings_name_an_existing_node() {
    valid("instrument lead = poly_sine {}; bind clarinet -> lead;");
}

#[test]
fn graph_inputs_retain_their_exact_kind() {
    valid("input notes: note_events; input expression: control;");
}

#[test]
fn graph_outputs_retain_their_audio_channel_count() {
    valid("output main: audio(2);");
}

#[test]
fn a_kind_mismatch_carries_the_connection_anchor() {
    let changed = TRIAL.replace(
        "connect expression_depth.control -> lead.expression;",
        "connect expression_depth.control -> room.audio_in;",
    );
    let found = notes(
        &changed,
        "match validate(description) {
            Err(PortKindMismatch(anchor, Control, Audio(channels))) -> if channels == 2 { anchored_note(anchor) } else { music { rest/4 } },
            _ -> music { rest/4 },
        }",
    );
    let [anchor] = found.as_slice() else {
        panic!("the mismatch retained one declaration anchor: {found:?}")
    };
    assert!(*anchor > MusicalTime::default());
}

#[test]
fn a_range_error_carries_the_parameter_value_anchor() {
    let changed = TRIAL.replace("mix = 3/10;", "mix = 3/2;");
    let found = notes(
        &changed,
        "match validate(description) {
            Err(ParameterOutOfRange(anchor, name)) -> if name == \"mix\" { anchored_note(anchor) } else { music { rest/4 } },
            _ -> music { rest/4 },
        }",
    );
    let [anchor] = found.as_slice() else {
        panic!("the range error retained one value anchor: {found:?}")
    };
    assert!(*anchor > MusicalTime::default());
}

#[test]
fn a_duplicate_name_carries_both_declaration_anchors() {
    let found = notes(
        "processor room = reverb {}; processor room = reverb {};",
        "match validate(description) {
            Err(DuplicateName(first, second, name)) -> if name == \"room\" { together(anchored_note(first), anchored_note(second)) } else { music { rest/4 } },
            _ -> music { rest/4 },
        }",
    );
    let [first, second] = found.as_slice() else {
        panic!("both declarations are named: {found:?}")
    };
    assert_ne!(first, second);
}

#[test]
fn a_cycle_carries_every_connection_anchor() {
    let found = notes(
        "processor first = scale {}; processor second = scale {}; \
         connect first.control -> second.control_in; \
         connect second.control -> first.control_in;",
        "match validate(description) {
            Err(InstantaneousCycle(anchors)) -> anchors_at(anchors),
            _ -> music { rest/4 },
        }",
    );
    let [first, second] = found.as_slice() else {
        panic!("both edges in the cycle are named: {found:?}")
    };
    assert_ne!(first, second);
}

#[test]
fn setting_mix_replaces_only_its_exact_value() {
    let out_of_range = TRIAL.replace("mix = 3/10;", "mix = 3/2;");
    let anchors = notes(
        &out_of_range,
        "match validate(description) {
            Err(ParameterOutOfRange(anchor, name)) -> anchored_note(anchor),
            _ -> music { rest/4 },
        }",
    );
    let [anchor] = anchors.as_slice() else {
        panic!("the range error names one value")
    };
    let text = source(TRIAL, "music { rest/4 }");
    let at = u32::try_from(text.find("syntax graph").expect("the region")).expect("a small file");
    let anchor = u64::try_from(anchor.as_ratio().to_integer()).expect("a nonnegative anchor");
    let edits = adapter_edits(
        &SourceDocument::new(&text, "graph-adapter-laws.musa"),
        &CompileOptions::default(),
        at,
        "set_parameter",
        anchor,
        "2/5",
    )
    .expect("the graph serves `set_parameter`");
    let [edit] = edits.as_slice() else {
        panic!("one command makes one edit: {edits:?}")
    };
    assert_eq!(text.get(edit.start as usize..edit.end as usize), Some("3/10"));
    let patched = format!(
        "{}{}{}",
        text.get(..edit.start as usize).unwrap_or_default(),
        edit.text,
        text.get(edit.end as usize..).unwrap_or_default()
    );
    assert_eq!(patched, text.replacen("mix = 3/10;", "mix = 2/5;", 1));
}

#[test]
fn a_printed_graph_reads_back_to_the_same_valid_description() {
    let document = SourceDocument::new(
        "piece \"Graph print law\" { import std::sound::graph; }",
        "graph-print-law.musa",
    );
    let printed = adapter_print(
        &document,
        &CompileOptions::default(),
        "std::adapters::graph",
        "StudioDescription { schema_version = 1, declarations = [
            Input(1, \"notes\", NoteEvents),
            Node(2, \"lead\", \"poly_sine\", [Parameter { anchor = 3, name = \"voices\", value = Count(16) }]),
            Output(4, \"main\", Audio(2)),
            Connect(5, PortPath { node = \"notes\", port = \"out\" }, PortPath { node = \"lead\", port = \"notes\" }),
            Connect(6, PortPath { node = \"lead\", port = \"audio\" }, PortPath { node = \"main\", port = \"in\" }),
            Bind(7, \"clarinet\", \"lead\"),
        ] }",
    )
    .expect("the graph can print the finite value");
    valid(&printed);
    assert!(printed.contains("processor lead = poly_sine { voices = 16; };"));
}
