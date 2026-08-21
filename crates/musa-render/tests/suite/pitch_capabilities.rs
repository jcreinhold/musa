#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_render::{NotationOptions, NotationTarget, RenderError, render_notation};

fn score_with(pitch: &str) -> musa_score::ScoreSnapshot {
    let source = format!("piece \"Pitch\" {{ score {{ part p {{ voice v {{ {pitch}/1 }} }} }} }}");
    let compilation = compile(
        &SourceDocument::new(&source, "pitch-capability.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:#?}", compilation.diagnostics());
    compilation.snapshot().expect("valid pitch has a score").clone()
}

#[test]
fn mei_encodes_natural_double_and_triple_alterations() {
    for (pitch, attribute) in [
        ("c4", "pname=\"c\""),
        ("c##4", "accid=\"x\""),
        ("c###4", "accid=\"ts\""),
    ] {
        let rendered = render_notation(&score_with(pitch), NotationTarget::Mei, &NotationOptions::default())
            .expect("MEI has an exact spelling for this alteration");
        assert!(rendered.text().contains(attribute), "{}", rendered.text());
    }
}

#[test]
fn mei_refuses_an_unrepresentable_extreme_instead_of_respelling_it() {
    let error = render_notation(&score_with("c####4"), NotationTarget::Mei, &NotationOptions::default())
        .expect_err("MEI has no exact four-sharp accid value");
    assert!(matches!(error, RenderError::Unsupported { .. }));
    assert!(error.to_string().contains("c####4"));
}

#[test]
fn capable_backends_preserve_the_extreme_spelling() {
    let score = score_with("c####4");
    let lily = render_notation(&score, NotationTarget::LilyPond, &NotationOptions::default())
        .expect("LilyPond can repeat accidental suffixes");
    assert!(lily.text().contains("cssss'"), "{}", lily.text());

    let musicxml = render_notation(&score, NotationTarget::MusicXml, &NotationOptions::default())
        .expect("MusicXML alter is an integer");
    assert!(musicxml.text().contains("<alter>4</alter>"), "{}", musicxml.text());
}
