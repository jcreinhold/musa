//! What a package's module tree is, read from the package's own declarations.
//!
//! A package is a directory with a manifest and a source root. The tree under
//! that root is its `mod` declarations — `src/lib.musa` names the top level,
//! and a directory module names its own children in `mod.musa` — and it is
//! *only* those declarations. Nothing here scans a directory looking for
//! importable files.
//!
//! That is the whole design, and both errors it makes possible are the reason
//! for it:
//!
//! - a `.musa` file under the source root that no `mod` reaches is **declared
//!   nowhere**, because a file nobody declared is not part of the package
//!   however much it looks like it;
//! - a `mod` naming no file is **missing**, because a declaration that reaches
//!   nothing is a claim the package cannot keep.
//!
//! Discovery would have made both of them silent. The case that motivated this
//! module was silent at first: `stdlib/sequences.musa` was committed,
//! reachable from no import, and the workspace built green.

use std::collections::{BTreeMap, BTreeSet};

use musa_syntax::ast::ModDecl;

/// A package's module tree, and everything wrong with it.
///
/// Both halves are always produced: a package with a missing module still has
/// the modules it does have, so one fault does not cost a reader every other
/// answer.
pub(crate) struct Package<'a> {
    modules: BTreeMap<String, &'a str>,
    faults: Vec<Fault>,
}

/// A way a package's declarations and its files can disagree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Fault {
    /// A file under the source root that no `mod` reaches.
    DeclaredNowhere { file: String },
    /// A `mod` that names neither `<name>.musa` nor `<name>/mod.musa`.
    Missing { path: String, declared_in: String },
}

impl Fault {
    /// What to tell whoever has to fix it.
    pub(crate) fn message(&self) -> String {
        match self {
            Self::DeclaredNowhere { file } => {
                format!("`{file}` is in the package but no `mod` declares it")
            }
            Self::Missing { path, declared_in } => {
                format!("`{declared_in}` declares `mod {path}`, but there is no such module")
            }
        }
    }

    /// The line that prevents the next one.
    pub(crate) fn help(&self) -> &'static str {
        match self {
            Self::DeclaredNowhere { .. } => "add a `mod` for it in the module file beside it, or delete the file",
            Self::Missing { .. } => "add the file the declaration names, or remove the declaration",
        }
    }
}

impl<'a> Package<'a> {
    /// Read the tree `files` declares, starting from `lib.musa`.
    ///
    /// `files` is every `.musa` file under the source root, keyed by its path
    /// relative to that root with `/` separators. Reading the same listing
    /// that the declarations are checked against is what makes the two errors
    /// above possible at all.
    pub(crate) fn read(files: &[(&'a str, &'a str)]) -> Self {
        let available: BTreeMap<&str, &'a str> = files.iter().copied().collect();
        let mut package = Self {
            modules: BTreeMap::new(),
            faults: Vec::new(),
        };
        let mut reached: BTreeSet<String> = BTreeSet::new();
        match available.get("lib.musa") {
            Some(source) => {
                reached.insert("lib.musa".to_owned());
                package.descend(&available, &mut reached, "", "lib.musa", source);
            }
            None => package.faults.push(Fault::Missing {
                path: "lib".to_owned(),
                declared_in: "musa.toml".to_owned(),
            }),
        }
        for (file, _) in files {
            if !reached.contains(*file) {
                package.faults.push(Fault::DeclaredNowhere {
                    file: (*file).to_owned(),
                });
            }
        }
        package
    }

    /// Follow one module file's `mod` declarations.
    ///
    /// `prefix` is the module path already walked (`""` at the root,
    /// `"tonal::"` inside a directory module), and `directory` is where the
    /// declarations look for their files.
    fn descend(
        &mut self,
        available: &BTreeMap<&str, &'a str>,
        reached: &mut BTreeSet<String>,
        prefix: &str,
        declared_in: &str,
        source: &'a str,
    ) {
        let directory = declared_in.rsplit_once('/').map_or("", |(head, _)| head);
        let parsed = musa_syntax::parse(source);
        for declaration in ModDecl::all_at_root(&parsed.syntax()) {
            let Some(name) = declaration.name() else { continue };
            let path = format!("{prefix}{name}");
            let at = |file: &str| {
                if directory.is_empty() {
                    file.to_owned()
                } else {
                    format!("{directory}/{file}")
                }
            };
            let leaf = at(&format!("{name}.musa"));
            let branch = at(&format!("{name}/mod.musa"));
            if let Some(text) = available.get(leaf.as_str()) {
                reached.insert(leaf);
                self.modules.insert(path, text);
            } else if let Some(text) = available.get(branch.as_str()) {
                reached.insert(branch.clone());
                self.descend(available, reached, &format!("{path}::"), &branch, text);
            } else {
                self.faults.push(Fault::Missing {
                    path,
                    declared_in: declared_in.to_owned(),
                });
            }
        }
    }

    /// The source of one module path, such as `tonal::harmony`.
    pub(crate) fn module(&self, path: &str) -> Option<&'a str> {
        self.modules.get(path).copied()
    }

    /// Every module the tree declares, in path order.
    pub(crate) fn modules(&self) -> impl Iterator<Item = (&str, &'a str)> {
        self.modules.iter().map(|(path, source)| (path.as_str(), *source))
    }

    /// Everything wrong with the package, in the order it was found.
    pub(crate) fn faults(&self) -> &[Fault] {
        &self.faults
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A well-formed miniature: a root file, a leaf, and a directory module.
    const TREE: [(&str, &str); 4] = [
        ("lib.musa", "mod core;\nmod tonal;\n"),
        ("core.musa", "library { }"),
        ("tonal/mod.musa", "mod harmony;\n"),
        ("tonal/harmony.musa", "library { }"),
    ];

    #[test]
    fn a_declared_tree_nests_as_deep_as_its_declarations() {
        let package = Package::read(&TREE);
        assert_eq!(package.faults(), &[]);
        assert_eq!(
            package.modules().map(|(path, _)| path).collect::<Vec<_>>(),
            ["core", "tonal::harmony"],
            "a directory module is a path segment, not a module of its own"
        );
    }

    #[test]
    fn a_file_no_declaration_reaches_is_declared_nowhere() {
        let mut files = TREE.to_vec();
        files.push(("stray.musa", "library { }"));
        let package = Package::read(&files);
        assert_eq!(
            package.faults(),
            &[Fault::DeclaredNowhere {
                file: "stray.musa".to_owned()
            }]
        );
        assert!(
            package.module("stray").is_none(),
            "and it is not importable either: being on disk is not being in the package"
        );
    }

    /// The same fault one directory down, which is where it is easiest to miss.
    #[test]
    fn a_stray_file_inside_a_directory_module_is_caught_too() {
        let mut files = TREE.to_vec();
        files.push(("tonal/sequences.musa", "library { }"));
        let package = Package::read(&files);
        assert_eq!(
            package.faults(),
            &[Fault::DeclaredNowhere {
                file: "tonal/sequences.musa".to_owned()
            }]
        );
    }

    #[test]
    fn a_declaration_reaching_no_file_is_missing() {
        let files = [("lib.musa", "mod core;\nmod absent;\n"), ("core.musa", "library { }")];
        let package = Package::read(&files);
        assert_eq!(
            package.faults(),
            &[Fault::Missing {
                path: "absent".to_owned(),
                declared_in: "lib.musa".to_owned()
            }]
        );
        assert!(package.module("core").is_some(), "the rest of the tree still reads");
    }

    #[test]
    fn a_missing_declaration_names_the_file_that_made_the_claim() {
        let files = [("lib.musa", "mod tonal;\n"), ("tonal/mod.musa", "mod harmony;\n")];
        let package = Package::read(&files);
        assert_eq!(
            package.faults(),
            &[Fault::Missing {
                path: "tonal::harmony".to_owned(),
                declared_in: "tonal/mod.musa".to_owned()
            }],
            "the path is the full one and the blame is the file that declared it"
        );
    }
}
