//! What a Roman numeral promises (OMT `020`, `021`, `050`, `061`, `062`,
//! `063`, `071`).
//!
//! A numeral is a coordinate: a degree, a member count, and a bass position.
//! It becomes a chord only against a collection, and the collection is what
//! supplies the quality — which is why every table below is a table of
//! *spellings* rather than of qualities. `ii` is minor in C major and `II` is
//! major in D Dorian for one reason: those are the notes there.
//!
//! Values are read here by voicing them and looking at the notes. Where the
//! neo-Riemannian laws had to count overlaid beats, a chord class can simply
//! be stacked from a bass and its written pitches read out of the score
//! snapshot, so the assertions below are the letters a copyist would write.
//! An absent `option` sounds a rest and reads as no pitches at all, which is
//! how the negative cases are stated.
//!
//! The bass under each probe is computed from the collection rather than
//! written as a literal, so a test cannot accidentally assert its own
//! premise: `frame_pitch(register, lower(degree_of(6)))` is the lowered sixth
//! of whatever collection was named, in whatever key, and if the degree
//! lookup were wrong the voicing would vanish rather than quietly agree.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{
    Code, CompileOptions, ScoreEventKind, ScoreSnapshot, Severity, SourceDocument, WrittenPitch, compile,
};

const TONAL_CONSTRUCTION: &str = include_str!("../../../../examples/tonal-construction.musa");

/// The imports and the one voicing policy every probe reads values through.
const PRELUDE: &str = r"
    import std::harmony;
    import std::list;
    import std::scale;
    import std::tonal::harmony;
    import std::voicing;

    meter 4/4;

    fn spelled(bass: Pitch, content: Option<ChordClass>) -> EventTrack<WrittenTime> { match content {
        None -> music { rest/1 },
        Some(sounding) -> stacked(close_position(sounding, bass)),
    } }

    fn stacked(chosen: Option<Voicing>) -> EventTrack<WrittenTime> { match chosen {
        None -> music { rest/1 },
        Some(spread) -> sound_for(spread, duration_of(1/1)),
    } }

    fn tick(one: EventTrack<WrittenTime>, carried: EventTrack<WrittenTime>) -> EventTrack<WrittenTime> { together(one, carried) }
    fn beat() -> EventTrack<WrittenTime> { music { c4/1 } }
    fn tally(count: Nat) -> EventTrack<WrittenTime> { repeated(beat(), count).fold_from_start(music { rest/1 }, fn (carried, one) { tick(one, carried) }) }

    fn numeral_in(collection: Scale, written: Option<Roman>) -> Option<ChordClass> { match written {
            None -> None,
            Some(numbered) -> numeral_chord(collection, numbered),
        } }
";

/// A piece whose one voice sounds `expression`.
fn probe(bindings: &str, expression: &str) -> String {
    format!(
        "piece \"Law\" {{\n{PRELUDE}\n{bindings}\n    score {{ part p {{ voice v {{ use {expression}; }} }} }}\n}}\n"
    )
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

/// Every written pitch the probe's voice sounds, in order.
fn sounded(bindings: &str, expression: &str) -> Vec<WrittenPitch> {
    snapshot_of(&probe(bindings, expression))
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

/// The chord `content` spells, stacked from `bass`, as a copyist would write
/// it without octaves.
///
/// Empty when the chord is absent, and also empty when the bass is not a
/// member of the class as spelled — the two are told apart by choosing a bass
/// the collection itself computed, which is what every caller below does.
fn spelling(bindings: &str, bass: &str, content: &str) -> Vec<String> {
    let sounded = sounded(
        &format!("{bindings}\n    let probed: EventTrack<WrittenTime> = spelled({bass}, {content});"),
        "probed",
    );
    sounded
        .into_iter()
        .map(|pitch| pitch.pitch_class().to_string())
        .collect()
}

/// A frame on `tonic` in `collection`, and the numerals the tables use.
fn keyed(collection: &str, tonic: &str) -> String {
    format!(
        "    let collection: Scale = {collection};
    let register: Option<Frame> = frame_on(collection, {tonic});
    fn root_of_degree(written: Degree) -> Pitch {{ match register {{
        None -> c0,
        Some(placed) -> frame_pitch(placed, written),
    }} }}"
    )
}

/// The root-position triad on `ordinal` of `collection`, spelled from that
/// degree's own pitch.
fn triad_on(collection: &str, tonic: &str, ordinal: u32) -> Vec<String> {
    spelling(
        &keyed(collection, tonic),
        &format!("root_of_degree(degree_of({ordinal}))"),
        &format!("numeral_in(collection, triad_numeral({ordinal}))"),
    )
}

/// The root-position seventh chord on `ordinal` of `collection`.
fn seventh_on(collection: &str, tonic: &str, ordinal: u32) -> Vec<String> {
    spelling(
        &keyed(collection, tonic),
        &format!("root_of_degree(degree_of({ordinal}))"),
        &format!("numeral_in(collection, seventh_numeral({ordinal}))"),
    )
}

fn words(members: &[&str]) -> Vec<String> {
    members.iter().map(|member| (*member).to_owned()).collect()
}

/// OMT 020's table of diatonic triads, in three keys, so that a spelling that
/// happened to be right in C is not mistaken for a rule.
///
/// The keys are chosen to exercise both accidentals: B-flat major spells its
/// `ii` as C–E-flat–G and F-sharp major spells its `V` as C-sharp–E-sharp–G-
/// sharp, which is where a semitone-counting implementation writes an F.
#[test]
fn the_diatonic_triads_are_the_ones_the_collection_stacks() {
    let cases: [(&str, &str, [[&str; 3]; 7]); 3] = [
        (
            "scale c major",
            "c4",
            [
                ["c", "e", "g"],
                ["d", "f", "a"],
                ["e", "g", "b"],
                ["f", "a", "c"],
                ["g", "b", "d"],
                ["a", "c", "e"],
                ["b", "d", "f"],
            ],
        ),
        (
            "scale bb major",
            "bb3",
            [
                ["bb", "d", "f"],
                ["c", "eb", "g"],
                ["d", "f", "a"],
                ["eb", "g", "bb"],
                ["f", "a", "c"],
                ["g", "bb", "d"],
                ["a", "c", "eb"],
            ],
        ),
        (
            "scale f# major",
            "f#3",
            [
                ["f#", "a#", "c#"],
                ["g#", "b", "d#"],
                ["a#", "c#", "e#"],
                ["b", "d#", "f#"],
                ["c#", "e#", "g#"],
                ["d#", "f#", "a#"],
                ["e#", "g#", "b"],
            ],
        ),
    ];

    for (collection, tonic, table) in cases {
        for (index, expected) in table.into_iter().enumerate() {
            let ordinal = u32::try_from(index).expect("a small index") + 1;
            assert_eq!(
                triad_on(collection, tonic, ordinal),
                words(&expected),
                "{collection} triad on {ordinal}"
            );
        }
    }
}

/// OMT 021: a seventh chord is the same stack one third further, and its
/// quality follows the collection for the same reason the triad's does. The
/// `V7` being the only dominant seventh in a major key is a consequence here,
/// not a rule that was written down.
#[test]
fn the_diatonic_sevenths_are_the_triads_with_one_more_third() {
    let table: [[&str; 4]; 7] = [
        ["c", "e", "g", "b"],
        ["d", "f", "a", "c"],
        ["e", "g", "b", "d"],
        ["f", "a", "c", "e"],
        ["g", "b", "d", "f"],
        ["a", "c", "e", "g"],
        ["b", "d", "f", "a"],
    ];
    for (index, expected) in table.into_iter().enumerate() {
        let ordinal = u32::try_from(index).expect("a small index") + 1;
        let seventh = seventh_on("scale c major", "c4", ordinal);
        assert_eq!(seventh, words(&expected), "seventh on {ordinal}");
        assert_eq!(
            seventh.get(..3).map(<[String]>::to_vec),
            Some(triad_on("scale c major", "c4", ordinal)),
            "the seventh on {ordinal} does not extend its own triad"
        );
    }
}

/// OMT 021's figures: an inversion designates a member as the bass, and the
/// designation is what a voicing then reads. `V6` accepts a B in the bass and
/// `V` in root position does not, which is the whole difference between them
/// as values rather than as engraving.
#[test]
fn an_inversion_designates_the_member_the_bass_must_be() {
    let keyed = keyed("scale c major", "c4");
    let third = "root_of_degree(degree_of(7))";
    let root = "root_of_degree(degree_of(5))";

    assert_eq!(
        spelling(&keyed, third, "numeral_in(collection, numeral(5, 3, 1))"),
        words(&["b", "d", "g"]),
        "the first inversion of V puts B under it"
    );
    assert_eq!(
        spelling(&keyed, third, "numeral_in(collection, numeral(5, 3, 0))"),
        Vec::<String>::new(),
        "root position does not accept a third in the bass"
    );
    assert_eq!(
        spelling(&keyed, root, "numeral_in(collection, numeral(5, 3, 2))"),
        Vec::<String>::new(),
        "second inversion does not accept a root in the bass"
    );
    assert_eq!(
        spelling(
            &keyed,
            "root_of_degree(degree_of(4))",
            "numeral_in(collection, numeral(5, 4, 3))"
        ),
        words(&["f", "g", "b", "d"]),
        "the third inversion of V7 puts F under it"
    );
}

/// The minor variants, side by side. C minor is three collections, and each
/// answers `V` differently: the natural collection stacks a minor triad, the
/// harmonic one a major triad with its raised leading tone, and the melodic
/// one the same. Nothing here picks between them.
#[test]
fn each_minor_collection_answers_the_dominant_its_own_way() {
    let cases: [(&str, [&str; 3], [&str; 3]); 3] = [
        ("scale c natural_minor", ["c", "eb", "g"], ["g", "bb", "d"]),
        ("scale c harmonic_minor", ["c", "eb", "g"], ["g", "b", "d"]),
        ("scale c melodic_minor", ["c", "eb", "g"], ["g", "b", "d"]),
    ];
    for (collection, tonic, dominant) in cases {
        assert_eq!(triad_on(collection, "c4", 1), words(&tonic), "{collection} tonic");
        assert_eq!(triad_on(collection, "c4", 5), words(&dominant), "{collection} dominant");
    }

    // OMT 020's minor `ii°`: diminished in the natural collection, and the
    // same in the harmonic one, because the raised seventh is not in it.
    assert_eq!(triad_on("scale c natural_minor", "c4", 2), words(&["d", "f", "ab"]));
    assert_eq!(triad_on("scale c harmonic_minor", "c4", 2), words(&["d", "f", "ab"]));
}

/// A collection can stack to a sonority the chord vocabulary does not name,
/// and then there is no chord. Harmonic minor's `III7` is an augmented major
/// seventh; reporting the absence is the honest answer, and rounding it to a
/// major seventh would silently change a note.
#[test]
fn a_stack_with_no_name_is_no_chord() {
    assert_eq!(
        triad_on("scale c harmonic_minor", "c4", 3),
        words(&["eb", "g", "b"]),
        "the augmented triad on III is named"
    );
    assert_eq!(
        seventh_on("scale c harmonic_minor", "c4", 3),
        Vec::<String>::new(),
        "the augmented major seventh on III is not"
    );
}

/// OMT 050: an applied chord is a numeral read in the collection that
/// tonicizes a target, which is why `V/V` in C major is D major and not the D
/// minor that C major stacks on its own second degree. The tonicizing
/// collection is written out, so nothing decided that an applied dominant is
/// major on the caller's behalf.
#[test]
fn an_applied_chord_is_read_in_the_collection_it_tonicizes() {
    let bindings = format!(
        "{}\n    let major_mode: Scale = scale c major;",
        keyed("scale c major", "c4")
    );
    let applied = |target: u32, numeral: &str| {
        format!(
            "match {numeral} {{ None -> None, Some(numbered) -> secondary(collection, degree_of({target}), major_mode, numbered) }}"
        )
    };

    assert_eq!(
        spelling(
            &bindings,
            "root_of_degree(degree_of(2))",
            &applied(5, "triad_numeral(5)")
        ),
        words(&["d", "f#", "a"]),
        "V/V is D major"
    );
    assert_eq!(
        triad_on("scale c major", "c4", 2),
        words(&["d", "f", "a"]),
        "and ii is not"
    );
    assert_eq!(
        spelling(
            &bindings,
            "root_of_degree(degree_of(3))",
            &applied(6, "seventh_numeral(5)")
        ),
        words(&["e", "g#", "b", "d"]),
        "V7/vi is an E dominant seventh"
    );
    assert_eq!(
        spelling(
            &bindings,
            "root_of_degree(degree_of(6))",
            &applied(2, "triad_numeral(5)")
        ),
        words(&["a", "c#", "e"]),
        "V/ii is A major"
    );
}

/// OMT 061: mixture is the same numeral against a borrowed collection on the
/// home tonic, and needs no altered degree at all. `bVI` in C major is `VI`
/// in C minor, spelled by that collection, which is what the word means.
#[test]
fn mixture_is_the_same_numeral_in_the_borrowed_collection() {
    let bindings = format!(
        "{}\n    let borrowed_mode: Scale = scale c natural_minor;",
        keyed("scale c major", "c4")
    );
    let mixed = |ordinal: u32| {
        format!(
            "match triad_numeral({ordinal}) {{ None -> None, Some(numbered) -> borrowed(collection, borrowed_mode, numbered) }}"
        )
    };

    assert_eq!(
        spelling(&bindings, "root_of_degree(lower(degree_of(6)))", &mixed(6)),
        words(&["ab", "c", "eb"]),
        "bVI of C major"
    );
    assert_eq!(
        spelling(&bindings, "root_of_degree(degree_of(4))", &mixed(4)),
        words(&["f", "ab", "c"]),
        "iv of C major"
    );
    // The same numeral, borrowed and not, is two different chords — which is
    // the only reason mixture is worth a name.
    assert_eq!(triad_on("scale c major", "c4", 4), words(&["f", "a", "c"]));
    assert_eq!(triad_on("scale c natural_minor", "c4", 6), words(&["ab", "c", "eb"]));
}

/// OMT 062: the Neapolitan is a major triad on the lowered second degree, and
/// nothing more. The `6` in its usual name is a position, taken separately,
/// and the chord is the same major triad without it.
#[test]
fn the_neapolitan_is_a_major_triad_on_the_lowered_second() {
    let major = keyed("scale c major", "c4");
    assert_eq!(
        spelling(
            &major,
            "root_of_degree(lower(degree_of(2)))",
            "neapolitan(collection, lower(degree_of(2)))"
        ),
        words(&["db", "f", "ab"]),
        "in C major the second degree is lowered"
    );

    // In minor the same chord names the plain second degree, because the
    // collection already lowered it. The caller writes which, and that is the
    // point: the library never picks a minor collection.
    let minor = keyed("scale c phrygian", "c4");
    assert_eq!(
        spelling(
            &minor,
            "root_of_degree(degree_of(2))",
            "neapolitan(collection, degree_of(2))"
        ),
        words(&["db", "f", "ab"]),
        "in Phrygian it is the plain second"
    );
}

/// OMT 063: the three augmented sixths, and the spelling that makes them
/// chords of their own. A German sixth sounds like a dominant seventh and is
/// not one — its top note is an augmented sixth above the root, written from
/// a different letter, so re-rooting `dominant7` onto the same degree writes
/// a different note for the same sound.
#[test]
fn the_augmented_sixths_spell_their_sixth_as_a_sixth() {
    let major = keyed("scale c major", "c4");
    let flat_six = "root_of_degree(lower(degree_of(6)))";
    let on_flat_six = "lower(degree_of(6))";

    assert_eq!(
        spelling(&major, flat_six, &format!("italian_sixth(collection, {on_flat_six})")),
        words(&["ab", "c", "f#"]),
        "the Italian sixth has no fifth"
    );
    assert_eq!(
        spelling(&major, flat_six, &format!("french_sixth(collection, {on_flat_six})")),
        words(&["ab", "c", "d", "f#"]),
        "the French sixth adds the second degree"
    );
    assert_eq!(
        spelling(&major, flat_six, &format!("german_sixth(collection, {on_flat_six})")),
        words(&["ab", "c", "eb", "f#"]),
        "the German sixth adds the lowered third"
    );

    let seventh = spelling(
        &major,
        flat_six,
        &format!("quality_on_degree(collection, {on_flat_six}, chord c dominant7)"),
    );
    assert_eq!(seventh, words(&["ab", "c", "eb", "gb"]), "the seventh spells a seventh");

    // Same sound, different chord. The semitones agree and the letters do not,
    // which is exactly the distinction `chord_class` exists to keep.
    let german = sounded_semitones(&major, flat_six, &format!("german_sixth(collection, {on_flat_six})"));
    let dominant = sounded_semitones(
        &major,
        flat_six,
        &format!("quality_on_degree(collection, {on_flat_six}, chord c dominant7)"),
    );
    assert_eq!(german, dominant, "the German sixth sounds like the dominant seventh");
}

/// The sounding semitones of a probe, which is the half of the German-sixth
/// law that the letters cannot state.
fn sounded_semitones(bindings: &str, bass: &str, content: &str) -> Vec<i64> {
    sounded(
        &format!("{bindings}\n    let probed: EventTrack<WrittenTime> = spelled({bass}, {content});"),
        "probed",
    )
    .into_iter()
    .map(WrittenPitch::semitone)
    .collect()
}

/// OMT 071: which alteration is present is the caller's. Two altered
/// dominants on the same degree of the same collection differ only in the
/// members they name, and neither of them is what the function chose.
#[test]
fn an_altered_dominant_names_the_members_it_has() {
    let major = keyed("scale c major", "c4");
    let root = "root_of_degree(degree_of(5))";
    let cases: [(&str, &[&str]); 4] = [
        ("chord c dominant7", &["g", "b", "d", "f"]),
        ("chord c dom7b9", &["g", "b", "d", "f", "ab"]),
        ("chord c dom7s5", &["g", "b", "d#", "f"]),
        ("chord c dom7b5", &["g", "b", "db", "f"]),
    ];
    for (quality, expected) in cases {
        assert_eq!(
            spelling(&major, root, &format!("altered_dominant(collection, {quality})")),
            words(expected),
            "{quality} on the fifth degree"
        );
    }
}

/// The refinement's whole job: the numerals that cannot be written have no
/// values, so no downstream function has to defend against them.
#[test]
fn a_numeral_that_cannot_be_written_is_not_a_numeral() {
    let major = keyed("scale c major", "c4");
    let cases: [(&str, &str); 5] = [
        ("numeral(8, 3, 0)", "there is no numeral eight"),
        ("numeral(0, 3, 0)", "there is no numeral zero"),
        ("numeral(1, 2, 0)", "two members is not a chord"),
        ("numeral(1, 3, 3)", "a triad has no third inversion"),
        ("numeral(1, 8, 0)", "a fifteenth is the octave again"),
    ];
    for (written, why) in cases {
        assert_eq!(
            spelling(&major, "c4", &format!("numeral_in(collection, {written})")),
            Vec::<String>::new(),
            "{why}"
        );
    }

    // The ordinal a numeral can name is not the ordinal a collection has. A
    // pentatonic collection has five degrees, so `vi` is a numeral and is not
    // a chord there — the two partialities are separate and both are real.
    assert_eq!(
        spelling(
            &keyed("scale c major_pentatonic", "c4"),
            "root_of_degree(degree_of(1))",
            "numeral_in(collection, triad_numeral(1))"
        ),
        Vec::<String>::new(),
        "a pentatonic collection stacks no third from its tonic"
    );
}

/// The accessors read back what the constructor was given, which is what lets
/// a caller take a numeral apart and rebuild it. A `nat` is read the way the
/// serial laws read one: as that many overlaid notes, so a wrong number is a
/// wrong count rather than a type error.
#[test]
fn a_numerals_parts_are_what_it_was_built_from() {
    let reader = |accessor: &str| {
        format!(
            "    fn read(written: Option<Roman>) -> Nat {{ match written {{
        None -> 0,
        Some(numbered) -> {accessor}(numbered),
    }} }}
    let counted: EventTrack<WrittenTime> = tally(read(numeral(6, 4, 2)));"
        )
    };
    assert_eq!(sounded(&reader("numeral_step"), "counted").len(), 6, "the degree");
    assert_eq!(sounded(&reader("numeral_size"), "counted").len(), 4, "the member count");
    assert_eq!(
        sounded(&reader("numeral_bass"), "counted").len(),
        2,
        "the bass position"
    );
}

/// The bundled example is a fixture, not a demo: it compiles clean and its
/// chromatic chords sound the notes their names claim.
#[test]
fn the_bundled_example_compiles() {
    let errors = errors_of(TONAL_CONSTRUCTION);
    assert!(errors.is_empty(), "the example must compile: {errors:?}");
}
