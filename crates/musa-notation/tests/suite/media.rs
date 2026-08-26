//! Recorded media is labelled on the page, while executable semantics are
//! reported as a backend loss rather than inferred from that label.

#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_notation::{NotationOptions, NotationTarget, render_notation};

const MEDIA: &str = r#"piece "Media" {
    meter 4/4;
    clip pulse from "assets/pulse.wav" fit 1/1 by rate;
    fixed_media harbor from "assets/harbor.wav";
    score {
        cue pulse at 1:1;
        cue harbor at 2:1;
        part guide { voice one { c4/1 c4/1 } }
    }
}"#;

#[test]
fn every_backend_labels_media_and_reports_semantic_loss() {
    let compilation = compile(&SourceDocument::new(MEDIA, "media.musa"), &CompileOptions::default());
    let score = compilation.snapshot().expect("media fixture compiles");
    for target in [NotationTarget::Mei, NotationTarget::LilyPond, NotationTarget::MusicXml] {
        let rendered = render_notation(score, target, &NotationOptions::default()).expect("media labels render");
        assert!(
            rendered.text().contains("clip pulse (rate)"),
            "{target:?}: {}",
            rendered.text()
        );
        assert!(
            rendered.text().contains("fixed media harbor"),
            "{target:?}: {}",
            rendered.text()
        );
        assert!(
            rendered
                .warnings()
                .iter()
                .any(|warning| warning.contains("cannot encode executable recorded media")),
            "{target:?}: {:?}",
            rendered.warnings()
        );
    }
}
