//! What `compute_edits` guarantees about the text it produces.
//!
//! The contract is narrow and worth stating: the computed edits, applied,
//! yield source that still parses, that differs from the original only where
//! the intent said, and that a composer would have typed the same way —
//! indentation included, because a structured editor that reflows the file
//! under the writer is not one anybody keeps using.

// A fixture that does not contain what a test looks for is the test failing,
// so panicking on one is the assertion rather than an oversight.
#![allow(clippy::panic)]

use musa_language::{Anchor, EditError, EditIntent, Statement, apply_edits, compute_edits, parse};

const PIECE: &str = r#"piece "Etude" {
    tempo quarter = 72;

    motif sigh(root: pitch = e5) {
        root 1/2;
        c5 1/2;
    }

    score {
        part piano {
            voice right {
                a4 1/4;
                b4 1/4;
            }

            voice left {
            }
        }
    }
}
"#;

/// Where a statement starts, the way an event's origin reports it.
fn at(source: &str, statement: &str) -> u32 {
    let index = source
        .find(statement)
        .unwrap_or_else(|| panic!("`{statement}` is in the fixture"));
    u32::try_from(index).unwrap_or(0)
}

fn edited(source: &str, intent: &EditIntent) -> String {
    let edits = compute_edits(source, intent).unwrap_or_else(|error| panic!("{error}"));
    let out = apply_edits(source, &edits);
    assert!(
        parse(&out).errors().is_empty(),
        "an edit that produces unparseable source is a bug, not a diagnostic:\n{out}"
    );
    out
}

#[test]
fn changing_a_pitch_touches_only_the_pitch_token() {
    let out = edited(
        PIECE,
        &EditIntent::SetPitch {
            at: at(PIECE, "a4 1/4;"),
            pitch: "g#4".to_owned(),
        },
    );
    assert_eq!(out, PIECE.replace("a4 1/4;", "g#4 1/4;"));
}

#[test]
fn changing_a_duration_touches_only_the_duration_token() {
    let out = edited(
        PIECE,
        &EditIntent::SetDuration {
            at: at(PIECE, "b4 1/4;"),
            duration: "3/8".to_owned(),
        },
    );
    assert_eq!(out, PIECE.replace("b4 1/4;", "b4 3/8;"));
}

#[test]
fn a_motif_note_is_reachable_by_its_own_span() {
    // The statement inside the motif body — the one an edit-definition edit
    // rewrites, and the reason offsets are the interface rather than voices.
    let out = edited(
        PIECE,
        &EditIntent::SetPitch {
            at: at(PIECE, "c5 1/2;"),
            pitch: "d5".to_owned(),
        },
    );
    assert!(
        out.contains("        d5 1/2;"),
        "the motif body changed, indented as it was"
    );
    assert_eq!(out.matches("d5").count(), 1, "and nothing else did");
}

#[test]
fn a_pitch_reference_is_a_pitch_like_any_other() {
    let out = edited(
        PIECE,
        &EditIntent::SetPitch {
            at: at(PIECE, "root 1/2;"),
            pitch: "f5".to_owned(),
        },
    );
    assert!(out.contains("        f5 1/2;"));
}

#[test]
fn inserting_after_a_note_matches_its_indentation() {
    let out = edited(
        PIECE,
        &EditIntent::Insert {
            anchor: Anchor::After {
                at: at(PIECE, "a4 1/4;"),
            },
            statement: Statement::Note {
                pitch: "c5".to_owned(),
                duration: "1/8".to_owned(),
            },
        },
    );
    assert!(out.contains("                a4 1/4;\n                c5 1/8;\n                b4 1/4;"));
}

#[test]
fn inserting_before_a_note_matches_its_indentation() {
    let out = edited(
        PIECE,
        &EditIntent::Insert {
            anchor: Anchor::Before {
                at: at(PIECE, "b4 1/4;"),
            },
            statement: Statement::Rest {
                duration: "1/8".to_owned(),
            },
        },
    );
    assert!(out.contains("                a4 1/4;\n                rest 1/8;\n                b4 1/4;"));
}

#[test]
fn entry_into_a_voice_appends_after_its_last_statement() {
    let out = edited(
        PIECE,
        &EditIntent::Insert {
            anchor: Anchor::EndOfVoice {
                part: "piano".to_owned(),
                voice: "right".to_owned(),
            },
            statement: Statement::Chord {
                pitches: vec!["a3".to_owned(), "c4".to_owned()],
                duration: "1/2".to_owned(),
            },
        },
    );
    assert!(out.contains("                b4 1/4;\n                chord [a3, c4] 1/2;\n"));
}

#[test]
fn entry_into_an_empty_voice_opens_it_one_level_in() {
    let out = edited(
        PIECE,
        &EditIntent::Insert {
            anchor: Anchor::EndOfVoice {
                part: "piano".to_owned(),
                voice: "left".to_owned(),
            },
            statement: Statement::Note {
                pitch: "a2".to_owned(),
                duration: "1".to_owned(),
            },
        },
    );
    assert!(out.contains("            voice left {\n                a2 1;\n            }"));
}

#[test]
fn extracting_a_motif_lifts_the_run_and_leaves_a_use() {
    let out = edited(
        PIECE,
        &EditIntent::ExtractMotif {
            first: at(PIECE, "a4 1/4;"),
            last: at(PIECE, "b4 1/4;"),
            name: "answer".to_owned(),
        },
    );
    // The declaration sits with the other declarations, not where the notes
    // were, and the notes are gone from the voice.
    assert!(out.contains("    motif answer() {\n        a4 1/4;\n        b4 1/4;\n    }\n\n    score {"));
    assert!(out.contains("            voice right {\n                use answer();\n            }"));
    assert!(!out.contains("a4 1/4;\n                b4"));
}

#[test]
fn extraction_of_one_statement_is_extraction_of_a_run_of_one() {
    let out = edited(
        PIECE,
        &EditIntent::ExtractMotif {
            first: at(PIECE, "b4 1/4;"),
            last: at(PIECE, "b4 1/4;"),
            name: "tail".to_owned(),
        },
    );
    assert!(out.contains("    motif tail() {\n        b4 1/4;\n    }"));
    assert!(out.contains("                a4 1/4;\n                use tail();"));
}

#[test]
fn an_offset_that_is_not_a_statement_is_refused() {
    let intent = EditIntent::SetPitch {
        at: at(PIECE, "tempo"),
        pitch: "c4".to_owned(),
    };
    assert!(matches!(
        compute_edits(PIECE, &intent),
        Err(EditError::NoStatement { .. })
    ));
}

#[test]
fn a_rest_has_no_pitch_to_change_and_says_so() {
    let source = PIECE.replace("a4 1/4;", "rest 1/4;");
    let intent = EditIntent::SetPitch {
        at: at(&source, "rest 1/4;"),
        pitch: "c4".to_owned(),
    };
    assert!(matches!(
        compute_edits(&source, &intent),
        Err(EditError::NotANote { .. })
    ));
}

#[test]
fn a_voice_that_does_not_exist_is_refused_by_name() {
    let intent = EditIntent::Insert {
        anchor: Anchor::EndOfVoice {
            part: "piano".to_owned(),
            voice: "middle".to_owned(),
        },
        statement: Statement::Rest {
            duration: "1".to_owned(),
        },
    };
    assert_eq!(
        compute_edits(PIECE, &intent),
        Err(EditError::NoVoice {
            part: "piano".to_owned(),
            voice: "middle".to_owned(),
        })
    );
}

#[test]
fn an_extraction_across_two_blocks_is_refused() {
    let intent = EditIntent::ExtractMotif {
        first: at(PIECE, "c5 1/2;"),
        last: at(PIECE, "b4 1/4;"),
        name: "wrong".to_owned(),
    };
    assert_eq!(compute_edits(PIECE, &intent), Err(EditError::NotSiblings));
}

/// A piece whose voice calls the same motif three times: plainly, with one
/// note already respelled, and with two.
const CALLS: &str = r#"piece "Etude" {
    motif sigh() {
        e5 1/2;
        c5 1/2;
        g5 1/2;
    }

    score {
        part piano {
            voice right {
                use sigh();
                use sigh() with { note 2 = d5; }
                use sigh() with { note 1 = f5; note 3 = a5; }
            }
        }
    }
}
"#;

/// The nth occurrence of `use sigh()` in the fixture, by its offset.
fn call(source: &str, nth: usize) -> u32 {
    let index = source
        .match_indices("use sigh()")
        .nth(nth)
        .unwrap_or_else(|| panic!("the fixture has fewer than {nth} calls"))
        .0;
    u32::try_from(index).unwrap_or(0)
}

#[test]
fn specializing_a_plain_call_gives_it_a_with_clause() {
    let out = edited(
        CALLS,
        &EditIntent::Specialize {
            at: call(CALLS, 0),
            position: 2,
            pitch: "d5".to_owned(),
        },
    );
    assert!(
        out.contains(
            "                use sigh() with { note 2 = d5; }\n                use sigh() with { note 2 = d5; }"
        ),
        "the first call grew the clause the second already has:\n{out}"
    );
}

#[test]
fn specializing_a_note_that_is_already_specialized_respells_it() {
    let out = edited(
        CALLS,
        &EditIntent::Specialize {
            at: call(CALLS, 1),
            position: 2,
            pitch: "eb5".to_owned(),
        },
    );
    assert_eq!(out, CALLS.replace("note 2 = d5;", "note 2 = eb5;"));
}

#[test]
fn a_new_override_joins_the_clause_in_playing_order() {
    // Overrides read in the order the notes sound, whichever order the
    // composer specialized them in.
    let out = edited(
        CALLS,
        &EditIntent::Specialize {
            at: call(CALLS, 2),
            position: 2,
            pitch: "d5".to_owned(),
        },
    );
    assert!(
        out.contains("with { note 1 = f5; note 2 = d5; note 3 = a5; }"),
        "the new override went between the two it belongs between:\n{out}"
    );
}

#[test]
fn specializing_something_that_is_not_an_occurrence_is_refused() {
    // An authored note has no occurrence to specialize: the composer is
    // editing the note itself, and `SetPitch` is that edit.
    assert!(matches!(
        compute_edits(
            PIECE,
            &EditIntent::Specialize {
                at: at(PIECE, "a4 1/4;"),
                position: 1,
                pitch: "g#4".to_owned(),
            }
        ),
        Err(EditError::NotAnOccurrence { .. })
    ));
}
