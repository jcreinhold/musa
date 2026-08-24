//! Formatter laws and layout snapshots.
//!
//! Laws (roadmap §17.3): `format(format(x)) == format(x)` and
//! `parse(format(parse(x)))` equals `parse(x)` up to whitespace trivia and a
//! list's own trailing comma, which is layout rather than program — see
//! [`significant_tokens`]. Both are tested as `proptest` properties over a
//! grammar-directed generator plus the example corpus.

// A fixture that does not hold what a test looks for is the test failing, so
// panicking on one is the assertion rather than an oversight — including
// reaching for the nth event of a bar this file wrote n events into. Counting
// events and columns in a line the formatter just wrote cannot overflow.
#![allow(clippy::panic)]
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::arithmetic_side_effects)]

use musa_syntax::{BarSpacing, ParsedDocument, SyntaxElement, SyntaxKind, beat_groups, format, parse};
use proptest::prelude::*;

const GLASS_MOUNTAIN: &str = include_str!("../../../../examples/glass-mountain.musa");
const INVENTION: &str = include_str!("../../../../examples/invention.musa");
const TUPLET_FIXTURE: &str = include_str!("../../../../examples/tuplet-fixture.musa");
const BULGARIAN: &str = include_str!("../../../../examples/bulgarian.musa");

fn fmt(source: &str) -> String {
    format(&parse(source), BarSpacing::Compact).text().to_string()
}

/// The same, laid out with every event at a column proportional to when it
/// sounds.
fn fmt_to_scale(source: &str) -> String {
    format(&parse(source), BarSpacing::Proportional).text().to_string()
}

/// The significant (non-whitespace) token sequence of a document: the
/// "same program up to formatting" oracle.
///
/// A comma directly in front of a `)` or a `]` is not significant, and that is
/// the whole of the exemption. It separates nothing, so a list means exactly
/// what it meant without it; it is punctuation the *layout* writes, the way an
/// indent is, and the formatter adds it when a list opens down the page and
/// drops it when the list joins back onto one line. Every other comma is a
/// separator between two items and changes the program, so a formatter that
/// lost one is still caught here.
fn significant_tokens(doc: &ParsedDocument) -> Vec<(SyntaxKind, String)> {
    let written: Vec<(SyntaxKind, String)> = doc
        .syntax()
        .descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| token.kind() != SyntaxKind::Whitespace)
        .map(|token| (token.kind(), token.text().to_string()))
        .collect();
    written
        .iter()
        .enumerate()
        .filter(|(index, (kind, _))| {
            *kind != SyntaxKind::Comma
                || !matches!(
                    written.get(index + 1).map(|(next, _)| *next),
                    Some(SyntaxKind::RParen | SyntaxKind::RBracket)
                )
        })
        .map(|(_, token)| token.clone())
        .collect()
}

fn assert_semantics_preserved(source: &str, formatted: &str) {
    assert_eq!(
        significant_tokens(&parse(source)),
        significant_tokens(&parse(formatted)),
        "formatting must not change the program"
    );
}

#[test]
fn examples_format_to_themselves() {
    assert_eq!(fmt(GLASS_MOUNTAIN), GLASS_MOUNTAIN);
    assert_eq!(fmt(INVENTION), INVENTION);
    assert_eq!(fmt(TUPLET_FIXTURE), TUPLET_FIXTURE);
    // The file this layout was written for: fifteen bars, and a 2+2+3 that is
    // in the whitespace rather than in a comment apologising for its absence.
    assert_eq!(fmt(BULGARIAN), BULGARIAN);
}

/// An import is one line whatever it names, and the alias is part of it.
///
/// The `::` in a module path is the one colon the formatter does not put a
/// space after, and `as` is an ordinary word between two names; neither is
/// worth a rule of its own, which is exactly what this pins.
#[test]
fn an_import_formats_to_one_line_with_its_alias() {
    let source = "piece \"Aliased\"{\nimport std::core as basics;\nimport   \"../lib.musa\"   as shared;\n}\n";
    let formatted = fmt(source);
    assert_eq!(
        formatted,
        "piece \"Aliased\" {\n    import std::core as basics;\n    import \"../lib.musa\" as shared;\n}\n"
    );
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
}

#[test]
fn messy_source_is_canonicalized() {
    let source = "piece   \"Messy\"{\n\ttempo quarter=72;\n\n\nmeter 4/4;\nkey a minor;\nscore{\npart p{\nclef treble;\nvoice v{\nc5   1/4\n[a3   c4  e4]/2\nrest 1/4\n}\n}\n}\n}\n";
    let formatted = fmt(source);
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
    insta::assert_snapshot!(formatted);
}

/// A short-form duration is part of the note's word, and the formatter leaves
/// both spellings exactly as the composer wrote them: `format` rewrites
/// whitespace, so it cannot prefer one over the other.
#[test]
fn both_spellings_of_a_duration_survive_formatting() {
    let source = "piece \"D\" {\n    score {\n        part p {\n            voice v {\n                c5/4\n                d5/4.\n                e5/4..\n                f5 1/4\n                rest/8\n                g5/4 to 1\n            }\n        }\n    }\n}\n";
    let formatted = fmt(source);
    assert_eq!(formatted, source, "the shapes are already canonical");
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
}

/// Spacing is not lost around a short duration when the source has too much
/// of it, and the dot never swallows the note after it.
#[test]
fn a_short_duration_closes_up_to_its_note() {
    let source = "piece \"D\" { score { part p { voice v { c5 / 4 .   d5/4 } } } }";
    let formatted = fmt(source);
    assert!(formatted.contains("c5/4."), "got:\n{formatted}");
    assert!(formatted.contains("d5/4"), "got:\n{formatted}");
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
}

/// A type parameter closes up to its type on both sides, however loosely it
/// was written and however deep it nests. `Option<Pitch>` is one word the way
/// `c5/4.` is one note — a gap after `<` reads as a comparison, which is the
/// one thing the character never means here.
#[test]
fn a_type_parameter_closes_up_to_its_type() {
    let source = "piece \"T\" { let held: Option < Pitch > = None;\nlet many: List < Option < Pitch > > = []; }";
    let formatted = fmt(source);
    assert!(formatted.contains("Option<Pitch>"), "got:\n{formatted}");
    assert!(formatted.contains("List<Option<Pitch>>"), "got:\n{formatted}");
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
}

#[test]
fn comments_keep_their_attachment() {
    let source = "piece \"C\" {\n    // header comment\n    meter 4/4; // trailing\n    score {\n        part p {\n            voice v {\n                c5/4 /* inline block */\n                // detached comment\n                d5/4\n            }\n        }\n    }\n}\n";
    let formatted = fmt(source);
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
    insta::assert_snapshot!(formatted);
}

#[test]
fn blank_lines_are_capped_at_one() {
    let source = "piece \"B\" {\n    meter 4/4;\n\n\n\n    key c major;\n\n\n    score {\n        part p {\n            voice v {\n                c5/1\n            }\n        }\n    }\n}\n";
    let formatted = fmt(source);
    assert!(!formatted.contains("\n\n\n"), "no double blank lines");
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
    insta::assert_snapshot!(formatted);
}

/// The one formatting exception: a bar is a horizontal thing.
///
/// Four cases in one fixture, because what matters is that they sit next to
/// each other and still read as four bars: a bar written stacked comes back
/// on one line, a bar already on one line stays there, a bar too wide for the
/// line breaks like any other block, and a bar carrying a comment breaks
/// because a comment needs a line of its own.
#[test]
fn a_bar_that_fits_is_written_on_one_line() {
    let source = "piece \"Bars\" {\n    meter 4/4;\n    score {\n        part p {\n            voice v {\n                bar head {\n                    c4/4\n                    d4/4\n                    e4/4\n                    f4/4\n                }\n                bar { g4/2 a4/2 }\n                bar { c4/16 d4/16 e4/16 f4/16 g4/16 a4/16 b4/16 c5/16 d5/16 e5/16 f5/16 g5/16 a5/16 b5/16 c6/8 }\n                bar {\n                    // the last one\n                    c4/1\n                }\n            }\n        }\n    }\n}\n";
    let formatted = fmt(source);
    assert!(
        formatted.contains("                bar head { c4/4 d4/4 e4/4 f4/4 }\n"),
        "a stacked bar comes back onto its line:\n{formatted}"
    );
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
    insta::assert_snapshot!(formatted);
}

// --- Beat groups in the whitespace ----------------------------------------

/// The bar the formatter writes for `bar` under `meter`, as one line.
///
/// A bar too wide for the line wraps, and it wraps *at* a group boundary and
/// nowhere else — so rejoining the continuation lines with the wide gap the
/// wrap stood in for gives back the bar the formatter drew.
fn bar_line(meter: &str, bar: &str) -> String {
    let source = format!("piece \"G\" {{ meter {meter}; score {{ part p {{ voice v {{ {bar} }} }} }} }}");
    let formatted = fmt(&source);
    let mut line: Option<String> = None;
    for written in formatted.lines() {
        let indent = written.len().saturating_sub(written.trim_start().len());
        match line.as_mut() {
            None if written.trim_start().starts_with('|') => line = Some(written.trim().to_owned()),
            Some(collected) if indent == 18 => {
                collected.push_str("  ");
                collected.push_str(written.trim());
            }
            Some(_) => break,
            None => {}
        }
    }
    assert_eq!(fmt(&formatted), formatted, "idempotence:\n{formatted}");
    line.unwrap_or_else(|| panic!("a bar line:\n{formatted}"))
}

/// Where the wide gaps fall in a bar line, counted in events.
///
/// A double space shows up as an empty piece between two events, so the number
/// of events already seen when one appears is the index of the event it opens
/// a group for.
fn gaps(line: &str) -> Vec<usize> {
    let body = line.strip_prefix("| ").unwrap_or(line);
    let mut gaps = Vec::new();
    let mut events = 0_usize;
    for piece in body.split(' ') {
        if piece.is_empty() {
            gaps.push(events);
        } else {
            events += 1;
        }
    }
    gaps
}

/// The gaps a measurable bar takes are exactly `beat_groups`' boundaries.
///
/// Stated against the function rather than against a table of expected lines,
/// because that is the claim worth making: the formatter and the engraver read
/// one answer, so a bar of 7/8 cannot be spaced 2+2+3 and beamed 1+1+1+1+1+1+1.
///
/// Each bar is written as two notes per meter unit, so every group holds more
/// than one event and there is a grouping to see.
#[test]
fn the_gaps_are_exactly_the_beat_group_boundaries() {
    for (numerator, denominator) in [
        (7_u32, 8_u32),
        (6, 8),
        (5, 8),
        (9, 8),
        (12, 8),
        (4, 4),
        (3, 4),
        (5, 4),
        (2, 2),
    ] {
        let unit = denominator * 2;
        let notes: Vec<String> = (0..numerator * 2).map(|_| format!("c4/{unit}")).collect();
        let line = bar_line(&format!("{numerator}/{denominator}"), &format!("| {}", notes.join(" ")));

        let groups = beat_groups(numerator, denominator);
        let mut expected = Vec::new();
        let mut counted = 0_u32;
        for group in groups.iter().take(groups.len().saturating_sub(1)) {
            counted += group;
            expected.push((counted * 2) as usize);
        }
        assert_eq!(gaps(&line), expected, "{numerator}/{denominator}: {line}");
    }
}

/// A group of one is not a group, so a bar whose every beat holds one note is
/// written with single spaces. A beam that beams one note is not a beam.
#[test]
fn a_bar_of_one_note_a_beat_is_not_grouped() {
    assert_eq!(bar_line("4/4", "| c4/4 d4/4 e4/4 f4/4"), "| c4/4 d4/4 e4/4 f4/4");
    assert_eq!(bar_line("7/8", "| c4/4 d4/4 e4/4."), "| c4/4 d4/4 e4/4.");
}

/// Everything the formatter cannot measure is spaced with single spaces.
///
/// The whitelist, seen from outside: a motif's length is a compiler fact, a
/// duration parameter is bound somewhere else, a tuplet writes its own time,
/// and `improvise` frames music that is not notated. None of them can be drawn
/// to the beat, and a grouping drawn anyway would be a claim about music the
/// formatter has not read.
#[test]
fn an_unmeasurable_bar_keeps_single_spaces() {
    for bar in [
        "| c4/8 d4/8 use m() e4/8 f4/8 g4/8 a4/8",
        "| c4/8 d4/8 tuplet 3/2 { e4/8 f4/8 g4/8 } a4/8 b4/8 c5/8",
        "| c4/8 d4/8 improvise 3/4 e4/8 f4/8",
        "| c4/8 d4/8 e4/8 f4/8 g4/8 a4/8 b4/8 c5/8 d5/8",
    ] {
        let line = bar_line("4/4", bar);
        assert_eq!(gaps(&line), Vec::<usize>::new(), "{line}");
    }
}

/// A bar that does not add up gets no grouping — the grouping would be a lie
/// about music `check_bar_duration` is about to complain of.
#[test]
fn a_bar_that_does_not_add_up_is_not_grouped() {
    for bar in [
        "| c4/8 d4/8 e4/8 f4/8 g4/8 a4/8",                // six eighths in 4/4
        "| c4/8 d4/8 e4/8 f4/8 g4/8 a4/8 b4/8 c5/8 d5/4", // nine
    ] {
        let line = bar_line("4/4", bar);
        assert_eq!(gaps(&line), Vec::<usize>::new(), "{line}");
    }
    // And with no meter in force there is nothing to group by.
    let unmetered = fmt("piece \"G\" { score { part p { voice v { | c4/8 d4/8 e4/8 f4/8 } } } }");
    assert!(unmetered.contains("| c4/8 d4/8 e4/8 f4/8\n"), "{unmetered}");
}

/// A part may state its meter after the voices it governs, so the formatter
/// pre-walks for it rather than reading left to right.
#[test]
fn a_parts_meter_governs_bars_written_above_it() {
    let source = "piece \"G\" { meter 4/4; score { part p { \
                  voice v { | c4/8 d4/8 e4/8 f4/8 g4/8 a4/8 b4/8 } meter 7/8; } } }";
    let formatted = fmt(source);
    assert!(
        formatted.contains("| c4/8 d4/8  e4/8 f4/8  g4/8 a4/8 b4/8\n"),
        "the part's 7/8 governs, not the piece's 4/4:\n{formatted}"
    );
}

/// A bar too wide for the line wraps at its groups, under the first event.
///
/// A `|` bar has no brace to break at, so `MEASURE`'s overflow path cannot be
/// "stack it like a block" — and a group split across two lines would put half
/// a beat on each.
#[test]
fn an_over_wide_bar_wraps_at_its_beat_groups() {
    let line = "| c4/16 d4/16 e4/16 f4/16 g4/16 a4/16 b4/16 c5/16 \
                d5/16 e5/16 f5/16 g5/16 a5/16 b5/16 c6/16 d6/16";
    let source = format!("piece \"G\" {{ meter 4/4; score {{ part p {{ voice v {{ {line} }} }} }} }}");
    let formatted = fmt(&source);
    assert!(
        formatted.contains(
            "                | c4/16 d4/16 e4/16 f4/16  g4/16 a4/16 b4/16 c5/16  d5/16 e5/16 f5/16 g5/16\n\
             \x20                 a5/16 b5/16 c6/16 d6/16\n"
        ),
        "{formatted}"
    );
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(&source, &formatted);
}

// --- Bars drawn to scale ---------------------------------------------

/// The column an event beginning at bar-relative time `t` may not begin
/// before: 64 columns to the whole note, so 16 to the quarter.
fn grid(numerator: u64, denominator: u64) -> usize {
    usize::try_from(numerator * 64 / denominator).expect("a bar is 64 columns wide")
}

/// Where each event of a bar drawn to scale begins, in columns from the `|`.
///
/// The events of the bars this reads are plain notes, so a run of non-space
/// characters is exactly one event and the scan needs no grammar.
fn columns_of(line: &str) -> Vec<usize> {
    let body = line
        .trim_start()
        .strip_prefix("| ")
        .unwrap_or_else(|| panic!("not a bar line: {line}"));
    let mut columns = Vec::new();
    let mut previous = ' ';
    for (column, character) in body.chars().enumerate() {
        if previous == ' ' && character != ' ' {
            columns.push(column);
        }
        previous = character;
    }
    columns
}

/// An event that sounds at bar-relative time *t* begins at column `grid(t)` or
/// later, and never earlier.
///
/// Never earlier is the whole claim: a column that is early is a column that
/// lies about when the note sounds, and a reader who has started trusting the
/// columns has no way to tell. Later is unavoidable — an event is at least as
/// wide as its own text, so a bar of sixteenths runs ahead of its own time and
/// the grid only ever catches up.
#[test]
fn an_event_begins_no_earlier_than_the_time_it_sounds() {
    for rhythm in [
        vec![("/4", (1_u64, 4_u64)); 4],
        vec![("/2", (1, 2)), ("/4", (1, 4)), ("/8", (1, 8)), ("/8", (1, 8))],
        vec![("/8", (1, 8)); 8],
        vec![("/1", (1, 1))],
        vec![("/4.", (3, 8)), ("/8", (1, 8)), ("/2", (1, 2))],
    ] {
        // Short forms only: the scan below reads a run of non-space characters
        // as one event, and `c4 3/8` is two. The long form's own columns are
        // covered by `drawing_to_scale_only_widens`, whose generator writes it.
        let events: Vec<String> = rhythm.iter().map(|&(written, _)| format!("c4{written}")).collect();
        let bar = format!("| {}", events.join(" "));
        let line = drawn_bar("4/4", &bar);
        let drawn = columns_of(&line);
        assert_eq!(drawn.len(), rhythm.len(), "{line}");

        let mut sounded = (0_u64, 1_u64);
        for (index, &(_, (numerator, denominator))) in rhythm.iter().enumerate() {
            let expected = grid(sounded.0, sounded.1);
            assert!(
                drawn[index] >= expected,
                "event {index} sounds at {}/{} so it may not begin before column {expected}:\n{line}",
                sounded.0,
                sounded.1
            );
            sounded = (sounded.0 * denominator + numerator * sounded.1, sounded.1 * denominator);
        }
    }
}

/// The one line a bar drawn to scale is written on.
fn drawn_bar(meter: &str, bar: &str) -> String {
    let source = format!("piece \"G\" {{ meter {meter}; score {{ part p {{ voice v {{ {bar} }} }} }} }}");
    let formatted = fmt_to_scale(&source);
    assert_eq!(fmt_to_scale(&formatted), formatted, "idempotence:\n{formatted}");
    formatted
        .lines()
        .find(|line| line.trim_start().starts_with("| "))
        .unwrap_or_else(|| panic!("a bar line:\n{formatted}"))
        .to_owned()
}

/// The bars a bar-per-line layout has to break are the same bars either way,
/// and a bar drawn to scale is never narrower than the same bar written
/// compactly.
///
/// This is what makes the setting safe to turn on. Because a column is at
/// least one past the end of the event before it, drawing to scale can only
/// push an event right — so a bar that fit compactly and now does not is a
/// bar that falls back to compact, and the two layouts break in the same
/// places. Turning the setting on cannot re-flow a piece, only widen it.
#[test]
fn drawing_to_scale_never_narrows_a_line_and_never_moves_a_break() {
    for source in [BULGARIAN, INVENTION, TUPLET_FIXTURE] {
        let compact = fmt(source);
        let to_scale = fmt_to_scale(source);
        let compact_lines: Vec<&str> = compact.lines().collect();
        let scaled_lines: Vec<&str> = to_scale.lines().collect();
        assert_eq!(
            compact_lines.len(),
            scaled_lines.len(),
            "the same lines break either way"
        );
        for (narrow, wide) in compact_lines.iter().zip(&scaled_lines) {
            assert!(
                wide.chars().count() >= narrow.chars().count(),
                "drawing to scale narrowed a line:\n{narrow}\n{wide}"
            );
        }
    }
}

/// A bar the formatter cannot measure is byte-identical either way.
///
/// The fallback is not "draw it approximately"; there is no approximation of a
/// time nobody knows. The same whitelist that withholds the grouping withholds
/// the grid, so the two settings differ only where the setting has something
/// true to say.
#[test]
fn an_unmeasurable_bar_is_the_same_either_way() {
    for bar in [
        "| c4/8 d4/8 use m() e4/8 f4/8 g4/8 a4/8",
        "| c4/8 d4/8 tuplet 3/2 { e4/8 f4/8 g4/8 } a4/8 b4/8 c5/8",
        "| c4/8 d4/8 improvise 3/4 e4/8 f4/8",
        "| c4/8 d4/8 e4/8 f4/8 g4/8 a4/8 b4/8 c5/8 d5/8",
        "| c4/8 d4/8 e4/8 f4/8 g4/8 a4/8",
    ] {
        let source = format!("piece \"G\" {{ meter 4/4; score {{ part p {{ voice v {{ {bar} }} }} }} }}");
        assert_eq!(fmt(&source), fmt_to_scale(&source), "{bar}");
    }
}

/// What the setting actually buys: the same beat in the same column, down the
/// page, across bars whose rhythms have nothing in common.
#[test]
fn the_same_beat_lands_in_the_same_column() {
    let source = "piece \"G\" { meter 4/4; score { part p { voice v { \
                  | c4/4 d4/4 e4/4 f4/4 \
                  | g4/2 a4/8 b4/8 c5/4 \
                  | d5/8 e5/8 f5/8 g5/8 a5/2 } } } }";
    let to_scale = fmt_to_scale(source);
    let bars: Vec<Vec<usize>> = to_scale
        .lines()
        .filter(|line| line.trim_start().starts_with("| "))
        .map(columns_of)
        .collect();
    // The event that sounds on the third beat, which each bar reaches after a
    // different number of notes.
    let halfway = [2_usize, 1, 4];
    let columns: Vec<usize> = bars.iter().zip(halfway).map(|(bar, index)| bar[index]).collect();
    assert_eq!(
        columns,
        vec![32, 32, 32],
        "half a whole note is column 32 in every bar:\n{to_scale}"
    );
}

/// A function body is one line when it fits and a stacked block when it does
/// not — the same two shapes every other braced body has, chosen by width
/// rather than by what the body happens to be.
#[test]
fn a_function_body_keeps_its_line_until_it_cannot() {
    let source = "piece \"P\" {\nfn near(x: Nat) -> Nat { add(x, x) }\nfn far(x: Nat) -> Nat { add(add(add(x, x), add(x, x)), add(add(x, x), add(add(x, x), add(x, x)))) }\n}\n";
    let formatted = fmt(source);
    assert!(
        formatted.contains("    fn near(x: Nat) -> Nat { add(x, x) }\n"),
        "a short body keeps its line:\n{formatted}"
    );
    assert!(
        formatted.contains("    fn far(x: Nat) -> Nat {\n        add("),
        "a long body stacks and indents:\n{formatted}"
    );
    assert!(
        formatted.contains("\n    }\n"),
        "and its brace takes a line:\n{formatted}"
    );
    assert_eq!(fmt(&formatted), formatted, "idempotent");
}

/// A match arm's braced body does not end the arm's line — the arm's comma
/// does.
///
/// A body written `Treble -> { … }` is an *expression* in the middle of an
/// arm, not the last thing on a line, so the `}` that closes it leaves the
/// line open for the `,` that separates this arm from the next. A brace that
/// ended the line first would strand that comma on a line of its own, which is
/// neither what was written nor a thing the formatter would then leave alone.
/// Both widths are here because a body is laid out two ways — one line when it
/// fits, stacked when it does not — and the comma follows either one.
///
/// The last arm is the same rule read from the other side: nothing follows its
/// body but the `}` of the `match`, and holding the line open for a comma that
/// was never written would close the match onto the arm. So the match's own
/// brace takes the line back.
#[test]
fn a_match_arms_braced_body_keeps_its_comma_on_its_line() {
    let source = concat!(
        "\n",
        "fn plain(written: Clef) -> Text {\n",
        "match written {\n",
        "Treble -> { text_join([\"a\", \"b\"]) },\n",
        "Tenor -> {\n",
        "let opening = \"a rather long piece of text indeed\";\n",
        "text_join([opening, \"b\", \"c\", \"d\", \"and one more after that\"])\n",
        "},\n",
        "Bass -> { text_join([\"c\", \"d\"]) }\n",
        "}\n}\n\n",
    );
    let formatted = fmt(source);
    assert!(
        formatted.contains("        Treble -> { text_join([\"a\", \"b\"]) },\n"),
        "a body that fits keeps its comma:\n{formatted}"
    );
    assert!(
        formatted.contains("        },\n        Bass ->"),
        "and so does one that stacked:\n{formatted}"
    );
    assert!(
        !formatted.contains("\n        ,"),
        "no comma is stranded on a line of its own:\n{formatted}"
    );
    assert!(
        formatted.contains("        Bass -> { text_join([\"c\", \"d\"]) }\n    }\n"),
        "the last arm has no comma, so the match's brace takes the line:\n{formatted}"
    );
    assert_eq!(fmt(&formatted), formatted, "idempotent");
    assert_semantics_preserved(source, &formatted);
}

/// Notation stays vertical. A `music` value in a body is a voice's worth of
/// statements, and this language writes only a *bar* horizontally.
#[test]
fn a_music_body_stacks_however_short_it_is() {
    let source = "piece \"P\" {\nfn figure() -> EventTrack<WrittenTime> { music { c5/4 } }\n}\n";
    let formatted = fmt(source);
    assert!(
        formatted.contains(
            "    fn figure() -> EventTrack<WrittenTime> {\n        music {\n            c5/4\n        }\n    }\n"
        ),
        "{formatted}"
    );
    assert_eq!(fmt(&formatted), formatted, "idempotent");
}

/// A declaration is a list of cases read downwards, the way a `match` is, so
/// each constructor takes a line and the brace that closes the declaration
/// takes its own — including when the last constructor was written without a
/// trailing comma. The commas *inside* a constructor separate its fields,
/// which are one word's worth of a line each, and stay horizontal.
#[test]
fn a_declaration_writes_one_constructor_to_a_line() {
    let source =
        "\ndata Shape { Silence, Sounded(sounded: Pitch, held: Duration), Then(first: Shape, second: Shape) }\n\n";
    let formatted = fmt(source);
    assert_eq!(
        formatted,
        concat!(
            "data Shape {\n",
            "    Silence,\n",
            "    Sounded(sounded: Pitch, held: Duration),\n",
            "    Then(first: Shape, second: Shape)\n",
            "}\n",
        ),
        "{formatted}"
    );
    assert_eq!(fmt(&formatted), formatted, "idempotent");
    assert_semantics_preserved(source, &formatted);
}

/// A parameter list binds to the name it abstracts — `Pair<A, B>`, never
/// `Pair <A, B>` — and a declaration with one constructor is still a list.
#[test]
fn a_parameterized_declaration_keeps_its_parameters_on_its_name() {
    let source = "\ndata Pair < A , B > { Both ( left : A , right : B ) , }\n\n";
    let formatted = fmt(source);
    assert_eq!(
        formatted,
        concat!("data Pair<A, B> {\n", "    Both(left: A, right: B),\n", "}\n",),
        "{formatted}"
    );
    assert_eq!(fmt(&formatted), formatted, "idempotent");
    assert_semantics_preserved(source, &formatted);
}

/// The longest line the formatter budgets for. Written here as the number the
/// tests below assert about, so a test that says "too wide" is saying it
/// against the same measure the formatter uses.
const MEASURE: usize = 96;

fn widest_line(text: &str) -> usize {
    text.lines().map(|line| line.chars().count()).max().unwrap_or_default()
}

/// A constructor's fields are a list, and a list is horizontal until it is
/// long. The one that fits keeps its line; the one that does not is read
/// downwards, rather than joined into a line nothing can read across.
#[test]
fn a_constructor_too_wide_for_its_line_stacks_its_fields() {
    let source = concat!(
        "\n",
        "data Staff {\n",
        "Bar(anchor: Nat, beats: Meter),\n",
        "Document(instrument: Text, sounding_shift: Interval, written_clef: Clef, ",
        "written_key: Key, beats: Meter, spelling: Spelling, items: StaffItem,),\n",
        "}\n\n",
    );
    let formatted = fmt(source);
    assert!(
        formatted.contains("    Bar(anchor: Nat, beats: Meter),\n"),
        "a constructor that fits keeps its line:\n{formatted}"
    );
    assert!(
        formatted.contains("    Document(\n        instrument: Text,\n"),
        "and one that does not takes a line per field:\n{formatted}"
    );
    assert!(
        formatted.contains("        items: StaffItem,\n    ),\n"),
        "with the closing paren back at the constructor's own indent:\n{formatted}"
    );
    assert!(widest_line(&formatted) <= MEASURE, "{formatted}");
    assert_eq!(fmt(&formatted), formatted, "idempotent");
    assert_semantics_preserved(source, &formatted);
}

/// The budget is the *line*, not the list. A parameter list that would fit on
/// its own is still too wide when the return type written after it does not
/// fit behind it — which is the whole of why a list is measured with
/// everything up to the next place the line can be cut.
#[test]
fn a_parameter_list_is_measured_with_what_follows_it() {
    let source = concat!(
        "\n",
        "fn rescaled(factor: Ratio, here: Position<WrittenTime>, point: Position<WrittenTime>,) ",
        "-> Result<Position<WrittenTime>, Text> { point }\n",
        "\n",
    );
    let formatted = fmt(source);
    assert!(
        formatted.contains("fn rescaled(\n    factor: Ratio,\n"),
        "the head is too long for one line, so its parameters stack:\n{formatted}"
    );
    assert!(
        formatted.contains(") -> Result<Position<WrittenTime>, Text> { point }\n"),
        "and the return type stays with the paren that closes them:\n{formatted}"
    );
    assert!(widest_line(&formatted) <= MEASURE, "{formatted}");
    assert_eq!(fmt(&formatted), formatted, "idempotent");
    assert_semantics_preserved(source, &formatted);
}

/// A list joined onto one line has no use for a trailing comma: it separates
/// nothing there, and the alternative to dropping it is `Beats(count: Nat, )`,
/// a gap with nothing on either side of it.
#[test]
fn a_joined_list_drops_the_comma_that_held_it_open() {
    let source = "\ndata Meter {\nBeats(count: Nat, unit: Nat,),\n}\n\n";
    let formatted = fmt(source);
    assert!(formatted.contains("    Beats(count: Nat, unit: Nat),\n"), "{formatted}");
    // The comma after the *variant* is the declaration's, not the list's: it
    // separates this variant from the next one that could be written under it.
    assert_eq!(fmt(&formatted), formatted, "idempotent");
    assert_semantics_preserved(source, &formatted);
}

/// A list written down the page ends with a trailing comma however short the
/// item that comes last happens to be.
///
/// The rule is about the layout and not about widths, and this is the shape
/// that proves it has to be: five sibling calls in `stdlib/src/adapters/staff.musa`
/// each stack their four arguments, and the one whose last argument was
/// shortest was the one that came back spelled differently from its siblings —
/// because the comma was being read from the source, where nobody had a reason
/// to keep five of them in step by hand.
#[test]
fn a_broken_list_ends_with_a_comma_however_short_its_last_item_is() {
    let call = concat!(
        "widening(a_long_enough_first_argument_right_here, ",
        "a_long_enough_second_argument_as_well_here, c)"
    );
    let source = format!("\nfn a() -> Nat {{ {call} }}\n\n");
    let formatted = fmt(&source);
    assert!(
        formatted.contains("        a_long_enough_second_argument_as_well_here,\n        c,\n    )\n"),
        "the list is too wide for one line, so it stacks and its last item takes a comma:\n{formatted}"
    );
    // And the same list with the comma already written formats identically:
    // the two spellings a corpus drifts into converge on one.
    let spelled = source.replace(", c)", ", c,)");
    assert_eq!(fmt(&spelled), formatted, "both spellings converge");
    assert_eq!(fmt(&formatted), formatted, "idempotent");
    assert_semantics_preserved(&source, &formatted);
    assert_semantics_preserved(&spelled, &formatted);
}

/// A bracketed literal in the last position opens on the line of the call that
/// holds it, the way a body's brace does. Stacking the call instead would put
/// three lines around a bracket that was going to open one anyway — and would
/// do it again at every level of a nested tree.
#[test]
fn a_trailing_bracket_opens_on_the_line_of_its_call() {
    let source = concat!(
        "\n",
        "let tree: Syntax = syntax_group(syntax_built(here, 9, 0), \"parentheses\", [",
        "syntax_identifier(syntax_built(here, 4, 0), \"repeat\"), ",
        "syntax_token(syntax_built(here, 8, 0), \"Integer\", \"2\")]);\n",
        "\n",
    );
    let formatted = fmt(source);
    assert!(
        formatted.contains("syntax_group(syntax_built(here, 9, 0), \"parentheses\", [\n"),
        "the call keeps its line and the bracket opens on it:\n{formatted}"
    );
    assert!(
        formatted.contains("\n]);\n"),
        "and the bracket closes at the indent the call started at:\n{formatted}"
    );
    assert!(widest_line(&formatted) <= MEASURE, "{formatted}");
    assert_eq!(fmt(&formatted), formatted, "idempotent");
    assert_semantics_preserved(source, &formatted);
}

#[test]
fn apply_edits_replaces_ranges_in_order() {
    use musa_syntax::{TextEdit, apply_edits};
    use text_size::{TextRange, TextSize};
    let range = |start: u32, end: u32| TextRange::new(TextSize::from(start), TextSize::from(end));
    let edits = [TextEdit::new(range(4, 7), "three "), TextEdit::new(range(0, 3), "one")];
    assert_eq!(apply_edits("one two three", &edits), "one three  three");
    // Overlapping edits are skipped, not applied destructively.
    let overlapping = [TextEdit::new(range(0, 5), "x"), TextEdit::new(range(2, 6), "y")];
    assert_eq!(apply_edits("one two three", &overlapping), "xwo three");
}

// --- Generator for the formatting laws ------------------------------------

fn pitch() -> impl Strategy<Value = String> {
    (
        prop::sample::select(vec!['a', 'b', 'c', 'd', 'e', 'f', 'g']),
        prop::sample::select(vec!["", "#", "##", "b", "bb", "n"]),
        -1i8..=8,
    )
        .prop_map(|(letter, accidental, octave)| format!("{letter}{accidental}{octave}"))
}

fn duration() -> impl Strategy<Value = String> {
    // Both spellings, because both are legal and the formatter must leave
    // either as it found it: `/12` cannot be written short, and `/4.` cannot
    // be written any other way without saying 3/8.
    prop::sample::select(vec!["/1", "/2", "/4", "/4.", "/8", "/16", "1/12", "3/8", "1/4"]).prop_map(str::to_string)
}

fn gap() -> impl Strategy<Value = String> {
    prop::sample::select(vec![" ", "  ", "\t", "\n", "\n   "]).prop_map(str::to_string)
}

fn separator() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["\n", "\n\n", "\n\n\n", "\n    "]).prop_map(str::to_string)
}

/// A statement with nothing after it on its line, so a run of them can be
/// joined with spaces without a comment eating what follows.
///
/// An event carries no terminator: `gap()` is what stands between the pitch
/// and the duration, and a short duration closes up to the pitch, so the
/// generator writes the gap only where the long form allows one.
fn statement() -> impl Strategy<Value = String> {
    prop_oneof![
        (pitch(), duration(), gap()).prop_map(|(p, d, g)| format!("{p}{}{d}", spacing(&d, &g))),
        duration().prop_map(|d| format!("rest{}{d}", spacing(&d, " "))),
        (prop::collection::vec(pitch(), 2..=4), duration()).prop_map(|(pitches, d)| format!(
            "[{}]{}{d}",
            pitches.join(" "),
            spacing(&d, " ")
        )),
    ]
}

/// What separates a pitch from its duration: nothing at all when the duration
/// is written short, and the generated whitespace when it is written long.
fn spacing<'a>(duration: &str, gap: &'a str) -> &'a str {
    if duration.starts_with('/') { "" } else { gap }
}

/// The same, plus a trailing comment and plus bars — which have their own
/// line-length rule, and are therefore the one construct whose formatting
/// depends on how wide it is. Bars do not nest, so what goes inside one is a
/// plain statement.
fn item() -> impl Strategy<Value = String> {
    prop_oneof![
        6 => statement(),
        2 => (pitch(), duration()).prop_map(|(p, d)| format!("{p}{}{d} // note", spacing(&d, " "))),
        3 => (prop::collection::vec(statement(), 1..=6), gap())
            .prop_map(|(inside, g)| format!("bar {{{g}{}{g}}}", inside.join(" "))),
        3 => (prop::collection::vec(statement(), 1..=6), gap())
            .prop_map(|(inside, g)| format!("|{g}{}", inside.join(" "))),
    ]
}

fn piece_source() -> impl Strategy<Value = String> {
    (prop::collection::vec(item(), 1..=12), prop::collection::vec(separator(), 12))
        .prop_map(|(items, separators)| {
            let mut body = String::new();
            for (index, item) in items.iter().enumerate() {
                body.push_str(item);
                if let Some(sep) = separators.get(index) {
                    body.push_str(sep);
                }
            }
            format!(
                "piece \"Gen\" {{\n    tempo quarter = 72;\n    meter 4/4;\n    key c major;\n    score {{\n        part p {{\n            voice v {{\n{body}\n            }}\n        }}\n    }}\n}}\n"
            )
        })
}

/// Either layout, so that every law below is stated about both for the price
/// of one parameter — the idempotence hazard is per-layout, and a law that ran
/// on one of them would leave the other untested.
fn bar_spacing() -> impl Strategy<Value = BarSpacing> {
    prop::sample::select(vec![BarSpacing::Compact, BarSpacing::Proportional])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn format_is_idempotent(source in piece_source(), bars in bar_spacing()) {
        let once = format(&parse(&source), bars).text().to_owned();
        let twice = format(&parse(&once), bars).text().to_owned();
        prop_assert_eq!(once, twice);
    }

    /// The two layouts write the same program, and it is the program they were
    /// given.
    ///
    /// This is the manifest invariant said at the level of the formatter: a
    /// project may choose how its bars are drawn and cannot, by choosing,
    /// change what they mean. Stated over three token sequences rather than
    /// two because a layout that quietly agreed with the other one and
    /// disagreed with the source would satisfy the weaker claim.
    #[test]
    fn the_two_spacings_write_the_same_program(source in piece_source()) {
        let written = significant_tokens(&parse(&source));
        prop_assert_eq!(&written, &significant_tokens(&parse(&fmt(&source))));
        prop_assert_eq!(&written, &significant_tokens(&parse(&fmt_to_scale(&source))));
    }

    /// A line drawn to scale is never narrower, and no break moves.
    #[test]
    fn drawing_to_scale_only_widens(source in piece_source()) {
        let compact = fmt(&source);
        let to_scale = fmt_to_scale(&source);
        let narrow: Vec<&str> = compact.lines().collect();
        let wide: Vec<&str> = to_scale.lines().collect();
        prop_assert_eq!(narrow.len(), wide.len());
        for (narrow, wide) in narrow.iter().zip(&wide) {
            prop_assert!(wide.chars().count() >= narrow.chars().count(), "{} / {}", narrow, wide);
        }
    }

    #[test]
    fn generated_sources_parse_without_errors(source in piece_source()) {
        prop_assert_eq!(parse(&source).errors().len(), 0);
    }
}

#[test]
fn examples_satisfy_the_laws() {
    for source in [GLASS_MOUNTAIN, INVENTION, BULGARIAN] {
        for formatted in [fmt(source), fmt_to_scale(source)] {
            assert_eq!(fmt(&formatted), fmt(&fmt(&formatted)), "idempotence");
            assert_semantics_preserved(source, &formatted);
        }
        assert_eq!(fmt_to_scale(&fmt_to_scale(source)), fmt_to_scale(source), "idempotence");
    }
}

/// A beat-group gap is two spaces wide in the compact layout, so the grid has
/// to reserve two as well.
///
/// The property above says the same thing over generated pieces, and found
/// this only when the generator happened to produce a bar dense enough for the
/// grid to fall back on its "one past the previous event" rule *at* a group
/// boundary. Written out, it is one bar and it fails every run.
#[test]
fn the_grid_reserves_the_beat_group_gap() {
    let source = concat!(
        "piece \"Gap\" {\n    tempo quarter = 72;\n    meter 4/4;\n    key c major;\n",
        "    score {\n        part p {\n            voice v {\n",
        "| [a0 a0]/8 [a0 a#0 a#0]/4 rest/8\na0/2\n",
        "            }\n        }\n    }\n}\n"
    );
    for (narrow, wide) in fmt(source).lines().zip(fmt_to_scale(source).lines()) {
        assert!(
            wide.chars().count() >= narrow.chars().count(),
            "drawing to scale narrowed a line:\n  compact {narrow:?}\n  to scale {wide:?}"
        );
    }
}

/// An omitted annotation is printed back omitted. The formatter writes the
/// tokens a file has, so the two spellings stay two spellings: nothing here
/// invents a type for `let held = c4;`, and nothing drops the one `halve`
/// wrote.
#[test]
fn an_omitted_annotation_is_printed_back_omitted() {
    let source = "piece \"Inferred\" { let held = c4; fn double(x) { add(x, x) } \
                  fn halve(x: Nat) -> Nat { div(x, 2) } }";
    let once = fmt(source);
    assert!(once.contains("let held = c4;"), "{once}");
    assert!(once.contains("fn double(x) {"), "{once}");
    assert!(once.contains("fn halve(x: Nat) -> Nat {"), "{once}");
    assert_eq!(fmt(&once), once, "format is idempotent on an inferred declaration");
}
