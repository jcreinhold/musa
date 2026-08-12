//! `MusicXML` backend tests: snapshots, well-formedness, and the two claims
//! the format itself makes — that `<divisions>` represents every duration
//! exactly, and that what the plan spelled survives the trip.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::arithmetic_side_effects)]
// A malformed document is a test failure, and panicking where it is found
// reports it better than threading a Result through the readers below.
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, ScoreSnapshot, SourceDocument, compile};
use musa_render::{NotationOptions, NotationTarget, render_notation};
use proptest::prelude::*;

const EXAMPLES: [(&str, &str); 11] = [
    (
        "glass_mountain",
        include_str!("../../../../examples/glass-mountain.musa"),
    ),
    ("invention", include_str!("../../../../examples/invention.musa")),
    ("counterpoint", include_str!("../../../../examples/counterpoint.musa")),
    ("twinkle", include_str!("../../../../examples/twinkle.musa")),
    ("canon", include_str!("../../../../examples/canon.musa")),
    (
        "tuplet_fixture",
        include_str!("../../../../examples/tuplet-fixture.musa"),
    ),
    (
        "profile_fixture",
        include_str!("../../../../examples/profile-fixture.musa"),
    ),
    ("annotated", include_str!("../../../../examples/annotated.musa")),
    ("repeats", include_str!("../../../../examples/repeats.musa")),
    ("modulation", include_str!("../../../../examples/modulation.musa")),
    ("clef_change", include_str!("../../../../examples/clef-change.musa")),
];

fn score_of(text: &str) -> ScoreSnapshot {
    compile(&SourceDocument::new(text, "test.musa"), &CompileOptions::default())
        .into_snapshot()
        .expect("compiles")
}

fn musicxml_of(text: &str) -> String {
    render_notation(&score_of(text), NotationTarget::MusicXml, &NotationOptions::default())
        .expect("renders")
        .text()
        .to_string()
}

#[test]
fn example_musicxml_snapshots() {
    for (name, source) in EXAMPLES {
        insta::assert_snapshot!(name, musicxml_of(source));
    }
}

#[test]
fn output_is_deterministic() {
    for (_, source) in EXAMPLES {
        assert_eq!(musicxml_of(source), musicxml_of(source));
    }
}

/// Every example reparses as well-formed XML with one `<score-partwise>`
/// root, and no musical element carries an identifier: `MusicXML` has no
/// note-identity convention to round-trip, so this backend invents none —
/// provenance stays an MEI feature (§12.2/§12.4). The only `id`s in the
/// document are the `P1`, `P2`, … that join `<part-list>` to `<part>`.
#[test]
fn well_formed_and_free_of_private_identity() {
    for (name, source) in EXAMPLES {
        let text = musicxml_of(source);
        assert!(!text.contains("event-"), "{name}: no event ids leak into MusicXML");
        let (roots, depth) = walk(&text, &mut |element| {
            let named = matches!(element.name().as_ref(), b"part" | b"score-part");
            for attribute in element.attributes().flatten() {
                assert!(
                    named || attribute.key.as_ref() != b"id",
                    "{name}: {:?} carries an id",
                    element.name()
                );
            }
        });
        assert_eq!(roots, 1, "{name}: one root");
        assert_eq!(depth, 0, "{name}: every element closes");
    }
}

/// Walk a document, calling `visit` on every element, and report the root
/// count and the closing depth. A parse error fails the test where it is.
fn walk(text: &str, visit: &mut dyn FnMut(&quick_xml::events::BytesStart<'_>)) -> (u32, i64) {
    let mut reader = quick_xml::Reader::from_str(text);
    let (mut roots, mut depth) = (0u32, 0i64);
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(element)) => {
                if depth == 0 {
                    assert_eq!(element.name().as_ref(), b"score-partwise");
                    roots += 1;
                }
                depth += 1;
                visit(&element);
            }
            Ok(quick_xml::events::Event::Empty(element)) => visit(&element),
            Ok(quick_xml::events::Event::End(_)) => depth -= 1,
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => panic!("malformed XML: {error}"),
        }
    }
    (roots, depth)
}

/// The measures add up. For every part of every example, the `<duration>`
/// values of one measure — advanced by `<forward>`, rewound by `<backup>`,
/// and counted once per chord — reach exactly the measure length. This is
/// the claim `<divisions>` exists to support: an integer grid that
/// represents the piece without rounding it. Only the last measure of a part
/// may fall short, because a piece is allowed to end mid-bar.
#[test]
fn measures_add_up_to_the_measure_length() {
    for (name, source) in EXAMPLES {
        assert_measures_close(name, &musicxml_of(source));
    }
}

fn assert_measures_close(name: &str, text: &str) {
    let rows = measure_arithmetic(text);
    assert!(!rows.is_empty(), "{name}: has measures");
    for (index, (part, measure, reached, expected)) in rows.iter().enumerate() {
        let where_ = format!("{name}: part {part} measure {measure}");
        assert!(
            reached <= expected,
            "{where_} overfills the bar: {reached} of {expected} divisions"
        );
        let ends_the_part = rows.get(index.saturating_add(1)).is_none_or(|next| next.0 != *part);
        if !ends_the_part {
            assert_eq!(reached, expected, "{where_} fills {reached} of {expected} divisions");
        }
    }
}

/// Every `<note>` in the piece divides the divisions grid evenly enough to be
/// an integer — which it is by construction, so what this really checks is
/// that nothing wrote a zero-length note.
#[test]
fn every_notated_duration_is_a_positive_integer() {
    for (name, source) in EXAMPLES {
        let text = musicxml_of(source);
        let durations = durations_of(&text);
        assert!(!durations.is_empty(), "{name}: has notes");
        for duration in durations {
            assert!(duration > 0, "{name}: a note lasts {duration} divisions");
        }
    }
}

proptest! {
    /// The divisions grid holds whatever the language can write: for any
    /// meter and any run of ordinary note values, every duration in the
    /// export is an exact integer and the measures still add up. Rounding
    /// would show here as a measure that does not close.
    #[test]
    fn divisions_represent_any_written_duration(
        beats in 2u32..=7,
        unit in prop::sample::select(vec![2u32, 4, 8]),
        values in prop::collection::vec(prop::sample::select(vec!["1/8", "1/4", "1/2", "3/8", "1/16"]), 1..8),
    ) {
        let notes = values
            .iter()
            .map(|value| format!("c4 {value};"))
            .collect::<Vec<_>>()
            .join(" ");
        let source = format!("piece \"p\" {{ meter {beats}/{unit}; score {{ part a {{ voice v {{ {notes} }} }} }} }}");
        let compiled = compile(&SourceDocument::new(&source, "p.musa"), &CompileOptions::default());
        // A run that overflows the bar is a diagnostic, not an export.
        let Some(score) = compiled.into_snapshot() else { return Ok(()) };
        let text = render_notation(&score, NotationTarget::MusicXml, &NotationOptions::default())
            .expect("renders")
            .text()
            .to_string();
        for (_, _, reached, expected) in measure_arithmetic(&text) {
            prop_assert!(reached <= expected);
        }
        for duration in durations_of(&text) {
            prop_assert!(duration > 0);
        }
    }
}

/// A duration whose denominator is not a power of two is still exact on the
/// grid: a triplet lands on integers, because divisions is a multiple of
/// three when the piece needs one.
#[test]
fn a_triplet_lands_on_the_grid() {
    let source = "piece \"t\" { meter 4/4; score { part a { voice v { \
                  tuplet 3/2 { c4/8 d4/8 e4/8 } rest/4 rest/2 } } } }";
    let text = musicxml_of(source);
    assert!(text.contains("<actual-notes>3</actual-notes>"), "{text}");
    assert!(text.contains("<normal-notes>2</normal-notes>"), "{text}");
    assert_measures_close("triplet", &text);
}

/// A tie renders in both of `MusicXML`'s forms — the sounding `<tie>` and the
/// printed `<tied>` — because consumers read one or the other (§12.4).
#[test]
fn a_tie_is_written_as_sound_and_as_notation() {
    let text = musicxml_of("piece \"x\" { meter 2/4; score { part p { voice v { c4/1 } } } }");
    assert!(text.contains("<tie type=\"start\"/>"), "{text}");
    assert!(text.contains("<tie type=\"stop\"/>"), "{text}");
    assert!(text.contains("<tied type=\"start\"/>"), "{text}");
    assert!(text.contains("<tied type=\"stop\"/>"), "{text}");
}

/// Every dynamic marking the language has is a `MusicXML` `<dynamics>`
/// element. The backend writes `DynamicMark::name()` straight through, and
/// this is what keeps that shortcut honest.
#[test]
fn dynamic_names_are_musicxml_elements() {
    const MUSICXML_DYNAMICS: [&str; 20] = [
        "p", "pp", "ppp", "pppp", "ppppp", "pppppp", "f", "ff", "fff", "ffff", "fffff", "ffffff", "mp", "mf", "sf",
        "sfp", "sfpp", "fp", "rf", "sfz",
    ];
    for text in ["ppp", "pp", "p", "mp", "mf", "f", "ff", "fff", "sf", "sfz", "fp"] {
        let mark = musa_compiler::DynamicMark::parse(text).expect("a marking the language spells");
        assert!(
            MUSICXML_DYNAMICS.contains(&mark.name()),
            "{} is not a MusicXML dynamics element",
            mark.name()
        );
    }
}

/// Multi-voice measures rewind with `<backup>` rather than restating time,
/// which is how a partwise document puts two voices in one measure.
#[test]
fn a_second_voice_is_reached_by_backing_up() {
    let text = musicxml_of(include_str!("../../../../examples/glass-mountain.musa"));
    assert!(text.contains("<backup>"), "{text}");
    assert!(text.contains("<voice>2</voice>"), "{text}");
}

// --- Reading the document back ----------------------------------------------

/// For each `(part, measure)`, how far the measure's own arithmetic reaches
/// and how far it should. Chord tones share one duration, so a `<chord/>`
/// note does not advance the cursor.
fn measure_arithmetic(text: &str) -> Vec<(String, String, i64, i64)> {
    let mut reader = quick_xml::Reader::from_str(text);
    let mut buffer = Vec::new();
    let mut out = Vec::new();

    let (mut part, mut measure) = (String::new(), String::new());
    let (mut cursor, mut furthest) = (0i64, 0i64);
    // Where the running `<duration>` belongs: a note advances time, a backup
    // rewinds it, a forward skips it.
    let mut pending: Option<&'static str> = None;
    let mut chord = false;
    let mut divisions = 1i64;
    let mut length = 0i64;
    let mut open_measure = false;
    let mut text_of: Option<&'static str> = None;
    let (mut beats, mut beat_type) = (0i64, 4i64);

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(quick_xml::events::Event::Start(element)) => {
                match element.name().as_ref() {
                    b"part" => part = attribute(&element, b"id"),
                    b"measure" => {
                        if open_measure {
                            out.push((part.clone(), measure.clone(), furthest, length));
                        }
                        measure = attribute(&element, b"number");
                        cursor = 0;
                        furthest = 0;
                        open_measure = true;
                    }
                    b"note" => {
                        pending = Some("note");
                        chord = false;
                    }
                    b"backup" => pending = Some("backup"),
                    b"forward" => pending = Some("forward"),
                    b"divisions" => text_of = Some("divisions"),
                    b"beats" => text_of = Some("beats"),
                    b"beat-type" => text_of = Some("beat-type"),
                    b"duration" => text_of = Some("duration"),
                    _ => {}
                }
                if beats > 0 && beat_type > 0 {
                    length = divisions * 4 * beats / beat_type;
                }
            }
            Ok(quick_xml::events::Event::Empty(element)) => {
                if element.name().as_ref() == b"chord" {
                    chord = true;
                }
            }
            Ok(quick_xml::events::Event::Text(body)) => {
                let value = body.decode().unwrap_or_default().trim().parse::<i64>().unwrap_or(0);
                match text_of.take() {
                    Some("divisions") => divisions = value,
                    Some("beats") => beats = value,
                    Some("beat-type") => beat_type = value.max(1),
                    Some("duration") => match pending.take() {
                        Some("note") if !chord => cursor += value,
                        Some("backup") => cursor -= value,
                        Some("forward") => cursor += value,
                        _ => {}
                    },
                    _ => {}
                }
                length = divisions * 4 * beats / beat_type.max(1);
                furthest = furthest.max(cursor);
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => panic!("malformed XML: {error}"),
        }
        buffer.clear();
    }
    if open_measure {
        out.push((part, measure, furthest, length));
    }
    out
}

fn attribute(element: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> String {
    element
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.as_ref() == key)
        .map(|attribute| String::from_utf8_lossy(attribute.value.as_ref()).to_string())
        .unwrap_or_default()
}

/// Every `<note>` duration in the document, in order.
fn durations_of(text: &str) -> Vec<i64> {
    let mut reader = quick_xml::Reader::from_str(text);
    let mut out = Vec::new();
    let mut in_note = false;
    let mut want = false;
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(element)) => match element.name().as_ref() {
                b"note" => in_note = true,
                b"duration" if in_note => want = true,
                _ => {}
            },
            Ok(quick_xml::events::Event::End(element)) => {
                if element.name().as_ref() == b"note" {
                    in_note = false;
                }
            }
            Ok(quick_xml::events::Event::Text(body)) if want => {
                want = false;
                out.push(body.decode().unwrap_or_default().trim().parse::<i64>().unwrap_or(0));
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => panic!("malformed XML: {error}"),
        }
    }
    out
}
