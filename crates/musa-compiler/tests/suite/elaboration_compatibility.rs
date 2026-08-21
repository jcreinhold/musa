//! Compatibility oracle for the elaboration-language migration.
//!
//! Refresh intentionally with
//! `UPDATE_ELABORATION_BASELINE=1 cargo test -p musa-compiler
//! --test suite` and review the manifest diff.

#![allow(clippy::expect_used)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use musa_compiler::{CompileOptions, SourceDocument, compile, events_normal_form};

use musa_score::{PerformanceEvent, PerformanceOptions, lower_performance};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const UPDATE: &str = "UPDATE_ELABORATION_BASELINE";
const FIXTURES: [(&str, &str); 4] = [
    ("open-shape", include_str!("../../../../tests/fixtures/open-shape.musa")),
    (
        "higher-order-shape",
        include_str!("../../../../tests/fixtures/higher-order-shape.musa"),
    ),
    (
        "declaration-heavy",
        include_str!("../../../../tests/fixtures/declaration-heavy.musa"),
    ),
    (
        "audio-bridge",
        include_str!("../../../../tests/fixtures/audio-bridge.musa"),
    ),
];
const LIBRARIES: [&str; 4] = [
    include_str!("../../../../tests/fixtures/elaboration-libraries/library-0.musa"),
    include_str!("../../../../tests/fixtures/elaboration-libraries/library-1.musa"),
    include_str!("../../../../tests/fixtures/elaboration-libraries/library-2.musa"),
    include_str!("../../../../tests/fixtures/elaboration-libraries/library-3.musa"),
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
    format!("{:032x}", musa_events::stable_digest(bytes))
}

fn event_digest(events: &[PerformanceEvent]) -> String {
    let mut text = String::new();
    for event in events {
        let _ = writeln!(text, "{event:?}");
    }
    digest(text.as_bytes())
}

fn backend_snapshot_digest(prefix: &str) -> Result<(usize, String)> {
    let directory = repository().join("crates/musa-notation/tests/suite/snapshots");
    let mut names: Vec<String> = [
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
    .map(|fixture| format!("{prefix}{fixture}.snap"))
    .collect();
    names.sort();
    // The corpus records the fixture's own name, not the file's. insta prefixes the
    // file with the test target (`suite__`), and the oracle must not move when the
    // test harness is rearranged — only when a backend's output actually changes.
    let mut corpus = Vec::new();
    for name in &names {
        corpus.extend_from_slice(name.as_bytes());
        corpus.push(0);
        corpus.extend_from_slice(&std::fs::read(directory.join(format!("suite__{name}")))?);
        corpus.push(0xff);
    }
    Ok((names.len(), digest(&corpus)))
}

/// The deliberate breaks this baseline has absorbed, and why each one was
/// allowed to move it.
///
/// `docs/rules/language/README.md`'s graduation criterion 2 asks that a pre-candidate
/// example either keep its meaning or have an explicit, tested migration
/// diagnostic. A refreshed manifest cannot tell those two apart on its own —
/// the digests simply become the new digests — so a break is named here before
/// it is refreshed, and the manifest carries the list. Reviewing a baseline
/// diff then means checking that the section above it explains the section
/// below it.
const BREAKS: [(u32, &str, &str); 7] = [
    (
        109,
        "`use` no longer imports; the import statement is spelled `import`",
        "parser::the_old_import_spelling_is_a_migration_error",
    ),
    (
        111,
        "`module` no longer declares the static layer; that declaration is spelled `structure`",
        "parser::the_old_static_layer_spelling_is_a_migration_error",
    ),
    (
        112,
        "a function body is a block: `fn f() -> t { e }` replaces `fn f() -> t = e;`",
        "parser::the_old_function_body_spelling_is_a_migration_error",
    ),
    (
        113,
        "a type is spelled with a capital: `Music` replaces `music`, and `NoteName` replaces `pitchclass`",
        "parser::the_old_type_spellings_are_migration_errors",
    ),
    (
        113,
        "`Option`'s constructors move with it: `Some` and `None` replace `some` and `none`",
        "parser::the_old_option_constructors_are_migration_errors",
    ),
    (
        114,
        "a type parameter is angle-bracketed: `Option<Pitch>` replaces `Option[Pitch]`, and `[` means a list",
        "parser::the_old_type_parameter_brackets_are_migration_errors",
    ),
    (
        116,
        "`assert` is a statement keyword, so it is no longer available as a name",
        "parser::assert_is_no_longer_available_as_a_name",
    ),
];

fn manifest() -> Result<String> {
    let mut out = String::from(
        "# musa elaboration compatibility manifest v1\n\
         # Test oracle only; this is not a serialization or public interface.\n\
         \n[breaks]\n",
    );
    for (prompt, what, migration) in BREAKS {
        let _ = writeln!(out, "break={prompt}:{what}:{migration}");
    }
    for (name, source) in FIXTURES {
        let document_name = format!("tests/fixtures/{name}.musa");
        let document = SourceDocument::new(source, &document_name);
        let options = options(name);
        let compilation = compile(&document, &options);
        let _ = writeln!(out, "\n[fixture {name}]");
        let _ = writeln!(out, "identity={}", compilation.identity());
        let normal = events_normal_form(&document, &options.realization, &options.imports)
            .ok_or_else(|| format!("fixture `{name}` has no events normal form"))?;
        let _ = writeln!(out, "events-normal-form={}", digest(normal.as_bytes()));

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
    out.push_str("\n[backend midi]\noracle=crates/musa-notation/tests/suite/midi.rs\n");
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

/// Every baselined defect names the prompt that repairs it, **by slug**.
///
/// A prompt's number is an execution rank and moves whenever one is inserted;
/// its slug is its identity and does not. The ledger recorded ranks until this
/// test caught them pointing at the wrong prompts — three insertions had moved
/// the sound block underneath it, and nothing checked. Ranks are not written
/// here at all now, so there is no second copy to drift.
///
/// The pairing is asserted, not merely the presence of both strings, and the
/// named prompt must exist on disk.
#[test]
fn every_expected_change_names_one_repairing_prompt() -> Result {
    const LEDGER: &str = include_str!("../../../../tests/fixtures/elaboration-expected-changes.json");
    let prompts = repository().join("docs/plan/prompts");
    for (defect, slug) in [
        ("processor-hover-gap", "studio-vocabulary"),
        ("eager-studio-f64-conversion", "exact-studio-values"),
        ("graph-topology-modulation-address", "expressive-control-realization"),
        ("shared-note-stream-warning", "part-instrument-routing"),
        ("ignored-parameter-event", "expressive-control-realization"),
    ] {
        let entry = LEDGER
            .split('{')
            .find(|entry| entry.contains(&format!("\"defect\": \"{defect}\"")))
            .ok_or_else(|| format!("no ledger entry for {defect}"))?;
        assert!(
            entry.contains(&format!("\"repaired_by\": \"{slug}\"")),
            "{defect} must be repaired by {slug}"
        );
        assert!(
            std::fs::read_dir(&prompts)?
                .filter_map(std::result::Result::ok)
                .any(|prompt| {
                    prompt
                        .file_name()
                        .to_str()
                        .is_some_and(|name| name.ends_with(&format!("-{slug}.md")))
                }),
            "no prompt file for {slug}"
        );
    }
    assert_eq!(LEDGER.matches("\"defect\"").count(), 5);
    assert!(!LEDGER.contains("repair_prompt"), "the ledger names prompts by slug");
    Ok(())
}
