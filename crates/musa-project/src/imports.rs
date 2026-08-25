//! The import closure of an open document (roadmap §16).
//!
//! The compiler resolves imports but reads no files: it is handed the text of
//! everything a piece may `use`, keyed by resolved path. This module is the
//! other half — the part that owns the filesystem. It walks the `use`
//! statements of the document and of every library it reaches, reads each
//! file once, and hands the compiler a closed world.
//!
//! A file that cannot be read is left out rather than reported here. The
//! compiler already has the one diagnostic worth showing — "cannot find
//! `../library/patches.musa`", pointed at the `import` that asked for it — and
//! an I/O error phrased against the same path would say it twice.

use std::collections::HashSet;
use std::path::PathBuf;

use musa_compiler::ImportSources;
use musa_syntax::ast::{ImportStmt, PieceDecl};

/// Everything `source` imports, transitively.
///
/// `name` is the document name the compiler will see, because imports
/// resolve against it: the paths this returns are exactly the keys the
/// compiler will look up.
pub(crate) fn closure(name: &str, source: &str, mut sources: ImportSources) -> (ImportSources, Vec<PathBuf>) {
    let mut files = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut pending: Vec<(String, String)> = vec![(name.to_owned(), source.to_owned())];
    while let Some((importer, text)) = pending.pop() {
        for written in written_imports(&text) {
            let path = sources.resolve(&importer, &written);
            if !seen.insert(path.clone()) {
                continue;
            }
            // Embedded and fetched package sources already belong to the
            // closed world. The compiler walks their own imports with the
            // importer-sensitive exact resolution map.
            if let Some(text) = sources.get(&path).map(str::to_owned) {
                if path.starts_with("musa-package:/") {
                    sources.activate(&path);
                    pending.push((path, text));
                }
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            sources.insert(path.clone(), text.clone());
            files.push(PathBuf::from(&path));
            pending.push((path, text));
        }
    }
    files.sort();
    (sources, files)
}

/// The paths one file's `import` statements name, as written.
fn written_imports(text: &str) -> Vec<String> {
    let document = musa_syntax::parse(text);
    let root = document.syntax();
    // The root's imports and the piece's, because a file may write them in
    // both places and a file with no piece writes them only at the root.
    let mut statements = ImportStmt::all_at_root(&root);
    statements.extend(
        PieceDecl::from_root(&root)
            .map(|piece| piece.imports())
            .unwrap_or_default(),
    );
    statements.iter().filter_map(ImportStmt::path).collect()
}
