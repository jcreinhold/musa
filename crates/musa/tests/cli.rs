//! CLI behavior tests: exit codes and user-visible output of `musa format`
//! and `musa check`.

use std::process::Command;

fn glass_mountain() -> String {
    format!("{}/../../examples/glass-mountain.musa", env!("CARGO_MANIFEST_DIR"))
}

fn musa(args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_musa")).args(args).output()
}

fn temp_file(name: &str, contents: &str) -> std::io::Result<std::path::PathBuf> {
    let path = std::env::temp_dir().join(format!("musa-test-{}-{name}", std::process::id()));
    std::fs::write(&path, contents)?;
    Ok(path)
}

#[test]
fn check_accepts_a_valid_piece() -> std::io::Result<()> {
    let output = musa(&["check", &glass_mountain()])?;
    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    Ok(())
}

#[test]
fn check_rejects_a_broken_piece_with_diagnostics() -> std::io::Result<()> {
    let path = temp_file("bad.musa", "piece \"x\" { meter 4/4 meter 2/2; }")?;
    let output = musa(&["check", &path.to_string_lossy()])?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing `;`"), "stderr: {stderr}");
    // The report carries the place, the fix, and the tally.
    assert!(stderr.contains("1:22"), "stderr: {stderr}");
    assert!(stderr.contains("fix: add `;`"), "stderr: {stderr}");
    assert!(stderr.contains("1 problem (1 error)"), "stderr: {stderr}");
    std::fs::remove_file(&path)
}

/// Every shipped example is already in canonical form.
///
/// This used to check `glass-mountain.musa` alone, which meant a new example
/// could arrive unformatted and stay that way — the examples are executable
/// specifications, and a specification written in a shape the formatter would
/// rewrite teaches the wrong shape.
#[test]
fn format_check_passes_on_canonical_examples() -> std::io::Result<()> {
    let root = format!("{}/../..", env!("CARGO_MANIFEST_DIR"));
    let mut count = 0usize;
    for directory in ["examples", "examples/analysis"] {
        for entry in std::fs::read_dir(format!("{root}/{directory}"))? {
            let path = entry?.path();
            if path.extension().is_none_or(|extension| extension != "musa") {
                continue;
            }
            let output = musa(&["format", "--check", &path.to_string_lossy()])?;
            assert!(
                output.status.success(),
                "{} is not formatted: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr)
            );
            count = count.saturating_add(1);
        }
    }
    assert!(count >= 3, "expected at least 3 examples, found {count}");
    Ok(())
}

#[test]
fn format_rewrites_a_messy_file_to_canonical_form() -> std::io::Result<()> {
    let messy = "piece   \"M\"{\nmeter 4/4;\nscore{\npart p{\nvoice v{\nc5   1\n}\n}\n}\n}\n";
    let path = temp_file("messy.musa", messy)?;
    let output = musa(&["format", &path.to_string_lossy()])?;
    assert!(output.status.success());
    let rewritten = std::fs::read_to_string(&path)?;
    assert_eq!(
        rewritten,
        "piece \"M\" {\n    meter 4/4;\n    score {\n        part p {\n            voice v {\n                c5 1\n            }\n        }\n    }\n}\n"
    );
    // And now --check passes.
    let output = musa(&["format", "--check", &path.to_string_lossy()])?;
    assert!(output.status.success());
    std::fs::remove_file(&path)
}

#[test]
fn format_check_fails_on_an_unformatted_file() -> std::io::Result<()> {
    let path = temp_file("unformatted.musa", "piece   \"U\"{\nmeter 4/4;\n}\n")?;
    let output = musa(&["format", "--check", &path.to_string_lossy()])?;
    assert!(!output.status.success());
    std::fs::remove_file(&path)
}

/// `--check` writes nothing, not even the file it rejects.
///
/// It used to format the document to find out, and an unsaved edit is
/// autosaved — so checking a file left a `.recovery` copy beside it, and
/// checking a working tree littered it.
#[test]
fn format_check_leaves_the_directory_alone() -> std::io::Result<()> {
    let path = temp_file("untouched.musa", "piece   \"U\"{\nmeter 4/4;\n}\n")?;
    let recovery = path.with_extension("musa.recovery");
    let output = musa(&["format", "--check", &path.to_string_lossy()])?;
    assert!(!output.status.success());
    assert!(!recovery.exists(), "{} was written", recovery.display());
    assert_eq!(std::fs::read_to_string(&path)?, "piece   \"U\"{\nmeter 4/4;\n}\n");
    std::fs::remove_file(&path)
}

const MESSY: &str = "piece   \"M\"{\nmeter 4/4;\n}\n";
const TIDY: &str = "piece \"M\" {\n    meter 4/4;\n}\n";

fn temp_dir(name: &str) -> std::io::Result<std::path::PathBuf> {
    let path = std::env::temp_dir().join(format!("musa-test-{}-{name}", std::process::id()));
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

/// One command formats a folder, the way `ruff format` and `cargo fmt` do.
///
/// The file worth formatting is rarely the one that happens to be open, so a
/// folder argument is walked whole: nested folders are reached, hidden ones
/// are not, and a file that is not `.musa` is not a musa file whatever it
/// holds.
#[test]
fn format_walks_a_folder_and_skips_what_is_not_its_business() -> std::io::Result<()> {
    let root = temp_dir("walk")?;
    std::fs::create_dir_all(root.join("inner"))?;
    std::fs::create_dir_all(root.join(".hidden"))?;
    std::fs::write(root.join("top.musa"), MESSY)?;
    std::fs::write(root.join("inner/deep.musa"), MESSY)?;
    std::fs::write(root.join(".hidden/kept.musa"), MESSY)?;
    std::fs::write(root.join("notes.txt"), MESSY)?;

    let output = musa(&["format", &root.to_string_lossy()])?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    assert_eq!(std::fs::read_to_string(root.join("top.musa"))?, TIDY);
    assert_eq!(std::fs::read_to_string(root.join("inner/deep.musa"))?, TIDY);
    assert_eq!(std::fs::read_to_string(root.join(".hidden/kept.musa"))?, MESSY);
    assert_eq!(std::fs::read_to_string(root.join("notes.txt"))?, MESSY);
    assert!(
        stderr.contains("2 files formatted, 0 already formatted"),
        "stderr: {stderr}"
    );
    std::fs::remove_dir_all(&root)
}

/// No path means this folder — the shape a formatter is run in.
#[test]
fn format_with_no_path_formats_the_folder_it_was_run_in() -> std::io::Result<()> {
    let root = temp_dir("here")?;
    std::fs::write(root.join("here.musa"), MESSY)?;
    let output = Command::new(env!("CARGO_BIN_EXE_musa"))
        .arg("format")
        .current_dir(&root)
        .output()?;
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read_to_string(root.join("here.musa"))?, TIDY);
    std::fs::remove_dir_all(&root)
}

/// `.musaignore` names what the walk passes over.
///
/// Some files are shaped the way they are on purpose — a generator writes
/// them, or a diagnostic's snapshot pins their byte positions — and a walk
/// that reaches them rewrites them by accident.
#[test]
fn format_passes_over_what_the_ignore_file_names() -> std::io::Result<()> {
    let root = temp_dir("ignored")?;
    std::fs::create_dir_all(root.join("fixtures"))?;
    std::fs::write(root.join(".musaignore"), "# on purpose\nfixtures/\n*.pinned.musa\n")?;
    std::fs::write(root.join("ordinary.musa"), MESSY)?;
    std::fs::write(root.join("a.pinned.musa"), MESSY)?;
    std::fs::write(root.join("fixtures/generated.musa"), MESSY)?;

    let output = musa(&["format", &root.to_string_lossy()])?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    assert_eq!(std::fs::read_to_string(root.join("ordinary.musa"))?, TIDY);
    assert_eq!(std::fs::read_to_string(root.join("a.pinned.musa"))?, MESSY);
    assert_eq!(std::fs::read_to_string(root.join("fixtures/generated.musa"))?, MESSY);
    // What was passed over is said out loud, not skipped in silence.
    assert!(stderr.contains("2 paths ignored"), "stderr: {stderr}");
    std::fs::remove_dir_all(&root)
}

/// The list governs a file named on the command line too, and `-f` is the way
/// past it.
///
/// An excluded file is excluded because its shape is a specification, and that
/// is as true of the file a script names as of the file a walk finds. The
/// override is spelled the way `git add -f` spells it: available, and never
/// taken by accident.
#[test]
fn format_passes_over_an_ignored_file_that_is_named() -> std::io::Result<()> {
    let root = temp_dir("named")?;
    std::fs::write(root.join(".musaignore"), "*.musa\n")?;
    let path = root.join("pinned.musa");
    std::fs::write(&path, MESSY)?;

    // Named, and left alone: every file found is excluded, so there is nothing
    // to format and the run says which list said so.
    let output = musa(&["format", &path.to_string_lossy()])?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("excluded by `.musaignore`"), "stderr: {stderr}");
    assert_eq!(std::fs::read_to_string(&path)?, MESSY);

    // The same file, reached by walking, is left alone the same way.
    let output = musa(&["format", &root.to_string_lossy()])?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("excluded by `.musaignore`"), "stderr: {stderr}");
    assert_eq!(std::fs::read_to_string(&path)?, MESSY);

    // `-f` formats it anyway.
    let output = musa(&["format", "-f", &path.to_string_lossy()])?;
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read_to_string(&path)?, TIDY);
    std::fs::remove_dir_all(&root)
}

/// `-f` reaches into an ignored folder that a walk would pass over, so the
/// override is a fact about the list rather than about how the path was typed.
#[test]
fn forcing_a_walk_formats_what_the_list_names() -> std::io::Result<()> {
    let root = temp_dir("forced")?;
    std::fs::create_dir_all(root.join("fixtures"))?;
    std::fs::write(root.join(".musaignore"), "fixtures/\n")?;
    let path = root.join("fixtures/generated.musa");
    std::fs::write(&path, MESSY)?;

    let output = musa(&["format", "-f", &path.to_string_lossy()])?;
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read_to_string(&path)?, TIDY);
    std::fs::remove_dir_all(&root)
}

/// The list is found from above, so it governs a walk started inside it.
#[test]
fn format_finds_the_ignore_file_above_the_folder_it_walks() -> std::io::Result<()> {
    let root = temp_dir("above")?;
    std::fs::create_dir_all(root.join("scores/drafts"))?;
    std::fs::write(root.join(".musaignore"), "scores/drafts/\n")?;
    std::fs::write(root.join("scores/keep.musa"), MESSY)?;
    std::fs::write(root.join("scores/drafts/skip.musa"), MESSY)?;

    let output = musa(&["format", &root.join("scores").to_string_lossy()])?;
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read_to_string(root.join("scores/keep.musa"))?, TIDY);
    assert_eq!(std::fs::read_to_string(root.join("scores/drafts/skip.musa"))?, MESSY);
    std::fs::remove_dir_all(&root)
}

/// A file that does not parse is reported and left exactly as it was.
///
/// It matters more now that one command reaches a whole folder: the formatter
/// would be guessing at what half-written text meant, and it would write the
/// guess over the text its author was in the middle of.
#[test]
fn format_leaves_a_file_that_does_not_parse_alone() -> std::io::Result<()> {
    let broken = "piece \"B\" { meter 4/4\n";
    let path = temp_file("unparsed.musa", broken)?;
    let output = musa(&["format", &path.to_string_lossy()])?;
    assert!(!output.status.success());
    assert_eq!(std::fs::read_to_string(&path)?, broken);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not parse"), "stderr: {stderr}");
    std::fs::remove_file(&path)
}

// --- WAV export ------------------------------------------------------------

/// Every shipped example renders to WAV deterministically (same source →
/// byte-identical bytes, §17.5) with the expected header.
///
/// Ignored by default because it costs about five minutes: it renders all
/// forty-eight examples twice through the binary, and one of them —
/// `glass-mountain.musa`, whose studio graph has a reverb bus and a modulated
/// filter — takes over a minute a pass in an unoptimized build. Run it with
/// `cargo nextest run -p musa --run-ignored all`, or `cargo test -p musa --
/// --include-ignored`.
///
/// What is *not* lost by ignoring it: the determinism contract itself is held
/// in-process and in milliseconds by
/// `musa-project`'s `session_laws::wav_export_is_deterministic`, and the exact
/// bytes are pinned by its
/// `elaboration_backend_compatibility::every_existing_backend_matches_the_migration_oracle`.
/// What this one adds is corpus breadth — that the audio pipeline still runs
/// over every shipped example — which is worth having and is not worth five
/// minutes of every routine run.
#[test]
#[ignore = "slow: renders every example twice; run with --run-ignored all"]
fn wav_export_is_deterministic_for_all_examples() -> std::io::Result<()> {
    let examples = format!("{}/../../examples", env!("CARGO_MANIFEST_DIR"));
    let mut count = 0usize;
    for entry in std::fs::read_dir(&examples)? {
        let entry = entry?;
        if entry.path().extension().and_then(|e| e.to_str()) != Some("musa") {
            continue;
        }
        count += 1;
        let source = entry.path();
        let first = std::env::temp_dir().join(format!("musa-test-{}-a.wav", std::process::id()));
        let second = std::env::temp_dir().join(format!("musa-test-{}-b.wav", std::process::id()));
        for target in [&first, &second] {
            let output = musa(&[
                "render",
                &source.to_string_lossy(),
                "--to",
                "wav",
                "-o",
                &target.to_string_lossy(),
            ])?;
            assert!(
                output.status.success(),
                "{}: stderr: {:?}",
                source.display(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let a = std::fs::read(&first)?;
        let b = std::fs::read(&second)?;
        assert_eq!(a, b, "{}: WAV render is not deterministic", source.display());
        assert_eq!(
            a.get(0..4),
            Some(b"RIFF".as_slice()),
            "{}: not a WAV file",
            source.display()
        );
        assert!(a.len() > 44, "{}: suspiciously small WAV", source.display());
        std::fs::remove_file(&first)?;
        std::fs::remove_file(&second)?;
    }
    assert!(count >= 3, "expected at least 3 examples, found {count}");
    Ok(())
}

/// The album fixture, end to end: a piece in a directory project renders to
/// audio through the libraries it imports, and its tempo change is in the
/// audio rather than only in the notation.
///
/// The check is the length. Twenty-four quarter notes at 72 bpm would last
/// 20 seconds; the same twenty-four with everything after the change at 108
/// last 15⅑. A render that ignored the change would be nearly five seconds
/// longer, which no tail or rounding accounts for.
#[test]
fn the_album_piece_renders_through_its_imports_at_the_tempos_it_writes() -> std::io::Result<()> {
    let source = format!(
        "{}/../../examples/album/pieces/01-opening.musa",
        env!("CARGO_MANIFEST_DIR")
    );
    let wav = std::env::temp_dir().join(format!("musa-album-{}.wav", std::process::id()));
    let output = musa(&["render", &source, "--to", "wav", "-o", &wav.to_string_lossy()])?;
    assert!(
        output.status.success(),
        "stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = std::fs::read(&wav)?;
    assert_eq!(bytes.get(0..4), Some(b"RIFF".as_slice()));
    // 32-bit float, two channels, 48 kHz: eight bytes a frame.
    let seconds = (bytes.len().saturating_sub(44) / 8) as f64 / 48_000.0;
    let written = 8.0 * 60.0 / 72.0 + 16.0 * 60.0 / 108.0;
    assert!(
        seconds > written && seconds < written + 4.0,
        "expected about {written:.2} s of music plus a release tail, got {seconds:.2} s"
    );
    std::fs::remove_file(&wav)
}

/// Every fixture in `examples/broken` renders the way it is supposed to.
///
/// These snapshots are the diagnostics' goldens. A message is writing, and
/// writing rots: the only way to notice that a help line stopped matching its
/// message, or that a fix started pointing at the wrong character, is to look
/// at the whole rendered report and keep looking at it.
///
/// The renderer is pinned to a fixed width and no colour (see
/// `install_renderer`), so what is snapshotted here is what a reader sees.
#[test]
fn every_broken_fixture_renders_the_way_it_reads() -> std::io::Result<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../");
    let mut fixtures: Vec<std::path::PathBuf> = std::fs::read_dir(root.join("examples/broken"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "musa"))
        .collect();
    fixtures.sort();
    assert!(fixtures.len() >= 5, "expected the broken corpus, found {fixtures:?}");
    for fixture in fixtures {
        let name = fixture
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        // Run from the repository root so the header reads `examples/broken/…`
        // on every machine.
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_musa"))
            .current_dir(&root)
            .args(["check", &format!("examples/broken/{name}.musa")])
            .output()?;
        assert!(!output.status.success(), "{name} is supposed to be broken");
        insta::assert_snapshot!(name, String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

#[test]
fn format_diff_is_quiet_and_successful_on_a_canonical_example() -> std::io::Result<()> {
    let output = musa(&["format", "--diff", &glass_mountain()])?;
    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    Ok(())
}

#[test]
fn format_diff_shows_the_change_and_writes_nothing() -> std::io::Result<()> {
    let messy = "piece   \"M\"{\nmeter 4/4;\nscore{\npart p{\nvoice v{\nc5   1\n}\n}\n}\n}\n";
    let path = temp_file("diff.musa", messy)?;
    let output = musa(&["format", "--diff", &path.to_string_lossy()])?;
    // Like --check: a difference is a failure, so CI can gate on it.
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--- "), "stdout: {stdout}");
    assert!(stdout.contains("+++ "), "stdout: {stdout}");
    assert!(stdout.contains("@@"), "stdout: {stdout}");
    assert!(stdout.contains("-piece   \"M\"{"), "stdout: {stdout}");
    assert!(stdout.contains("+piece \"M\" {"), "stdout: {stdout}");
    // The promise that makes it a preview: the file is untouched.
    assert_eq!(std::fs::read_to_string(&path)?, messy);
    std::fs::remove_file(&path)
}

/// `musa analyze` prints the report the compiler built, in the order the
/// compiler built it, and says nothing about whether the piece is good.
///
/// The snapshot is the point: a report a reader cannot diff against yesterday's
/// is a report nobody can act on (`docs/rules/language/07-analysis.md` §4).
#[test]
fn analyze_prints_a_deterministic_report() -> std::io::Result<()> {
    let annotated = format!("{}/../../examples/annotated.musa", env!("CARGO_MANIFEST_DIR"));
    let output = musa(&["analyze", &annotated, "--kind", "facts"])?;
    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let again = musa(&["analyze", &annotated, "--kind", "facts"])?;
    assert_eq!(
        text,
        String::from_utf8_lossy(&again.stdout),
        "two runs printed different reports"
    );
    insta::assert_snapshot!("analyze_facts", text);
    Ok(())
}

/// The window and the scope both narrow the report, and JSON says the same
/// thing the text does.
#[test]
fn analyze_honours_scope_window_and_format() -> std::io::Result<()> {
    let annotated = format!("{}/../../examples/annotated.musa", env!("CARGO_MANIFEST_DIR"));
    let output = musa(&[
        "analyze", &annotated, "--kind", "facts", "--part", "piano", "--voice", "lead", "--from", "1", "--to", "2",
        "--format", "json",
    ])?;
    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    insta::assert_snapshot!("analyze_facts_window_json", String::from_utf8_lossy(&output.stdout));
    Ok(())
}

/// The tonal reading prints its candidates and the criteria they failed, and
/// prints the same bytes twice.
///
/// The fixture is the one whose header states the candidate set it should
/// produce (`examples/analysis/pivot-ambiguity.musa`), so the snapshot and the
/// fixture's own comment are two statements of one expectation and a change to
/// either has to answer the other.
#[test]
fn analyze_reads_a_modulation_as_two_candidates() -> std::io::Result<()> {
    let pivot = format!(
        "{}/../../examples/analysis/pivot-ambiguity.musa",
        env!("CARGO_MANIFEST_DIR")
    );
    let output = musa(&["analyze", &pivot, "--kind", "tonal", "--format", "text"])?;
    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let again = musa(&["analyze", &pivot, "--kind", "tonal", "--format", "text"])?;
    assert_eq!(
        text,
        String::from_utf8_lossy(&again.stdout),
        "two runs printed different readings"
    );
    insta::assert_snapshot!("analyze_tonal", text);
    Ok(())
}

/// A cadence reading names what it could not see, one line per criterion.
#[test]
fn analyze_says_which_cadence_evidence_is_missing() -> std::io::Result<()> {
    let cadences = format!(
        "{}/../../examples/analysis/cadence-evidence.musa",
        env!("CARGO_MANIFEST_DIR")
    );
    let output = musa(&["analyze", &cadences, "--kind", "cadences"])?;
    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        text.contains("but not: the tonic is in the top voice"),
        "the imperfect cadence did not say what it lacked: {text}"
    );
    insta::assert_snapshot!("analyze_cadences", text);
    Ok(())
}

/// A segmentation the analysis does not have is refused by name, and an
/// unreadable key is refused rather than guessed at.
#[test]
fn analyze_refuses_a_policy_it_does_not_have() -> std::io::Result<()> {
    let pivot = format!(
        "{}/../../examples/analysis/pivot-ambiguity.musa",
        env!("CARGO_MANIFEST_DIR")
    );
    let bad = musa(&["analyze", &pivot, "--kind", "chords", "--segmentation", "salami"])?;
    assert!(!bad.status.success());
    assert!(
        String::from_utf8_lossy(&bad.stderr).contains("is not a segmentation"),
        "an unknown segmentation was accepted"
    );
    let key = musa(&["analyze", &pivot, "--kind", "tonal", "--key", "h major"])?;
    assert!(!key.status.success());
    assert!(
        String::from_utf8_lossy(&key.stderr).contains("is not a key"),
        "an unreadable key was accepted"
    );
    Ok(())
}

/// A request the score cannot answer fails; a request with nothing in it does
/// not. An analysis is not a check.
#[test]
fn analyze_refuses_a_bad_request_and_accepts_an_empty_one() -> std::io::Result<()> {
    let annotated = format!("{}/../../examples/annotated.musa", env!("CARGO_MANIFEST_DIR"));
    let bad = musa(&["analyze", &annotated, "--kind", "facts", "--part", "harpsichord"])?;
    assert!(!bad.status.success());
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(stderr.contains("no part named `harpsichord`"), "stderr: {stderr}");
    assert!(
        stderr.contains("`piano`"),
        "the error did not say what there is: {stderr}"
    );

    let unknown = musa(&["analyze", &annotated, "--kind", "vibes"])?;
    assert!(!unknown.status.success());
    assert!(
        String::from_utf8_lossy(&unknown.stderr).contains("is not an analysis"),
        "an unadmitted kind was accepted"
    );

    // A kind that reads against a style says which style it wanted, because
    // "voice leading" alone does not name a tradition to read against.
    let unprofiled = musa(&["analyze", &annotated, "--kind", "voice-leading"])?;
    assert!(!unprofiled.status.success());
    assert!(
        String::from_utf8_lossy(&unprofiled.stderr).contains("reads against a style profile"),
        "a style reading was accepted without a style"
    );

    let empty = musa(&["analyze", &annotated, "--kind", "facts", "--from", "90", "--to", "99"])?;
    assert!(empty.status.success(), "an empty window was treated as a failure");
    assert!(
        String::from_utf8_lossy(&empty.stdout).contains("0 findings"),
        "an empty window reported something"
    );
    Ok(())
}

/// Logs never touch stdout, however loud they are asked to be.
///
/// `musa render -o -` writes a score to stdout for a pipe to read, and
/// `musa kernel` writes interchange text there. A single log line on that
/// stream is a corrupted file that nothing downstream can diagnose — so the
/// destination is stderr, with no flag to change it, and this is the law that
/// says so. `-vvv` and `MUSA_LOG=trace` are asked for together deliberately:
/// each is a separate way to raise the volume and both must miss stdout.
#[test]
fn logging_never_writes_to_the_stream_a_score_is_piped_on() -> std::io::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_musa"))
        .args(["-vvv", "render", &glass_mountain(), "--to", "lilypond", "-o", "-"])
        .env("MUSA_LOG", "trace")
        .output()?;
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with(r"\version"),
        "stdout must be LilyPond and nothing else"
    );
    for line in stdout.lines() {
        assert!(
            !line.contains("musa_compiler") && !line.contains("musa_project") && !line.contains("musa_render"),
            "a log line reached stdout: {line}"
        );
    }
    // The volume was genuinely turned up, so the law above is about a stream
    // that had something to lose rather than about a silent run.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("musa_compiler"), "nothing was logged at all: {stderr}");
    Ok(())
}
