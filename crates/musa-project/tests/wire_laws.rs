//! What is true of every offset that crosses the wire.
//!
//! `crate::utf16` translates spans *structurally*: a `{ "start": n, "end": n }`
//! object is a source span, wherever it appears, so a span added to the facts
//! is translated the day it appears rather than the day someone remembers.
//! That walk is only correct while two claims hold, and they are the two laws
//! here — an object of that shape is always a span, and every span indexes the
//! document the snapshot carries. The second is why a bundled declaration
//! travels as a URI and an opaque range rather than as a span
//! (`docs/rules/desktop/08-elaboration.md` §3).

// A law that trips is a bug in the wire it protects; panicking is the report.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use serde_json::Value;

/// A piece that reaches the standard library, so its wire carries a
/// declaration written in a document the snapshot does not send.
const IMPORTING: &str = "piece \"Standard library\" {
    import std::core;
    let answer: Nat = identity_nat(42);
    tempo 1/4 = 60;
    meter 4/4;
    key c major;
    score { part piano { voice melody { c4/1 } } }
}
";

/// A piece whose source is not ASCII, so bytes and code units disagree and a
/// mistranslated offset is visible rather than a coincidence.
const ACCENTED: &str = "piece \"Étude — n°1\" {
    // a comment with an em dash — and an accent, é
    meter 4/4;
    key c major;
    motif sigh() { e5/8 d5/8 }
    score { part piano { voice right { use sigh(); } } }
}
";

fn wire(source: &str, name: &str) -> Value {
    let session = musa_project::ProjectSession::from_text(source, name);
    let wire = session.snapshot().to_wire();
    assert_eq!(
        wire.pointer("/compiles"),
        Some(&Value::Bool(true)),
        "the fixture must compile: {:#?}",
        wire.pointer("/diagnostics")
    );
    wire
}

/// Every `{start, end}` object in a wire snapshot, with the JSON pointer that
/// found it.
fn spans(value: &Value, at: String, found: &mut Vec<(String, u32, u32)>) {
    match *value {
        Value::Object(ref fields) => {
            let number = |name: &str| fields.get(name).and_then(Value::as_u64).map(|at| at as u32);
            if let (2, Some(start), Some(end)) = (fields.len(), number("start"), number("end")) {
                found.push((at, start, end));
                return;
            }
            for (key, field) in fields {
                spans(field, format!("{at}/{key}"), found);
            }
        }
        Value::Array(ref items) => {
            for (index, item) in items.iter().enumerate() {
                spans(item, format!("{at}/{index}"), found);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn all_spans(wire: &Value) -> Vec<(String, u32, u32)> {
    let mut found = Vec::new();
    spans(wire, String::new(), &mut found);
    found
}

/// Every two-field `{start, end}` object in the wire is a range of the source
/// the wire carries, measured in the units the frontend counts in.
///
/// The claim `crate::utf16::translate_spans` is written against. If an object
/// of that shape ever comes to mean something else — a frame range, a loop
/// region, a pair of counts — this fails, and the walk has to stop being
/// structural before that field ships.
#[test]
fn every_start_end_object_is_a_source_span() {
    for (source, name) in [(IMPORTING, "importing.musa"), (ACCENTED, "accented.musa")] {
        let wire = wire(source, name);
        let text = wire.pointer("/source").and_then(Value::as_str).expect("the source");
        let length = u32::try_from(text.encode_utf16().count()).expect("a fixture fits in u32");
        let found = all_spans(&wire);
        assert!(!found.is_empty(), "{name} carries spans");
        for (at, start, end) in found {
            assert!(start <= end, "{name}{at}: {start}..{end} runs backwards");
            assert!(
                end <= length,
                "{name}{at}: {start}..{end} is outside a source of {length} code units"
            );
        }
    }
}

/// No span on the wire indexes a document the wire did not send.
///
/// A bundled declaration is written in `musa-stdlib:/std/core.musa`, which no
/// snapshot carries. Sent as a span it would be translated against the open
/// document's index and land somewhere arbitrary in the piece; so it is sent
/// as a URI and an opaque byte range, and `library_document` restates it in
/// the module's own measure when the reader actually opens it.
#[test]
fn a_bundled_declaration_travels_as_a_uri_and_never_as_a_span() {
    let wire = wire(IMPORTING, "importing.musa");
    let terms = wire.pointer("/terms").and_then(Value::as_array).expect("terms");
    let bundled: Vec<&Value> = terms
        .iter()
        .filter(|term| term.pointer("/site/where") == Some(&Value::String("library".to_owned())))
        .collect();
    assert!(
        !bundled.is_empty(),
        "the fixture imports `std::core`, so some term is declared there: {terms:#?}"
    );
    for term in bundled {
        let site = term.pointer("/site").expect("a site");
        assert!(
            site.pointer("/uri")
                .and_then(Value::as_str)
                .is_some_and(|uri| uri.starts_with("musa-stdlib:")),
            "a library site names the module: {site:#?}"
        );
        assert!(
            all_spans(site).is_empty(),
            "a library site carries no span-shaped object: {site:#?}"
        );
    }

    // And the handle round-trips: what the wire handed out opens the module.
    let term = wire
        .pointer("/terms")
        .and_then(Value::as_array)
        .and_then(|terms| {
            terms
                .iter()
                .find(|term| term.pointer("/site/where") == Some(&Value::String("library".to_owned())))
        })
        .expect("a bundled term");
    let uri = term.pointer("/site/uri").and_then(Value::as_str).expect("a uri");
    let start = term.pointer("/site/start").and_then(Value::as_u64).expect("a start") as u32;
    let end = term.pointer("/site/end").and_then(Value::as_u64).expect("an end") as u32;
    let document = musa_project::library_document(uri, Some((start, end))).expect("the module opens");
    let span = document.span.expect("the place it named");
    let quoted = quote(&document.text, span.start, span.end);
    assert_eq!(
        quoted,
        term.pointer("/name").and_then(Value::as_str).unwrap_or_default(),
        "the handle points at the declaration's own name"
    );
}

/// A reading arrives in the same units, and says which compile it read.
///
/// Two claims in one test because they are one property: the report crosses the
/// wire the way the snapshot does. The revision is what lets a frontend say a
/// reading is of an older score instead of showing it as though it were current
/// (`docs/rules/desktop/08-elaboration.md` §8) — and it must come from this side,
/// because only this side knows which compile the analysis actually read.
#[test]
fn a_reading_arrives_in_code_units_and_names_the_compile_it_read() {
    let mut session = musa_project::ProjectSession::from_text(ACCENTED, "accented.musa");
    let request = musa_compiler::AnalysisRequest::new(musa_compiler::AnalysisKind::Facts);
    let reading = session.analyze_wire(&request).expect("the piece compiles");

    let revision = reading.pointer("/revision").and_then(Value::as_u64);
    assert_eq!(revision, Some(0), "a piece just opened is at its first revision");

    let units = ACCENTED.encode_utf16().count() as u32;
    let found = all_spans(&reading);
    assert!(
        !found.is_empty(),
        "a facts reading of a piece with notes in it names them"
    );
    for (at, start, end) in found {
        assert!(
            end <= units,
            "{at}: {start}..{end} is outside a source of {units} code units — sent in bytes"
        );
        assert!(
            !quote(ACCENTED, start, end).is_empty(),
            "{at}: a reading's span quotes something"
        );
    }

    // An edit that compiles moves the score on, and the next reading says so.
    session
        .apply(musa_project::ProjectCommand::SetSource(format!("{ACCENTED}\n")))
        .expect("the edit applies");
    let later = session.analyze_wire(&request).expect("the piece still compiles");
    assert_eq!(
        later.pointer("/revision").and_then(Value::as_u64),
        Some(1),
        "the second reading is of the second compile"
    );
}

/// The text a UTF-16 span quotes — the frontend's own reading of a span.
fn quote(text: &str, start: u32, end: u32) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let range = start as usize..end as usize;
    String::from_utf16(units.get(range).expect("a span inside its own text")).expect("valid text")
}

/// A name declared in this document keeps a real span; only foreign ones are
/// projected away.
///
/// The complement of the law above, and the reason it is not simply "no spans
/// on terms": the open document's declarations are exactly what the source
/// column reveals, and losing them to protect the foreign case would trade one
/// correct answer for another.
#[test]
fn a_declaration_in_the_open_document_keeps_its_span() {
    let wire = wire(ACCENTED, "accented.musa");
    let terms = wire.pointer("/terms").and_then(Value::as_array).expect("terms");
    let sigh = terms
        .iter()
        .find(|term| term.pointer("/name") == Some(&Value::String("sigh".to_owned())))
        .expect("the motif is a declaration");
    assert_eq!(sigh.pointer("/site/where"), Some(&Value::String("open".to_owned())));
    let span = sigh.pointer("/site/span").expect("a span");
    let text = wire.pointer("/source").and_then(Value::as_str).expect("the source");
    let start = span.pointer("/start").and_then(Value::as_u64).expect("a start") as u32;
    let end = span.pointer("/end").and_then(Value::as_u64).expect("an end") as u32;
    assert_eq!(
        quote(text, start, end),
        "sigh",
        "the span is measured in code units, so it lands on the name even after an em dash"
    );
}
