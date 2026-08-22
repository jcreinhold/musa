//! The boundary between `kernel/` and `elaboration/`, stated as a law.
//!
//! Prompt 148 splits this crate into two programs: a kernel that decides
//! typing and definitional equality on finished terms, and an elaborator that
//! turns what an author wrote into such a term. The direction is one-way — the
//! elaborator reads the kernel, and the kernel knows nothing of the elaborator.
//!
//! Rust cannot express that. A descendant module may always name `crate::`, so
//! `mod kernel;` and `mod elaboration;` are filing, not enforcement, and the
//! crate root re-exports [`Refusal`](musa_calculus::Refusal) and
//! [`Raw`](musa_calculus::Raw) where any file can reach them. The rule is
//! therefore checked the way `editors/tree-sitter-musa` is held to the real
//! lexer: a test reads the source text and fails on the violation.
//!
//! ```text
//! law:      no code line under src/kernel/ names the module `elaboration`,
//!           nor any item defined under it
//! control:  the same check, run on a file that does, reports the violation
//! ```
//!
//! The control is not decoration. A source-text law that has never been seen
//! to fail is indistinguishable from one whose scanner reads nothing, and this
//! one has three ways to read nothing: an empty file list, an empty set of
//! forbidden names, and a comment-stripper that strips everything.
//!
//! **Code lines only.** The kernel's doc comments name `Refusal` and `Raw`
//! constantly, on purpose: a doc comment is how a kernel item explains who
//! reads it and what the elaborator turns its outcome into. A link creates no
//! dependency, and a law that forbade one would stop the kernel from
//! documenting its own callers.
//!
//! **What the forbidden set holds, and why that is enough.** Types, traits, and
//! aliases declared under `elaboration/`, plus every name the crate root
//! re-exports from there. Methods and free functions are absent and need not be
//! present: `Raw::app` cannot be called without naming `Raw`, and
//! `declare::declare` cannot be called without naming the module. The crate
//! root is the one path that evades both, so its re-exports are named
//! explicitly.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// This crate's `src/` directory.
fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.rs` file under `dir`, sorted, so a failure names a stable file.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).expect("read a source directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The identifier tokens of a line: maximal runs of `[A-Za-z0-9_]`.
///
/// Coarse on purpose. A path, a `use` item, a turbofish and a bare mention all
/// tokenize the same way, so the law does not depend on parsing Rust — and a
/// string literal that spells one of these names is caught too, which is the
/// right answer: the kernel has no business spelling the elaborator's types.
fn tokens(line: &str) -> impl Iterator<Item = &str> {
    line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|token| !token.is_empty())
}

/// The code half of a source line: everything before the first `//`.
///
/// The kernel has no block comments, and this test is the reason to keep it
/// that way — see the note in the module docs about why comments are exempt.
fn code(line: &str) -> &str {
    match line.find("//") {
        Some(at) => &line[..at],
        None => line,
    }
}

/// The item declared by a `pub`-visible line, if it declares a type.
fn declared_type(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("pub")?;
    // `pub(crate)`, `pub(super)`, `pub(in path)` — anything but `pubfoo`.
    let rest = match rest.strip_prefix('(') {
        Some(restricted) => restricted.split_once(')')?.1,
        None => rest,
    };
    let mut words = rest.split_whitespace();
    let keyword = words.next()?;
    if !matches!(keyword, "struct" | "enum" | "trait" | "type" | "union") {
        return None;
    }
    let name = words.next()?;
    name.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).next()
}

/// Every name a file under `kernel/` could use to reach the elaborator.
fn forbidden_names() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for path in rust_files(&src().join("elaboration")) {
        let text = std::fs::read_to_string(&path).expect("read an elaboration source file");
        for line in text.lines() {
            if let Some(name) = declared_type(code(line)) {
                names.insert(name.to_owned());
            }
        }
    }
    names.extend(root_reexports());
    names
}

/// The names `lib.rs` re-exports from `elaboration`, which a kernel file could
/// otherwise reach as `crate::Refusal` without ever writing the module's name.
fn root_reexports() -> BTreeSet<String> {
    let text = std::fs::read_to_string(src().join("lib.rs")).expect("read lib.rs");
    let mut names = BTreeSet::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find("pub use crate::elaboration::") {
        let (statement, after) = rest[at..]
            .split_once(';')
            .expect("a `use` statement ends in a semicolon");
        // The re-exported names are whatever follows the module path: either a
        // braced list or a single trailing item.
        let items = match statement.split_once('{') {
            Some((_, braced)) => braced.trim_end_matches('}').to_owned(),
            None => statement.rsplit("::").next().unwrap_or_default().to_owned(),
        };
        names.extend(tokens(&items).map(str::to_owned));
        rest = after;
    }
    names
}

/// Every violation of the law in one file's text, as reportable lines.
fn violations(label: &str, text: &str, forbidden: &BTreeSet<String>) -> Vec<String> {
    let mut found = Vec::new();
    for (offset, line) in text.lines().enumerate() {
        for token in tokens(code(line)) {
            if token == "elaboration" || forbidden.contains(token) {
                found.push(format!("{label}:{}: names `{token}`", offset.saturating_add(1)));
            }
        }
    }
    found
}

#[test]
fn the_kernel_names_nothing_the_elaborator_defines() {
    let forbidden = forbidden_names();
    // A scanner with nothing to look for finds nothing and passes. These two
    // names are the load-bearing ones: the elaborator's input and its refusal.
    assert!(forbidden.contains("Raw"), "the forbidden set should hold `Raw`");
    assert!(forbidden.contains("Refusal"), "the forbidden set should hold `Refusal`");

    let files = rust_files(&src().join("kernel"));
    assert!(files.len() > 20, "the kernel should have more than twenty files");

    let root = src();
    let mut found = Vec::new();
    for path in files {
        let label = path.strip_prefix(&root).unwrap_or(&path).display().to_string();
        let text = std::fs::read_to_string(&path).expect("read a kernel source file");
        found.extend(violations(&label, &text, &forbidden));
    }
    assert!(
        found.is_empty(),
        "the kernel reaches into the elaborator:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_boundary_check_reports_a_kernel_file_that_reaches_up() {
    let forbidden = forbidden_names();

    let by_path = "use crate::elaboration::refuse::Refusal;\n";
    let by_root = "fn f() -> crate::Refusal { todo!() }\n";
    for offender in [by_path, by_root] {
        assert!(
            !violations("control.rs", offender, &forbidden).is_empty(),
            "the check should reject `{}`",
            offender.trim()
        );
    }
}

#[test]
fn the_boundary_check_permits_a_doc_comment() {
    let forbidden = forbidden_names();
    let documented = "/// Refused with [`Refusal`], which `elaboration` defines.\nfn f() {}\n";
    assert!(
        violations("control.rs", documented, &forbidden).is_empty(),
        "a doc comment is a link, not a dependency"
    );
}
