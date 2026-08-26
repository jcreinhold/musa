//! Exact-pinned source packages and their disposable verified cache.
//!
//! Package declarations are project policy; fetched Git objects and cache
//! paths are host facts. Compilation receives only verified source text under
//! stable read-only URIs and never receives a network or Git capability.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Deserialize;
use sha2::{Digest as _, Sha256};

use crate::error::ProjectError;
use crate::lock::{LockedPackage, LockedPackageFile};

const MAX_FILES: usize = 10_000;
const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
const MAX_PACKAGE_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_GRAPH_NODES: usize = 256;
const MAX_GRAPH_DEPTH: usize = 64;

/// One exact package edge from a project or another package manifest.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageRequirement {
    pub(crate) git: String,
    pub(crate) rev: String,
}

pub(crate) fn validate_requirement(alias: &str, requirement: &PackageRequirement) -> Result<(), String> {
    validate_git(&requirement.git).map_err(|detail| format!("package `{alias}`: {detail}"))?;
    validate_revision(&requirement.rev).map_err(|detail| format!("package `{alias}`: {detail}"))
}

/// Fetch the exact package graph declared by the project containing `path`.
///
/// This is the only package operation permitted to contact a Git remote.
///
/// # Errors
/// Returns [`ProjectError::Packages`] for an invalid graph or remote and
/// [`ProjectError::Io`] for local manifest/cache failures.
pub fn fetch_packages(path: impl AsRef<Path>) -> Result<usize, ProjectError> {
    let path = path.as_ref();
    let root = project_root(path)?;
    let manifest_path = root.join("musa.toml");
    let text =
        std::fs::read_to_string(&manifest_path).map_err(|error| ProjectError::io(manifest_path.display(), error))?;
    let manifest: crate::project::ProjectFile = toml::from_str(&text).map_err(|error| {
        ProjectError::Packages(format!("{} is not a valid manifest: {error}", manifest_path.display()))
    })?;
    let requirements = manifest.package_requirements().map_err(ProjectError::Packages)?;
    fetch(&root, &requirements)
}

/// Verify that the manifest, lock graph, and complete cached byte closure agree.
///
/// This is the `musa fetch --locked` operation. It performs no network access
/// and writes nothing.
///
/// # Errors
/// Returns [`ProjectError::Packages`] when the exact closure is absent or has
/// drifted, and [`ProjectError::Io`] for unreadable local inputs.
pub fn verify_packages(path: impl AsRef<Path>) -> Result<usize, ProjectError> {
    let path = path.as_ref();
    let root = project_root(path)?;
    let manifest_path = root.join("musa.toml");
    let text =
        std::fs::read_to_string(&manifest_path).map_err(|error| ProjectError::io(manifest_path.display(), error))?;
    let manifest: crate::project::ProjectFile = toml::from_str(&text).map_err(|error| {
        ProjectError::Packages(format!("{} is not a valid manifest: {error}", manifest_path.display()))
    })?;
    let requirements = manifest.package_requirements().map_err(ProjectError::Packages)?;
    offline(&root, &requirements, "musa-locked:/root.musa")?;
    Ok(requirements.len())
}

fn project_root(path: &Path) -> Result<PathBuf, ProjectError> {
    if path.is_dir() {
        if path.join("musa.toml").is_file() {
            return Ok(path.to_path_buf());
        }
    } else if let Some(root) = crate::project::manifest_root(path) {
        return Ok(root);
    }
    Err(ProjectError::Packages(format!(
        "{} is not inside a project with musa.toml",
        path.display()
    )))
}

fn validate_git(git: &str) -> Result<(), String> {
    if let Some(rest) = git.strip_prefix("https://") {
        let authority = rest.split('/').next().unwrap_or_default();
        if authority.is_empty() || authority.contains('@') {
            return Err("Git URL must be public HTTPS without embedded credentials".to_owned());
        }
        return Ok(());
    }
    if git.starts_with("file://") || Path::new(git).is_absolute() {
        return Ok(());
    }
    Err("Git location must be public HTTPS or an explicit absolute local path".to_owned())
}

fn validate_revision(revision: &str) -> Result<(), String> {
    let (algorithm, hexadecimal) = revision
        .split_once(':')
        .ok_or_else(|| "revision must name its object algorithm (`sha1:` or `sha256:`)".to_owned())?;
    let expected = match algorithm {
        "sha1" => 40,
        "sha256" => 64,
        _ => return Err(format!("unsupported Git object algorithm `{algorithm}`")),
    };
    if hexadecimal.len() != expected
        || !hexadecimal
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(format!(
            "{algorithm} revision must contain exactly {expected} lowercase hexadecimal digits"
        ));
    }
    Ok(())
}

#[derive(Deserialize)]
struct PackageManifest {
    package: PackageSection,
    build: BuildSection,
    #[serde(default)]
    packages: Option<toml::Value>,
    #[serde(default)]
    assets: Option<toml::Value>,
}

#[derive(Deserialize)]
struct PackageSection {
    name: String,
    language_version: u32,
}

#[derive(Deserialize)]
struct BuildSection {
    source: String,
}

impl PackageManifest {
    fn requirements(&self) -> Result<BTreeMap<String, PackageRequirement>, ProjectError> {
        let Some(toml::Value::Table(packages)) = self.packages.as_ref() else {
            return if self.packages.is_none() {
                Ok(BTreeMap::new())
            } else {
                Err(ProjectError::Packages(
                    "a package's `[packages]` value must be a table".to_owned(),
                ))
            };
        };
        packages
            .iter()
            .map(|(alias, value)| {
                if !valid_alias(alias) {
                    return Err(ProjectError::Packages(format!(
                        "package alias `{alias}` is not a Musa identifier"
                    )));
                }
                let requirement: PackageRequirement = value.clone().try_into().map_err(|error| {
                    ProjectError::Packages(format!("package `{alias}` has invalid requirement: {error}"))
                })?;
                validate_requirement(alias, &requirement).map_err(ProjectError::Packages)?;
                Ok((alias.clone(), requirement))
            })
            .collect()
    }

    fn asset_policies(&self) -> Result<BTreeMap<String, crate::assets::AssetPolicy>, ProjectError> {
        let Some(toml::Value::Table(assets)) = self.assets.as_ref() else {
            return if self.assets.is_none() {
                Ok(BTreeMap::new())
            } else {
                Err(ProjectError::Packages(
                    "a package's `[assets]` value must be a table".to_owned(),
                ))
            };
        };
        let policies: BTreeMap<String, crate::assets::AssetPolicy> = assets
            .iter()
            .map(|(path, value)| {
                let policy = value.clone().try_into().map_err(|error| {
                    ProjectError::Packages(format!("package asset `{path}` has invalid policy: {error}"))
                })?;
                crate::assets::validate_policy(path, &policy).map_err(ProjectError::Packages)?;
                Ok((path.clone(), policy))
            })
            .collect::<Result<_, _>>()?;
        crate::assets::validate_logical_paths(policies.keys().map(String::as_str)).map_err(ProjectError::Packages)?;
        Ok(policies)
    }
}

fn valid_alias(alias: &str) -> bool {
    let mut bytes = alias.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

struct Materialized {
    cache: PathBuf,
    manifest: PackageManifest,
    files: Vec<LockedPackageFile>,
    tree: String,
}

/// The exact read-only source closure admitted by a verified lock and cache.
pub(crate) struct PackageClosure {
    pub(crate) sources: musa_compiler::ImportSources,
    pub(crate) assets: BTreeMap<String, crate::assets::PackageAsset>,
}

/// Reconstruct package source inputs without contacting the network.
///
/// Every cache byte and every lock graph edge is checked before any source is
/// handed to the compiler. `document` is the open root document whose package
/// aliases come from the project manifest.
pub(crate) fn offline(
    root: &Path,
    requirements: &BTreeMap<String, PackageRequirement>,
    document: &str,
) -> Result<PackageClosure, ProjectError> {
    if requirements.is_empty() {
        return Ok(PackageClosure {
            sources: musa_compiler::ImportSources::default(),
            assets: BTreeMap::new(),
        });
    }
    let lock = crate::lock::read(root)?.ok_or_else(|| {
        ProjectError::Packages(format!(
            "{} declares packages but has no musa.lock; run `musa fetch {}`",
            root.join("musa.toml").display(),
            root.display()
        ))
    })?;
    if lock.version != crate::lock::VERSION {
        return Err(ProjectError::Packages(format!(
            "package closure needs musa.lock version {}; run `musa fetch {}`",
            crate::lock::VERSION,
            root.display()
        )));
    }
    if lock.package_roots.keys().ne(requirements.keys()) {
        return Err(ProjectError::Packages(format!(
            "package aliases in musa.toml and musa.lock differ; run `musa fetch {}`",
            root.display()
        )));
    }
    let nodes: BTreeMap<&str, &LockedPackage> = lock
        .packages
        .iter()
        .map(|package| (package.node.as_str(), package))
        .collect();
    if nodes.len() != lock.packages.len() {
        return Err(ProjectError::Packages(
            "musa.lock repeats a package node with unequal records".to_owned(),
        ));
    }
    for (alias, requirement) in requirements {
        let node = lock
            .package_roots
            .get(alias)
            .ok_or_else(|| ProjectError::Packages(format!("package root `{alias}` is absent from musa.lock")))?;
        let package = nodes.get(node.as_str()).ok_or_else(|| {
            ProjectError::Packages(format!("locked package root `{alias}` names missing node `{node}`"))
        })?;
        if package.git != requirement.git || package.rev != requirement.rev {
            return Err(ProjectError::Packages(format!(
                "package `{alias}` changed from {} at {}; run `musa fetch {}`",
                package.git,
                package.rev,
                root.display()
            )));
        }
    }

    let mut module_uris = BTreeMap::<String, BTreeMap<String, String>>::new();
    let mut module_text = BTreeMap::<String, String>::new();
    let mut assets_by_node = BTreeMap::<String, BTreeMap<String, crate::assets::PackageAsset>>::new();
    for package in &lock.packages {
        validate_revision(&package.rev).map_err(ProjectError::Packages)?;
        validate_git(&package.git).map_err(ProjectError::Packages)?;
        for target in package.dependencies.values() {
            if !nodes.contains_key(target.as_str()) {
                return Err(ProjectError::Packages(format!(
                    "package `{}` names missing dependency node `{target}`",
                    package.name
                )));
            }
        }
        let expected_node = node_identity(
            &package.name,
            &PackageRequirement {
                git: package.git.clone(),
                rev: package.rev.clone(),
            },
            &package.source,
            &package.tree,
            &package.dependencies,
        );
        if package.node != expected_node {
            return Err(ProjectError::Packages(format!(
                "locked descriptor for package `{}` does not match its complete record",
                package.name
            )));
        }
        let cache = cache_path(root, &package.tree)?;
        verify_cache(&cache, &package.files, &package.tree).map_err(|error| {
            ProjectError::Packages(format!(
                "package `{}` at {} is unavailable offline: {error}; run `musa fetch {}`",
                package.name,
                package.rev,
                root.display()
            ))
        })?;
        let manifest = read_package_manifest(&cache, package)?;
        if manifest.package.name != package.name
            || manifest.package.language_version != musa_compiler::STANDARD_LIBRARY_LANGUAGE_VERSION
            || manifest.build.source != package.source
        {
            return Err(ProjectError::Packages(format!(
                "locked metadata for package `{}` disagrees with its verified musa.toml",
                package.name
            )));
        }
        let declared = manifest.requirements()?;
        if declared.len() != package.dependencies.len() {
            return Err(ProjectError::Packages(format!(
                "dependency aliases for package `{}` differ between cache and lock",
                package.name
            )));
        }
        for (alias, requirement) in declared {
            let target = package.dependencies.get(&alias).ok_or_else(|| {
                ProjectError::Packages(format!(
                    "dependency `{alias}` of package `{}` is absent from musa.lock",
                    package.name
                ))
            })?;
            let target_package = nodes
                .get(target.as_str())
                .ok_or_else(|| ProjectError::Packages(format!("dependency `{alias}` names missing node `{target}`")))?;
            if target_package.git != requirement.git || target_package.rev != requirement.rev {
                return Err(ProjectError::Packages(format!(
                    "dependency `{alias}` of package `{}` has a different exact identity in musa.lock",
                    package.name
                )));
            }
        }
        let mut package_assets = BTreeMap::new();
        for (path, policy) in manifest.asset_policies()? {
            let physical_path = cache.join(logical_path(&path)?);
            let locked = package.files.iter().find(|file| file.path == path).ok_or_else(|| {
                ProjectError::Packages(format!(
                    "package asset `{path}` in `{}` is absent from the locked file table",
                    package.name
                ))
            })?;
            package_assets.insert(
                path,
                crate::assets::PackageAsset {
                    policy,
                    digest: locked.digest.clone(),
                    bytes: locked.bytes,
                    path: physical_path,
                    boundary: cache.clone(),
                },
            );
        }
        assets_by_node.insert(package.node.clone(), package_assets);
        let modules = verify_source_root(&cache, package)?;
        let mut uris = BTreeMap::new();
        for (module, file) in modules {
            let logical = format!("{}/{}", package.source.trim_end_matches('/'), file);
            let text = read_locked_text(&cache, package, &logical)?;
            let uri = package_uri(&package.node, &module);
            uris.insert(module, uri.clone());
            module_text.insert(uri, text);
        }
        module_uris.insert(package.node.clone(), uris);
    }
    reject_graph_cycles(&lock.package_roots, &nodes)?;

    let mut sources = musa_compiler::ImportSources::default();
    for (uri, text) in module_text {
        sources.insert_read_only(uri, text);
    }
    for (alias, node) in &lock.package_roots {
        let modules = module_uris.get(node).ok_or_else(|| {
            ProjectError::Packages(format!("package root `{alias}` names missing module node `{node}`"))
        })?;
        bind_namespace(&mut sources, document, alias, modules);
    }
    for package in &lock.packages {
        let own = module_uris
            .get(&package.node)
            .ok_or_else(|| ProjectError::Packages(format!("package `{}` has no verified module map", package.name)))?;
        for importer in own.values() {
            bind_namespace(&mut sources, importer, &package.name, own);
            for (alias, target) in &package.dependencies {
                let modules = module_uris.get(target).ok_or_else(|| {
                    ProjectError::Packages(format!(
                        "dependency `{alias}` of package `{}` has no verified module map",
                        package.name
                    ))
                })?;
                bind_namespace(&mut sources, importer, alias, modules);
            }
        }
    }
    let mut assets = BTreeMap::new();
    for (alias, node) in &lock.package_roots {
        let package_assets = assets_by_node
            .get(node)
            .ok_or_else(|| ProjectError::Packages(format!("package root `{alias}` has no verified asset map")))?;
        for (path, asset) in package_assets {
            assets.insert(format!("pkg:{alias}/{path}"), asset.clone());
        }
    }
    Ok(PackageClosure { sources, assets })
}

fn bind_namespace(
    sources: &mut musa_compiler::ImportSources,
    importer: &str,
    alias: &str,
    modules: &BTreeMap<String, String>,
) {
    for (module, uri) in modules {
        sources.insert_resolution(importer, format!("{alias}::{module}"), uri);
    }
}

fn package_uri(node: &str, module: &str) -> String {
    format!(
        "musa-package:/{}/{}.musa",
        node.trim_start_matches("sha256:"),
        module.replace("::", "/")
    )
}

fn cache_path(root: &Path, tree: &str) -> Result<PathBuf, ProjectError> {
    let digest = tree.strip_prefix("sha256:").ok_or_else(|| {
        ProjectError::Packages(format!("locked package tree `{tree}` is not algorithm-tagged SHA-256"))
    })?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ProjectError::Packages(format!(
            "locked package tree `{tree}` is malformed"
        )));
    }
    Ok(root.join(".musa/cache/packages").join(digest))
}

fn read_package_manifest(cache: &Path, package: &LockedPackage) -> Result<PackageManifest, ProjectError> {
    let path = cache.join("musa.toml");
    let text = read_locked_text(cache, package, "musa.toml")?;
    toml::from_str(&text)
        .map_err(|error| ProjectError::Packages(format!("{} is not a valid package manifest: {error}", path.display())))
}

fn read_locked_text(cache: &Path, package: &LockedPackage, logical: &str) -> Result<String, ProjectError> {
    let expected = package
        .files
        .iter()
        .find(|file| file.path == logical)
        .ok_or_else(|| ProjectError::Packages(format!("package `{}` lock omits `{logical}`", package.name)))?;
    let path = cache.join(logical_path(logical)?);
    let file = std::fs::File::open(&path).map_err(|error| ProjectError::io(path.display(), error))?;
    let metadata = file
        .metadata()
        .map_err(|error| ProjectError::io(path.display(), error))?;
    if metadata.len() != expected.bytes || metadata.len() > u64::try_from(MAX_FILE_BYTES).unwrap_or(u64::MAX) {
        return Err(ProjectError::Packages(format!(
            "package `{}` source `{logical}` has a different byte length than its lock",
            package.name
        )));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(expected.bytes).unwrap_or(MAX_FILE_BYTES));
    std::io::BufReader::new(file)
        .take(expected.bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| ProjectError::io(path.display(), error))?;
    let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
    if length != expected.bytes || digest != expected.digest {
        return Err(ProjectError::Packages(format!(
            "package `{}` source `{logical}` changed after cache verification",
            package.name
        )));
    }
    String::from_utf8(bytes)
        .map_err(|error| ProjectError::Packages(format!("package source {} is not UTF-8: {error}", path.display())))
}

fn reject_graph_cycles(
    roots: &BTreeMap<String, String>,
    nodes: &BTreeMap<&str, &LockedPackage>,
) -> Result<(), ProjectError> {
    fn visit<'a>(
        node: &'a str,
        nodes: &BTreeMap<&'a str, &'a LockedPackage>,
        active: &mut Vec<&'a str>,
        done: &mut std::collections::BTreeSet<&'a str>,
    ) -> Result<(), ProjectError> {
        if active.contains(&node) {
            return Err(ProjectError::Packages(
                "musa.lock contains a package dependency cycle".to_owned(),
            ));
        }
        if !done.insert(node) {
            return Ok(());
        }
        active.push(node);
        let package = nodes
            .get(node)
            .ok_or_else(|| ProjectError::Packages(format!("musa.lock names missing node `{node}`")))?;
        for target in package.dependencies.values() {
            visit(target, nodes, active, done)?;
        }
        active.pop();
        Ok(())
    }
    let mut done = std::collections::BTreeSet::new();
    for node in roots.values() {
        visit(node, nodes, &mut Vec::new(), &mut done)?;
    }
    if done.len() != nodes.len() {
        return Err(ProjectError::Packages(
            "musa.lock contains package nodes unreachable from project roots".to_owned(),
        ));
    }
    Ok(())
}

struct Graph<'a> {
    root: &'a Path,
    nodes: BTreeMap<String, LockedPackage>,
    descriptors: BTreeMap<(String, String), String>,
}

/// Fetch every exact root and transitive package, atomically publish verified
/// trees, and replace only the package portion of `musa.lock`.
pub(crate) fn fetch(root: &Path, requirements: &BTreeMap<String, PackageRequirement>) -> Result<usize, ProjectError> {
    let mut graph = Graph {
        root,
        nodes: BTreeMap::new(),
        descriptors: BTreeMap::new(),
    };
    let mut roots = BTreeMap::new();
    for (alias, requirement) in requirements {
        let mut names = BTreeMap::new();
        let node = graph.resolve(requirement, &mut Vec::new(), &mut names)?;
        roots.insert(alias.clone(), node);
    }
    let mut lock = crate::lock::read(root)?.unwrap_or_default();
    lock.version = crate::lock::VERSION;
    lock.package_roots = roots;
    lock.packages = graph.nodes.into_values().collect();
    crate::lock::write(root, &lock)?;
    Ok(lock.packages.len())
}

impl Graph<'_> {
    fn resolve(
        &mut self,
        requirement: &PackageRequirement,
        stack: &mut Vec<(String, String)>,
        names: &mut BTreeMap<String, (String, String)>,
    ) -> Result<String, ProjectError> {
        let descriptor = (requirement.git.clone(), requirement.rev.clone());
        if stack.contains(&descriptor) {
            return Err(ProjectError::Packages(format!(
                "exact package dependency cycle reaches {} at {}",
                requirement.git, requirement.rev
            )));
        }
        if let Some(node) = self.descriptors.get(&descriptor) {
            let package = self
                .nodes
                .get(node)
                .ok_or_else(|| ProjectError::Packages("resolved package descriptor names no graph node".to_owned()))?;
            let identity = (package.git.clone(), package.rev.clone());
            if let Some(previous) = names.insert(package.name.clone(), identity.clone())
                && previous != identity
            {
                return Err(ProjectError::Packages(format!(
                    "package name `{}` resolves to incompatible exact identities within one root graph",
                    package.name
                )));
            }
            return Ok(node.clone());
        }
        if stack.len() >= MAX_GRAPH_DEPTH || self.nodes.len() >= MAX_GRAPH_NODES {
            return Err(ProjectError::Packages(
                "package graph exceeds its finite bound".to_owned(),
            ));
        }
        stack.push(descriptor.clone());
        let materialized = materialize(self.root, requirement)?;
        if materialized.manifest.package.language_version != musa_compiler::STANDARD_LIBRARY_LANGUAGE_VERSION {
            return Err(ProjectError::Packages(format!(
                "package `{}` needs Musa language version {}, but this build reads {}",
                materialized.manifest.package.name,
                materialized.manifest.package.language_version,
                musa_compiler::STANDARD_LIBRARY_LANGUAGE_VERSION
            )));
        }
        let identity = (requirement.git.clone(), requirement.rev.clone());
        if let Some(previous) = names.insert(materialized.manifest.package.name.clone(), identity.clone())
            && previous != identity
        {
            return Err(ProjectError::Packages(format!(
                "package name `{}` resolves to incompatible exact identities within one root graph",
                materialized.manifest.package.name
            )));
        }
        let mut dependencies = BTreeMap::new();
        for (alias, child) in materialized.manifest.requirements()? {
            let node = self.resolve(&child, stack, names)?;
            dependencies.insert(alias, node);
        }
        stack.pop();
        let node = node_identity(
            &materialized.manifest.package.name,
            requirement,
            &materialized.manifest.build.source,
            &materialized.tree,
            &dependencies,
        );
        let locked = LockedPackage {
            node: node.clone(),
            name: materialized.manifest.package.name,
            git: requirement.git.clone(),
            rev: requirement.rev.clone(),
            source: materialized.manifest.build.source,
            tree: materialized.tree,
            dependencies,
            files: materialized.files,
        };
        verify_source_root(&materialized.cache, &locked)?;
        self.descriptors.insert(descriptor, node.clone());
        self.nodes.insert(node.clone(), locked);
        Ok(node)
    }
}

fn node_identity(
    name: &str,
    requirement: &PackageRequirement,
    source: &str,
    tree: &str,
    dependencies: &BTreeMap<String, String>,
) -> String {
    let mut hash = Sha256::new();
    for field in [name, requirement.git.as_str(), requirement.rev.as_str(), source, tree] {
        hash.update(u64::try_from(field.len()).unwrap_or(u64::MAX).to_be_bytes());
        hash.update(field.as_bytes());
    }
    for (alias, node) in dependencies {
        hash.update(u64::try_from(alias.len()).unwrap_or(u64::MAX).to_be_bytes());
        hash.update(alias.as_bytes());
        hash.update(u64::try_from(node.len()).unwrap_or(u64::MAX).to_be_bytes());
        hash.update(node.as_bytes());
    }
    format!("sha256:{:x}", hash.finalize())
}

fn materialize(root: &Path, requirement: &PackageRequirement) -> Result<Materialized, ProjectError> {
    let cache_root = root.join(".musa/cache/packages");
    std::fs::create_dir_all(&cache_root).map_err(|error| ProjectError::io(cache_root.display(), error))?;
    let staging = tempfile::tempdir_in(&cache_root).map_err(|error| ProjectError::io(cache_root.display(), error))?;
    let repository = staging.path().join("repository");
    let tree = staging.path().join("tree");
    std::fs::create_dir(&tree).map_err(|error| ProjectError::io(tree.display(), error))?;

    let (algorithm, hexadecimal) = requirement
        .rev
        .split_once(':')
        .ok_or_else(|| ProjectError::Packages("revision lost its validated object algorithm".to_owned()))?;
    let object_format = format!("--object-format={algorithm}");
    run_git(
        None,
        [
            OsStr::new("init"),
            OsStr::new("--quiet"),
            OsStr::new(&object_format),
            repository.as_os_str(),
        ],
    )?;
    run_git(
        Some(&repository),
        [
            OsStr::new("fetch"),
            OsStr::new("--quiet"),
            OsStr::new("--depth=1"),
            OsStr::new("--no-tags"),
            OsStr::new("--no-recurse-submodules"),
            OsStr::new(&requirement.git),
            OsStr::new(hexadecimal),
        ],
    )?;
    let resolved = git_text(
        Some(&repository),
        [OsStr::new("rev-parse"), OsStr::new("FETCH_HEAD^{commit}")],
    )?;
    if resolved.trim() != hexadecimal {
        return Err(ProjectError::Packages(format!(
            "{} resolved {}, not the exact requested {}",
            requirement.git,
            resolved.trim(),
            requirement.rev
        )));
    }
    let object_format = git_text(
        Some(&repository),
        [OsStr::new("rev-parse"), OsStr::new("--show-object-format")],
    )?;
    if requirement
        .rev
        .split_once(':')
        .is_none_or(|(algorithm, _)| algorithm != object_format.trim())
    {
        return Err(ProjectError::Packages(format!(
            "repository uses Git {}, but the manifest pins {}",
            object_format.trim(),
            requirement.rev
        )));
    }

    let listing = git_output(
        Some(&repository),
        [
            OsStr::new("ls-tree"),
            OsStr::new("-r"),
            OsStr::new("-z"),
            OsStr::new("--full-tree"),
            OsStr::new("FETCH_HEAD"),
        ],
    )?;
    if listing.len() > 8 * 1024 * 1024 {
        return Err(ProjectError::Packages(
            "Git tree listing exceeds its finite bound".to_owned(),
        ));
    }
    let mut entries = parse_tree(&listing)?;
    if entries.len() > MAX_FILES {
        return Err(ProjectError::Packages(format!(
            "package has {} files, above the {MAX_FILES}-file limit",
            entries.len()
        )));
    }
    entries.sort_by(|left, right| left.1.cmp(&right.1));
    reject_case_collisions(entries.iter().map(|(_, path, _)| path.as_str()))?;

    let mut tree_hash = Sha256::new();
    let mut total = 0_u64;
    let mut files = Vec::with_capacity(entries.len());
    for (mode, logical, object) in entries {
        if !matches!(mode.as_str(), "100644" | "100755") {
            return Err(ProjectError::Packages(format!(
                "package path `{logical}` is mode {mode}; symlinks, submodules, and special files are not package data"
            )));
        }
        let bytes = cat_blob(&repository, &object)?;
        total = total.saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        if total > MAX_PACKAGE_BYTES {
            return Err(ProjectError::Packages(format!(
                "package bytes exceed the {MAX_PACKAGE_BYTES}-byte limit"
            )));
        }
        let destination = tree.join(logical_path(&logical)?);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| ProjectError::io(parent.display(), error))?;
        }
        std::fs::write(&destination, &bytes).map_err(|error| ProjectError::io(destination.display(), error))?;
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        tree_hash.update(u64::try_from(logical.len()).unwrap_or(u64::MAX).to_be_bytes());
        tree_hash.update(logical.as_bytes());
        tree_hash.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
        tree_hash.update(&bytes);
        files.push(LockedPackageFile {
            path: logical,
            bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            digest,
        });
    }
    let tree_digest = format!("sha256:{:x}", tree_hash.finalize());
    let cache = cache_root.join(tree_digest.trim_start_matches("sha256:"));
    if cache.exists() {
        verify_cache(&cache, &files, &tree_digest)?;
    } else {
        std::fs::rename(&tree, &cache).map_err(|error| ProjectError::io(cache.display(), error))?;
    }
    let manifest_path = cache.join("musa.toml");
    let manifest_text =
        std::fs::read_to_string(&manifest_path).map_err(|error| ProjectError::io(manifest_path.display(), error))?;
    let manifest: PackageManifest = toml::from_str(&manifest_text).map_err(|error| {
        ProjectError::Packages(format!(
            "{} is not a valid package manifest: {error}",
            manifest_path.display()
        ))
    })?;
    Ok(Materialized {
        cache,
        manifest,
        files,
        tree: tree_digest,
    })
}

fn hardened_git() -> Command {
    let mut command = Command::new("git");
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", if cfg!(windows) { "NUL" } else { "/dev/null" })
        .env("GIT_TERMINAL_PROMPT", "0")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .arg("-c")
        .arg("credential.helper=")
        .arg("-c")
        .arg("core.askPass=")
        .arg("-c")
        .arg("protocol.version=2");
    command
}

fn command_for<const N: usize>(repository: Option<&Path>, arguments: [&OsStr; N]) -> Command {
    let mut command = hardened_git();
    if let Some(repository) = repository {
        command.arg("-C").arg(repository);
    }
    command.args(arguments);
    command
}

fn run_git<const N: usize>(repository: Option<&Path>, arguments: [&OsStr; N]) -> Result<(), ProjectError> {
    let output = command_for(repository, arguments)
        .output()
        .map_err(|error| ProjectError::Packages(format!("cannot run Git: {error}")))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(ProjectError::Packages(format!(
            "Git refused the exact package fetch: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn git_output<const N: usize>(repository: Option<&Path>, arguments: [&OsStr; N]) -> Result<Vec<u8>, ProjectError> {
    let output = command_for(repository, arguments)
        .output()
        .map_err(|error| ProjectError::Packages(format!("cannot run Git: {error}")))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(ProjectError::Packages(format!(
            "Git could not read the pinned object: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn git_text<const N: usize>(repository: Option<&Path>, arguments: [&OsStr; N]) -> Result<String, ProjectError> {
    String::from_utf8(git_output(repository, arguments)?)
        .map_err(|_| ProjectError::Packages("Git returned non-UTF-8 object metadata".to_owned()))
}

fn cat_blob(repository: &Path, object: &str) -> Result<Vec<u8>, ProjectError> {
    let mut child = command_for(
        Some(repository),
        [OsStr::new("cat-file"), OsStr::new("blob"), OsStr::new(object)],
    )
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .map_err(|error| ProjectError::Packages(format!("cannot run Git: {error}")))?;
    let mut bytes = Vec::new();
    child
        .stdout
        .take()
        .ok_or_else(|| ProjectError::Packages("Git blob pipe did not open".to_owned()))?
        .take(u64::try_from(MAX_FILE_BYTES).unwrap_or(u64::MAX).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| ProjectError::Packages(format!("cannot read Git blob: {error}")))?;
    if bytes.len() > MAX_FILE_BYTES {
        drop(child.kill());
        drop(child.wait());
        return Err(ProjectError::Packages(format!(
            "one package file exceeds the {MAX_FILE_BYTES}-byte limit"
        )));
    }
    let status = child
        .wait()
        .map_err(|error| ProjectError::Packages(format!("cannot wait for Git: {error}")))?;
    if !status.success() {
        return Err(ProjectError::Packages(
            "Git could not materialize a locked blob".to_owned(),
        ));
    }
    Ok(bytes)
}

fn parse_tree(listing: &[u8]) -> Result<Vec<(String, String, String)>, ProjectError> {
    listing
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(|record| {
            let record = std::str::from_utf8(record)
                .map_err(|_| ProjectError::Packages("Git tree contains a non-UTF-8 path".to_owned()))?;
            let (header, path) = record
                .split_once('\t')
                .ok_or_else(|| ProjectError::Packages("Git returned a malformed tree record".to_owned()))?;
            let mut fields = header.split_ascii_whitespace();
            let mode = fields.next().unwrap_or_default();
            let kind = fields.next().unwrap_or_default();
            let object = fields.next().unwrap_or_default();
            if kind != "blob" || object.is_empty() || fields.next().is_some() {
                return Err(ProjectError::Packages(format!("unsupported Git tree entry `{record}`")));
            }
            logical_path(path)?;
            Ok((mode.to_owned(), path.to_owned(), object.to_owned()))
        })
        .collect()
}

fn logical_path(logical: &str) -> Result<PathBuf, ProjectError> {
    if logical.is_empty() || logical.starts_with('/') || logical.contains('\\') || logical.contains(':') {
        return Err(ProjectError::Packages(format!(
            "package path `{logical}` is not portable and relative"
        )));
    }
    let mut path = PathBuf::new();
    for segment in logical.split('/') {
        match segment {
            "" | "." | ".." => {
                return Err(ProjectError::Packages(format!(
                    "package path `{logical}` is not canonical"
                )));
            }
            segment => path.push(segment),
        }
    }
    Ok(path)
}

fn reject_case_collisions<'a>(paths: impl Iterator<Item = &'a str>) -> Result<(), ProjectError> {
    let mut folded = BTreeMap::<String, String>::new();
    for path in paths {
        let key = path.to_lowercase();
        if let Some(previous) = folded.insert(key, path.to_owned())
            && previous != path
        {
            return Err(ProjectError::Packages(format!(
                "package paths `{previous}` and `{path}` collide on a case-insensitive filesystem"
            )));
        }
    }
    Ok(())
}

fn verify_cache(cache: &Path, expected: &[LockedPackageFile], expected_tree: &str) -> Result<(), ProjectError> {
    let actual = scan_cache(cache)?;
    if actual != expected {
        return Err(ProjectError::Packages(format!(
            "content-addressed cache collision or corruption at {}: complete file tables differ",
            cache.display()
        )));
    }
    let mut tree_hash = Sha256::new();
    for file in expected {
        let path = cache.join(logical_path(&file.path)?);
        tree_hash.update(u64::try_from(file.path.len()).unwrap_or(u64::MAX).to_be_bytes());
        tree_hash.update(file.path.as_bytes());
        tree_hash.update(file.bytes.to_be_bytes());
        stream_file(&path, Some(&mut tree_hash))?;
    }
    let actual_tree = format!("sha256:{:x}", tree_hash.finalize());
    if actual_tree != expected_tree {
        return Err(ProjectError::Packages(format!(
            "cache tree at {} claims {expected_tree} but contains {actual_tree}",
            cache.display()
        )));
    }
    Ok(())
}

fn scan_cache(root: &Path) -> Result<Vec<LockedPackageFile>, ProjectError> {
    fn visit(
        root: &Path,
        directory: &Path,
        files: &mut Vec<LockedPackageFile>,
        total: &mut u64,
    ) -> Result<(), ProjectError> {
        for entry in std::fs::read_dir(directory).map_err(|error| ProjectError::io(directory.display(), error))? {
            let entry = entry.map_err(|error| ProjectError::io(directory.display(), error))?;
            let kind = entry
                .file_type()
                .map_err(|error| ProjectError::io(entry.path().display(), error))?;
            if kind.is_symlink() || (!kind.is_file() && !kind.is_dir()) {
                return Err(ProjectError::Packages(format!(
                    "cache contains unsupported object {}",
                    entry.path().display()
                )));
            }
            if kind.is_dir() {
                visit(root, &entry.path(), files, total)?;
            } else {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| ProjectError::Packages("cache path escaped its root".to_owned()))?
                    .components()
                    .map(|component| component.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                logical_path(&relative)?;
                let (bytes, digest) = stream_file(&entry.path(), None)?;
                *total = total.saturating_add(bytes);
                if *total > MAX_PACKAGE_BYTES {
                    return Err(ProjectError::Packages(format!(
                        "cached package bytes exceed the {MAX_PACKAGE_BYTES}-byte limit"
                    )));
                }
                files.push(LockedPackageFile {
                    path: relative,
                    bytes,
                    digest,
                });
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    let mut total = 0_u64;
    visit(root, root, &mut files, &mut total)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    if files.len() > MAX_FILES {
        return Err(ProjectError::Packages(
            "cached package exceeds its file-count bound".to_owned(),
        ));
    }
    Ok(files)
}

fn stream_file(path: &Path, mut tree: Option<&mut Sha256>) -> Result<(u64, String), ProjectError> {
    let file = std::fs::File::open(path).map_err(|error| ProjectError::io(path.display(), error))?;
    let metadata = file
        .metadata()
        .map_err(|error| ProjectError::io(path.display(), error))?;
    if metadata.len() > u64::try_from(MAX_FILE_BYTES).unwrap_or(u64::MAX) {
        return Err(ProjectError::Packages(format!(
            "cached package file {} exceeds the {MAX_FILE_BYTES}-byte limit",
            path.display()
        )));
    }
    let mut reader = std::io::BufReader::new(file);
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();
    let mut bytes = 0_u64;
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| ProjectError::io(path.display(), error))?;
        if count == 0 {
            break;
        }
        let chunk = buffer.get(..count).unwrap_or_default();
        digest.update(chunk);
        if let Some(tree) = tree.as_deref_mut() {
            tree.update(chunk);
        }
        bytes = bytes.saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
    }
    if bytes != metadata.len() {
        return Err(ProjectError::Packages(format!(
            "cached package file {} changed while it was verified",
            path.display()
        )));
    }
    Ok((bytes, format!("sha256:{:x}", digest.finalize())))
}

fn verify_source_root(cache: &Path, package: &LockedPackage) -> Result<Vec<(String, String)>, ProjectError> {
    let source = logical_path(&package.source)?;
    let source_root = cache.join(source);
    let root_module = source_root.join("lib.musa");
    if !root_module.is_file() {
        return Err(ProjectError::Packages(format!(
            "package `{}` declares source root `{}` without lib.musa",
            package.name, package.source
        )));
    }
    let mut files = Vec::new();
    for locked in &package.files {
        let Some(relative) = locked
            .path
            .strip_prefix(package.source.trim_end_matches('/'))
            .and_then(|path| path.strip_prefix('/'))
        else {
            continue;
        };
        if !Path::new(relative)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("musa"))
        {
            continue;
        }
        let logical = format!("{}/{}", package.source.trim_end_matches('/'), relative);
        let text = read_locked_text(cache, package, &logical)?;
        files.push((relative.to_owned(), text));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    musa_compiler::package_module_files(&files).map_err(|faults| {
        ProjectError::Packages(format!(
            "package `{}` has an invalid declared module tree: {}",
            package.name,
            faults.join("; ")
        ))
    })
}
