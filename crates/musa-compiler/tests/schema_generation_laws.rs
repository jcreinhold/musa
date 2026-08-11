//! What a schema and a harmonization promise (OMT
//! `033`, `034`, `035`, `049`).
//!
//! Two families live here and they are proved differently.
//!
//! A *galant schema* is a table. OMT 34 prints each one as rows of scale
//! degrees, and `std::tonal::schemas` writes those rows out as list literals,
//! so the law is agreement with the printed row and nothing more. There is no
//! formula to check because there is no formula: a Romanesca is four numbers
//! somebody wrote down in the eighteenth century.
//!
//! A *sequence* and the *Rule of the Octave* are computed, so they get the
//! stronger treatment: the reference patterns below are written in Rust,
//! independently of the library, and the library has to agree with them at
//! every index. A sequence's roots must follow its interval modulo the
//! collection's period, its count must determine its extent, and zero and one
//! must mean the empty walk and the start alone.
//!
//! How a list is read back. The language has no combinator that turns a
//! `List<Music>` into sequential music — `use` sequences at the cursor and
//! `overlay` is simultaneous — so the probes below build one out of the two
//! primitives that do exist: fold with `overlay(one, shift(1, carried))`, which
//! lands each element a whole note after the rest of the accumulator. The fold
//! accumulates left to right, so that lays the list out backwards, and
//! `retrograde` turns it round again. A test proves the pair reads forwards
//! before anything relies on it, rather than assuming which way it came out.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{
    Code, CompileOptions, ScoreEventKind, ScoreSnapshot, Severity, SourceDocument, WrittenPitch, compile,
};

const SCHEMAS: &str = include_str!("../../../stdlib/src/tonal/schemas.musa");
const RULE_EXAMPLE: &str = include_str!("../../../examples/rule-of-the-octave.musa");
const SEQUENCE_EXAMPLE: &str = include_str!("../../../examples/diatonic-sequences.musa");

/// The imports, the register frame every degree is read through, and the
/// list-to-sequence fold the whole file rests on.
const PRELUDE: &str = r"
    import std::harmony;
    import std::scale;
    import std::tonal::schemas;
    import std::tonal::sequences;
    import std::voicing;

    meter 4/4;

    let collection: Scale = scale c major;
    let register: Option<Frame> = frame_on(collection, c4);

    fn placed(written: Degree) -> Pitch { match register {
        None -> c0,
        Some(located) -> frame_pitch(located, written),
    } }

    fn fixed(written: Degree, ignored: Pitch) -> Pitch { placed(written) }
    fn degree_note(written: Degree) -> Music { map_note_pitches(fixed(written), music { c0/1 }) }

    fn after(one: Music, carried: Music) -> Music { overlay(one, shift(1, carried)) }
    fn laid_out(values: List<Music>) -> Music { list_fold(music { rest/1 }, after, values) }
    fn line(written: List<Degree>) -> Music { retrograde(laid_out(map(degree_note, written))) }

    fn spelled(bass: Pitch, content: Option<ChordClass>) -> Music { match content {
        None -> music { rest/1 },
        Some(sounding) -> stacked(close_position(sounding, bass)),
    } }

    fn stacked(chosen: Option<Voicing>) -> Music { match chosen {
        None -> music { rest/1 },
        Some(spread) -> sound_for(spread, 1),
    } }
";

/// A piece whose one voice sounds `expression`.
fn probe(expression: &str) -> String {
    format!("piece \"Law\" {{\n{PRELUDE}\n    score {{ part p {{ voice v {{ use {expression}; }} }} }}\n}}\n")
}

fn compile_named(source: &str) -> musa_compiler::Compilation {
    compile(&SourceDocument::new(source, "test.musa"), &CompileOptions::default())
}

fn errors_of(source: &str) -> Vec<(Code, String)> {
    compile_named(source)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
        .collect()
}

fn snapshot_of(source: &str) -> ScoreSnapshot {
    let errors = errors_of(source);
    assert!(errors.is_empty(), "expected a clean compile, got {errors:?}");
    compile_named(source).into_snapshot().expect("compiles")
}

/// Every written pitch the probe's voice sounds, in the order the score puts
/// them.
fn sounded(expression: &str) -> Vec<WrittenPitch> {
    snapshot_of(&probe(expression))
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events())
        .flat_map(|event| match event.kind {
            ScoreEventKind::Note { pitch } => vec![pitch],
            ScoreEventKind::Chord { ref pitches } => pitches.clone(),
            ScoreEventKind::Rest => Vec::new(),
        })
        .collect()
}

/// A schema's line, as the pitch-class letters a copyist would write.
fn spellings(expression: &str) -> Vec<String> {
    sounded(expression)
        .into_iter()
        .map(|pitch| pitch.pitch_class().to_string())
        .collect()
}

/// The letter the `ordinal`th degree of C major carries, counted from one so
/// that the tables below read the way OMT prints them. Eight is the octave and
/// is C again.
fn letter(ordinal: usize) -> String {
    const LETTERS: [&str; CYCLE] = ["c", "d", "e", "f", "g", "a", "b"];
    let zero = ordinal.saturating_sub(1).wrapping_rem(CYCLE);
    LETTERS.get(zero).copied().unwrap_or("c").to_owned()
}

/// How many degrees a diatonic collection has to its period.
const CYCLE: usize = 7;

/// One OMT row, as letters. Ordinals are the scale degrees OMT prints, so
/// `[1, 7, 6, 3]` is the Romanesca bass.
fn row(ordinals: &[usize]) -> Vec<String> {
    ordinals.iter().copied().map(letter).collect()
}

// ---- The fold that reads a list ----------------------------------------

/// The list-reading fold is the premise of every table law below, so it is
/// proved before it is used: a known line comes back in the order it was
/// written, not reversed and not as a chord.
#[test]
fn a_folded_line_reads_in_the_order_it_was_written() {
    assert_eq!(
        spellings("line(degrees_of([1, 2, 3, 4]))"),
        row(&[1, 2, 3, 4]),
        "the fold must preserve order, or every table law below is reading its tables backwards"
    );
}

/// An empty line sounds nothing, which is what makes a count of zero readable
/// as the empty walk rather than as a silent failure.
#[test]
fn an_empty_line_sounds_nothing() {
    assert!(spellings("line(degrees_of([]))").is_empty());
}

// ---- The galant schemas, against OMT 34's printed rows ------------------

/// Every schema's bass is the row OMT 34 prints for it.
///
/// The two rows carrying an alteration are checked separately below, because
/// a raised degree is not a letter — that is the whole reason it is written
/// with `raise` rather than as another ordinal.
#[test]
fn every_schema_bass_is_the_row_omt_prints() {
    let tables: [(&str, &[usize]); 6] = [
        ("romanesca_bass()", &[1, 7, 6, 3]),
        ("do_re_mi_bass()", &[1, 7, 1]),
        ("prinner_bass()", &[4, 3, 2, 1]),
        ("prinner_with_dominant_bass()", &[4, 3, 2, 5, 1]),
        ("fenaroli_bass()", &[7, 1, 2, 3]),
        ("cadenza_semplice_bass()", &[3, 4, 5, 1]),
    ];
    for (call, ordinals) in tables {
        assert_eq!(spellings(&format!("line({call})")), row(ordinals), "{call}");
    }
}

/// Every schema's melody is the row OMT 34 prints for it.
#[test]
fn every_schema_melody_is_the_row_omt_prints() {
    let tables: [(&str, &[usize]); 6] = [
        ("romanesca_melody()", &[1, 5, 1, 1]),
        ("do_re_mi_melody()", &[1, 2, 3]),
        ("prinner_melody()", &[6, 5, 4, 3]),
        ("prinner_with_dominant_melody()", &[6, 5, 4, 4, 3]),
        ("fenaroli_melody()", &[4, 3, 7, 1]),
        ("cadenza_semplice_melody()", &[1, 2, 2, 1]),
    ];
    for (call, ordinals) in tables {
        assert_eq!(spellings(&format!("line({call})")), row(ordinals), "{call}");
    }
}

/// The roots OMT's figures name, which are not the bass wherever a figure is
/// a 6. The Romanesca's second stage is the case: the bass is *ti* and the
/// chord is rooted on *sol*.
#[test]
fn a_figured_stage_roots_its_chord_below_its_bass() {
    assert_eq!(spellings("line(romanesca_roots())"), row(&[1, 5, 6, 1]));
    assert_eq!(spellings("line(prinner_roots())"), row(&[4, 1, 7, 1]));
    assert_eq!(spellings("line(fenaroli_roots())"), row(&[5, 1, 5, 1]));
    assert_ne!(
        spellings("line(romanesca_roots())"),
        spellings("line(romanesca_bass())"),
        "a schema whose roots equalled its bass would have no figures to read"
    );
}

/// The Fonte's bass carries a raised first degree and the Monte's a raised
/// fourth, so those rows are sharps where the plain rows are naturals. This is
/// the law that an alteration travels with the degree rather than being lost.
#[test]
fn an_altered_schema_degree_keeps_its_alteration() {
    assert_eq!(spellings("line(fonte_bass())"), ["c#", "d", "b", "c"]);
    assert_eq!(spellings("line(monte_bass())"), ["e", "f", "f#", "g"]);
    assert_eq!(
        spellings("line(quiescenza_melody())"),
        ["bb", "a", "b", "c"],
        "the Quiescenza lowers where the Fonte raises"
    );
}

/// The Quiescenza's bass does not move: four stages on the tonic. It is the
/// one schema here whose bass is a pedal, and a table that had quietly given
/// it motion would be a different schema.
#[test]
fn the_quiescenza_stands_still() {
    assert_eq!(spellings("line(quiescenza_bass())"), row(&[1, 1, 1, 1]));
}

/// A schema harmonized in a collection takes the collection's own qualities,
/// and an altered root is absent because it is not the collection's chord.
#[test]
fn a_schema_harmonizes_to_the_collections_own_chords() {
    assert_eq!(
        spellings("spelled(placed(degree_of(1)), schema_triad(collection, degree_of(1)))"),
        ["c", "e", "g"],
        "the tonic triad of C major"
    );
    assert_eq!(
        spellings("spelled(placed(degree_of(2)), schema_triad(collection, degree_of(2)))"),
        ["d", "f", "a"],
        "ii is minor in major because those are the notes there"
    );
    assert!(
        spellings("spelled(placed(degree_of(1)), schema_triad(collection, raise(degree_of(1))))").is_empty(),
        "an applied dominant is not the collection's own chord, and absence is the honest answer"
    );
}

// ---- The Rule of the Octave, against OMT 35's recipe --------------------

/// The reference Rule, written independently of the library from the recipe
/// OMT 35 states in prose: parallel 6/3 over every bass degree, 5/3 on the
/// tonic and the dominant, and a 6/5 on whatever precedes one of those.
///
/// Returns the root-position spelling of the chord over each bass degree, as
/// letters, so that agreement is checked against notes rather than against
/// another copy of the same arithmetic.
fn reference_rule(ascending: bool) -> Vec<Vec<String>> {
    // Which bass degrees carry a 5/3, and which carry a 6/5. Everything else
    // is the parallel 6/3 the recipe starts from. Eight means the octave.
    let fifths = [1usize, 5, 8];
    let sevenths: &[usize] = if ascending { &[4, 7] } else { &[2, 6] };
    let order: Vec<usize> = if ascending {
        (1..=8).collect()
    } else {
        (1..=8).rev().collect()
    };
    order
        .into_iter()
        .map(|bass| {
            let root = if fifths.contains(&bass) { bass } else { wrap(bass, 2) };
            let members = if sevenths.contains(&bass) { 4 } else { 3 };
            stack(root, members)
        })
        .collect()
}

/// A degree `down` steps below `from`, on a seven-degree cycle written from
/// one. Eight is the octave and reads as one.
fn wrap(from: usize, down: usize) -> usize {
    let zero = from.saturating_sub(1).wrapping_rem(CYCLE);
    let back = CYCLE.saturating_sub(down.wrapping_rem(CYCLE));
    zero.saturating_add(back).wrapping_rem(CYCLE).saturating_add(1)
}

/// A degree `up` steps above `from`, on the same cycle.
fn rise(from: usize, up: usize) -> usize {
    let zero = from.saturating_sub(1).wrapping_rem(CYCLE);
    zero.saturating_add(up.wrapping_rem(CYCLE))
        .wrapping_rem(CYCLE)
        .saturating_add(1)
}

/// The letters of the diatonic stack of `members` notes on `root` in C major.
fn stack(root: usize, members: usize) -> Vec<String> {
    (0..members)
        .map(|step| letter(rise(root, step.saturating_mul(2))))
        .collect()
}

/// The library's ascending Rule spells, degree by degree, what the recipe
/// says it should.
#[test]
fn the_ascending_rule_agrees_with_the_recipe() {
    for (index, expected) in reference_rule(true).into_iter().enumerate() {
        let bass = index.saturating_add(1);
        let sounded = spellings(&format!(
            "spelled(placed(degree_of({bass})), rule_ascending_chord(collection, {bass}))"
        ));
        assert_eq!(sorted(&sounded), sorted(&expected), "ascending bass degree {bass}");
    }
}

/// And the descending Rule likewise, with the sevenths where the descent puts
/// them.
#[test]
fn the_descending_rule_agrees_with_the_recipe() {
    for (index, expected) in reference_rule(false).into_iter().enumerate() {
        let bass = 8usize.saturating_sub(index);
        let sounded = spellings(&format!(
            "spelled(placed(degree_of({bass})), rule_descending_chord(collection, {bass}))"
        ));
        assert_eq!(sorted(&sounded), sorted(&expected), "descending bass degree {bass}");
    }
}

/// A voicing may double or reorder; membership is what the recipe fixes.
fn sorted(letters: &[String]) -> Vec<String> {
    let mut unique: Vec<String> = letters.to_vec();
    unique.sort();
    unique.dedup();
    unique
}

/// Direction is a real difference and not a presentation of one. OMT 35 gives
/// the ascent and the descent separately, and the two disagree at exactly the
/// degrees where the recipe's "precedes" changes meaning.
#[test]
fn the_two_directions_disagree_where_the_recipe_says_they_should() {
    let mut differing = Vec::new();
    for bass in 1..=8 {
        let up = spellings(&format!(
            "spelled(placed(degree_of({bass})), rule_ascending_chord(collection, {bass}))"
        ));
        let down = spellings(&format!(
            "spelled(placed(degree_of({bass})), rule_descending_chord(collection, {bass}))"
        ));
        if sorted(&up) != sorted(&down) {
            differing.push(bass);
        }
    }
    assert_eq!(
        differing,
        vec![2, 4, 6, 7],
        "the sevenths move and nothing else does: 4 and 7 precede in the ascent, 2 and 6 in the descent"
    );
}

/// The tonic and dominant keep their root-position triads whichever way the
/// bass is walking. That is steps two and three of the recipe, and it is the
/// part direction does *not* touch.
#[test]
fn the_tonic_and_dominant_stand_in_both_directions() {
    for bass in [1, 5, 8] {
        let up = spellings(&format!(
            "spelled(placed(degree_of({bass})), rule_ascending_chord(collection, {bass}))"
        ));
        let down = spellings(&format!(
            "spelled(placed(degree_of({bass})), rule_descending_chord(collection, {bass}))"
        ));
        assert_eq!(sorted(&up), sorted(&down), "bass degree {bass}");
        assert_eq!(sorted(&up).len(), 3, "a 5/3 is a triad, not a seventh chord");
    }
}

/// The Rule is over the octave: eight chords, and the last is the first again.
#[test]
fn the_rule_closes_on_the_tonic_it_opened_on() {
    let first = spellings("spelled(placed(degree_of(1)), rule_ascending_chord(collection, 1))");
    let last = spellings("spelled(placed(degree_of(1)), rule_ascending_chord(collection, 8))");
    assert_eq!(first, last);
    assert_eq!(sorted(&first), ["c", "e", "g"]);
}

// ---- The sequences, against their index formulas ------------------------

/// A falling walk's degrees follow the interval modulo the collection's
/// period, at every index, for every interval the library names.
#[test]
fn a_walk_follows_its_interval_modulo_the_cycle() {
    for steps in [1usize, 2, 4] {
        for index in 0..9 {
            let fell = spellings(&format!("degree_note(falling_degree(degree_of(1), {steps}, {index}))"));
            assert_eq!(fell, [letter(wrap(1, steps * index))], "fall by {steps} at {index}");

            let rose = spellings(&format!("degree_note(rising_degree(degree_of(1), {steps}, {index}))"));
            assert_eq!(rose, [letter(rise(1, steps * index))], "rise by {steps} at {index}");
        }
    }
}

/// A count determines the extent, and zero and one mean what they ordinarily
/// mean: the empty walk, and the start alone.
#[test]
fn a_count_determines_a_finite_extent() {
    assert!(spellings("line(falling_degrees(degree_of(1), 4, 0))").is_empty());
    assert_eq!(spellings("line(falling_degrees(degree_of(1), 4, 1))"), row(&[1]));
    assert_eq!(
        spellings("line(falling_degrees(degree_of(1), 4, 8))"),
        ["c", "f", "b", "e", "a", "d", "g", "c"],
        "the descending-fifths walk OMT 49 writes I–IV–vii°–iii–vi–ii–V–I"
    );
    assert_eq!(spellings("line(rising_degrees(degree_of(1), 1, 8))").len(), 8);
}

/// The plural form is the singular form mapped over the range, which is the
/// agreement the library claims in its own documentation.
#[test]
fn the_plural_form_agrees_with_the_singular_at_every_index() {
    let walked = spellings("line(falling_degrees(degree_of(1), 4, 8))");
    for (index, expected) in walked.iter().enumerate() {
        let one = spellings(&format!("degree_note(descending_fifths_degree(degree_of(1), {index}))"));
        assert_eq!(one.first(), Some(expected), "index {index}");
    }
}

// ---- What this layer does not add --------------------------------------

/// No compiler primitive.
///
/// The primitive registry in `crates/musa-compiler/src/core.rs` is the only
/// route by which a primitive enters, and the check here is behavioral rather
/// than a read of that list: a compiler-owned operation resolves with no
/// import, and a library function does not. So every name added here is asked
/// for *without* its module, and every one must fail to resolve. The moment a
/// schema is implemented by widening the compiler instead of by writing a
/// function, one of these starts compiling and this test says so.
#[test]
fn the_schema_libraries_add_no_compiler_primitive() {
    for call in [
        "romanesca_bass()",
        "prinner_bass()",
        "fonte_bass()",
        "quiescenza_bass()",
        "rule_ascending_chord(scale c major, 1)",
        "rule_descending_chord(scale c major, 1)",
        "descending_fifths_degree(degree_of(1), 0)",
        "ascending_seconds_degree(degree_of(1), 0)",
    ] {
        let source =
            format!("piece \"Unimported\" {{\n    let probed: Nat = 0;\n    let asked: Nat = named({call});\n}}\n");
        let errors = errors_of(&source);
        assert!(
            errors.iter().any(|(_, message)| message.contains("cannot find")),
            "`{call}` resolved without its module, so it is compiler-owned rather than library source: {errors:?}"
        );
    }
}

/// And the same names *do* resolve once their module is imported, so the test
/// above is failing for the right reason rather than because every one of
/// those calls is malformed.
#[test]
fn the_same_names_resolve_once_their_module_is_imported() {
    for call in [
        "line(romanesca_bass())",
        "line(prinner_bass())",
        "line(fonte_bass())",
        "line(quiescenza_bass())",
        "spelled(placed(degree_of(1)), rule_ascending_chord(collection, 1))",
        "degree_note(descending_fifths_degree(degree_of(1), 0))",
    ] {
        let errors = errors_of(&probe(call));
        assert!(errors.is_empty(), "{call}: {errors:?}");
    }
}

/// Both examples compile, and they are the fixtures that prove the libraries
/// are reachable: `std::tonal::sequences` was committed orphaned, and an
/// orphaned module is one nothing can be wrong about.
#[test]
fn the_examples_compile_and_reach_the_libraries() {
    for (name, source) in [
        ("rule-of-the-octave.musa", RULE_EXAMPLE),
        ("diatonic-sequences.musa", SEQUENCE_EXAMPLE),
    ] {
        let errors = errors_of(source);
        assert!(errors.is_empty(), "{name}: {errors:?}");
    }
    assert!(
        SCHEMAS.contains("035-"),
        "the schemas module must cite the OMT section its Rule of the Octave follows"
    );
}
