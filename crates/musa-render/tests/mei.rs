//! MEI backend tests: snapshots, well-formedness, and the `xml:id` →
//! `EventId` contract.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, EventId, ScoreSnapshot, SourceDocument, compile};
use musa_render::{NotationOptions, NotationTarget, render_notation};

const EXAMPLES: [(&str, &str); 8] = [
    ("glass_mountain", include_str!("../../../examples/glass-mountain.musa")),
    ("invention", include_str!("../../../examples/invention.musa")),
    ("counterpoint", include_str!("../../../examples/counterpoint.musa")),
    ("twinkle", include_str!("../../../examples/twinkle.musa")),
    ("canon", include_str!("../../../examples/canon.musa")),
    ("tuplet_fixture", include_str!("../../../examples/tuplet-fixture.musa")),
    (
        "profile_fixture",
        include_str!("../../../examples/profile-fixture.musa"),
    ),
    ("annotated", include_str!("../../../examples/annotated.musa")),
];

fn score_of(text: &str) -> ScoreSnapshot {
    compile(&SourceDocument::new(text, "test.musa"), &CompileOptions::default())
        .into_snapshot()
        .expect("compiles")
}

fn mei_of(text: &str) -> String {
    let score = score_of(text);
    render_notation(&score, NotationTarget::Mei, &NotationOptions::default())
        .expect("renders")
        .text()
        .to_string()
}

#[test]
fn example_mei_snapshots() {
    for (name, source) in EXAMPLES {
        insta::assert_snapshot!(name, mei_of(source));
    }
}

#[test]
fn output_is_deterministic() {
    assert_eq!(mei_of(EXAMPLES[0].1), mei_of(EXAMPLES[0].1));
}

/// The MEI parses back as well-formed XML with one `<mei>` root, and every
/// `event-` id (stripping `-tN` tie suffixes) resolves to an `EventId` in the
/// snapshot.
#[test]
fn well_formed_and_event_ids_resolve() {
    for (_, source) in EXAMPLES {
        let score = score_of(source);
        let mei = render_notation(&score, NotationTarget::Mei, &NotationOptions::default())
            .expect("renders")
            .text()
            .to_string();

        let mut reader = quick_xml::Reader::from_str(&mei);
        let mut roots = 0u32;
        let mut depth = 0i64;
        let mut ids: Vec<String> = Vec::new();
        let mut parse_error: Option<String> = None;
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Start(element)) => {
                    if depth == 0 {
                        assert_eq!(element.name().as_ref(), b"mei");
                        roots = roots.saturating_add(1);
                    }
                    depth = depth.saturating_add(1);
                    collect_event_id(&element, &mut ids);
                }
                Ok(quick_xml::events::Event::Empty(element)) => collect_event_id(&element, &mut ids),
                Ok(quick_xml::events::Event::End(_)) => depth = depth.saturating_sub(1),
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(error) => {
                    parse_error = Some(error.to_string());
                    break;
                }
            }
        }
        assert!(parse_error.is_none(), "malformed XML: {parse_error:?}");
        assert_eq!(roots, 1);
        assert_eq!(depth, 0);
        assert!(!ids.is_empty());

        let event_ids: std::collections::HashSet<u64> = score
            .parts()
            .iter()
            .map(|(_, part)| part)
            .flat_map(|part| part.voices().map(|(_, voice)| voice))
            .flat_map(|voice| voice.events().iter())
            .map(|event| event.id.0)
            .collect();
        for id in ids {
            let base = id.split("-t").next().unwrap_or(&id);
            let raw = base.strip_prefix("event-").expect("event prefix");
            let value = u64::from_str_radix(raw, 16).expect("hex id");
            assert!(event_ids.contains(&value), "id {id} resolves to no event");
        }
    }
}

fn collect_event_id(element: &quick_xml::events::BytesStart<'_>, ids: &mut Vec<String>) {
    for attribute in element.attributes().flatten() {
        if attribute.key.as_ref() == b"xml:id" {
            let value = String::from_utf8_lossy(attribute.value.as_ref()).to_string();
            if value.starts_with("event-") {
                ids.push(value);
            }
        }
    }
}

/// Tied pieces share the base id: `event-<hex>-t2` resolves to the same
/// `EventId`, and `tie` attributes chain i → t.
#[test]
fn tie_pieces_share_the_event_id() {
    let source = "piece \"x\" { meter 2/4; score { part p { voice v { c4 1; } } } }";
    let score = score_of(source);
    let mei = render_notation(&score, NotationTarget::Mei, &NotationOptions::default())
        .expect("renders")
        .text()
        .to_string();
    let Some(event) = score
        .parts()
        .iter()
        .map(|(_, part)| part)
        .flat_map(|part| part.voices().map(|(_, voice)| voice))
        .flat_map(|voice| voice.events().iter())
        .next()
    else {
        return;
    };
    let EventId(raw) = event.id;
    let base = format!("event-{raw:x}");
    assert!(
        mei.contains(&format!("xml:id=\"{base}\" dur=\"2\" oct=\"4\" pname=\"c\" tie=\"i\"")),
        "first piece ties on:\n{mei}"
    );
    assert!(
        mei.contains(&format!(
            "xml:id=\"{base}-t2\" dur=\"2\" oct=\"4\" pname=\"c\" tie=\"t\""
        )),
        "second piece terminates:\n{mei}"
    );
}
