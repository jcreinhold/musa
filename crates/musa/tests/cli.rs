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
    let examples = format!("{}/../../examples", env!("CARGO_MANIFEST_DIR"));
    let mut count = 0usize;
    for entry in std::fs::read_dir(&examples)? {
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

// --- WAV export (prompt 17) -----------------------------------------------------

/// Every shipped example renders to WAV deterministically (same source →
/// byte-identical bytes, §17.5) with the expected header.
#[test]
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
