use musa_compiler::{CompileOptions, SourceDocument, compile};

#[test]
fn contextual_music_crosses_the_expression_stage_at_use() {
    let source = SourceDocument::new(
        "piece \"staged\" { let answer: EventTrack<WrittenTime> = music { c4/4 d4/4 }; score { part p { voice v { use answer; } } } }",
        "staged.musa",
    );
    let compilation = compile(&source, &CompileOptions::default());
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    let events = compilation
        .snapshot()
        .and_then(|snapshot| snapshot.parts().iter().next().map(|(_, part)| part))
        .and_then(|part| part.voices().next().map(|(_, voice)| voice.events()));
    assert!(matches!(events, Some(events) if events.len() == 2));
}
