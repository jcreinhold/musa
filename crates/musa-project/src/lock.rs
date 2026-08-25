//! One deterministic lockfile for the complete external-input closure.
//!
//! Assets and fetched packages are independent producers of lock facts, but
//! they are not independent files. This owner prevents either command from
//! rewriting `musa.lock` from a partial view and erasing the other half.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{AssetKind, ProjectError};

pub(crate) const VERSION: u32 = 2;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct LockedAsset {
    pub(crate) kind: AssetKind,
    pub(crate) adapter: String,
    pub(crate) digest: String,
    pub(crate) bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct LockedPackageFile {
    pub(crate) path: String,
    pub(crate) bytes: u64,
    pub(crate) digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct LockedPackage {
    pub(crate) node: String,
    pub(crate) name: String,
    pub(crate) git: String,
    pub(crate) rev: String,
    pub(crate) source: String,
    pub(crate) tree: String,
    #[serde(default)]
    pub(crate) dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) files: Vec<LockedPackageFile>,
}

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct LockFile {
    pub(crate) version: u32,
    #[serde(default)]
    pub(crate) assets: BTreeMap<String, LockedAsset>,
    #[serde(default)]
    pub(crate) package_roots: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) packages: Vec<LockedPackage>,
}

pub(crate) fn read(root: &Path) -> Result<Option<LockFile>, ProjectError> {
    let path = root.join("musa.lock");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(ProjectError::io(path.display(), error)),
    };
    let lock: LockFile = toml::from_str(&text)
        .map_err(|error| ProjectError::Assets(format!("{} is not a valid lock: {error}", path.display())))?;
    if !matches!(lock.version, 1 | VERSION) {
        return Err(ProjectError::Assets(format!(
            "{} uses closure lock version {}, but this build reads 1 and {VERSION}",
            path.display(),
            lock.version
        )));
    }
    Ok(Some(lock))
}

pub(crate) fn write(root: &Path, lock: &LockFile) -> Result<(), ProjectError> {
    let encoded = toml::to_string_pretty(lock).map_err(|error| ProjectError::Assets(error.to_string()))?;
    let destination = root.join("musa.lock");
    let mut temporary =
        tempfile::NamedTempFile::new_in(root).map_err(|error| ProjectError::io(root.display(), error))?;
    temporary
        .write_all(encoded.as_bytes())
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| ProjectError::io(destination.display(), error))?;
    temporary
        .persist(&destination)
        .map_err(|error| ProjectError::io(destination.display(), error.error))?;
    Ok(())
}
