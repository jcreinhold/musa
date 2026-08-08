//! What a second `key` and a second `clef` do.
//!
//! The two kinds share a prompt and share almost no behaviour, which is what
//! these tests are for: a key is the piece's and lands on a barline, a clef is
//! the part's and lands wherever the player's hand does. Getting either rule
//! onto the other kind would produce a page that is wrong in a way no test of
//! one kind alone would catch.

#![allow(clippy::expect_used)]

use musa_compiler::{Clef, CompileOptions, Mode, MusicalTime, PartId, Scope, SourceDocument, compile};
use num_rational::Ratio;

fn compiled(source: &str) -> musa_compiler::Compilation {
    compile(&SourceDocument::new(source, "context.musa"), &CompileOptions::default())
}

fn errors(source: &str) -> Vec<String> {
    compiled(source)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_compiler::Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

fn whole(n: i64) -> MusicalTime {
    MusicalTime::new(Ratio::from_integer(n))
}

/// Two bars in F major, then two in D minor.
const MODULATION: &str = "piece \"p\" { meter 4/4; key f major; score { part a { voice b { \
                          bar { c4 1; } bar { d4 1; } key d minor; bar { e4 1; } bar { f4 1; } \
                          } } } }";

#[test]
fn a_modulation_moves_the_key_after_it() {
    let compilation = compiled(MODULATION);
    assert!(compilation.diagnostics().is_empty(), "{:?}", compilation.diagnostics());
    let score = compilation.snapshot().expect("it compiles");
    let opening = score.key_at(Scope::Piece, whole(0)).expect("the header states one");
    assert_eq!(opening.mode(), Mode::Major);
    let later = score.key_at(Scope::Piece, whole(2)).expect("the voice states one");
    assert_eq!(later.mode(), Mode::Minor);
    // Right up to the barline the first key still stands.
    assert_eq!(
        score
            .key_at(Scope::Piece, MusicalTime::new(Ratio::new(7, 4)))
            .map(|key| key.mode()),
        Some(Mode::Major)
    );
    assert!(!score.keys().is_constant(), "a piece that modulates is not constant");
}

#[test]
fn a_modulation_that_misses_a_barline_is_refused() {
    // Half a measure in, which is where a key signature cannot be printed.
    let source = "piece \"p\" { meter 4/4; key f major; score { part a { voice b { \
                  c4 1/2; key d minor; d4 1/2; } } } }";
    assert_eq!(errors(source), vec!["a key change must land on a barline".to_owned()]);
}

#[test]
fn a_clef_may_change_mid_measure_and_a_key_may_not() {
    // The same place in the same piece: the clef is accepted, the key is not.
    // This is the asymmetry the two kinds exist to keep apart.
    let clef = "piece \"p\" { meter 4/4; score { part a { clef bass; voice b { \
                bar { c3 1/2; clef treble; c5 1/2; } } } } }";
    assert_eq!(errors(clef), Vec::<String>::new());
    let key = "piece \"p\" { meter 4/4; key c major; score { part a { voice b { \
               bar { c4 1/2; key a minor; d4 1/2; } } } } }";
    assert_eq!(errors(key), vec!["a key change must land on a barline".to_owned()]);
}

#[test]
fn a_clef_belongs_to_its_part_and_reaches_no_other() {
    let source = "piece \"p\" { meter 4/4; score { \
                  part upper { clef treble; voice one { bar { c5 1; } bar { d5 1; } } } \
                  part lower { clef bass; voice two { bar { c3 1; } clef tenor; bar { c4 1; } } } \
                  } }";
    let compilation = compiled(source);
    assert!(compilation.diagnostics().is_empty(), "{:?}", compilation.diagnostics());
    let score = compilation.snapshot().expect("it compiles");
    // The lower part changes; the upper one is untouched by it.
    assert_eq!(score.clef_at(PartId(1), whole(0)), Some(Clef::Bass));
    assert_eq!(score.clef_at(PartId(1), whole(1)), Some(Clef::Tenor));
    assert_eq!(score.clef_at(PartId(0), whole(1)), Some(Clef::Treble));
}

#[test]
fn a_context_change_inside_material_is_refused() {
    // A motif is played wherever it is used, so "from here on" has no here.
    let key = "piece \"p\" { meter 4/4; key c major; \
               motif m() { key a minor; c4 1; } score { part a { voice b { use m(); } } } }";
    assert_eq!(
        errors(key),
        vec!["a key change belongs to the piece, not to material".to_owned()]
    );
    let clef = "piece \"p\" { meter 4/4; \
                motif m() { clef bass; c4 1; } score { part a { voice b { use m(); } } } }";
    assert_eq!(
        errors(clef),
        vec!["a clef change belongs to the piece, not to material".to_owned()]
    );
}

#[test]
fn two_voices_may_both_name_the_same_modulation_and_may_not_disagree() {
    let agreeing = "piece \"p\" { meter 4/4; key c major; score { part a { \
                    voice one { bar { c4 1; } key a minor; bar { d4 1; } } \
                    voice two { bar { e4 1; } key a minor; bar { f4 1; } } } } }";
    assert_eq!(errors(agreeing), Vec::<String>::new());
    let disagreeing = "piece \"p\" { meter 4/4; key c major; score { part a { \
                       voice one { bar { c4 1; } key a minor; bar { d4 1; } } \
                       voice two { bar { e4 1; } key g major; bar { f4 1; } } } } }";
    assert_eq!(errors(disagreeing), vec!["two keys at the same place".to_owned()]);
}
