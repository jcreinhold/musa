//! Splitting a composite literal into the parts the lexer already found.
//!
//! `c#5` is one token, `M3` is one token, `3/8` is one token — the lexer
//! decides that, and nothing here changes it. What the lexer's regexes found
//! on the way past, and then threw away, is the letter, the accidental run,
//! the octave, the quality, the size, the numerator and the denominator. This
//! module hands them back, and the parser writes them into the tree as the
//! literal node's children.
//!
//! Every split below reads the same shape its regex in [`crate::lexer`] does,
//! and is written beside the regex it mirrors so the two can be compared. A
//! split that disagreed with the regex would produce a node whose text is
//! still the token's text — losslessness does not catch it — and a part that
//! is a lie, which is why the laws check the parts and not only the whole.

use crate::SyntaxKind;

/// The parts of a composite literal, in source order, or `None` for a kind
/// that has none.
///
/// The parts partition the text: their spellings concatenate back to `text`
/// exactly, with nothing dropped and nothing invented. That is what keeps the
/// tree lossless once the parser writes a node where a token stood.
/// The three answered here are exactly the three
/// [`SyntaxKind::is_composite_literal`] admits, which is what every reader
/// asks; a law holds the two in step, because a literal admitted there and not
/// split here would reach the tree as a node with no parts at all.
pub(super) fn parts(kind: SyntaxKind, text: &str) -> Option<Vec<(SyntaxKind, &str)>> {
    if kind == SyntaxKind::PitchLiteral {
        return Some(pitch(text));
    }
    if kind == SyntaxKind::IntervalLiteral {
        return Some(interval(text));
    }
    if kind == SyntaxKind::Rational {
        return Some(rational(text));
    }
    None
}

/// `[a-g](#+|b+|n)?-?[0-9]+` — letter, accidental run, octave.
///
/// The letter comes off first and that is what makes the accidental
/// unambiguous, for the reason the lexer's own comment gives: `b` is both a
/// letter and a flat, and the two never collide because the letter is always
/// first. So `b2` is B, `bb2` is B flat, and `bbb2` is B double flat, decided
/// here by taking one character and then a *run* of whatever follows it.
fn pitch(text: &str) -> Vec<(SyntaxKind, &str)> {
    let mut found = Vec::with_capacity(3);
    let letter = text.len().min(1);
    found.push((SyntaxKind::PitchLetter, &text[..letter]));
    let rest = &text[letter..];
    let accidental = match rest.as_bytes().first() {
        Some(&b'#') => run(rest, b'#'),
        Some(&b'b') => run(rest, b'b'),
        Some(&b'n') => 1,
        _ => 0,
    };
    // Absent where there is none: an empty child would be a node standing for
    // no text, and the tree has one of those already (`SyntaxKind::Error`)
    // meaning something else.
    if accidental > 0 {
        found.push((SyntaxKind::PitchAccidental, &rest[..accidental]));
    }
    // Sign and all — `a-1` names an octave and not a subtraction.
    found.push((SyntaxKind::PitchOctave, &rest[accidental..]));
    found
}

/// `(P|M|m|A+|d{2,}|dim)[0-9]+` — quality, size.
///
/// The split is at the first digit rather than by naming the six qualities,
/// because the regex's alternatives are all digit-free and the size is all
/// digits: one boundary decides both, and a table of qualities here would be
/// a second place for the lexer's list to drift from.
fn interval(text: &str) -> Vec<(SyntaxKind, &str)> {
    let size = text
        .bytes()
        .position(|byte| byte.is_ascii_digit())
        .unwrap_or(text.len());
    let mut found = Vec::with_capacity(2);
    if size > 0 {
        found.push((SyntaxKind::IntervalQuality, &text[..size]));
    }
    found.push((SyntaxKind::IntervalSize, &text[size..]));
    found
}

/// `[0-9]+/[0-9]+` — numerator, the bar, denominator.
fn rational(text: &str) -> Vec<(SyntaxKind, &str)> {
    let Some(bar) = text.bytes().position(|byte| byte == b'/') else {
        return vec![(SyntaxKind::RationalNumerator, text)];
    };
    let after = bar.saturating_add(1);
    vec![
        (SyntaxKind::RationalNumerator, &text[..bar]),
        (SyntaxKind::Slash, &text[bar..after]),
        (SyntaxKind::RationalDenominator, &text[after..]),
    ]
}

/// How many bytes of `text` are `byte`, from the start.
fn run(text: &str, byte: u8) -> usize {
    text.bytes().take_while(|&found| found == byte).count()
}
