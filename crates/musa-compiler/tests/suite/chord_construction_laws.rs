//! Chord classes, the triad refinement, and voicings.
//!
//! The domain values are crate-private, so the laws are stated the way a
//! composer states them: in source text, through the pitches a piece
//! elaborates to and the diagnostics it reports. A law about a private struct
//! that the surface language does not honour is not a law.
//!
//! Open Music Theory `017-triads.md` and `018-seventh-chords.md` supply the
//! spelling rule these laws check against; `019-inversion.md` supplies the
//! distinction between a root and a bass.

// The reference formula is small integer arithmetic over letters and
// semitones, indexing fixed seven-element tables. A panic here is a broken
// test rather than a broken compiler, and it names itself immediately.
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, ScoreEventKind, SourceDocument, compile};

/// Every sounding event of a compiled piece, in order, spelled as written.
/// A chord is one entry, its tones joined by spaces.
fn events(source: &str) -> Vec<String> {
    let compilation = compile(
        &SourceDocument::new(source, "chord-laws.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:#?}", compilation.diagnostics());
    compilation
        .snapshot()
        .map(|snapshot| {
            snapshot
                .parts()
                .iter()
                .flat_map(|(_, part)| part.voices())
                .flat_map(|(_, voice)| voice.events())
                .filter_map(|event| match &event.kind {
                    ScoreEventKind::Note { pitch } => Some(pitch.to_string()),
                    ScoreEventKind::Chord { pitches } => {
                        Some(pitches.iter().map(ToString::to_string).collect::<Vec<_>>().join(" "))
                    }
                    ScoreEventKind::Rest => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The errors a piece reports, as one string.
fn errors(source: &str) -> String {
    let compilation = compile(
        &SourceDocument::new(source, "chord-laws.musa"),
        &CompileOptions::default(),
    );
    compilation
        .diagnostics()
        .iter()
        .map(|diagnostic| format!("{diagnostic:?}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A piece with declarations before the score and `body` in its one voice.
fn piece_with(declarations: &str, body: &str) -> String {
    format!("piece \"Laws\" {{\n{declarations}\n    score {{ part p {{ voice v {{\n{body}\n    }} }} }}\n}}\n")
}

/// A piece whose single voice holds `body`.
fn piece(body: &str) -> String {
    piece_with("", body)
}

/// The declarations every law below shares: how a voicing is sounded, and how
/// a pitch class is made audible.
///
/// A pitch class chooses no register, so it cannot be written as a note. It
/// is observed instead through a scale rooted on it: a C-rooted major
/// collection takes a register frame on `c4` and no other collection does, so
/// the pitch that comes back names the class that went in.
const PRELUDE: &str = "\
    import std::collections;
    import std::harmony;
    import std::scale;
    import std::voicing;

    fn held(chosen: Voicing) -> Music { play(chosen, 1/1) }
    fn sounded(chosen: Option<Voicing>) -> Music { option_fold(music { rest/1 }, held, chosen) }
    fn tonic_of(register: Frame) -> Music { music { (frame_degree(register, 1))/1 } }
    fn named(root: NoteName) -> Music { option_fold(music { rest/1 }, tonic_of, frame_on(major_on(root), c4)) }
";

// --- The reference spelling formula -----------------------------------------
//
// Chord spelling is a diatonic letter stack plus a chromatic size, and it is
// derived here from those two numbers alone. Nothing below reads the
// compiler's table; the table is what is being checked.

/// Semitones above C of each natural letter, `c` through `b`.
const NATURAL: [i64; 7] = [0, 2, 4, 5, 7, 9, 11];

/// The `(diatonic steps, semitones)` above the root of the chord types under
/// test. This is the definition of each type; the spelling derived from it is
/// what the law compares.
const REFERENCE: [(&str, &[(i64, i64)]); 9] = [
    ("major", &[(0, 0), (2, 4), (4, 7)]),
    ("minor", &[(0, 0), (2, 3), (4, 7)]),
    ("diminished", &[(0, 0), (2, 3), (4, 6)]),
    ("augmented", &[(0, 0), (2, 4), (4, 8)]),
    ("major7", &[(0, 0), (2, 4), (4, 7), (6, 11)]),
    ("dominant7", &[(0, 0), (2, 4), (4, 7), (6, 10)]),
    ("minor7", &[(0, 0), (2, 3), (4, 7), (6, 10)]),
    ("half_diminished7", &[(0, 0), (2, 3), (4, 6), (6, 10)]),
    ("diminished7", &[(0, 0), (2, 3), (4, 6), (6, 9)]),
];

/// The written pitch `steps` letters and `semitones` above `root`, spelled.
///
/// The letter is fixed by the diatonic stack and the accidental is whatever
/// it takes to reach the chromatic size — which is why a major third is a
/// third with an accidental, never a fourth that happens to sound the same.
fn member(root: &str, steps: i64, semitones: i64) -> String {
    let (letter, alteration, octave) = read(root);
    let raised = letter + steps;
    let index = raised.rem_euclid(7);
    let octave_above = octave + raised.div_euclid(7);
    let wanted = 12 * octave + NATURAL[index_of(letter)] + alteration + semitones;
    let natural = 12 * octave_above + NATURAL[index_of(index)];
    write(index, wanted - natural, octave_above)
}

/// A letter index as a slice index.
fn index_of(letter: i64) -> usize {
    usize::try_from(letter).expect("a letter index in range")
}

/// Read a written pitch into `(letter index, alteration, octave)`.
fn read(pitch: &str) -> (i64, i64, i64) {
    let mut characters = pitch.chars();
    let letter = characters.next().expect("a written pitch starts with a letter");
    let letter = i64::from(u32::from(letter)) - i64::from(u32::from('c'));
    let letter = letter.rem_euclid(7);
    let rest: String = characters.collect();
    let sharps = rest.chars().take_while(|character| *character == '#').count();
    let flats = rest.chars().take_while(|character| *character == 'b').count();
    let alteration = i64::try_from(sharps).expect("few sharps") - i64::try_from(flats).expect("few flats");
    let octave = rest
        .trim_start_matches(['#', 'b'])
        .parse::<i64>()
        .expect("an octave number");
    (letter, alteration, octave)
}

/// Spell `(letter index, alteration, octave)` the way the language writes it.
fn write(letter: i64, alteration: i64, octave: i64) -> String {
    let name = ['c', 'd', 'e', 'f', 'g', 'a', 'b'][index_of(letter)];
    let count = usize::try_from(alteration.abs()).expect("a small alteration");
    let accidental = if alteration < 0 { "b" } else { "#" }.repeat(count);
    format!("{name}{accidental}{octave}")
}

// --- The laws ---------------------------------------------------------------

#[test]
fn every_chord_type_spells_the_letter_stack_the_formula_names() {
    // Across five roots — natural, flat, sharp, and two registers — the
    // sounded close-position voicing is exactly the reference spelling.
    for root in ["c4", "eb4", "f#4", "bb3", "a4"] {
        for (name, members) in REFERENCE {
            let expected: Vec<String> = members
                .iter()
                .map(|(steps, semitones)| member(root, *steps, *semitones))
                .collect();
            assert_eq!(
                events(&piece(&format!("stack {root} {name}/1"))),
                [expected.join(" ")],
                "`stack {root} {name}` must spell the letter stack"
            );
        }
    }
}

#[test]
fn a_chord_class_chooses_no_register_and_stacking_is_where_one_arrives() {
    // `stack` needs a written root, and the diagnostic says why rather than
    // supplying an octave nobody asked for.
    let reported = errors(&piece("stack c major7/1"));
    assert!(
        reported.contains("needs a register"),
        "expected the register diagnostic, got: {reported}"
    );
    // The same content in two registers is two voicings of one class.
    assert_eq!(
        events(&piece("stack c4 major/2 stack c5 major/2")),
        ["c4 e4 g4", "c5 e5 g5"]
    );
}

#[test]
fn the_triad_refinement_admits_major_and_minor_and_refuses_the_rest() {
    let source = piece_with(
        &format!(
            "{PRELUDE}
    fn refined(content: ChordClass) -> Music {{ option_fold(music {{ rest/1 }}, voiced, as_triad(content)) }}
    fn voiced(shape: Triad) -> Music {{ sounded(close_position(triad_content(shape), c4)) }}

    let major: Music = refined(chord c major);
    let minor: Music = refined(chord c minor);
    let diminished: Music = refined(chord c diminished);
    let seventh: Music = refined(chord c major7);"
        ),
        "        use major;\n        use minor;\n        use diminished;\n        use seventh;",
    );
    // Major and minor sound; the diminished triad and the seventh chord are
    // rests, because the refinement is absent and nothing was assumed.
    assert_eq!(events(&source), ["c4 e4 g4", "c4 eb4 g4"]);
}

#[test]
fn an_inversion_designates_a_bass_and_leaves_the_root_alone() {
    let source = piece_with(
        &format!(
            "{PRELUDE}
    fn from_e(content: ChordClass) -> Option<Voicing> {{ close_position(content, e4) }}

    let content: ChordClass = chord c major;
    let first: Option<ChordClass> = inversion(content, 1);
    let voiced: Music = sounded(option_fold(None, from_e, first));
    let root_after: Music = named(option_fold(root_of(content), root_of, first));"
        ),
        "        use voiced;\n        use root_after;",
    );
    // The third is in the bass, and the root is still C: an inversion moves
    // the bass, not the root.
    assert_eq!(events(&source), ["e4 g4 c5", "c4"]);
}

#[test]
fn a_slash_bass_is_not_an_inversion() {
    // A slash bass accepts a bass the class does not contain, which is the
    // point: `c/d` is a real chord and D is not a chord tone. An inversion
    // asking for a member that is not there is absent instead.
    let source = piece_with(
        &format!(
            "{PRELUDE}
    fn from_d(content: ChordClass) -> Option<Voicing> {{ close_position(content, d3) }}

    let slash: ChordClass = slash_bass(chord c major, pitchclass_of(d3));
    let under_d: Music = sounded(close_position(slash, d3));
    let absent: Music = sounded(option_fold(None, from_d, inversion(chord c major, 7)));
    let slash_bass_class: Music = named(option_fold(c_root, root_of, some_slash));
    let some_slash: Option<ChordClass> = Some(slash);
    let c_root: NoteName = root_of(chord c major);"
        ),
        "        use under_d;\n        use absent;\n        use slash_bass_class;",
    );
    // The slash chord sounds with D under the triad, the missing inversion is
    // silence, and the slash chord's root is still C — a bass is not a root.
    assert_eq!(events(&source), ["d3 c4 e4 g4", "c4"]);
}

#[test]
fn a_voicing_policy_says_no_rather_than_nearly() {
    // `voiced_as` states the invariant: ascending, distinct, and every pitch
    // a member of the class it claims to voice.
    let source = piece_with(
        &format!(
            "{PRELUDE}
    let content: ChordClass = chord c major;
    let good: Music = sounded(voiced_as(content, [c3, g3, e4]));
    let descending: Music = sounded(voiced_as(content, [g3, c3, e4]));
    let foreign: Music = sounded(voiced_as(content, [c3, d3, e4]));"
        ),
        "        use good;\n        use descending;\n        use foreign;",
    );
    // Only the ascending, all-member spelling is a voicing. The other two are
    // rests: a policy that cannot apply produces nothing, not something close.
    assert_eq!(events(&source), ["c3 g3 e4"]);
}

#[test]
fn a_drop_voicing_is_not_its_close_position() {
    let source = piece_with(
        &format!(
            "{PRELUDE}
    let content: ChordClass = chord c major7;
    let close: Music = sounded(close_position(content, c3));
    let dropped: Music = sounded(drop_position(content, c3, 2));"
        ),
        "        use close;\n        use dropped;",
    );
    // Same class, same bass note asked for, different spacing — and the drop
    // voicing's lowest pitch is below the one close position was given.
    assert_eq!(events(&source), ["c3 e3 g3 b3", "g2 c3 e3 b3"]);
}

#[test]
fn an_omission_keeps_the_class_it_omits_from() {
    let source = piece_with(
        &format!(
            "{PRELUDE}
    fn class_root(chosen: Voicing) -> NoteName {{ root_of(chord_of(chosen)) }}

    let content: ChordClass = chord c major7;
    let close: Option<Voicing> = close_position(content, c4);
    let without_root: Option<Voicing> = option_fold(None, rootless, close);
    let voiced: Music = sounded(without_root);
    let still_c: Music = named(option_fold(root_of(content), class_root, without_root));"
        ),
        "        use voiced;\n        use still_c;",
    );
    // The root is gone from the sound and still names the chord: an omission
    // is a choice about what sounds, not a claim about what the chord is.
    assert_eq!(events(&source), ["e4 g4 b4", "c4"]);
}

#[test]
fn a_written_annotation_and_a_chord_class_never_imply_each_other() {
    // A harmony lane records `cmaj7` above the staff; the voice below sounds
    // a G major triad. Nothing objects, because nothing claims they agree.
    let source = "piece \"Laws\" {\n    score {\n        harmony { at 1:1 cmaj7; }\n        part p { voice v { stack g4 major/1 } }\n    }\n}\n";
    assert_eq!(events(source), ["g4 b4 d5"]);
}

#[test]
fn an_unknown_chord_type_is_named_rather_than_guessed() {
    let reported = errors(&piece("stack c4 mystic/1"));
    assert!(
        reported.contains("unknown chord type"),
        "expected the unknown-type diagnostic, got: {reported}"
    );
}
