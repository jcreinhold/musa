//! Recorded-media laws: written support is exact, fixed duration is absent,
//! and the checked projection retains source identity and provenance.

#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, ImportSources, SourceDocument, compile, resolve_import, standard_library_source};
use musa_score::score::{MediaFit, MediaKind};
use musa_syntax::ast::{DataDecl, RecordDecl};

fn source(policy: &str) -> String {
    format!(
        r#"piece "Media" {{
    meter 4/4;
    clip pulse from "assets/pulse.wav" fit 1/1 by {policy};
    fixed_media harbor from "assets/harbor.wav";
    score {{
        cue pulse at 1:1;
        cue harbor at 2:1;
        part guide {{ voice one {{ c4/1 c4/1 }} }}
    }}
}}"#
    )
}

#[test]
fn host_media_types_are_an_exact_projection_of_the_source_schema() {
    let source = standard_library_source("musa-stdlib:/std/sound/media.musa").expect("bundled media module");
    let parsed = musa_syntax::parse(source);
    assert_eq!(parsed.errors(), &[], "{:#?}", parsed.errors());
    let root = parsed.syntax();
    let declarations = DataDecl::all_at_root(&root);

    let fit = declarations
        .iter()
        .find(|declaration| declaration.name().as_deref() == Some("MediaFit"))
        .expect("source MediaFit");
    let source_fit: Vec<String> = fit.variants().iter().filter_map(|variant| variant.name()).collect();
    let projected_fit: Vec<String> = [MediaFit::Crop, MediaFit::Loop, MediaFit::Rate]
        .into_iter()
        .map(|fit| {
            match fit {
                MediaFit::Crop => "Crop",
                MediaFit::Loop => "Loop",
                MediaFit::Rate => "Rate",
            }
            .to_owned()
        })
        .collect();
    assert_eq!(source_fit, projected_fit);

    let actions = declarations
        .iter()
        .find(|declaration| declaration.name().as_deref() == Some("MediaAction"))
        .expect("source MediaAction");
    let source_actions: Vec<(String, Vec<String>)> = actions
        .variants()
        .iter()
        .map(|variant| {
            (
                variant.name().unwrap_or_default(),
                variant.fields().iter().filter_map(|field| field.name()).collect(),
            )
        })
        .collect();
    let projected_actions: Vec<String> = [MediaKind::MusicalClip(MediaFit::Crop), MediaKind::FixedMediaCue]
        .into_iter()
        .map(|kind| {
            match kind {
                MediaKind::MusicalClip(_) => "MusicalClip",
                MediaKind::FixedMediaCue => "FixedMediaCue",
            }
            .to_owned()
        })
        .collect();
    assert_eq!(
        source_actions,
        vec![
            (
                projected_actions.first().expect("clip projection").clone(),
                vec!["name", "asset", "fit", "playback"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            ),
            (
                projected_actions.get(1).expect("fixed projection").clone(),
                vec!["name", "asset", "playback"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            ),
        ]
    );

    let playback = RecordDecl::all_at_root(&root)
        .into_iter()
        .find(|declaration| declaration.name().as_deref() == Some("MediaPlayback"))
        .expect("source MediaPlayback");
    assert_eq!(
        playback
            .fields()
            .iter()
            .filter_map(|field| field.name())
            .collect::<Vec<_>>(),
        ["gain_db"]
    );
}

#[test]
fn clip_and_fixed_media_have_disjoint_written_support() {
    let compilation = compile(
        &SourceDocument::new(source("rate"), "media.musa"),
        &CompileOptions::default(),
    );
    assert_eq!(compilation.diagnostics(), &[], "{:#?}", compilation.diagnostics());
    let snapshot = compilation.snapshot().expect("compiled snapshot");
    let media = snapshot.annotations().media();
    assert_eq!(media.len(), 2);
    let clip = media.first().expect("clip occurrence");
    let fixed = media.get(1).expect("fixed-media occurrence");
    assert_eq!(clip.name, "pulse");
    assert_eq!(clip.asset, "assets/pulse.wav");
    assert_eq!(clip.kind, MediaKind::MusicalClip(MediaFit::Rate));
    assert_eq!(clip.start.as_ratio().to_string(), "0");
    assert_eq!(clip.end.as_ratio().to_string(), "1");
    assert_eq!(fixed.kind, MediaKind::FixedMediaCue);
    assert_eq!(fixed.start, fixed.end, "fixed media is a written-time point");
    assert_eq!(fixed.start.as_ratio().to_string(), "1");
    assert_ne!(clip.origin.source_span, clip.origin.definition_span);
}

#[test]
fn every_declared_fit_policy_projects_exactly() {
    for (written, expected) in [
        ("crop", MediaFit::Crop),
        ("loop", MediaFit::Loop),
        ("rate", MediaFit::Rate),
    ] {
        let compilation = compile(
            &SourceDocument::new(source(written), "media.musa"),
            &CompileOptions::default(),
        );
        let snapshot = compilation.snapshot().expect("every policy compiles");
        assert_eq!(
            snapshot.annotations().media().first().expect("clip occurrence").kind,
            MediaKind::MusicalClip(expected)
        );
    }
}

#[test]
fn unknown_media_and_policy_are_diagnostics_not_inference() {
    let bad_policy = compile(
        &SourceDocument::new(source("warp"), "media.musa"),
        &CompileOptions::default(),
    );
    assert!(
        bad_policy
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("fit policy"))
    );

    let missing = source("crop").replace("cue harbor at", "cue absent at");
    let unknown = compile(&SourceDocument::new(missing, "media.musa"), &CompileOptions::default());
    assert!(
        unknown
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("cannot find recorded media"))
    );
}

#[test]
fn file_and_imported_declarations_share_the_source_namespace() {
    let root = r#"clip pulse from "assets/pulse.wav" fit 1/1 by crop;
piece "Root media" {
    meter 4/4;
    score {
        cue pulse at 1:1;
        part guide { voice one { c4/1 } }
    }
}"#;
    let compilation = compile(
        &SourceDocument::new(root, "root-media.musa"),
        &CompileOptions::default(),
    );
    assert_eq!(compilation.diagnostics(), &[], "{:#?}", compilation.diagnostics());
    assert_eq!(
        compilation
            .snapshot()
            .expect("root declaration compiles")
            .annotations()
            .media()
            .len(),
        1
    );

    let source = r#"import "media.musa";
piece "Imported media" {
    meter 4/4;
    score {
        cue harbor at 1:1;
        part guide { voice one { c4/1 } }
    }
}"#;
    let mut imports = ImportSources::default();
    imports.insert(
        resolve_import("piece.musa", "media.musa"),
        r#"fixed_media harbor from "assets/harbor.wav";"#,
    );
    let compilation = compile(
        &SourceDocument::new(source, "piece.musa"),
        &CompileOptions {
            imports,
            ..CompileOptions::default()
        },
    );
    assert_eq!(compilation.diagnostics(), &[], "{:#?}", compilation.diagnostics());
    let media = compilation
        .snapshot()
        .expect("imported declaration compiles")
        .annotations()
        .media();
    assert_eq!(media.len(), 1);
    assert_eq!(media.first().expect("imported media occurrence").name, "harbor");
}
