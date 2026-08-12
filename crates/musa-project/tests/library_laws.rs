//! What a bundled module is, when the interface opens one.
//!
//! `docs/rules/desktop/08-elaboration.md` §3: a reader who follows a term they did
//! not declare is shown the module's own text, read-only, under the name the
//! language spells rather than the locator the compiler files it under. The
//! place inside it is restated here because only this side has the text — the
//! caller is holding a byte range into a document it has never seen.

// A law that trips is a bug in the boundary it protects; panicking is the report.
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use musa_project::library_document;

/// A module the compiler bundles opens, with its own name rather than its
/// locator.
#[test]
fn a_bundled_module_opens_under_the_name_the_language_spells() {
    let (uri, source) = musa_compiler::standard_library_modules()
        .next()
        .expect("the standard library has modules");
    let document = library_document(&uri, None).expect("a bundled module opens");
    assert_eq!(document.text, source);
    assert!(
        !document.name.contains(".musa"),
        "a name, not a file: {}",
        document.name
    );
    assert!(document.span.is_none(), "nothing was followed into");
}

/// A URI nothing answers to is refused rather than answered blank.
#[test]
fn an_unbundled_uri_is_refused() {
    assert!(library_document("musa-stdlib:/std/not-a-module.musa", None).is_none());
    assert!(library_document("/Users/someone/piece.musa", None).is_none());
}

/// The span comes back in the module's own measure.
///
/// The whole reason this function exists: the caller holds a byte range into a
/// document it has never seen, and only the side that has the text can say
/// where that is in code units.
#[test]
fn the_place_is_restated_in_the_documents_own_units() {
    let (uri, source) = musa_compiler::standard_library_modules()
        .next()
        .expect("the standard library has modules");
    let end = u32::try_from(source.len()).expect("a module fits in u32");
    let document = library_document(&uri, Some((0, end))).expect("a bundled module opens");
    let span = document.span.expect("a place was asked for");
    assert_eq!(span.start, 0);
    assert_eq!(
        usize::try_from(span.end).unwrap_or(0),
        source.encode_utf16().count(),
        "the end is the text's length in the units the frontend counts in"
    );
}
