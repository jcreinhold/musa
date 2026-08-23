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
//! signature and truncated any declaration whose parameters wrapped. Each of
//! those is a second reader of Musa source disagreeing with the compiler — the
//! exact failure `crate::docs` exists to remove.
//!
//! What is listed is every declaration a bundled module writes under its own
//! name, in module order. A name reached through a trait — `Interval.compose`,
//! written inside an `impl` — is not one of them: it is documented by the trait
//! that requires it, and repeating it here would state the contract twice.

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
            // A phase module is not written in the language ordinary source is
            // written in, and an ordinary import cannot change syntax
            // (`crate::expand`). Importing one here would ask the checker to
            // read names that ordinary source has no way to spell, so the
            // reference documents what an importing *document* may name.
            if path.starts_with("adapters/") {
                continue;
            }
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

/// The reference, as `stdlib/reference.md` holds it.
///
/// Generated rather than written: the checked-in copy makes the library
/// readable outside tooling, and its law test refuses to let the prose and the
/// executable source drift apart.
#[must_use]
pub fn standard_library_reference() -> String {
    let items = recorded_items();

    let mut out = String::from(
        "# Musa standard library 1\n\nThis reference is generated from what the compiler records about each bundled declaration — the same record an\neditor shows on hover. Standard definitions are ordinary Musa definitions; importing a module is explicit and never\nsearches the filesystem.\n",
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
        // A dotted name is an `impl` block's, reached as `Interval.compose`
        // through the trait it implements rather than written by a reader of
        // this file, and `10-traits.md` §6 documents it where the trait is.
        for item in here.iter().filter(|item| !item.name.contains(crate::lower::DOT)) {
            entry(&mut out, item, item.summary.as_deref());
        }
    }
    out
}

/// One list line: the signature, and the sentence that explains it.
fn entry(out: &mut String, item: &ItemDoc, summary: Option<&str>) {
    let _ = write!(out, "- `{}`", item.signature);
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
            .filter(|diagnostic| diagnostic.severity == musa_score::diagnose::Severity::Error)
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

    /// What a bundled context looks like now that it is a record and two
    /// ordinary bindings: the type and its values, each on its own line, and
    /// nothing published under a dot.
    #[test]
    fn a_bundled_record_and_its_values_are_published() {
        let reference = standard_library_reference();
        assert!(reference.contains("`record TonalContext: Type`"), "{reference}");
        assert!(reference.contains("`let c_major: TonalContext`"), "{reference}");
        assert!(
            !reference.contains("CMajor."),
            "the structure layer is gone, so nothing is reached through one"
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
