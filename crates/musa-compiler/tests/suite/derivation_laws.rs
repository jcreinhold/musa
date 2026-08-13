//! Where a score came from, over real compilations
//! (`docs/rules/across-stages/02-derivation-diagrams.md`).
//!
//! The graph's own algebra — grafting, coverage, associativity, exact
//! identity — is proved beside the type, in `musa-compiler`'s own suite. What
//! these laws add is the part no unit test can claim: that the record the
//! elaborator actually leaves behind covers every note it produced, and that
//! it tells two uses of one motif apart, which is the defect
//! `37-final-blocker.md` §3 named.

#![allow(clippy::expect_used, clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

/// Compile `source` and hand back the score and its derivation.
fn compiled(source: &str) -> (musa_compiler::ScoreSnapshot, musa_compiler::Derivation) {
    let compilation = compile(
        &SourceDocument::new(source, "derivation-laws.musa"),
        &CompileOptions::default(),
    );
    assert!(
        !compilation.has_errors(),
        "the piece must compile: {:?}",
        compilation
            .diagnostics()
            .iter()
            .map(|it| &it.message)
            .collect::<Vec<_>>()
    );
    let derivation = compilation
        .derivation()
        .cloned()
        .expect("a piece that compiled has a score, and a score has a derivation");
    let score = compilation
        .snapshot()
        .cloned()
        .expect("a piece that compiled has a score");
    (score, derivation)
}

const AUTHORED: &str = r#"piece "authored" {
    score {
        part piano {
            voice upper {
                c4/4
                d4/4
                e4/4
                f4/4
            }
        }
    }
}"#;

/// One motif, used at two sites, so that the two uses can be told apart.
const TWO_USES: &str = r#"piece "two uses" {
    motif sigh() {
        c5/4
        b4/4
    }

    score {
        part piano {
            voice upper {
                use sigh();
                d4/2
                use sigh();
            }
        }
    }
}"#;

#[test]
fn every_note_a_pass_produced_reaches_the_source() {
    for source in [AUTHORED, TWO_USES] {
        let (_, derivation) = compiled(source);
        assert!(
            derivation.complete(),
            "a note with no path to a source anchor is a defect in the pass that produced it"
        );
        assert!(
            !derivation.is_empty(),
            "a piece with notes must leave a derivation behind"
        );
    }
}

#[test]
fn an_authored_note_is_supported_by_the_place_it_is_written() {
    let (score, derivation) = compiled(AUTHORED);
    let event = score
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events().to_vec())
        .next()
        .expect("the piece has a note");
    let places = derivation.sources_of(event.id);
    assert_eq!(
        places,
        vec![event.origin.definition_span],
        "an authored note is supported by exactly the place it is written"
    );
}

#[test]
fn two_uses_of_one_motif_agree_on_the_body_and_differ_in_the_site() {
    let (score, derivation) = compiled(TWO_USES);
    let generated: Vec<_> = score
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events().to_vec())
        .filter(|event| !event.origin.expansion_path.is_empty())
        .collect();
    assert!(generated.len() >= 4, "both uses of a two-note motif produce four notes");
    // The *same* note of the body, at each of the two uses: what they share
    // is the place it is written, and what tells them apart is the site.
    let first = derivation.sources_of(generated.first().expect("a first note").id);
    let last = derivation.sources_of(generated.get(2).expect("the same note at the second use").id);
    assert!(
        first.len() >= 2,
        "an instantiated note is supported by the body it instantiates and the site that instantiated it"
    );
    assert!(
        first.iter().any(|span| last.contains(span)),
        "two uses of one motif must agree on the shared body they instantiate"
    );
    assert!(
        first.iter().any(|span| !last.contains(span)) || last.iter().any(|span| !first.contains(span)),
        "two uses of one motif must keep their two generation sites, or the uses are indistinguishable"
    );
}

#[test]
fn a_piece_with_no_score_leaves_no_derivation() {
    let compilation = compile(
        &SourceDocument::new(
            "library {\n    fn twice(x: Nat) -> Nat {\n        add(x, x)\n    }\n}",
            "material.musa",
        ),
        &CompileOptions::default(),
    );
    assert!(
        compilation.derivation().is_none(),
        "material has no score, so there is nothing to say about where one came from"
    );
}
