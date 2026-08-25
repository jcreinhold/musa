//! Verified, content-addressed project assets.
//!
//! Musa source states *which* logical asset it means with an instrument's
//! `from "…"` clause. `musa.toml` states policy and attribution once, and the
//! generated `musa.lock` records the filesystem observations needed for a
//! reproducible build. This module is the filesystem authority between those
//! declarations and later audio adapters. It deliberately exposes facts, not
//! bytes or open files.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Read as _};
use std::path::{Path, PathBuf};

use musa_syntax::ast::{Document, PieceDecl};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::diagnostic::{Diagnostic, Label, Severity, Span};
use crate::error::ProjectError;
use crate::position::Lines;

/// No initial asset is allowed to make an ordinary verification consume more
/// than four gibibytes. Decoders may impose narrower prepared-audio budgets.
const MAX_ASSET_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// The physical format family an asset declaration promises.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssetKind {
    /// An SFZ text sample map. Its referenced samples are resolved by prompt
    /// 185; this prompt verifies the map itself.
    Sfz,
    /// A `SoundFont` bank (`.sf2` or `.sf3`).
    SoundFont,
    /// Recorded audio for a later clip or fixed-media declaration.
    Audio,
}

impl AssetKind {
    fn accepts(self, path: &Path) -> bool {
        let extension = path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .map(str::to_ascii_lowercase);
        match (self, extension.as_deref()) {
            (Self::Sfz, Some("sfz"))
            | (Self::SoundFont, Some("sf2" | "sf3"))
            | (Self::Audio, Some("wav" | "flac" | "aif" | "aiff")) => true,
            (Self::Sfz | Self::SoundFont | Self::Audio, _) => false,
        }
    }

    fn supports_instrument(self) -> bool {
        matches!(self, Self::Sfz | Self::SoundFont)
    }
}

impl std::fmt::Display for AssetKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Sfz => "sfz",
            Self::SoundFont => "sound-font",
            Self::Audio => "audio",
        })
    }
}

/// Whether a logical asset is ready to enter the immutable build closure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssetStatus {
    /// Manifest, lock, path, kind, size, and digest all agree.
    Verified,
    /// Source names a path absent from `[assets]`.
    Undeclared,
    /// The manifest entry has no matching `musa.lock` entry.
    Unlocked,
    /// The path is absolute, traverses upward, or otherwise is not canonical.
    PathEscape,
    /// Canonicalization followed a symlink outside the owning root.
    SymlinkEscape,
    /// The file is absent or is not a regular file.
    Missing,
    /// The declared kind disagrees with the extension or source use.
    KindMismatch,
    /// The declared or actual byte length exceeds a bound.
    SizeExceeded,
    /// Lock metadata disagrees with the manifest or file length.
    MetadataMismatch,
    /// The raw bytes do not have the locked SHA-256 digest.
    DigestMismatch,
}

impl AssetStatus {
    /// Whether this asset may enter audio preparation.
    pub const fn is_verified(self) -> bool {
        matches!(self, Self::Verified)
    }
}

impl std::fmt::Display for AssetStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Verified => "verified",
            Self::Undeclared => "undeclared",
            Self::Unlocked => "unlocked",
            Self::PathEscape => "path-escape",
            Self::SymlinkEscape => "symlink-escape",
            Self::Missing => "missing",
            Self::KindMismatch => "kind-mismatch",
            Self::SizeExceeded => "size-exceeded",
            Self::MetadataMismatch => "metadata-mismatch",
            Self::DigestMismatch => "digest-mismatch",
        })
    }
}

/// One resolved asset, safe to serialize to a UI or print from the CLI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetFact {
    /// Canonical project/package-relative logical path.
    pub path: String,
    /// Manifest-declared media kind, if there was a declaration.
    pub kind: Option<AssetKind>,
    /// Locked and verified SHA-256 identity, including its algorithm prefix.
    pub digest: Option<String>,
    /// Exact raw byte length when known.
    pub bytes: Option<u64>,
    /// Versioned adapter identity from the manifest/lock.
    pub adapter: Option<String>,
    /// Optional SPDX expression written by the author.
    pub license: Option<String>,
    /// Optional attribution/source text written by the author.
    pub source: Option<String>,
    /// Current verification result.
    pub status: AssetStatus,
    /// Document containing the source reference, when the asset is used.
    pub origin: Option<String>,
    /// The `"path"` token in the open source, when that is the origin.
    pub span: Option<Span>,
    /// Actionable detail for a non-verified status.
    pub detail: Option<String>,
}

/// A deterministic view of the project's asset closure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetInventory {
    facts: Vec<AssetFact>,
    diagnostics: Vec<Diagnostic>,
    identity: [u8; 32],
}

impl AssetInventory {
    fn manifest_error(error: &str) -> Self {
        Self::boundary_error(
            "asset-manifest",
            "project asset policy is invalid",
            error,
            "fix `[assets]` in musa.toml before locking or preparing audio",
        )
    }

    fn boundary_error(code: &str, message: &str, error: &str, note: &str) -> Self {
        let diagnostic = Diagnostic {
            severity: Severity::Error,
            code: code.to_owned(),
            message: message.to_owned(),
            labels: Vec::new(),
            help: Some(error.to_owned()),
            note: Some(note.to_owned()),
            fixes: Vec::new(),
            causes: Vec::new(),
            span: None,
        };
        Self {
            facts: Vec::new(),
            diagnostics: vec![diagnostic],
            identity: closure_identity(&[]),
        }
    }

    /// Assets in canonical logical-path order.
    pub fn facts(&self) -> &[AssetFact] {
        &self.facts
    }

    /// Problems that prevent one or more assets entering the closure.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Whether every declared or referenced asset is verified.
    pub fn is_verified(&self) -> bool {
        self.diagnostics.is_empty() && self.facts.iter().all(|fact| fact.status.is_verified())
    }

    pub(crate) const fn identity(&self) -> [u8; 32] {
        self.identity
    }
}

impl Default for AssetInventory {
    fn default() -> Self {
        Self {
            facts: Vec::new(),
            diagnostics: Vec::new(),
            identity: closure_identity(&[]),
        }
    }
}

/// Manifest policy for one logical asset.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AssetPolicy {
    pub(crate) kind: AssetKind,
    pub(crate) adapter: String,
    pub(crate) max_bytes: Option<u64>,
    pub(crate) license: Option<String>,
    pub(crate) source: Option<String>,
}

/// Verified package-owned asset metadata, never raw bytes or a cache path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PackageAsset {
    pub(crate) policy: AssetPolicy,
    pub(crate) digest: String,
    pub(crate) bytes: u64,
}

pub(crate) fn validate_policy(path: &str, policy: &AssetPolicy) -> Result<(), String> {
    let Some((adapter, version)) = policy.adapter.rsplit_once('@') else {
        return Err(format!(
            "asset `{path}` adapter must be a versioned identity such as `sfz@1`"
        ));
    };
    if adapter.is_empty() || version.is_empty() {
        return Err(format!(
            "asset `{path}` adapter must name both an adapter and a version"
        ));
    }
    if policy.max_bytes == Some(0) {
        return Err(format!("asset `{path}` max_bytes must be greater than zero"));
    }
    Ok(())
}

pub(crate) fn validate_logical_paths<'a>(paths: impl Iterator<Item = &'a str>) -> Result<(), String> {
    let mut folded = BTreeMap::<String, String>::new();
    for logical in paths {
        safe_relative(logical)?;
        let case_folded = logical.to_lowercase();
        if let Some(previous) = folded.insert(case_folded, logical.to_owned())
            && previous != logical
        {
            return Err(format!(
                "asset paths `{previous}` and `{logical}` collide on a case-insensitive filesystem"
            ));
        }
    }
    Ok(())
}

#[derive(Clone)]
struct AssetUse {
    path: String,
    document: String,
    span: Option<Span>,
    instrument: bool,
}

/// Inspect and verify the assets belonging to `path` without changing files.
///
/// `path` may name a piece or its project directory.
///
/// # Errors
/// Returns [`ProjectError::Io`] when the project manifest or lock exists but
/// cannot be read.
pub fn asset_inventory(path: impl AsRef<Path>) -> Result<AssetInventory, ProjectError> {
    let (root, source) = locate(path.as_ref())?;
    let policies = read_policies(&root)?;
    let lock = crate::lock::read(&root)?;
    let package_assets = if let Some(project) = crate::project::read(&root) {
        if let Some(error) = project.package_error {
            return Err(ProjectError::Packages(error));
        }
        let document = source
            .as_ref()
            .map_or("musa-assets:/project.musa", |(name, _)| name.as_str());
        crate::packages::offline(&root, &project.packages, document)?.assets
    } else {
        BTreeMap::new()
    };
    let uses = match source {
        Some((name, text)) => source_uses(&name, &text, true),
        None => project_uses(&root)?,
    };
    Ok(resolve(&root, &policies, lock.as_ref(), uses, None, &package_assets))
}

/// Write a deterministic `musa.lock` for all manifest-declared local assets.
///
/// Existing lock contents are replaced only after every asset has passed path,
/// kind, and size validation, so a failed lock operation leaves the previous
/// closure intact.
///
/// # Errors
/// Returns [`ProjectError::Io`] for filesystem failures and
/// [`ProjectError::Assets`] when any declaration cannot be locked safely.
pub fn lock_assets(path: impl AsRef<Path>) -> Result<AssetInventory, ProjectError> {
    let (root, source) = locate(path.as_ref())?;
    let policies = read_policies(&root)?;
    let uses = match source {
        Some((name, text)) => source_uses(&name, &text, true),
        None => project_uses(&root)?,
    };
    let assets = generate_lock(&root, &policies)?;
    let mut generated = crate::lock::read(&root)?.unwrap_or_default();
    generated.version = crate::lock::VERSION;
    generated.assets = assets;
    let inventory = resolve(&root, &policies, Some(&generated), uses, None, &BTreeMap::new());
    if !inventory.is_verified() {
        let detail = inventory
            .facts()
            .iter()
            .find_map(|fact| fact.detail.as_deref())
            .unwrap_or("the generated asset closure is not verified");
        return Err(ProjectError::Assets(detail.to_owned()));
    }
    crate::lock::write(&root, &generated)?;
    Ok(inventory)
}

pub(crate) fn session_inventory(
    path: Option<&Path>,
    project: Option<&crate::project::ProjectMeta>,
    name: &str,
    source: &str,
    imports: &musa_compiler::ImportSources,
    package_assets: &BTreeMap<String, PackageAsset>,
) -> AssetInventory {
    if let Some(error) = project.and_then(|meta| meta.asset_error.as_deref()) {
        return AssetInventory::manifest_error(error);
    }
    let root = project
        .map(|meta| meta.root.clone())
        .or_else(|| path.and_then(crate::project::manifest_root))
        .or_else(|| path.and_then(Path::parent).map(Path::to_path_buf));
    let Some(root) = root else {
        let uses = source_uses(name, source, true);
        return resolve(
            Path::new("."),
            &BTreeMap::new(),
            None,
            uses,
            Some(source),
            package_assets,
        );
    };
    let policies = if let Some(project) = project {
        project.assets.clone()
    } else if root.join("musa.toml").is_file() {
        match read_policies(&root) {
            Ok(policies) => policies,
            Err(error) => return AssetInventory::manifest_error(&error.to_string()),
        }
    } else {
        BTreeMap::new()
    };
    let lock = match crate::lock::read(&root) {
        Ok(lock) => lock,
        Err(error) => {
            return AssetInventory::boundary_error(
                "asset-lock",
                "project asset lock is invalid",
                &error.to_string(),
                "regenerate musa.lock explicitly with `musa assets lock`",
            );
        }
    };
    let mut uses = source_uses(name, source, true);
    uses.extend(
        imports
            .iter()
            .flat_map(|(document, text)| source_uses(document, text, false)),
    );
    resolve(&root, &policies, lock.as_ref(), uses, Some(source), package_assets)
}

fn locate(path: &Path) -> Result<(PathBuf, Option<(String, String)>), ProjectError> {
    if path.is_dir() {
        return Ok((path.to_path_buf(), None));
    }
    let source = std::fs::read_to_string(path).map_err(|error| ProjectError::io(path.display(), error))?;
    let root = crate::project::manifest_root(path)
        .or_else(|| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    Ok((root, Some((path.to_string_lossy().into_owned(), source))))
}

fn project_uses(root: &Path) -> Result<Vec<AssetUse>, ProjectError> {
    fn visit(root: &Path, directory: &Path, uses: &mut Vec<AssetUse>) -> Result<(), ProjectError> {
        let entries = std::fs::read_dir(directory).map_err(|error| ProjectError::io(directory.display(), error))?;
        for entry in entries {
            let entry = entry.map_err(|error| ProjectError::io(directory.display(), error))?;
            let kind = entry
                .file_type()
                .map_err(|error| ProjectError::io(entry.path().display(), error))?;
            if kind.is_symlink() {
                continue;
            }
            let path = entry.path();
            if kind.is_dir() {
                if path == root.join(".musa") {
                    continue;
                }
                visit(root, &path, uses)?;
            } else if kind.is_file() && path.extension().is_some_and(|extension| extension == "musa") {
                let source = std::fs::read_to_string(&path).map_err(|error| ProjectError::io(path.display(), error))?;
                let document = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().into_owned();
                uses.extend(source_uses(&document, &source, false));
            }
        }
        Ok(())
    }

    let mut uses = Vec::new();
    visit(root, root, &mut uses)?;
    Ok(uses)
}

fn read_policies(root: &Path) -> Result<BTreeMap<String, AssetPolicy>, ProjectError> {
    let path = root.join("musa.toml");
    let text = std::fs::read_to_string(&path).map_err(|error| ProjectError::io(path.display(), error))?;
    let file: crate::project::ProjectFile = toml::from_str(&text)
        .map_err(|error| ProjectError::Assets(format!("{} is not a valid manifest: {error}", path.display())))?;
    file.asset_policies()
        .map_err(|error| ProjectError::Assets(format!("{}: {error}", path.display())))
}

fn generate_lock(
    root: &Path,
    policies: &BTreeMap<String, AssetPolicy>,
) -> Result<BTreeMap<String, crate::lock::LockedAsset>, ProjectError> {
    let canonical_root = std::fs::canonicalize(root).map_err(|error| ProjectError::io(root.display(), error))?;
    let mut assets = BTreeMap::new();
    for (logical, policy) in policies {
        let relative = safe_relative(logical).map_err(ProjectError::Assets)?;
        if !policy.kind.accepts(&relative) {
            return Err(ProjectError::Assets(format!(
                "asset `{logical}` is declared {}, which does not match its extension",
                policy.kind
            )));
        }
        let path = root.join(&relative);
        let canonical = std::fs::canonicalize(&path).map_err(|error| ProjectError::io(path.display(), error))?;
        if !canonical.starts_with(&canonical_root) {
            return Err(ProjectError::Assets(format!(
                "asset `{logical}` follows a symlink outside the project"
            )));
        }
        let metadata = canonical
            .metadata()
            .map_err(|error| ProjectError::io(canonical.display(), error))?;
        if !metadata.is_file() {
            return Err(ProjectError::Assets(format!("asset `{logical}` is not a regular file")));
        }
        let limit = policy.max_bytes.unwrap_or(MAX_ASSET_BYTES).min(MAX_ASSET_BYTES);
        if metadata.len() > limit {
            return Err(ProjectError::Assets(format!(
                "asset `{logical}` is {} bytes, above its {limit}-byte limit",
                metadata.len()
            )));
        }
        let digest = digest_file(&canonical)?;
        assets.insert(
            logical.clone(),
            crate::lock::LockedAsset {
                kind: policy.kind,
                adapter: policy.adapter.clone(),
                digest,
                bytes: metadata.len(),
            },
        );
    }
    Ok(assets)
}

fn resolve(
    root: &Path,
    policies: &BTreeMap<String, AssetPolicy>,
    lock: Option<&crate::lock::LockFile>,
    uses: Vec<AssetUse>,
    open_source: Option<&str>,
    package_assets: &BTreeMap<String, PackageAsset>,
) -> AssetInventory {
    let (package_uses, uses): (Vec<_>, Vec<_>) = uses.into_iter().partition(|used| used.path.starts_with("pkg:"));
    let mut by_path: BTreeMap<String, Vec<AssetUse>> = BTreeMap::new();
    for used in uses {
        by_path.entry(used.path.clone()).or_default().push(used);
    }
    for path in policies.keys() {
        by_path.entry(path.clone()).or_default();
    }
    if let Some(lock) = lock {
        for path in lock.assets.keys() {
            by_path.entry(path.clone()).or_default();
        }
    }
    let canonical_root = std::fs::canonicalize(root).ok();
    let mut facts = Vec::with_capacity(by_path.len());
    let mut diagnostics = Vec::new();
    for (logical, origins) in by_path {
        let policy = policies.get(&logical);
        let locked = lock.and_then(|file| file.assets.get(&logical));
        let primary = origins.first();
        let (status, detail, bytes, digest) =
            verify_one(root, canonical_root.as_deref(), &logical, policy, locked, &origins);
        let fact = AssetFact {
            path: logical.clone(),
            kind: policy.map(|entry| entry.kind),
            digest: (status == AssetStatus::Verified).then_some(digest).flatten(),
            bytes,
            adapter: policy.map(|entry| entry.adapter.clone()),
            license: policy.and_then(|entry| entry.license.clone()),
            source: policy.and_then(|entry| entry.source.clone()),
            status,
            origin: primary.map(|origin| origin.document.clone()),
            span: primary.and_then(|origin| origin.span),
            detail,
        };
        if !status.is_verified() {
            diagnostics.push(diagnostic(&fact, open_source));
        }
        facts.push(fact);
    }
    let mut package_origins: BTreeMap<String, Vec<AssetUse>> = BTreeMap::new();
    for used in package_uses {
        package_origins.entry(used.path.clone()).or_default().push(used);
    }
    for logical in package_assets.keys() {
        package_origins.entry(logical.clone()).or_default();
    }
    for (logical, origins) in package_origins {
        let primary = origins.first();
        let (status, detail, package) = match package_assets.get(&logical) {
            Some(package)
                if package.policy.kind.accepts(Path::new(&logical))
                    && (!origins.iter().any(|origin| origin.instrument)
                        || package.policy.kind.supports_instrument()) =>
            {
                (AssetStatus::Verified, None, Some(package))
            }
            Some(package) => (
                AssetStatus::KindMismatch,
                Some(format!(
                    "`{logical}` does not match declared kind {} and its source use",
                    package.policy.kind
                )),
                Some(package),
            ),
            None => (
                AssetStatus::Undeclared,
                Some(format!(
                    "declare `{logical}` in the selected package's `[assets]` table and run `musa fetch`"
                )),
                None,
            ),
        };
        let fact = AssetFact {
            path: logical,
            kind: package.map(|asset| asset.policy.kind),
            digest: status
                .is_verified()
                .then(|| package.map(|asset| asset.digest.clone()))
                .flatten(),
            bytes: package.map(|asset| asset.bytes),
            adapter: package.map(|asset| asset.policy.adapter.clone()),
            license: package.and_then(|asset| asset.policy.license.clone()),
            source: package.and_then(|asset| asset.policy.source.clone()),
            status,
            origin: primary.map(|origin| origin.document.clone()),
            span: primary.and_then(|origin| origin.span),
            detail,
        };
        if !status.is_verified() {
            diagnostics.push(diagnostic(&fact, open_source));
        }
        facts.push(fact);
    }
    facts.sort_by(|left, right| left.path.cmp(&right.path));
    let identity = closure_identity(&facts);
    AssetInventory {
        facts,
        diagnostics,
        identity,
    }
}

fn verify_one(
    root: &Path,
    canonical_root: Option<&Path>,
    logical: &str,
    policy: Option<&AssetPolicy>,
    locked: Option<&crate::lock::LockedAsset>,
    origins: &[AssetUse],
) -> (AssetStatus, Option<String>, Option<u64>, Option<String>) {
    let Some(policy) = policy else {
        let detail = if locked.is_some() {
            format!("remove stale `{logical}` from musa.lock by running `musa assets lock`")
        } else {
            format!("add `[assets.\"{logical}\"]` to musa.toml")
        };
        return (AssetStatus::Undeclared, Some(detail), None, None);
    };
    let relative = match safe_relative(logical) {
        Ok(path) => path,
        Err(detail) => return (AssetStatus::PathEscape, Some(detail), None, None),
    };
    if !policy.kind.accepts(&relative)
        || (origins.iter().any(|origin| origin.instrument) && !policy.kind.supports_instrument())
    {
        return (
            AssetStatus::KindMismatch,
            Some(format!(
                "`{logical}` does not match declared kind {} and its source use",
                policy.kind
            )),
            None,
            None,
        );
    }
    let path = root.join(&relative);
    let canonical = match std::fs::canonicalize(&path) {
        Ok(path) => path,
        Err(error) => {
            return (
                AssetStatus::Missing,
                Some(format!("{}: {error}", path.display())),
                None,
                None,
            );
        }
    };
    if canonical_root.is_some_and(|owner| !canonical.starts_with(owner)) {
        return (
            AssetStatus::SymlinkEscape,
            Some(format!("`{logical}` resolves outside {}", root.display())),
            None,
            None,
        );
    }
    let metadata = match canonical.metadata() {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => {
            return (
                AssetStatus::Missing,
                Some("the resolved object is not a regular file".to_owned()),
                None,
                None,
            );
        }
        Err(error) => return (AssetStatus::Missing, Some(error.to_string()), None, None),
    };
    let limit = policy.max_bytes.unwrap_or(MAX_ASSET_BYTES).min(MAX_ASSET_BYTES);
    if policy.max_bytes.is_some_and(|declared| declared > MAX_ASSET_BYTES) || metadata.len() > limit {
        return (
            AssetStatus::SizeExceeded,
            Some(format!("{} bytes exceeds the {limit}-byte limit", metadata.len())),
            Some(metadata.len()),
            None,
        );
    }
    let Some(locked) = locked else {
        return (
            AssetStatus::Unlocked,
            Some("run `musa assets lock` explicitly".to_owned()),
            Some(metadata.len()),
            None,
        );
    };
    if locked.kind != policy.kind || locked.adapter != policy.adapter || locked.bytes != metadata.len() {
        return (
            AssetStatus::MetadataMismatch,
            Some("manifest, lock, and file metadata do not agree".to_owned()),
            Some(metadata.len()),
            None,
        );
    }
    if !valid_digest(&locked.digest) {
        return (
            AssetStatus::MetadataMismatch,
            Some("the lock digest is not `sha256:` followed by 64 hexadecimal digits".to_owned()),
            Some(metadata.len()),
            None,
        );
    }
    match digest_file(&canonical) {
        Ok(actual) if actual == locked.digest => (AssetStatus::Verified, None, Some(metadata.len()), Some(actual)),
        Ok(actual) => (
            AssetStatus::DigestMismatch,
            Some(format!("locked {}, found {actual}", locked.digest)),
            Some(metadata.len()),
            Some(actual),
        ),
        Err(error) => (
            AssetStatus::Missing,
            Some(error.to_string()),
            Some(metadata.len()),
            None,
        ),
    }
}

fn safe_relative(logical: &str) -> Result<PathBuf, String> {
    if logical.is_empty() || logical.starts_with('/') || logical.contains('\\') || logical.contains(':') {
        return Err(format!("asset path `{logical}` must be project-relative"));
    }
    let mut relative = PathBuf::new();
    for segment in logical.split('/') {
        match segment {
            ".." => return Err(format!("asset path `{logical}` escapes its owning project")),
            "" | "." => return Err(format!("asset path `{logical}` is not in canonical relative form")),
            segment => relative.push(segment),
        }
    }
    Ok(relative)
}

fn digest_file(path: &Path) -> Result<String, ProjectError> {
    let file = File::open(path).map_err(|error| ProjectError::io(path.display(), error))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| ProjectError::io(path.display(), error))?;
        if count == 0 {
            break;
        }
        hasher.update(buffer.get(..count).unwrap_or_default());
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn valid_digest(digest: &str) -> bool {
    digest
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn source_uses(document: &str, source: &str, open: bool) -> Vec<AssetUse> {
    let parsed = musa_syntax::parse(source);
    let root = parsed.syntax();
    let document_instruments = Document::of_root(&root)
        .into_iter()
        .flat_map(|document| document.instruments());
    let piece_instruments = PieceDecl::from_root(&root)
        .into_iter()
        .flat_map(|piece| piece.instruments());
    document_instruments
        .chain(piece_instruments)
        .filter_map(|instrument| {
            let path = instrument.asset()?;
            let span = open
                .then(|| {
                    instrument.asset_token().map(|token| {
                        let range = token.text_range();
                        Span {
                            start: u32::from(range.start()),
                            end: u32::from(range.end()),
                        }
                    })
                })
                .flatten();
            Some(AssetUse {
                path,
                document: document.to_owned(),
                span,
                instrument: true,
            })
        })
        .collect()
}

fn diagnostic(fact: &AssetFact, source: Option<&str>) -> Diagnostic {
    let label = fact.span.zip(source).map(|(span, source)| Label {
        span,
        at: Lines::new(source).at(span.start),
        text: "this asset reference cannot enter the locked build closure".to_owned(),
        primary: true,
    });
    Diagnostic {
        severity: Severity::Error,
        code: "asset".to_owned(),
        message: format!("asset `{}` is not verified", fact.path),
        labels: label.clone().into_iter().collect(),
        help: fact.detail.clone(),
        note: Some(
            "assets are immutable build inputs: path, kind, byte length, adapter, and SHA-256 lock must agree"
                .to_owned(),
        ),
        fixes: Vec::new(),
        causes: Vec::new(),
        span: label.map(|label| label.span),
    }
}

fn closure_identity(facts: &[AssetFact]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for fact in facts.iter().filter(|fact| fact.status.is_verified()) {
        for field in [
            fact.path.as_str(),
            fact.digest.as_deref().unwrap_or_default(),
            fact.adapter.as_deref().unwrap_or_default(),
        ] {
            hasher.update(u64::try_from(field.len()).unwrap_or(u64::MAX).to_be_bytes());
            hasher.update(field.as_bytes());
        }
        hasher.update(fact.bytes.unwrap_or_default().to_be_bytes());
    }
    hasher.finalize().into()
}
