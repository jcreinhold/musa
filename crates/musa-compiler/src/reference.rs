//! The bundled library's reference, written from what the checker recorded.
//!
//! One fact, three readers. A composer hovering `close_position` in the editor,
//! a reader of `stdlib/reference.md`, and the guide under `docs/book/`
//! are all asking the same question — what does this name mean, and what may I
//! write after it — and the answer is [`crate::docs::ItemDoc`], produced by the
//! pass that checked the declaration. Nothing here re-reads the source.
//!
//! That matters more than it sounds. The generator this replaces scanned the
//! `.musa` files line by line, and so it published a function's *body* as its
//! signature, truncated any declaration whose parameters wrapped, and leaked
//! private structure members. Each of those is a second reader of Musa source
//! disagreeing with the compiler — the exact failure `crate::docs` exists to
//! remove.
//!
//! What is listed is what an importing document may name: every declaration at
//! a bundled module's root, every member a `signature` requires, and, under a
//! `structure`, only the members its signature exports. A member the signature
//! does not list is private (`docs/rules/language/04-templates-and-modules.md` §4),
//! and documenting it would advertise a name that does not resolve.

use std::fmt::Write as _;

use crate::docs::ItemDoc;

/// The probe document's name, as diagnostics would spell it.
const PROBE: &str = "musa-stdlib:/reference.musa";

/// A piece that imports every bundled module, so the checker records them all.
///
/// The score is the smallest one the language accepts: the reference is about
/// the declarations, and the piece exists only to give them somewhere to be
/// checked. Imports are written in the package's own module order, so the
/// records arrive in reading order and the generated file is stable.
fn probe() -> String {
    let mut source = String::from("piece \"Standard library\" {\n");
    for (uri, _) in crate::imports::standard_library_modules() {
        if let Some(path) = uri
            .strip_prefix("musa-stdlib:/std/")
            .and_then(|rest| rest.strip_suffix(".musa"))
        {
            let _ = writeln!(source, "    import std::{};", path.replace('/', "::"));
        }
    }
    source.push_str("\n    tempo 1/4 = 60;\n    meter 4/4;\n\n");
    source.push_str("    score {\n        part p {\n            voice v {\n                rest/1\n            }\n        }\n    }\n}\n");
    source
}

/// Everything the checker recorded about the bundled modules, in module order.
///
/// Public to the crate rather than to the world: the shape a *reader* is given
/// is markdown, and a caller who wanted the records themselves would be asking
/// the compiler for its own bookkeeping. The tests use it to hold the library
/// to its documentation rule.
pub(crate) fn recorded_items() -> Vec<ItemDoc> {
    let document = crate::compile::SourceDocument::new(probe(), PROBE);
    let compilation = crate::compile::compile(&document, &crate::compile::CompileOptions::default());
    compilation.items().to_vec()
}

/// Whatever `structure X: S` ascribes, or `None` for anything else.
///
/// Read back off the signature line that `crate::module` wrote a few hundred
/// lines away, because the ascription is not otherwise a fact a record
/// carries — and it need not be, since nothing but this file has ever wanted
/// it. The three spellings `crate::module` emits all end in `: <signature>`.
fn ascribes(item: &ItemDoc) -> Option<&str> {
    let rest = item.signature.strip_prefix("structure ")?;
    rest.rsplit(": ").next()
}

/// The reference, as `stdlib/reference.md` holds it.
///
/// Generated rather than written: the checked-in copy makes the library
/// readable outside tooling, and its law test refuses to let the prose and the
/// executable source drift apart.
#[must_use]
pub fn standard_library_reference() -> String {
    let items = recorded_items();

    // Which qualified names a signature requires. That set is what a structure
    // ascribing the signature exports; everything else it defines is private.
    let signatures: std::collections::HashSet<&str> = items
        .iter()
        .filter(|item| item.signature.starts_with("signature "))
        .map(|item| item.name.as_str())
        .collect();
    // Keyed by the member's qualified name, holding the sentence the signature
    // wrote about it. A structure that answers the requirement without saying
    // anything new inherits that sentence rather than repeating it: the
    // contract is stated once, where it is required, and every structure
    // meeting it is a projection of that one statement.
    let required: std::collections::HashMap<&str, Option<&str>> = items
        .iter()
        .filter(|item| {
            item.name
                .split_once('.')
                .is_some_and(|(owner, _)| signatures.contains(owner))
        })
        .map(|item| (item.name.as_str(), item.summary.as_deref()))
        .collect();

    let mut out = String::from(
        "# Musa standard library 1\n\nThis reference is generated from what the compiler records about each bundled declaration — the same record an\neditor shows on hover. Standard definitions are ordinary Musa definitions; importing a module is explicit and never\nsearches the filesystem. Under a `structure`, only the members its signature exports are listed, because the rest are\nprivate to it.\n",
    );

    for (uri, _) in crate::imports::standard_library_modules() {
        let Some(module) = uri
            .strip_prefix("musa-stdlib:/std/")
            .and_then(|rest| rest.strip_suffix(".musa"))
            .map(|path| path.replace('/', "::"))
        else {
            continue;
        };
        let here: Vec<&ItemDoc> = items
            .iter()
            .filter(|item| item.source.uri.as_deref() == Some(uri.as_str()))
            .collect();
        if here.is_empty() {
            continue;
        }
        // The module path, not its URI: a reader of this file writes
        // `import std::tonal::harmony;` and never sees where the file sits.
        let _ = write!(out, "\n## `std::{module}`\n\n");
        for item in here.iter().filter(|item| !item.name.contains('.')) {
            entry(&mut out, item, "", item.summary.as_deref());
            let prefix = format!("{}.", item.name);
            let exported = ascribes(item);
            for member in here.iter().filter(|member| member.name.starts_with(&prefix)) {
                match exported {
                    // A structure exports what its signature required, and
                    // borrows the sentence written there when it wrote none.
                    Some(signature) => {
                        let short = member.name.trim_start_matches(&prefix);
                        if let Some(stated) = required.get(format!("{signature}.{short}").as_str()) {
                            entry(&mut out, member, "  ", member.summary.as_deref().or(*stated));
                        }
                    }
                    // A signature requires everything listed under it.
                    None => entry(&mut out, member, "  ", member.summary.as_deref()),
                }
            }
        }
    }
    out
}

/// One list line: the signature, and the sentence that explains it.
///
/// The sentence is passed in rather than read off `item`, because a structure
/// member may be explained by the signature that required it.
fn entry(out: &mut String, item: &ItemDoc, indent: &str, summary: Option<&str>) {
    let _ = write!(out, "{indent}- `{}`", item.signature);
    if let Some(deprecated) = &item.deprecation {
        let _ = write!(out, " — **deprecated**: {deprecated}");
    } else if let Some(summary) = summary {
        let _ = write!(out, " — {summary}");
    }
    out.push('\n');
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    /// The reference is only as good as the compilation behind it: a module
    /// that failed to import would quietly contribute nothing, and the file
    /// would lose a section without anything going red.
    #[test]
    fn the_probe_importing_every_bundled_module_compiles() {
        let document = crate::compile::SourceDocument::new(probe(), PROBE);
        let compilation = crate::compile::compile(&document, &crate::compile::CompileOptions::default());
        let problems: Vec<&str> = compilation
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.severity == crate::diagnose::Severity::Error)
            .map(|diagnostic| diagnostic.message.as_str())
            .collect();
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn checked_in_reference_is_derived_from_executable_source() {
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(
                concat!(env!("CARGO_MANIFEST_DIR"), "/../../stdlib/reference.md"),
                standard_library_reference(),
            )
            .expect("write the reference");
        }
        assert_eq!(
            standard_library_reference(),
            include_str!("../../../stdlib/reference.md"),
            "re-run with UPDATE_FIXTURES=1 when a bundled comment or signature changes"
        );
    }

    /// A name the library publishes and does not explain is a name a composer
    /// meets in completion and cannot use. The rule is checkable because the
    /// summary is a record rather than a convention.
    #[test]
    fn every_published_name_says_what_it_is_for() {
        let reference = standard_library_reference();
        let undocumented: Vec<&str> = reference
            .lines()
            .map(str::trim_start)
            .filter(|line| line.starts_with("- `"))
            .filter(|line| !line.contains(" — "))
            .collect();
        assert!(undocumented.is_empty(), "{undocumented:#?}");
    }

    /// Ownership, as the reference must show it: a structure publishes what
    /// its signature asked for, and keeps the rest.
    #[test]
    fn a_structure_publishes_its_signatures_members_and_no_others() {
        let reference = standard_library_reference();
        assert!(
            reference.contains("`fn CMajor.spell(ordinal: Nat) -> Option<Pitch>`"),
            "{reference}"
        );
        assert!(
            reference.contains("`let TonalContext.spell: Nat -> Option<Pitch>`"),
            "{reference}"
        );
        assert!(
            !reference.contains("CMajor.home"),
            "`home` is private to the structure, so nothing outside may name it"
        );
    }

    /// The signature a reader must *write*. A nullary function is called with
    /// the empty parameter list it was declared with, and a reference that
    /// spelled it `let do_re_mi_strong: List<Bool>` would be telling a
    /// composer to write an expression the checker rejects.
    #[test]
    fn a_nullary_function_is_published_as_a_function() {
        let reference = standard_library_reference();
        assert!(
            reference.contains("`fn do_re_mi_strong() -> List<Bool>`"),
            "{reference}"
        );
    }
}
