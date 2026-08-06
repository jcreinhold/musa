//! CLI behavior tests: exit codes and user-visible output of `musa format`
//! and `musa check`.

use std::process::Command;

fn glass_mountain() -> String {
    format!("{}/../../examples/glass-mountain.musa", env!("CARGO_MANIFEST_DIR"))
}

fn musa(args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_musa-cli")).args(args).output()
}

fn temp_file(name: &str, contents: &str) -> std::io::Result<std::path::PathBuf> {
    let path = std::env::temp_dir().join(format!("musa-cli-test-{}-{name}", std::process::id()));
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
    assert!(stderr.contains("expected `;`"), "stderr: {stderr}");
    std::fs::remove_file(&path)
}

#[test]
fn format_check_passes_on_canonical_examples() -> std::io::Result<()> {
    let output = musa(&["format", "--check", &glass_mountain()])?;
    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    Ok(())
}

#[test]
fn format_rewrites_a_messy_file_to_canonical_form() -> std::io::Result<()> {
    let messy = "piece   \"M\"{\nmeter 4/4;\nscore{\npart p{\nvoice v{\nc5   1;\n}\n}\n}\n}\n";
    let path = temp_file("messy.musa", messy)?;
    let output = musa(&["format", &path.to_string_lossy()])?;
    assert!(output.status.success());
    let rewritten = std::fs::read_to_string(&path)?;
    assert_eq!(
        rewritten,
        "piece \"M\" {\n    meter 4/4;\n    score {\n        part p {\n            voice v {\n                c5 1;\n            }\n        }\n    }\n}\n"
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
