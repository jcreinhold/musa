//! A composite literal's parts are the ones the lexer already found.
//!
//! `c#5`, `M3` and `3/8` are one token each — the lexer decides that and
//! nothing here changes it — and the parser writes each as a node over the
//! letter, the accidental run, the octave, the quality, the size, the
//! numerator and the denominator its regex found on the way past. Root
//! `AGENTS.md` names the alternative in as many words: "An adapter re-parsing
//! `3/8` out of a token's spelling is the shape of the mistake."
//!
//! Three things have to stay true for that to be a fact and not a claim. The
//! parts must *partition* the lexeme, so that a lossless tree is still
//! lossless; each must carry the range it occupies in the source, so that a
//! consumer can point at one; and the formatter must write the lexeme back
//! unchanged, because a node whose children took spaces between them would be
//! three things to the lexer that read it again.

#![allow(clippy::expect_used, clippy::panic)]

use musa_syntax::{BarSpacing, SyntaxKind, SyntaxNode, format, parse};

/// The corpus: both accidental runs and the natural, a negative octave, the
/// five interval qualities that spell differently, and two rationals.
///
/// `b2`, `bb2` and `bbb2` are the reason the letter comes off first — `b` is
/// a letter and a flat and the two never collide. `d2` and `dim7` are the
/// other collision the lexer's comment names, from the interval side.
const CORPUS: &[&str] = &[
    "b2", "bb2", "bbb2", "cn4", "c#-1", "M3", "dim7", "AA4", "d2", "3/8", "12/16",
];

/// A piece whose one binding is `written`, and the literal node in it.
fn literal(written: &str) -> SyntaxNode {
    let source = format!("piece \"parts\" {{\n    let it = {written};\n}}\n");
    let document = parse(&source);
    assert!(document.errors().is_empty(), "{written}: {:?}", document.errors());
    document
        .syntax()
        .descendants()
        .find(|node| node.kind().is_composite_literal())
        .unwrap_or_else(|| panic!("{written} is written as a composite literal"))
}

/// The lexeme is exactly its parts, in order, with nothing dropped and
/// nothing invented.
#[test]
fn a_literal_is_the_concatenation_of_its_parts() {
    for written in CORPUS {
        let node = literal(written);
        let joined: String = node
            .children_with_tokens()
            .filter_map(musa_syntax::SyntaxElement::into_token)
            .map(|part| part.text().to_owned())
            .collect();
        assert_eq!(&joined, written, "{written}: the parts do not spell the lexeme");
        assert_eq!(
            node.text().to_string(),
            *written,
            "{written}: the node's text is not the lexeme"
        );
    }
}

/// A part carries the range it occupies in the source, so a consumer can
/// point at one. Consecutive, non-empty, and inside the literal's own range:
/// a part that stood at no text would be a node standing for nothing.
#[test]
fn a_part_carries_the_range_it_occupies() {
    for written in CORPUS {
        let node = literal(written);
        let whole = node.text_range();
        let mut next = whole.start();
        for part in node
            .children_with_tokens()
            .filter_map(musa_syntax::SyntaxElement::into_token)
        {
            let range = part.text_range();
            assert_eq!(range.start(), next, "{written}: a gap before `{}`", part.text());
            assert!(range.end() > range.start(), "{written}: an empty part");
            next = range.end();
        }
        assert_eq!(next, whole.end(), "{written}: the parts stop short of the lexeme");
    }
}

/// Each part is one of the seven kinds, and each literal writes the shape its
/// regex describes. A split that disagreed with the regex would produce a
/// node whose text is still the token's text — losslessness does not catch it
/// — and a part that is a lie.
#[test]
fn the_parts_are_the_ones_the_regex_found() {
    let kinds = |written: &str| -> Vec<(SyntaxKind, String)> {
        literal(written)
            .children_with_tokens()
            .filter_map(musa_syntax::SyntaxElement::into_token)
            .map(|part| (part.kind(), part.text().to_owned()))
            .collect()
    };
    assert_eq!(
        kinds("bb2"),
        [
            (SyntaxKind::PitchLetter, "b".to_owned()),
            (SyntaxKind::PitchAccidental, "b".to_owned()),
            (SyntaxKind::PitchOctave, "2".to_owned()),
        ],
        "`bb2` is B flat: the letter comes off first"
    );
    assert_eq!(
        kinds("b2"),
        [
            (SyntaxKind::PitchLetter, "b".to_owned()),
            (SyntaxKind::PitchOctave, "2".to_owned()),
        ],
        "`b2` is B: no accidental, and no empty child standing for one"
    );
    assert_eq!(
        kinds("c#-1"),
        [
            (SyntaxKind::PitchLetter, "c".to_owned()),
            (SyntaxKind::PitchAccidental, "#".to_owned()),
            (SyntaxKind::PitchOctave, "-1".to_owned()),
        ],
        "the octave's sign belongs to the octave"
    );
    assert_eq!(
        kinds("dim7"),
        [
            (SyntaxKind::IntervalQuality, "dim".to_owned()),
            (SyntaxKind::IntervalSize, "7".to_owned()),
        ],
        "the split is at the first digit"
    );
    assert_eq!(
        kinds("12/16"),
        [
            (SyntaxKind::RationalNumerator, "12".to_owned()),
            (SyntaxKind::Slash, "/".to_owned()),
            (SyntaxKind::RationalDenominator, "16".to_owned()),
        ],
        "the bar is its own token between the two numbers"
    );
}

/// Every kind [`SyntaxKind::is_composite_literal`] admits really is written
/// with parts, and no other kind is.
///
/// The two live apart on purpose — the predicate is what every *reader* asks
/// and the split is what the *parser* does — so a fourth literal admitted by
/// one and not the other would reach the tree as a node with the whole text as
/// its only child, which nothing else here would notice.
#[test]
fn the_kinds_that_say_they_have_parts_are_the_ones_written_with_parts() {
    let composite: Vec<SyntaxKind> = SyntaxKind::all().filter(|kind| kind.is_composite_literal()).collect();
    assert_eq!(
        composite,
        [
            SyntaxKind::Rational,
            SyntaxKind::PitchLiteral,
            SyntaxKind::IntervalLiteral
        ],
        "the three composite literals"
    );
    for written in CORPUS {
        let node = literal(written);
        assert!(
            node.kind().is_composite_literal(),
            "{written} was written as `{:?}`",
            node.kind()
        );
        assert!(
            node.children_with_tokens().count() > 1 || node.text().to_string().len() == 1,
            "{written}: a composite literal with one child has no parts"
        );
    }
}

/// The formatter writes the lexeme back unchanged.
///
/// This is the law the parts are most able to break: a node's children are
/// written with a space between them everywhere else in the tree, and `c # 5`
/// is three things to the lexer that would read it again.
#[test]
fn the_formatter_writes_a_literal_back_as_one_word() {
    for written in CORPUS {
        let source = format!("piece \"parts\" {{\n    let it = {written};\n}}\n");
        let formatted = format(&parse(&source), BarSpacing::default()).text().to_owned();
        assert_eq!(formatted, source, "{written}: the formatter moved a byte");
    }
}

/// The kinds that say they are parts are the ones the splitter mints.
///
/// The third answer a consumer that partitions the kinds needs — a lexer
/// token, a parser node, or a part — and the one a hand-written list can fall
/// behind on. `Slash` is under a `Rational` and is *not* a part: it is a token
/// the lexer produces in its own right, and a consumer that treated it as
/// minted would be told the wrong thing about a `/` anywhere else.
#[test]
fn the_kinds_that_say_they_are_parts_are_the_ones_the_splitter_mints() {
    let mut minted: Vec<SyntaxKind> = Vec::new();
    for written in CORPUS {
        for part in literal(written).children_with_tokens() {
            let kind = part.kind();
            assert!(
                kind.is_literal_part() || musa_syntax::TokenClass::of(kind).is_some(),
                "{written}: `{kind:?}` is neither a part nor a kind the lexer emits"
            );
            if kind.is_literal_part() && !minted.contains(&kind) {
                minted.push(kind);
            }
        }
    }
    minted.sort_by_key(|kind| u16::from(*kind));
    let claimed: Vec<SyntaxKind> = SyntaxKind::all().filter(|kind| kind.is_literal_part()).collect();
    assert_eq!(
        minted, claimed,
        "a kind claims to be a part the splitter never mints, or mints one it does not claim"
    );
}
