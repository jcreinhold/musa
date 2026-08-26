//! Exact package fetch and cache laws (language candidate §09, prompt 183).

#![allow(clippy::expect_used)]

use std::path::Path;
use std::process::Command;

use musa_project::{ProjectError, ProjectSession, SfzLimits, fetch_packages, lock_assets, verify_packages};

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("fixture directory");
    }
    std::fs::write(path, contents).expect("fixture file");
}

fn write_bytes(path: &Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("fixture directory");
    }
    std::fs::write(path, contents).expect("fixture file");
}

fn wav() -> Vec<u8> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(
            &mut bytes,
            hound::WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .expect("WAV writer");
        for _ in 0..64 {
            writer.write_sample(0.25f32).expect("WAV sample");
        }
        writer.finalize().expect("WAV finish");
    }
    bytes.into_inner()
}

fn git(directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .expect("Git fixture command");
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("Git UTF-8").trim().to_owned()
}

fn remote() -> tempfile::TempDir {
    let remote = tempfile::tempdir().expect("remote");
    write(
        &remote.path().join("musa.toml"),
        r#"[package]
name = "orchestra"
language_version = 1

[build]
source = "src"

[assets."assets/tone.sfz"]
kind = "sfz"
adapter = "sfz@1"

[assets."assets/tone.wav"]
kind = "audio"
adapter = "wav@1"
"#,
    );
    write(&remote.path().join("src/lib.musa"), "mod strings;\n");
    write(
        &remote.path().join("src/strings.musa"),
        "fn doubled(n: Nat) -> Nat { n + n }\n",
    );
    write(&remote.path().join("assets/tone.sfz"), "<region> sample=tone.wav\n");
    write_bytes(&remote.path().join("assets/tone.wav"), &wav());
    git(remote.path(), &["init", "--quiet"]);
    git(remote.path(), &["config", "user.name", "Musa Test"]);
    git(remote.path(), &["config", "user.email", "musa@example.invalid"]);
    git(remote.path(), &["add", "."]);
    git(remote.path(), &["commit", "--quiet", "-m", "fixture"]);
    remote
}

fn project(root: &Path, remote: &Path, revision: &str) {
    write(
        &root.join("musa.toml"),
        &format!(
            r#"[project]
name = "Packages"

[packages.orchestra]
git = "{}"
rev = "sha1:{revision}"
"#,
            remote.display()
        ),
    );
}

#[test]
fn fetch_materializes_exact_blobs_and_writes_a_deterministic_lock() {
    let remote = remote();
    let revision = git(remote.path(), &["rev-parse", "HEAD"]);
    let first = tempfile::tempdir().expect("first project");
    let second = tempfile::tempdir().expect("second project");
    for root in [first.path(), second.path()] {
        project(root, remote.path(), &revision);
        assert_eq!(fetch_packages(root).expect("fetch"), 1);
    }
    let first_lock = std::fs::read(first.path().join("musa.lock")).expect("first lock");
    let second_lock = std::fs::read(second.path().join("musa.lock")).expect("second lock");
    assert_eq!(first_lock, second_lock, "project location is not package identity");
    let text = String::from_utf8(first_lock).expect("lock text");
    assert!(text.contains("version = 2"));
    assert!(text.contains(&format!("rev = \"sha1:{revision}\"")));
    assert!(text.contains("src/strings.musa"));
    assert!(!text.contains(first.path().to_string_lossy().as_ref()));
}

#[test]
fn a_corrupt_candidate_cache_cannot_substitute_for_the_locked_tree() {
    let remote = remote();
    let revision = git(remote.path(), &["rev-parse", "HEAD"]);
    let root = tempfile::tempdir().expect("project");
    project(root.path(), remote.path(), &revision);
    fetch_packages(root.path()).expect("first fetch");
    let cache_root = root.path().join(".musa/cache/packages");
    let cache = std::fs::read_dir(&cache_root)
        .expect("cache")
        .next()
        .expect("one cache entry")
        .expect("cache entry")
        .path();
    write(&cache.join("src/strings.musa"), "different bytes\n");
    assert!(matches!(
        fetch_packages(root.path()),
        Err(ProjectError::Packages(message)) if message.contains("complete file tables differ")
    ));
}

#[test]
fn a_locked_package_compiles_offline_and_its_definitions_are_read_only() {
    let remote = remote();
    let revision = git(remote.path(), &["rev-parse", "HEAD"]);
    let root = tempfile::tempdir().expect("project");
    project(root.path(), remote.path(), &revision);
    let piece = root.path().join("piece.musa");
    write(
        &piece,
        r#"piece "Offline" {
    import orchestra::strings;
    let four: Nat = doubled(2);
    score { part p { voice v { c4/1 } } }
}
"#,
    );
    fetch_packages(root.path()).expect("fetch");
    std::fs::rename(remote.path(), root.path().join("remote-unavailable")).expect("remote unavailable");

    let session = ProjectSession::open(&piece).expect("open offline");
    let snapshot = session.snapshot();
    assert!(snapshot.diagnostics().is_empty(), "{:?}", snapshot.diagnostics());
    let doubled = snapshot
        .items()
        .iter()
        .find(|item| item.name == "doubled")
        .expect("package definition");
    assert!(doubled.read_only);
    assert!(
        doubled
            .uri
            .as_deref()
            .is_some_and(|uri| uri.starts_with("musa-package:/"))
    );
}

#[test]
fn locked_verification_rejects_manifest_drift_without_fetching() {
    let remote = remote();
    let revision = git(remote.path(), &["rev-parse", "HEAD"]);
    let root = tempfile::tempdir().expect("project");
    project(root.path(), remote.path(), &revision);
    fetch_packages(root.path()).expect("fetch");
    assert_eq!(verify_packages(root.path()).expect("locked verification"), 1);
    let changed = "0".repeat(40);
    project(root.path(), remote.path(), &changed);
    assert!(matches!(
        verify_packages(root.path()),
        Err(ProjectError::Packages(message)) if message.contains("changed")
    ));
}

#[test]
fn package_asset_addresses_resolve_to_verified_locked_metadata() {
    let remote = remote();
    let revision = git(remote.path(), &["rev-parse", "HEAD"]);
    let root = tempfile::tempdir().expect("project");
    project(root.path(), remote.path(), &revision);
    let piece = root.path().join("piece.musa");
    write(
        &piece,
        r#"instrument tone from "pkg:orchestra/assets/tone.sfz" conforms note_instrument;

piece "Package asset" {
    score { part p { voice v { c4/1 } } }
}
"#,
    );
    fetch_packages(root.path()).expect("fetch");
    let session = ProjectSession::open(&piece).expect("open");
    let snapshot = session.snapshot();
    assert!(snapshot.diagnostics().is_empty(), "{:?}", snapshot.diagnostics());
    assert_eq!(snapshot.assets().len(), 2, "{:?}", snapshot.assets());
    let asset = snapshot
        .assets()
        .iter()
        .find(|asset| asset.path.ends_with("tone.sfz"))
        .expect("package SFZ asset");
    assert_eq!(asset.path, "pkg:orchestra/assets/tone.sfz");
    assert_eq!(asset.status.to_string(), "verified");
    assert!(
        asset
            .digest
            .as_deref()
            .is_some_and(|digest| digest.starts_with("sha256:"))
    );
    let (prepared, facts) = session
        .prepare_sfz_instrument(
            "tone",
            SfzLimits {
                max_file_bytes: 4096,
                max_regions: 8,
                max_opcodes: 32,
                max_value_bytes: 256,
            },
            musa_dsp::SamplerLimits {
                sample_rate: 48_000,
                max_voices: 64,
                max_regions: 8,
                max_decoded_bytes: 1 << 20,
                max_selection_work: 8,
                max_step_work: 64 * 48,
            },
        )
        .expect("package SFZ and its package sample prepare offline");
    assert_eq!(facts.regions, 1);
    assert_eq!(prepared.resources().decoded_pcm_bytes, 64 * size_of::<f32>());
}

#[test]
fn asset_locking_and_package_fetch_preserve_one_complete_lock() {
    let remote = remote();
    let revision = git(remote.path(), &["rev-parse", "HEAD"]);
    let root = tempfile::tempdir().expect("project");
    project(root.path(), remote.path(), &revision);
    let manifest_path = root.path().join("musa.toml");
    let mut manifest = std::fs::read_to_string(&manifest_path).expect("manifest");
    manifest.push_str(
        r#"
[assets."assets/local.sfz"]
kind = "sfz"
adapter = "sfz@1"
"#,
    );
    write(&manifest_path, &manifest);
    write(&root.path().join("assets/local.sfz"), "<region> sample=local.wav\n");

    lock_assets(root.path()).expect("asset lock");
    fetch_packages(root.path()).expect("package fetch");
    let both = std::fs::read_to_string(root.path().join("musa.lock")).expect("combined lock");
    assert!(both.contains("assets/local.sfz"));
    assert!(both.contains("package_roots"));
    lock_assets(root.path()).expect("asset relock");
    let relocked = std::fs::read_to_string(root.path().join("musa.lock")).expect("relocked");
    assert!(relocked.contains("assets/local.sfz"));
    assert!(relocked.contains("package_roots"));
}

#[test]
fn undeclared_module_files_and_malformed_exact_pins_are_refused() {
    let remote = remote();
    write(&remote.path().join("src/orphan.musa"), "let orphan: Nat = 1;\n");
    git(remote.path(), &["add", "."]);
    git(remote.path(), &["commit", "--quiet", "-m", "orphan"]);
    let revision = git(remote.path(), &["rev-parse", "HEAD"]);
    let root = tempfile::tempdir().expect("project");
    project(root.path(), remote.path(), &revision);
    assert!(matches!(
        fetch_packages(root.path()),
        Err(ProjectError::Packages(message)) if message.contains("module tree")
    ));

    project(root.path(), remote.path(), "branch-main");
    assert!(matches!(
        fetch_packages(root.path()),
        Err(ProjectError::Packages(message)) if message.contains("lowercase hexadecimal digits")
    ));
}

#[test]
fn sha256_git_repositories_use_the_pins_declared_object_format() {
    let remote = tempfile::tempdir().expect("SHA-256 remote");
    write(
        &remote.path().join("musa.toml"),
        "[package]\nname = \"sha_tools\"\nlanguage_version = 1\n\n[build]\nsource = \"src\"\n",
    );
    write(&remote.path().join("src/lib.musa"), "mod math;\n");
    write(&remote.path().join("src/math.musa"), "let answer: Nat = 4;\n");
    git(remote.path(), &["init", "--quiet", "--object-format=sha256"]);
    git(remote.path(), &["config", "user.name", "Musa Test"]);
    git(remote.path(), &["config", "user.email", "musa@example.invalid"]);
    git(remote.path(), &["add", "."]);
    git(remote.path(), &["commit", "--quiet", "-m", "SHA-256 fixture"]);
    let revision = git(remote.path(), &["rev-parse", "HEAD"]);
    assert_eq!(revision.len(), 64);

    let root = tempfile::tempdir().expect("project");
    write(
        &root.path().join("musa.toml"),
        &format!(
            "[project]\nname = \"SHA-256\"\n\n[packages.tools]\ngit = \"{}\"\nrev = \"sha256:{revision}\"\n",
            remote.path().display()
        ),
    );
    assert_eq!(fetch_packages(root.path()).expect("SHA-256 fetch"), 1);
    assert_eq!(verify_packages(root.path()).expect("SHA-256 verify"), 1);
}

#[test]
fn one_root_graph_refuses_two_exact_identities_for_one_package_name() {
    let first = remote();
    let first_rev = git(first.path(), &["rev-parse", "HEAD"]);
    let second = remote();
    write(
        &second.path().join("src/strings.musa"),
        "fn doubled(n: Nat) -> Nat { n + n + 0 }\n",
    );
    git(second.path(), &["add", "."]);
    git(second.path(), &["commit", "--quiet", "-m", "other exact identity"]);
    let second_rev = git(second.path(), &["rev-parse", "HEAD"]);

    let parent = tempfile::tempdir().expect("parent remote");
    write(
        &parent.path().join("musa.toml"),
        &format!(
            r#"[package]
name = "parent"
language_version = 1

[build]
source = "src"

[packages.left]
git = "{}"
rev = "sha1:{first_rev}"

[packages.right]
git = "{}"
rev = "sha1:{second_rev}"
"#,
            first.path().display(),
            second.path().display()
        ),
    );
    write(&parent.path().join("src/lib.musa"), "mod api;\n");
    write(&parent.path().join("src/api.musa"), "let answer: Nat = 4;\n");
    git(parent.path(), &["init", "--quiet"]);
    git(parent.path(), &["config", "user.name", "Musa Test"]);
    git(parent.path(), &["config", "user.email", "musa@example.invalid"]);
    git(parent.path(), &["add", "."]);
    git(parent.path(), &["commit", "--quiet", "-m", "parent"]);
    let parent_rev = git(parent.path(), &["rev-parse", "HEAD"]);

    let root = tempfile::tempdir().expect("project");
    project(root.path(), parent.path(), &parent_rev);
    assert!(matches!(
        fetch_packages(root.path()),
        Err(ProjectError::Packages(message)) if message.contains("incompatible exact identities")
    ));
}

#[test]
fn transitive_package_aliases_resolve_inside_the_declaring_node() {
    let shared = tempfile::tempdir().expect("shared remote");
    write(
        &shared.path().join("musa.toml"),
        r#"[package]
name = "shared"
language_version = 1

[build]
source = "src"
"#,
    );
    write(&shared.path().join("src/lib.musa"), "mod math;\n");
    write(
        &shared.path().join("src/math.musa"),
        "fn twice(n: Nat) -> Nat { n + n }\n",
    );
    git(shared.path(), &["init", "--quiet"]);
    git(shared.path(), &["config", "user.name", "Musa Test"]);
    git(shared.path(), &["config", "user.email", "musa@example.invalid"]);
    git(shared.path(), &["add", "."]);
    git(shared.path(), &["commit", "--quiet", "-m", "shared"]);
    let shared_rev = git(shared.path(), &["rev-parse", "HEAD"]);

    let parent = tempfile::tempdir().expect("parent remote");
    write(
        &parent.path().join("musa.toml"),
        &format!(
            r#"[package]
name = "orchestra"
language_version = 1

[build]
source = "src"

[packages.shared]
git = "{}"
rev = "sha1:{shared_rev}"
"#,
            shared.path().display()
        ),
    );
    write(&parent.path().join("src/lib.musa"), "mod strings;\n");
    write(
        &parent.path().join("src/strings.musa"),
        "import shared::math;\nfn doubled(n: Nat) -> Nat { twice(n) }\n",
    );
    git(parent.path(), &["init", "--quiet"]);
    git(parent.path(), &["config", "user.name", "Musa Test"]);
    git(parent.path(), &["config", "user.email", "musa@example.invalid"]);
    git(parent.path(), &["add", "."]);
    git(parent.path(), &["commit", "--quiet", "-m", "parent"]);
    let parent_rev = git(parent.path(), &["rev-parse", "HEAD"]);

    let root = tempfile::tempdir().expect("project");
    project(root.path(), parent.path(), &parent_rev);
    let piece = root.path().join("piece.musa");
    write(
        &piece,
        r#"piece "Transitive" {
    import orchestra::strings;
    let four: Nat = doubled(2);
    score { part p { voice v { c4/1 } } }
}
"#,
    );
    assert_eq!(fetch_packages(root.path()).expect("fetch graph"), 2);
    let session = ProjectSession::open(&piece).expect("open");
    assert!(
        session.snapshot().diagnostics().is_empty(),
        "{:?}",
        session.snapshot().diagnostics()
    );

    // A hostile lock edit cannot manufacture a cycle: the node descriptor
    // frames its outgoing map, so the edit is rejected before compilation.
    let lock_path = root.path().join("musa.lock");
    let mut lock: toml::Value = toml::from_str(&std::fs::read_to_string(&lock_path).expect("lock")).expect("lock TOML");
    let parent_node = lock
        .get("package_roots")
        .and_then(toml::Value::as_table)
        .and_then(|roots| roots.get("orchestra"))
        .and_then(toml::Value::as_str)
        .expect("parent node")
        .to_owned();
    let packages = lock
        .get_mut("packages")
        .and_then(toml::Value::as_array_mut)
        .expect("packages");
    let parent = packages
        .iter_mut()
        .find(|package| package.get("name").and_then(toml::Value::as_str) == Some("orchestra"))
        .expect("parent package");
    parent
        .get_mut("dependencies")
        .and_then(toml::Value::as_table_mut)
        .expect("dependencies")
        .insert("shared".to_owned(), toml::Value::String(parent_node));
    write(&lock_path, &toml::to_string_pretty(&lock).expect("lock text"));
    assert!(matches!(verify_packages(root.path()), Err(ProjectError::Packages(_))));
}
