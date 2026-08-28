//! Reading what a statement writes, and rewriting several at once.
//!
//! Two contracts, and they are the two halves of a group transformation.
//! Reading has to report the *written* spelling — the dotted quarter as the
//! three eighths it is worth, the tie as the chain it belongs to, the tuplet
//! member as scaled — because a caller that transformed a rendered value
//! would be transforming something the source never says. Rewriting several
//! statements has to be one answer: sorted, and refused outright when two
//! replacements would land on the same text.

// A fixture that does not contain what a test looks for is the test failing.
#![allow(clippy::panic)]

use musa_syntax::{
    EditError, EditIntent, WrittenKind, apply_edits, compute_edits, compute_group_edits, parse, read_statements,
};

const PIECE: &str = r#"piece "Etude" {
    meter 4/4;

    motif sigh(root: Pitch) {
        root/2
        c5/4.
    }

    score {
        part piano {
            voice right {
                a4/4 ~
                a4/4
                [c4 e4 g4]/2
                rest/4
                tuplet 3/2 { d5/8 e5/8 f5/8 }
                invert around c5 { g4/4 }
                use sigh(e5);
                use sigh(e5) with { note 2 = a5; }
            }
        }
    }
}
"#;

fn at(source: &str, statement: &str) -> u32 {
    let index = source
        .find(statement)
        .unwrap_or_else(|| panic!("`{statement}` is in the fixture"));
    u32::try_from(index).unwrap_or(0)
}

fn read(statement: &str) -> musa_syntax::WrittenStatement {
    read_statements(PIECE, &[at(PIECE, statement)])
        .into_iter()
        .next()
        .flatten()
        .unwrap_or_else(|| panic!("`{statement}` is a written statement"))
}

#[test]
fn a_note_reads_back_as_the_source_spells_it() {
    let note = read("[c4 e4 g4]/2");
    assert_eq!(note.kind, WrittenKind::Chord);
    assert_eq!(note.pitches, ["c4", "e4", "g4"]);
    assert_eq!(note.duration.as_deref(), Some("1/2"));
    assert!(note.chain.is_empty());
    assert!(!note.duration_scaled);
    assert!(!note.pitch_mirrored);
}

#[test]
fn a_dotted_duration_reads_back_as_what_it_is_worth() {
    // `/4.` is three eighths. A caller scaling it needs the value, and the
    // dot is a spelling of that value rather than a second fact about it.
    assert_eq!(read("c5/4.").duration.as_deref(), Some("3/8"));
}

#[test]
fn a_pitch_named_by_a_parameter_reads_back_as_the_name() {
    let note = read("root/2");
    assert_eq!(note.pitches, ["root"]);
    assert_eq!(note.duration.as_deref(), Some("1/2"));
}

#[test]
fn a_rest_writes_a_duration_and_no_pitch() {
    let rest = read("rest/4");
    assert_eq!(rest.kind, WrittenKind::Rest);
    assert!(rest.pitches.is_empty());
    assert_eq!(rest.duration.as_deref(), Some("1/4"));
}

#[test]
fn a_tie_reads_back_as_its_whole_chain() {
    let first = read("a4/4 ~");
    assert_eq!(first.chain.len(), 2, "both links, not just this one");
    assert!(first.chain.contains(&first.at));
}

#[test]
fn a_group_that_scales_time_or_mirrors_pitch_says_so() {
    assert!(read("d5/8").duration_scaled, "a tuplet member is counted differently");
    assert!(read("g4/4").pitch_mirrored, "an inverted note is spelled by the block");
}

#[test]
fn an_offset_that_is_not_a_written_statement_reads_back_as_nothing() {
    let read = read_statements(PIECE, &[at(PIECE, "meter 4/4"), at(PIECE, "use sigh(e5);")]);
    assert_eq!(read, [None, None], "neither a header nor a call writes an event");
}

#[test]
fn group_edits_come_back_in_source_order() {
    let intents = vec![
        EditIntent::SetDuration {
            at: at(PIECE, "rest/4"),
            duration: "1/8".to_owned(),
        },
        EditIntent::SetPitches {
            at: at(PIECE, "[c4 e4 g4]/2"),
            pitches: vec!["d4".to_owned(), "f4".to_owned(), "a4".to_owned()],
        },
    ];
    let edits = compute_group_edits(PIECE, &intents).unwrap_or_else(|error| panic!("{error}"));
    assert!(
        edits
            .windows(2)
            .all(|pair| matches!(pair, [before, after] if before.range.start() <= after.range.start())),
        "an applier reads them once, from the front"
    );
    let out = apply_edits(PIECE, &edits);
    assert!(parse(&out).errors().is_empty(), "{out}");
    assert!(out.contains("[d4 f4 a4]/2") && out.contains("rest/8"), "{out}");
}

#[test]
fn two_replacements_of_one_span_are_refused_rather_than_ordered() {
    let intents = vec![
        EditIntent::SetDuration {
            at: at(PIECE, "rest/4"),
            duration: "1/8".to_owned(),
        },
        EditIntent::SetDuration {
            at: at(PIECE, "rest/4"),
            duration: "1/2".to_owned(),
        },
    ];
    assert!(
        matches!(compute_group_edits(PIECE, &intents), Err(EditError::Overlapping { .. })),
        "picking one of two answers would be picking for the composer"
    );
}

#[test]
fn respelling_a_chord_needs_a_spelling_for_every_member() {
    let intent = EditIntent::SetPitches {
        at: at(PIECE, "[c4 e4 g4]/2"),
        pitches: vec!["d4".to_owned(), "f4".to_owned()],
    };
    assert!(
        matches!(
            compute_edits(PIECE, &intent),
            Err(EditError::WrongPitchCount {
                writes: 3,
                given: 2,
                ..
            })
        ),
        "a chord is one simultaneity, not a list a caller may shorten"
    );
}

#[test]
fn specializing_a_whole_call_writes_one_clause() {
    let intent = EditIntent::SpecializeAll {
        at: at(PIECE, "use sigh(e5);"),
        overrides: vec![(2, "d5".to_owned()), (1, "f5".to_owned())],
    };
    let edits = compute_edits(PIECE, &intent).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(edits.len(), 1, "one clause is one replacement");
    let out = apply_edits(PIECE, &edits);
    assert!(parse(&out).errors().is_empty(), "{out}");
    assert!(
        out.contains("use sigh(e5) with { note 1 = f5; note 2 = d5; }"),
        "positions read back in playing order:\n{out}"
    );
}

#[test]
fn specializing_a_call_that_already_has_overrides_merges_with_them() {
    let intent = EditIntent::SpecializeAll {
        at: at(PIECE, "use sigh(e5) with { note 2 = a5; }"),
        overrides: vec![(2, "b5".to_owned()), (1, "f5".to_owned())],
    };
    let out = apply_edits(
        PIECE,
        &compute_edits(PIECE, &intent).unwrap_or_else(|error| panic!("{error}")),
    );
    assert!(parse(&out).errors().is_empty(), "{out}");
    assert!(
        out.contains("use sigh(e5) with { note 1 = f5; note 2 = b5; }"),
        "the new spelling wins the position it names, and the rest survives:\n{out}"
    );
}
