//! Compatibility oracle for the elaboration-language migration (prompt 93).
//!
//! Refresh intentionally with
//! `UPDATE_ELABORATION_BASELINE=1 cargo test -p musa-compiler
//! --test elaboration_compatibility` and review the manifest diff.

#![allow(clippy::expect_used)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use musa_compiler::{
    CompileOptions, PerformanceEvent, PerformanceOptions, SourceDocument, compile, kernel_normal_form,
    lower_performance,
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const UPDATE: &str = "UPDATE_ELABORATION_BASELINE";
const FIXTURES: [(&str, &str); 4] = [
    ("open-shape", include_str!("../../../tests/fixtures/open-shape.musa")),
    (
        "higher-order-shape",
        include_str!("../../../tests/fixtures/higher-order-shape.musa"),
    ),
    (
        "declaration-heavy",
        include_str!("../../../tests/fixtures/declaration-heavy.musa"),
    ),
    (
        "audio-bridge",
        include_str!("../../../tests/fixtures/audio-bridge.musa"),
    ),
];
const LIBRARIES: [&str; 4] = [
    include_str!("../../../tests/fixtures/elaboration-libraries/library-0.musa"),
    include_str!("../../../tests/fixtures/elaboration-libraries/library-1.musa"),
    include_str!("../../../tests/fixtures/elaboration-libraries/library-2.musa"),
    include_str!("../../../tests/fixtures/elaboration-libraries/library-3.musa"),
];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn options(name: &str) -> CompileOptions {
    let mut options = CompileOptions::default();
    if name == "declaration-heavy" {
        for (index, source) in LIBRARIES.into_iter().enumerate() {
            options.imports.insert(
                format!("tests/fixtures/elaboration-libraries/library-{index}.musa"),
                source,
            );
        }
    }
    options
}

fn digest(bytes: &[u8]) -> String {
    format!("{:032x}", musa_kernel::stable_digest(bytes))
}

fn event_digest(events: &[PerformanceEvent]) -> String {
    let mut text = String::new();
    for event in events {
        let _ = writeln!(text, "{event:?}");
    }
    digest(text.as_bytes())
}

fn backend_snapshot_digest(prefix: &str) -> Result<(usize, String)> {
    let directory = repository().join("crates/musa-render/tests/snapshots");
    let mut paths: Vec<PathBuf> = [
        "annotated",
        "canon",
        "clef_change",
        "counterpoint",
        "glass_mountain",
        "invention",
        "modulation",
        "profile_fixture",
        "repeats",
        "tuplet_fixture",
        "twinkle",
    ]
    .into_iter()
    .map(|fixture| directory.join(format!("{prefix}{fixture}.snap")))
    .collect();
    paths.sort();
    let mut corpus = Vec::new();
    for path in &paths {
        corpus.extend_from_slice(path.file_name().unwrap_or_default().as_encoded_bytes());
        corpus.push(0);
        corpus.extend_from_slice(&std::fs::read(path)?);
        corpus.push(0xff);
    }
    Ok((paths.len(), digest(&corpus)))
}

fn manifest() -> Result<String> {
    let mut out = String::from(
        "# musa elaboration compatibility manifest v1\n\
         # Test oracle only; this is not a serialization or public interface.\n",
    );
    for (name, source) in FIXTURES {
        let document_name = format!("tests/fixtures/{name}.musa");
        let document = SourceDocument::new(source, &document_name);
        let options = options(name);
        let compilation = compile(&document, &options);
        let _ = writeln!(out, "\n[fixture {name}]");
        let _ = writeln!(out, "identity={}", compilation.identity());
        let normal = kernel_normal_form(&document, &options.realization)
            .ok_or_else(|| format!("fixture `{name}` has no kernel normal form"))?;
        let _ = writeln!(out, "kernel-normal-form={}", digest(normal.as_bytes()));

        for diagnostic in compilation.diagnostics() {
            let _ = write!(out, "diagnostic={}:{}", diagnostic.severity as u8, diagnostic.code);
            for label in &diagnostic.labels {
                let _ = write!(
                    out,
                    ":{}-{}:{}:{}",
                    label.span.start, label.span.end, label.primary, label.text
                );
            }
            out.push('\n');
        }

        let score = compilation.snapshot().expect("fixture compiles to a score");
        for (part_id, part) in score.parts().iter() {
            for (voice_id, voice) in part.voices() {
                for event in voice.events() {
                    let _ = writeln!(
                        out,
                        "origin=part:{}:{}:voice:{}:{}:event:{}:{}-{}:{}-{}:{:?}",
                        part_id.0,
                        part.name(),
                        voice_id.0,
                        part.voice_name(voice_id).unwrap_or(""),
                        event.id.0,
                        event.origin.source_span.start,
                        event.origin.source_span.end,
                        event.origin.definition_span.start,
                        event.origin.definition_span.end,
                        event.origin.expansion_path
                    );
                }
            }
        }

        let performance = lower_performance(score, &PerformanceOptions::default())?;
        for lane in performance.lanes() {
            let _ = writeln!(
                out,
                "lane={}:{}:{}:{}",
                lane.part().0,
                lane.name(),
                lane.events().len(),
                event_digest(lane.events())
            );
        }

        let studio = compilation.studio();
        for (patch_name, patch) in studio.patches() {
            let _ = write!(out, "patch={patch_name}:output={:?}", patch.output());
            for node in patch.nodes() {
                let _ = write!(out, ":{}:{:?}", node.processor.name(), node.label);
                for value in &node.params {
                    let _ = write!(out, ":{}{}", value.magnitude, value.unit.spelling().unwrap_or(""));
                }
            }
            out.push('\n');
        }
        for (bus_name, bus) in studio.buses() {
            let processors: Vec<&str> = bus.nodes().iter().map(|node| node.processor.name()).collect();
            let _ = writeln!(out, "bus={bus_name}:{processors:?}");
        }
        for (part, patch) in studio.assignments() {
            let _ = writeln!(out, "assign={part}->{patch}");
        }
        for route in studio.routes() {
            let _ = writeln!(out, "route={}->{}", route.source, route.destination);
        }
        for send in studio.sends() {
            let _ = writeln!(
                out,
                "send={}->{}:{}{}",
                send.source,
                send.bus,
                send.level.magnitude,
                send.level.unit.spelling().unwrap_or("")
            );
        }
        for modulation in studio.modulations() {
            let _ = writeln!(
                out,
                "modulate={}->{}:{}:{}",
                modulation.source, modulation.patch, modulation.node, modulation.param
            );
        }
    }

    for (name, prefix) in [("mei", "mei__"), ("lilypond", "lilypond__"), ("musicxml", "musicxml__")] {
        let (count, corpus_digest) = backend_snapshot_digest(prefix)?;
        let _ = writeln!(out, "\n[backend {name}]\nfiles={count}\ndigest={corpus_digest}");
    }
    // MIDI and WAV use byte-level law tests instead of insta snapshots. The
    // audio-bridge oracle owns their concrete bytes below the compiler layer.
    out.push_str("\n[backend midi]\noracle=crates/musa-render/tests/midi.rs\n");
    out.push_str("\n[backend wav]\noracle=tests/fixtures/audio-bridge-audio.compat\n");
    Ok(out)
}

fn compare_or_update(path: &Path, actual: &str) -> Result {
    let expected = std::fs::read_to_string(path).ok();
    if expected.as_deref() == Some(actual) {
        return Ok(());
    }
    if std::env::var_os(UPDATE).is_some() {
        std::fs::write(path, actual)?;
        return Ok(());
    }
    Err(format!(
        "{} differs — rerun with {UPDATE}=1, review, and commit the manifest",
        path.display()
    )
    .into())
}

#[test]
fn existing_language_behavior_matches_the_migration_oracle() -> Result {
    let actual = manifest()?;
    compare_or_update(
        &repository().join("tests/fixtures/elaboration-compatibility.txt"),
        &actual,
    )
}

#[test]
fn every_expected_change_names_one_repairing_prompt() -> Result {
    const LEDGER: &str = include_str!("../../../tests/fixtures/elaboration-expected-changes.json");
    for (defect, prompt) in [
        ("shared-note-stream-warning", "123"),
        ("ignored-parameter-event", "124"),
        ("processor-hover-gap", "119"),
        ("graph-topology-modulation-address", "122"),
        ("eager-studio-f64-conversion", "120"),
    ] {
        assert!(LEDGER.contains(&format!("\"defect\": \"{defect}\"")));
        assert!(LEDGER.contains(&format!("\"repair_prompt\": {prompt}")));
    }
    assert_eq!(LEDGER.matches("\"defect\"").count(), 5);
    Ok(())
}
