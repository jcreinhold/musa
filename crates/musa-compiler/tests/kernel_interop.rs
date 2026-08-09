//! The round-trip law of the interchange format (prompt 48, docs/kernel/01).
//!
//! ```text
//! for every fixture:  parse(print(t)) evaluates to a timeline with
//!                     the same canonical form and semantic hash as t
//! ```
//!
//! The generated half lives in `musa-kernel/tests/terms.rs`, where terms can
//! be built directly. This half is the one that catches payload-escaping
//! bugs, because real payloads contain what a generator will not invent:
//! phrase names with spaces, chord symbols, expansion paths, and hairpin
//! shapes that must survive as exact rationals.

// A failure of these `expect`s is a bug in the fixture corpus, not in a
// caller's input.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
// The failure messages name which fixture broke, which a bare `expect` cannot.
#![allow(clippy::panic)]

use musa_compiler::{
    Realization, SourceDocument, check_kernel_text, kernel_normal_form, kernel_normalized_text, kernel_text,
    kernel_text_meaning,
};

/// The realization every fixture is pinned to.
///
/// `docs/kernel/11-realization.md` concedes the cost up front: a regression
/// fixture whose realization is unpinned is not a fixture. Determinate pieces
/// are unaffected by the number, which is what
/// [`a_determinate_piece_is_the_same_under_every_seed`] checks.
const FIXTURE_SEED: u64 = 42;

/// The realization the fixtures are printed under.
fn pinned() -> Realization {
    Realization::seeded(FIXTURE_SEED)
}

/// Every `.musa` fixture, by the stem its `.musa.kernel` golden uses.
const EXAMPLES: &[(&str, &str)] = &[
    ("annotated", include_str!("../../../examples/annotated.musa")),
    ("canon", include_str!("../../../examples/canon.musa")),
    ("changing-meter", include_str!("../../../examples/changing-meter.musa")),
    ("clef-change", include_str!("../../../examples/clef-change.musa")),
    ("counterpoint", include_str!("../../../examples/counterpoint.musa")),
    ("glass-mountain", include_str!("../../../examples/glass-mountain.musa")),
    ("invention", include_str!("../../../examples/invention.musa")),
    ("modulation", include_str!("../../../examples/modulation.musa")),
    (
        "profile-fixture",
        include_str!("../../../examples/profile-fixture.musa"),
    ),
    ("tuplet-fixture", include_str!("../../../examples/tuplet-fixture.musa")),
    ("twinkle", include_str!("../../../examples/twinkle.musa")),
    ("variation", include_str!("../../../examples/variation.musa")),
    ("loop-lengths", include_str!("../../../examples/loop-lengths.musa")),
    ("mobile", include_str!("../../../examples/mobile.musa")),
    ("changes", include_str!("../../../examples/changes.musa")),
    ("in-c", include_str!("../../../examples/in-c.musa")),
    ("rubato", include_str!("../../../examples/rubato.musa")),
    ("riser", include_str!("../../../examples/riser.musa")),
    ("cadenza", include_str!("../../../examples/cadenza.musa")),
    ("chant", include_str!("../../../examples/chant.musa")),
    ("bulgarian", include_str!("../../../examples/bulgarian.musa")),
    ("hemiola", include_str!("../../../examples/hemiola.musa")),
    ("canon-x", include_str!("../../../examples/canon-x.musa")),
];

/// The round-trip law over the real corpus: printing a piece and reading it
/// back preserves both what it means (N5) and its identity (N6).
#[test]
fn printing_and_parsing_an_example_preserves_its_meaning() {
    for &(name, source) in EXAMPLES {
        let document = SourceDocument::new(source, name);
        let printed = kernel_text(&document, &pinned()).expect("the fixture elaborates");
        let (form, hash) = kernel_text_meaning(&printed).unwrap_or_else(|error| panic!("{name}: {error}"));
        let expected = kernel_normal_form(&document, &pinned()).expect("the fixture elaborates");
        assert_eq!(form, expected, "{name}: the round trip changed the normal form");

        // N6 follows from N5 by construction, so this asserts the derivation
        // rather than a second fact — and it is the value a consumer keys a
        // cache on, so a silent change to it would be the expensive kind.
        let again = kernel_text_meaning(&printed).expect("parses twice").1;
        assert_eq!(hash, again, "{name}: the hash is not a function of the text");
    }
}

/// The normalized spelling is *also* kernel text — which is the useful
/// consequence of it not being N5's bytes (see `kernel_normalized_text`).
#[test]
fn normalized_kernel_text_is_still_kernel_text() {
    for &(name, source) in EXAMPLES {
        let document = SourceDocument::new(source, name);
        let normalized = kernel_normalized_text(&document, &pinned()).expect("the fixture elaborates");
        let (form, _) = kernel_text_meaning(&normalized).unwrap_or_else(|error| panic!("{name}: {error}"));
        let expected = kernel_normal_form(&document, &pinned()).expect("the fixture elaborates");
        assert_eq!(form, expected, "{name}: normalizing changed the meaning");
    }
}

/// Structure survives printing: a piece with more than one voice prints as an
/// `overlay` of literals, not as one flattened timeline. Normalizing is the
/// caller's separate decision, and this is the observable difference.
#[test]
fn printed_kernel_text_keeps_the_terms_structure() {
    let document = SourceDocument::new(include_str!("../../../examples/counterpoint.musa"), "counterpoint");
    let printed = kernel_text(&document, &pinned()).expect("elaborates");
    assert!(
        printed.contains("overlay {"),
        "the overlay was flattened away: {printed}"
    );
    let normalized = kernel_normalized_text(&document, &pinned()).expect("elaborates");
    assert!(
        !normalized.contains("overlay {"),
        "the normal form is a value, not a composition: {normalized}"
    );
}

/// A hairpin's shape crosses the boundary as exact rationals, never a
/// decimal. Elaboration only builds linear shapes today; the non-dyadic
/// multi-segment case — the one where a `f64` round trip would visibly lose —
/// is `factext.rs`'s own test, because only that module can name a payload.
#[test]
fn a_hairpin_shape_survives_as_exact_rationals() {
    let source = "piece \"x\" { score { part p { voice v { crescendo to ff { c4 1/4; d4 1/4; e4 1/4; } } } } }";
    let printed = kernel_text(&SourceDocument::new(source, "hairpin"), &pinned()).expect("elaborates");
    assert!(printed.contains("hairpin cres ff "), "no hairpin printed: {printed}");
    assert!(!printed.contains('.'), "a rational was written as a decimal: {printed}");
    check_kernel_text(&printed).expect("a printed hairpin reads back");
}

/// A whole rational round-trips as a rational.
///
/// `Ratio`'s `Display` drops a denominator of one, so a beat unit of `1/1` or
/// a ramp reaching over `2/1` printed as `1` and `2` — and read back as
/// nothing at all. Caught by adding `rubato.musa` to the corpus, which is
/// what a corpus is for; pinned here so the next payload field that spells a
/// rational by hand does not lose it again.
#[test]
fn a_whole_rational_survives_the_round_trip() {
    let source = "piece \"p\" { tempo 1/1 = 60 to 30 over 2/1 \"rit.\"; meter 4/4;
        score { part a { voice b { c5 1; c5 1; c5 1; } } } }";
    let document = SourceDocument::new(source, "whole.musa");
    let printed = kernel_text(&document, &pinned()).expect("elaborates");
    assert!(printed.contains("tempo 1=60 'rit.' to 30 over 2 "), "{printed}");
    check_kernel_text(&printed).expect("a printed ramp reads back");
}

/// Kernel text is read, not trusted: a truncated file, a bad rational, and an
/// unbound name are each rejected rather than evaluated to something.
#[test]
fn malformed_kernel_text_is_rejected() {
    let good = kernel_text(
        &SourceDocument::new(include_str!("../../../examples/twinkle.musa"), "twinkle"),
        &pinned(),
    )
    .expect("elaborates");
    let cases = [
        ("empty", String::new()),
        ("no header", good.replacen("% musa-kernel-1\n", "", 1)),
        ("truncated", good[..good.len() / 2].to_owned()),
        ("bad payload", good.replacen("note ", "nyote ", 1)),
    ];
    for (what, text) in cases {
        assert!(
            check_kernel_text(&text).is_err(),
            "{what}: malformed kernel text was accepted"
        );
    }
}

/// The committed corpus in `examples/kernel/` — the artifacts a second
/// implementation would be validated against, and the answer to Q6.
///
/// Plain `.musa.kernel` files rather than insta snapshots, deliberately: a
/// `.snap`
/// wraps its payload in a YAML preamble, and a corpus whose whole purpose is
/// to be read by another implementation must be readable *as kernel text*.
/// Regenerate with `UPDATE_KERNEL_GOLDENS=1 cargo test -p musa-compiler`.
#[test]
fn the_kernel_corpus_is_up_to_date() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/kernel");
    let update = std::env::var_os("UPDATE_KERNEL_GOLDENS").is_some();
    let mut cases: Vec<(String, String)> = Vec::new();
    for &(name, source) in EXAMPLES {
        let document = SourceDocument::new(source, name);
        cases.push((
            format!("{name}.musa.kernel"),
            golden(
                &format!("{name}.musa"),
                &kernel_text(&document, &pinned()).expect("the fixture elaborates"),
            ),
        ));
    }
    // One normalized golden, because the normal form is a *derived* artifact:
    // pinning every fixture's would pin the same derivation nine times.
    let canon = SourceDocument::new(include_str!("../../../examples/canon.musa"), "canon");
    cases.push((
        "canon.normal.musa.kernel".to_owned(),
        golden(
            "canon.musa, normalized",
            &kernel_normalized_text(&canon, &pinned()).expect("elaborates"),
        ),
    ));

    for (file, expected) in cases {
        let path = root.join(&file);
        if update {
            std::fs::write(&path, &expected).expect("the corpus directory is writable");
            continue;
        }
        let found = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{file}: {error}"));
        assert_eq!(found, expected, "{file} is stale — rerun with UPDATE_KERNEL_GOLDENS=1");
    }
}

/// A golden file: the printed text with a provenance line after the version
/// header, naming what produced it. Comments are trivia, so the file still
/// parses as the kernel text it is.
fn golden(source: &str, printed: &str) -> String {
    let (header, rest) = printed.split_once('\n').unwrap_or((printed, ""));
    format!("{header}\n% generated from examples/{source} by musa-compiler\n{rest}")
}
