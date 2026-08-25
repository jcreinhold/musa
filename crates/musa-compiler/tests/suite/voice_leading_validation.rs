//! What the voice-leading and counterpoint readings may and may not say.
//!
//! `docs/rules/language/07-analysis.md` §2 admits a kind only with an abstract
//! domain, an abstraction map, and a soundness claim. These are the
//! claims, written as tests:
//!
//! - **A departure states a motion, not a verdict.** Every departure is
//!   decidable from the notated score and is reported as a fact about the
//!   motion. What one pedagogy makes of that motion travels beside it as the
//!   rule's strength, and the report never collapses the two.
//! - **Every rule cites a page.** A rule with no citation cannot be
//!   constructed, and a finding carries the id, the sentence, the strength,
//!   and the page it came from.
//! - **A profile looks at what it says it looks at.** The rules in force are
//!   announced before any departure, and no departure names a rule the profile
//!   does not check.
//! - **Each rule has a boundary and a false positive.** For every rule id in
//!   the registry, one passage that departs and one that sits exactly on the
//!   line and does not — because a rule that fires on the boundary is a rule
//!   nobody can write music against.
//! - **A rule that needs a key is silent without one.** The two SATB
//!   leading-tone rules read the key in force; where none is written they say
//!   nothing rather than guessing one, and under a key the *request* supplied
//!   their departures are candidates.
//! - **Ties are one note.** A suspension is a note held across a barline, and
//!   the reading sees one occurrence with one span, not two notes that touch.
//! - **The fourth is a dissonance below and a consonance above.** Which one it
//!   is depends on the voice underneath, which is why the profile needs the
//!   cantus named rather than inferred.
//! - **Analysis and assertion are separate.** `analyze` never raises a
//!   diagnostic; `assert follows(…)` is the only place a style rule stops a
//!   build, and it can name only the rules decidable from a bare passage.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::indexing_slicing)]
// This suite picks the departure and rule-in-force shapes out of the
// observation enum and ignores the rest; the lint exists so a new variant is
// considered where it matters, and "which of these are departures" is not one
// of those places.
#![allow(clippy::wildcard_enum_match_arm)]

use std::collections::BTreeSet;
use std::fmt::Write as _;

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::{
    AnalysisKind, AnalysisProfile, AnalysisReport, AnalysisRequest, Evidence, Observation, ScoreSnapshot, Severity,
    Standing, Strength, analyze, rule_names,
};

const SATB: &str = include_str!("../../../../examples/analysis/satb.musa");
const SPECIES_1: &str = include_str!("../../../../examples/analysis/species-1.musa");
const SPECIES_2: &str = include_str!("../../../../examples/analysis/species-2.musa");
const SPECIES_3: &str = include_str!("../../../../examples/analysis/species-3.musa");
const SPECIES_4: &str = include_str!("../../../../examples/analysis/species-4.musa");
const SPECIES_5: &str = include_str!("../../../../examples/analysis/species-5.musa");
const JAZZ: &str = include_str!("../../../../examples/analysis/jazz-voice-leading.musa");
const ASSERTED: &str = include_str!("../../../../examples/analysis/asserted-voicings.musa");

// ------------------------------------------------------------------ fixtures

/// Compile a source and hand back the snapshot the analyses read.
fn score(source: &str, name: &str) -> ScoreSnapshot {
    let compiled = compile(&SourceDocument::new(source, name), &CompileOptions::default());
    assert!(
        !compiled.has_errors(),
        "the fixture must compile ({name}): {:?}",
        compiled
            .diagnostics()
            .iter()
            .filter(|found| found.severity == Severity::Error)
            .map(|found| found.message.clone())
            .collect::<Vec<_>>()
    );
    compiled.into_snapshot().expect("a score")
}

/// Read a source under a profile, naming `cantus` where the profile needs one.
fn read(source: &str, name: &str, profile: AnalysisProfile) -> AnalysisReport {
    let mut request = AnalysisRequest::new(profile.kind()).under(profile);
    if profile.needs_cantus() {
        request = request.designating("cantus");
    }
    analyze(&score(source, name), &request).expect("a well-formed request")
}

/// The rule ids this report reports a departure from, with repeats kept.
fn departed(report: &AnalysisReport) -> Vec<&'static str> {
    report
        .findings()
        .iter()
        .filter_map(|finding| match *finding.observation() {
            Observation::Departure { rule, .. } => Some(rule.id()),
            _ => None,
        })
        .collect()
}

/// The rule ids this report announces it looked at.
fn in_force(report: &AnalysisReport) -> BTreeSet<&'static str> {
    report
        .findings()
        .iter()
        .filter_map(|finding| match *finding.observation() {
            Observation::RuleInForce { rule, .. } => Some(rule.id()),
            _ => None,
        })
        .collect()
}

/// Every committed fixture with the profile its header says to read it under.
fn corpus() -> Vec<(&'static str, &'static str, AnalysisProfile)> {
    vec![
        (SATB, "satb.musa", AnalysisProfile::Satb),
        (SPECIES_1, "species-1.musa", AnalysisProfile::Species1),
        (SPECIES_2, "species-2.musa", AnalysisProfile::Species2),
        (SPECIES_3, "species-3.musa", AnalysisProfile::Species3),
        (SPECIES_4, "species-4.musa", AnalysisProfile::Species4),
        (SPECIES_5, "species-5.musa", AnalysisProfile::Species5),
        (JAZZ, "jazz.musa", AnalysisProfile::JazzVoiceLeading),
    ]
}

// -------------------------------------------------------------- source shapes

/// A four-voice piece, bottom voice first, in the key given.
///
/// The voices are named as OMT `022` names them, because the findings print
/// these names and a report that said "voice 2" would be describing the data
/// structure rather than the music.
fn satb_in(key: Option<&str>, voices: [&str; 4]) -> String {
    let names = ["bass", "tenor", "alto", "soprano"];
    let lines = names.iter().zip(voices).fold(String::new(), |mut text, (name, bars)| {
        let _ = writeln!(text, "            voice {name} {{ {bars} }}");
        text
    });
    let key = key.map_or(String::new(), |key| format!("    key {key};\n"));
    format!(
        "piece \"case\" {{\n    composer \"musa\";\n\n    tempo 1/4 = 60;\n    meter 4/4;\n{key}\n\
             score {{\n        part choir {{\n            clef treble;\n\n{lines}        }}\n    }}\n}}\n"
    )
}

/// Four voices in C major, the ordinary case.
fn satb(voices: [&str; 4]) -> String {
    satb_in(Some("c major"), voices)
}

/// Two voices, the second of which is the cantus firmus a species request
/// designates.
fn species(counter: &str, cantus: &str) -> String {
    format!(
        "piece \"case\" {{\n    composer \"musa\";\n\n    tempo 1/4 = 60;\n    meter 4/4;\n    key c major;\n\n\
             score {{\n        part exercise {{\n            clef treble;\n\n\
         \x20           voice counter {{ {counter} }}\n\n            voice cantus {{ {cantus} }}\n\
         \x20       }}\n    }}\n}}\n"
    )
}

/// A voicing sequence under a written harmony lane, which is where the jazz
/// profile gets its chords: nothing here derives a symbol from the notes.
fn jazz(harmony: &[&str], voices: [&str; 4]) -> String {
    let marks = harmony
        .iter()
        .enumerate()
        .fold(String::new(), |mut text, (bar, symbol)| {
            let _ = writeln!(text, "            at {}:1 {symbol};", bar.saturating_add(1));
            text
        });
    let names = ["bass", "tenor", "alto", "soprano"];
    let lines = names.iter().zip(voices).fold(String::new(), |mut text, (name, bars)| {
        let _ = writeln!(text, "            voice {name} {{ {bars} }}");
        text
    });
    format!(
        "piece \"case\" {{\n    composer \"musa\";\n\n    tempo 1/4 = 120;\n    meter 4/4;\n    key c major;\n\n\
             score {{\n        harmony {{\n{marks}        }}\n\n        part combo {{\n            clef treble;\n\n\
         {lines}        }}\n    }}\n}}\n"
    )
}

// ------------------------------------------------------- the per-rule cases

/// One rule, the passage that departs from it, and the passage that does not.
///
/// `silent` is the whole point of the row. Anyone can write a checker that
/// fires; the question is whether it fires on music that sits exactly on the
/// line the rule draws, and on music that merely resembles a departure.
struct Case {
    /// The rule id this row is about.
    rule: &'static str,
    /// The profile that checks it.
    profile: AnalysisProfile,
    /// A passage that departs from the rule, and why it does.
    departs: (String, &'static str),
    /// Passages that must not depart from it: the boundary, then the false
    /// positive the rule is most likely to produce.
    silent: [(String, &'static str); 2],
}

#[expect(clippy::too_many_lines, reason = "one row per rule id is the point of the table")]
fn cases() -> Vec<Case> {
    use AnalysisProfile::{JazzVoiceLeading, Satb, Species1, Species2, Species3, Species4, Species5};
    vec![
        Case {
            rule: "satb_four_voices",
            profile: Satb,
            departs: (
                satb(["| c3/1", "| g3/1", "| c4/1", "| rest/1"]),
                "three voices sound where the profile is about four",
            ),
            silent: [
                (
                    satb(["| c3/1", "| g3/1", "| c4/1", "| e4/1"]),
                    "four voices, one note each, is the definition itself",
                ),
                (
                    satb(["| c3/1", "| g3/1", "| c4/1", "| c4/1"]),
                    "two voices at the same pitch are still two voices",
                ),
            ],
        },
        Case {
            rule: "satb_ranges",
            profile: Satb,
            departs: (
                satb(["| e2/1", "| g3/1", "| c4/1", "| e4/1"]),
                "the bass sits a semitone below the f2 OMT 022 writes for it",
            ),
            silent: [
                (
                    satb(["| f2/1", "| g3/1", "| c4/1", "| e4/1"]),
                    "f2 is the bottom of the range, and the bottom is inside it",
                ),
                (
                    satb(["| c3/1", "| g3/1", "| d5/1", "| g5/1"]),
                    "the top of the alto and soprano ranges is inside them too",
                ),
            ],
        },
        Case {
            rule: "satb_spacing",
            profile: Satb,
            departs: (
                satb(["| c3/1", "| g3/1", "| c4/1", "| d5/1"]),
                "a ninth between alto and soprano is past the octave",
            ),
            silent: [
                (
                    satb(["| c3/1", "| g3/1", "| c4/1", "| c5/1"]),
                    "exactly an octave between adjacent upper voices",
                ),
                (
                    satb(["| f2/1", "| c4/1", "| e4/1", "| g4/1"]),
                    "a thirteenth from bass to tenor: the bass is exempt",
                ),
            ],
        },
        Case {
            rule: "satb_crossing",
            profile: Satb,
            departs: (
                satb([
                    "| c3/1 | c3/1 | c3/1",
                    "| g3/1 | b3/1 | g3/1",
                    "| c4/1 | a3/1 | c4/1",
                    "| e4/1 | e4/1 | e4/1",
                ]),
                "the alto dips to a3 in the second bar, below the tenor's b3",
            ),
            silent: [
                (
                    satb(["| c3/1", "| g3/1", "| g3/1", "| e4/1"]),
                    "tenor and alto in unison have not crossed",
                ),
                (
                    satb(["| c3/1", "| c4/1", "| e4/1", "| g4/1"]),
                    "voices close together but in order",
                ),
            ],
        },
        Case {
            rule: "satb_overlap",
            profile: Satb,
            departs: (
                satb(["| c3/1 | c3/1", "| g3/1 | g4/1", "| d4/1 | d5/1", "| f4/1 | f5/1"]),
                "the alto climbs to d5, past the f4 the soprano has just left",
            ),
            silent: [
                (
                    satb(["| c3/1 | c3/1", "| g3/1 | g3/1", "| c4/1 | f4/1", "| f4/1 | a4/1"]),
                    "the alto arrives exactly on the note the soprano left",
                ),
                (
                    satb(["| c3/1 | c3/1", "| g3/1 | g3/1", "| c4/1 | e4/1", "| e4/1 | g4/1"]),
                    "both voices rise without either passing where its neighbour was",
                ),
            ],
        },
        Case {
            rule: "satb_parallel_perfects",
            profile: Satb,
            departs: (
                satb(["| c3/1 | d3/1", "| g3/1 | a3/1", "| c4/1 | d4/1", "| e4/1 | f4/1"]),
                "bass and tenor move from a fifth to a fifth",
            ),
            silent: [
                (
                    satb(["| c3/1 | d3/1", "| g3/1 | g3/1", "| c4/1 | b3/1", "| e4/1 | d4/1"]),
                    "a fifth reached from a fifth with one voice still is oblique motion",
                ),
                (
                    satb(["| c3/1 | d3/1", "| g3/1 | d4/1", "| c4/1 | f4/1", "| e4/1 | a4/1"]),
                    "bass and tenor move from a fifth to an octave, which is not the same size",
                ),
            ],
        },
        Case {
            rule: "satb_direct_perfects",
            profile: Satb,
            departs: (
                satb(["| d3/1 | f3/1", "| a3/1 | b3/1", "| d4/1 | a4/1", "| b4/1 | f5/1"]),
                "the outer voices arrive at an octave in similar motion with a leap on top",
            ),
            silent: [
                (
                    satb(["| d3/1 | f3/1", "| a3/1 | b3/1", "| d4/1 | a4/1", "| e5/1 | f5/1"]),
                    "the same arrival with the soprano moving by step",
                ),
                (
                    satb(["| a3/1 | f3/1", "| c4/1 | b3/1", "| d4/1 | a4/1", "| b4/1 | f5/1"]),
                    "the same arrival in contrary motion",
                ),
            ],
        },
        Case {
            rule: "satb_doubling",
            profile: Satb,
            departs: (
                satb(["| c3/1", "| b3/1", "| b4/1", "| e4/1"]),
                "the leading tone of C major sounds in two voices at once",
            ),
            silent: [
                (
                    satb(["| c3/1", "| b3/1", "| c4/1", "| e4/1"]),
                    "one leading tone is not a doubled one",
                ),
                (
                    satb_in(None, ["| c3/1", "| b3/1", "| b4/1", "| e4/1"]),
                    "with no key written there is no leading tone to double",
                ),
            ],
        },
        Case {
            rule: "satb_tendency_resolution",
            profile: Satb,
            departs: (
                satb(["| c3/1 | c3/1", "| b3/1 | g3/1", "| d4/1 | d4/1", "| e4/1 | e4/1"]),
                "the leading tone falls a third instead of rising",
            ),
            silent: [
                (
                    satb(["| c3/1 | c3/1", "| b3/1 | c4/1", "| d4/1 | e4/1", "| g4/1 | g4/1"]),
                    "the leading tone rises to the tonic",
                ),
                (
                    satb_in(
                        None,
                        ["| c3/1 | c3/1", "| b3/1 | g3/1", "| d4/1 | d4/1", "| e4/1 | e4/1"],
                    ),
                    "with no key written the rule has nothing to read",
                ),
            ],
        },
        Case {
            rule: "species_rhythm",
            profile: Species1,
            departs: (
                species("| c4/2 d4/2 | e4/1 | c4/1", "| c3/1 | c3/1 | c3/1"),
                "two notes against one is not first species",
            ),
            silent: [
                (
                    species("| g3/1 | a3/1 | g3/1", "| c3/1 | f3/1 | c3/1"),
                    "note against note throughout",
                ),
                (
                    species("| g3/1 | a3/1 | g3/1", "| c3/1 | f3/1 | c3/1"),
                    "the same passage read again is still first species",
                ),
            ],
        },
        Case {
            rule: "species_begin",
            profile: Species1,
            departs: (
                species("| e3/1 | a3/1 | g3/1", "| c3/1 | f3/1 | c3/1"),
                "a third is not a perfect consonance to begin on",
            ),
            silent: [
                (
                    species("| g3/1 | a3/1 | g3/1", "| c3/1 | f3/1 | c3/1"),
                    "a fifth is perfect, and a fifth is how it may begin",
                ),
                (
                    species("| c4/1 | a3/1 | g3/1", "| c3/1 | f3/1 | c3/1"),
                    "an octave is the other way to begin",
                ),
            ],
        },
        Case {
            rule: "species_end",
            profile: Species1,
            departs: (
                species("| c4/1 | e3/1 | c4/1", "| c3/1 | f3/1 | c3/1"),
                "the final octave is reached by leap rather than by step",
            ),
            silent: [
                (
                    species("| c4/1 | b3/1 | c4/1", "| c3/1 | d3/1 | c3/1"),
                    "the octave is approached by step in both voices",
                ),
                (
                    species("| c4/1 | d4/1 | c4/1", "| c4/1 | b3/1 | c4/1"),
                    "a unison is an ending too, and this one is stepped into",
                ),
            ],
        },
        Case {
            rule: "species_consonance",
            profile: Species1,
            departs: (
                species("| c4/1 | b3/1 | c4/1", "| c3/1 | f3/1 | c3/1"),
                "b3 over f3 is a diminished fifth, which is not a consonance",
            ),
            silent: [
                (
                    species("| c4/1 | a3/1 | c4/1", "| c3/1 | f3/1 | c3/1"),
                    "a third over the cantus is a consonance",
                ),
                (
                    species("| c4/1 | d4/1 | c4/1", "| c3/1 | f3/1 | c3/1"),
                    "a sixth over the cantus is a consonance, as is the octave either side of it",
                ),
            ],
        },
        Case {
            rule: "species_dissonance_passing",
            profile: Species2,
            departs: (
                species("| c4/2 d4/2 | b3/2 g3/2 | c4/1", "| c3/1 | e3/1 | c3/1"),
                "the dissonant d4 is quitted by leap instead of continuing by step",
            ),
            silent: [
                (
                    species("| c4/2 b3/2 | a3/2 b3/2 | c4/1", "| c3/1 | f3/1 | c3/1"),
                    "each dissonance is passed through by step in one direction",
                ),
                (
                    species("| c4/2 a3/2 | c4/2 a3/2 | c4/1", "| c3/1 | f3/1 | c3/1"),
                    "every offbeat note here is consonant, so the rule has nothing to judge",
                ),
            ],
        },
        Case {
            rule: "species_parallel_perfects",
            profile: Species1,
            departs: (
                species("| g3/1 | a3/1 | g3/1", "| c3/1 | d3/1 | c3/1"),
                "both voices rise a step from a fifth to a fifth",
            ),
            silent: [
                (
                    species("| g3/1 | g3/1 | g3/1", "| c3/1 | d3/1 | c3/1"),
                    "the upper voice stays: a fifth reached obliquely is no parallel",
                ),
                (
                    species("| g3/1 | f3/1 | g3/1", "| c3/1 | d3/1 | c3/1"),
                    "contrary motion into a third",
                ),
            ],
        },
        Case {
            rule: "species_direct_perfects",
            profile: Species1,
            departs: (
                species("| a3/1 | d4/1 | c4/1", "| c3/1 | d3/1 | c3/1"),
                "both voices rise into an octave with a leap in the upper one",
            ),
            silent: [
                (
                    species("| c4/1 | d4/1 | c4/1", "| c3/1 | d3/1 | c3/1"),
                    "the same octave reached with the upper voice moving by step",
                ),
                (
                    species("| e4/1 | d4/1 | c4/1", "| c3/1 | d3/1 | c3/1"),
                    "the same octave reached in contrary motion",
                ),
            ],
        },
        Case {
            rule: "species_leap_recovery",
            profile: Species1,
            departs: (
                species("| c4/1 | c5/1 | d5/1 | c4/1", "| c3/1 | e3/1 | f3/1 | c3/1"),
                "an octave leap upward continues upward instead of turning back",
            ),
            silent: [
                (
                    species("| c4/1 | g4/1 | f4/1 | c4/1", "| c3/1 | e3/1 | f3/1 | c3/1"),
                    "the leap is answered by a step in the other direction",
                ),
                (
                    species("| c4/1 | d4/1 | e4/1 | c4/1", "| c3/1 | e3/1 | f3/1 | c3/1"),
                    "stepwise motion is not a leap to recover from",
                ),
            ],
        },
        Case {
            rule: "species_suspension",
            profile: Species4,
            departs: (
                species("| rest/2 d4/2 ~ | d4/2 e4/2 ~ | e4/2 c4/2", "| c3/1 | e3/1 | c3/1"),
                "the suspended note is quitted upward, which a suspension may not do",
            ),
            silent: [
                (
                    species("| rest/2 g4/2 ~ | g4/2 f4/2 ~ | f4/2 e4/2", "| c3/1 | f3/1 | c3/1"),
                    "prepared, held, and resolved down by step",
                ),
                (
                    species("| rest/2 e4/2 ~ | e4/2 c4/2 ~ | c4/2 c4/2", "| c3/1 | a3/1 | f3/1"),
                    "a tie over a consonance is not a suspension to judge",
                ),
            ],
        },
        Case {
            rule: "jazz_guide_tones",
            profile: JazzVoiceLeading,
            departs: (
                jazz(&["dm7"], ["| d3/1", "| a3/1", "| d4/1", "| e4/1"]),
                "neither the third nor the seventh of the chord is voiced",
            ),
            silent: [
                (
                    jazz(&["dm7"], ["| d3/1", "| f3/1", "| c4/1", "| e4/1"]),
                    "both guide tones are present, which is the whole of the rule",
                ),
                (
                    jazz(&["dm7"], ["| d3/1", "| c4/1", "| f4/1", "| a4/1"]),
                    "the guide tones voiced in another order are still present",
                ),
            ],
        },
        Case {
            rule: "jazz_guide_tone_motion",
            profile: JazzVoiceLeading,
            departs: (
                jazz(
                    &["dm7", "g7"],
                    ["| d3/1 | g2/1", "| f3/1 | b3/1", "| c4/1 | f4/1", "| e4/1 | d4/1"],
                ),
                "the third leaps a fourth to reach the next voicing",
            ),
            silent: [
                (
                    jazz(
                        &["dm7", "g7"],
                        ["| d3/1 | g2/1", "| f3/1 | f3/1", "| c4/1 | b3/1", "| e4/1 | d4/1"],
                    ),
                    "one guide tone is held and the other moves a step",
                ),
                (
                    jazz(
                        &["dm7", "g7"],
                        ["| d3/1 | g2/1", "| f3/1 | f3/1", "| c4/1 | b3/1", "| a4/1 | g4/1"],
                    ),
                    "a voice that is not a guide tone may move further",
                ),
            ],
        },
        Case {
            rule: "jazz_common_tone",
            profile: JazzVoiceLeading,
            departs: (
                jazz(
                    &["dm7", "g7"],
                    ["| d3/1 | g2/1", "| f3/1 | f4/1", "| c4/1 | b4/1", "| a4/1 | d5/1"],
                ),
                "the two chords share F and C and not one of them stays where it is",
            ),
            silent: [
                (
                    jazz(
                        &["dm7", "g7"],
                        ["| d3/1 | g2/1", "| f3/1 | f3/1", "| c4/1 | b3/1", "| e4/1 | d4/1"],
                    ),
                    "the shared F is held in the same voice at the same pitch",
                ),
                (
                    jazz(
                        &["cmaj7", "dm7"],
                        ["| c3/1 | d3/1", "| e3/1 | f3/1", "| b3/1 | c4/1", "| e4/1 | e4/1"],
                    ),
                    "the shared E stays where it is while the rest of the voicing steps",
                ),
            ],
        },
        Case {
            rule: "jazz_small_motion",
            profile: JazzVoiceLeading,
            departs: (
                jazz(
                    &["dm7", "g7"],
                    ["| d3/1 | g2/1", "| f3/1 | f3/1", "| c4/1 | b3/1", "| e4/1 | d5/1"],
                ),
                "the soprano leaps a seventh between voicings",
            ),
            silent: [
                (
                    jazz(
                        &["dm7", "g7"],
                        ["| d3/1 | g2/1", "| f3/1 | f3/1", "| c4/1 | b3/1", "| e4/1 | g4/1"],
                    ),
                    "a third is exactly as far as an upper voice may travel",
                ),
                (
                    jazz(
                        &["dm7", "g7"],
                        ["| d3/1 | g2/1", "| f3/1 | f3/1", "| c4/1 | b3/1", "| e4/1 | d4/1"],
                    ),
                    "the bass walks a fifth and is exempt, because the rule is about the voicing",
                ),
            ],
        },
        Case {
            rule: "jazz_spacing",
            profile: JazzVoiceLeading,
            departs: (
                jazz(&["dm7"], ["| d3/1", "| f3/1", "| c4/1", "| e5/1"]),
                "a tenth between the two top voices opens the voicing past an octave",
            ),
            silent: [
                (
                    jazz(&["dm7"], ["| d3/1", "| f3/1", "| c4/1", "| c5/1"]),
                    "exactly an octave is inside the interval the rule allows",
                ),
                (
                    jazz(&["dm7"], ["| d4/1", "| e4/1", "| a4/1", "| c5/1"]),
                    "a second above middle C is bright rather than muddy",
                ),
            ],
        },
        Case {
            rule: "jazz_omission",
            profile: JazzVoiceLeading,
            departs: (
                jazz(&["dm7"], ["| e3/1", "| f3/1", "| c4/1", "| e4/1"]),
                "neither the root nor the fifth of the chord is anywhere in the voicing",
            ),
            silent: [
                (
                    jazz(&["dm7"], ["| a2/1", "| f3/1", "| c4/1", "| e4/1"]),
                    "the fifth alone accounts for the chord: the root may be left to the bass player",
                ),
                (
                    jazz(&["dm7"], ["| d3/1", "| f3/1", "| c4/1", "| e4/1"]),
                    "the root is present",
                ),
            ],
        },
        Case {
            rule: "species_dissonance_passing",
            profile: Species3,
            departs: (
                species(
                    "| c4/4 d4/4 g4/4 e4/4 | c4/4 d4/4 e4/4 d4/4 | c4/1",
                    "| c3/1 | e3/1 | c3/1",
                ),
                "third species dissonance quitted by a leap",
            ),
            silent: [
                (
                    species(
                        "| c4/4 b3/4 a3/4 g3/4 | a3/4 b3/4 c4/4 d4/4 | c4/1",
                        "| c3/1 | f3/1 | c3/1",
                    ),
                    "every dissonance is passed through by step",
                ),
                (
                    species(
                        "| c4/4 a3/4 c4/4 e4/4 | c4/4 a3/4 c4/4 f4/4 | c4/1",
                        "| c3/1 | f3/1 | c3/1",
                    ),
                    "all four notes of each bar are consonant",
                ),
            ],
        },
        Case {
            rule: "species_suspension",
            profile: Species5,
            departs: (
                species(
                    "| c4/1 | rest/2 d4/2 ~ | d4/2 e4/2 | c4/1",
                    "| c3/1 | c3/1 | e3/1 | c3/1",
                ),
                "a fifth-species measure suspends and then quits upward",
            ),
            silent: [
                (
                    species(
                        "| c4/1 | rest/2 g4/2 ~ | g4/2 f4/2 | c4/1",
                        "| c3/1 | c3/1 | f3/1 | c3/1",
                    ),
                    "the suspension resolves downward by step",
                ),
                (
                    species(
                        "| c4/1 | a3/2 b3/2 | c4/4 b3/4 a3/4 g3/4 | c4/1",
                        "| c3/1 | c3/1 | f3/1 | c3/1",
                    ),
                    "a measure with no tie at all has no suspension to judge",
                ),
            ],
        },
    ]
}

// ------------------------------------------------------------------ the tests

/// Every rule the registry knows is checked by some profile and exercised by
/// some row of the table.
///
/// A rule nothing checks is a sentence in a table pretending to be a rule; a
/// rule no case departs from is a checker nobody has ever seen fire.
#[test]
fn every_rule_is_checked_and_exercised() {
    let registry: BTreeSet<&str> = rule_names().map(|rule| rule.id()).collect();
    let mut announced: BTreeSet<&str> = BTreeSet::new();
    for profile in AnalysisProfile::ALL {
        let (source, name, _) = corpus()
            .into_iter()
            .find(|(_, _, under)| *under == profile)
            .expect("every profile has a fixture");
        announced.extend(in_force(&read(source, name, profile)));
    }
    assert_eq!(
        registry, announced,
        "a rule in the registry that no profile announces, or the other way round"
    );

    let exercised: BTreeSet<&str> = cases().iter().map(|case| case.rule).collect();
    assert_eq!(registry, exercised, "the case table must have a row for every rule id");
}

/// For each rule: the departure fires, and neither the boundary nor the
/// lookalike does.
#[test]
fn each_rule_has_a_boundary_and_a_false_positive() {
    for case in cases() {
        let (source, why) = &case.departs;
        let found = departed(&read(source, "departs.musa", case.profile));
        assert!(
            found.contains(&case.rule),
            "{}: expected a departure because {why}, got {found:?}",
            case.rule
        );
        for (source, why) in &case.silent {
            let found = departed(&read(source, "silent.musa", case.profile));
            assert!(
                !found.contains(&case.rule),
                "{}: fired where it must not, because {why}; got {found:?}",
                case.rule
            );
        }
    }
}

/// Every departure carries the rule it departs from, with its sentence, its
/// strength, and the page it is written on.
#[test]
fn every_departure_cites_its_page() {
    for (source, name, profile) in corpus() {
        for finding in read(source, name, profile).findings() {
            let (Observation::Departure { rule, .. } | Observation::RuleInForce { rule, .. }) = *finding.observation()
            else {
                continue;
            };
            assert!(!rule.id().is_empty(), "{name}: a rule with no id");
            assert!(!rule.states().is_empty(), "{name}: {} states nothing", rule.id());
            assert!(
                rule.cites().starts_with("OMT "),
                "{name}: {} cites {:?}, which is not a page of Open EventTrack(WrittenTime) Theory",
                rule.id(),
                rule.cites()
            );
        }
    }
}

/// A departure states a motion; the strength beside it states what a pedagogy
/// makes of that motion. The two never merge.
#[test]
fn a_departure_states_the_motion_and_the_strength_states_the_pedagogy() {
    for (source, name, profile) in corpus() {
        let report = read(source, name, profile);
        for finding in report.findings() {
            let Observation::Departure { rule, .. } = *finding.observation() else {
                continue;
            };
            let expected = if rule.strength() == Strength::Definitional {
                // A texture the profile is not about is a conflict between
                // the request and the music, not a departure within a style.
                Standing::Conflict
            } else {
                Standing::Fact
            };
            assert_eq!(
                finding.standing(),
                expected,
                "{name}: {} reports a {:?} where the notated score decides it",
                rule.id(),
                finding.standing()
            );
            assert!(
                finding.grounds().iter().any(|ground| !ground.satisfied),
                "{name}: {} reports a departure with nothing unsatisfied",
                rule.id()
            );
        }
    }
    // OMT 076 titles its own section "Guidelines versus Rules". The profile
    // takes it at its word, and none of its rules may be anything else.
    let jazz = read(JAZZ, "jazz.musa", AnalysisProfile::JazzVoiceLeading);
    for finding in jazz.findings() {
        let (Observation::Departure { rule, .. } | Observation::RuleInForce { rule, .. }) = *finding.observation()
        else {
            continue;
        };
        assert_eq!(
            rule.strength(),
            Strength::Guideline,
            "{} is not a guideline, and every jazz rule is one",
            rule.id()
        );
    }
}

/// No profile reports a departure from a rule it never said it would check.
#[test]
fn a_profile_departs_only_from_the_rules_it_announced() {
    for (source, name, profile) in corpus() {
        let report = read(source, name, profile);
        let announced = in_force(&report);
        for rule in departed(&report) {
            assert!(
                announced.contains(rule),
                "{name}: departed from {rule}, which this profile never announced"
            );
        }
    }
}

/// The two rules that read a key are silent without one, and are candidates
/// when the key came from the request rather than the source.
#[test]
fn the_rules_that_need_a_key_say_so() {
    let keyed = ["satb_doubling", "satb_tendency_resolution"];
    let bare = satb_in(
        None,
        ["| c3/1 | c3/1", "| b3/1 | g3/1", "| b4/1 | b4/1", "| e4/1 | e4/1"],
    );
    let found = departed(&read(&bare, "keyless.musa", AnalysisProfile::Satb));
    for rule in keyed {
        assert!(
            !found.contains(&rule),
            "{rule} read a key that the source never wrote: {found:?}"
        );
    }

    let request = AnalysisRequest::new(AnalysisKind::VoiceLeading)
        .under(AnalysisProfile::Satb)
        .in_key(musa_score::Key::new(
            musa_score::PitchClass::parse("c").expect("c is a pitch class"),
            musa_score::Mode::Major,
        ));
    let report = analyze(&score(&bare, "keyless.musa"), &request).expect("a well-formed request");
    let assumed: Vec<Standing> = report
        .findings()
        .iter()
        .filter(|finding| match *finding.observation() {
            Observation::Departure { rule, .. } => keyed.contains(&rule.id()),
            _ => false,
        })
        .map(musa_score::AnalysisFinding::standing)
        .collect();
    assert!(!assumed.is_empty(), "the assumed key should let both rules speak");
    assert!(
        assumed.iter().all(|standing| *standing == Standing::Candidate),
        "under an assumed key these are candidates, not facts: {assumed:?}"
    );
}

/// In a minor key the leading tone is the *raised* seventh, and the seventh
/// the key signature spells is not one.
///
/// The two rules that read a key ask for a note a semitone below the tonic, and
/// a minor key signature does not supply one: it spells `bb` in C minor, a whole
/// step down, which is a subtonic and leads nowhere. What leads is the `b` a
/// minor piece writes as an accidental. So the reading is against the collection
/// that has a leading tone (harmonic minor), not against the signature's own —
/// and the two answers are opposite on both notes, in both directions, which is
/// why this is pinned rather than left to the C-major rows above.
#[test]
fn a_minor_key_leads_with_its_raised_seventh() {
    let doubled_raised = satb_in(Some("c minor"), ["| c3/1", "| b3/1", "| b4/1", "| eb4/1"]);
    let found = departed(&read(&doubled_raised, "raised.musa", AnalysisProfile::Satb));
    assert!(
        found.contains(&"satb_doubling"),
        "the raised seventh of C minor is its leading tone, and this doubles it: {found:?}"
    );

    let doubled_natural = satb_in(Some("c minor"), ["| c3/1", "| bb3/1", "| bb4/1", "| eb4/1"]);
    let found = departed(&read(&doubled_natural, "natural.musa", AnalysisProfile::Satb));
    assert!(
        !found.contains(&"satb_doubling"),
        "the signature's own seventh is a subtonic, not a leading tone to double: {found:?}"
    );

    let falls = satb_in(
        Some("c minor"),
        ["| c3/1 | c3/1", "| b3/1 | g3/1", "| d4/1 | d4/1", "| eb4/1 | eb4/1"],
    );
    let found = departed(&read(&falls, "falls.musa", AnalysisProfile::Satb));
    assert!(
        found.contains(&"satb_tendency_resolution"),
        "the raised seventh falls a third instead of rising: {found:?}"
    );

    let subtonic_falls = satb_in(
        Some("c minor"),
        ["| c3/1 | c3/1", "| bb3/1 | g3/1", "| d4/1 | d4/1", "| eb4/1 | eb4/1"],
    );
    let found = departed(&read(&subtonic_falls, "subtonic.musa", AnalysisProfile::Satb));
    assert!(
        !found.contains(&"satb_tendency_resolution"),
        "a subtonic is under no obligation to rise: {found:?}"
    );
}

/// A tie is one note. The suspension in `species-4.musa` is a single
/// occurrence spanning the barline, so the departure's span crosses it too.
#[test]
fn a_tie_is_one_occurrence_with_one_span() {
    let report = read(SPECIES_4, "species-4.musa", AnalysisProfile::Species4);
    let suspension = report
        .findings()
        .iter()
        .find(|finding| match *finding.observation() {
            Observation::Departure { rule, .. } => rule.id() == "species_suspension",
            _ => false,
        })
        .expect("the one departure this fixture is built around");
    let Observation::Departure { from, to, .. } = *suspension.observation() else {
        panic!("just matched a departure")
    };
    assert!(from < to, "a held note spans time: {from:?}..{to:?}");
    let Evidence::Passage { ref notes, .. } = *suspension.evidence() else {
        panic!("a departure cites the passage it read")
    };
    assert!(
        !notes.is_empty(),
        "a departure with no notes is a claim with no evidence"
    );
    // Ties are joined in elaboration, so the two written halves are one note
    // and the reading cannot see them as two.
    let mut seen = notes.clone();
    seen.dedup();
    assert_eq!(seen.len(), notes.len(), "the same note cited twice: {notes:?}");
}

/// The fourth is a dissonance against the bass and a consonance above it
/// (OMT 023), so the reading has to know which voice is underneath rather than
/// which voice is the counterpoint.
#[test]
fn the_fourth_depends_on_which_voice_is_below() {
    // The counterpoint a fourth above the cantus: the cantus is the bass, and
    // the fourth is the dissonant one.
    let above = species("| c4/1 | g3/1 | c4/1", "| c3/1 | d3/1 | c3/1");
    let found = departed(&read(&above, "fourth-above.musa", AnalysisProfile::Species1));
    assert!(
        found.contains(&"species_consonance"),
        "a fourth over the bass is a dissonance: {found:?}"
    );

    // The same written fourth with the cantus on top. The counterpoint is now
    // the bass, and the interval is still measured from the voice underneath —
    // so it is still the dissonant fourth, and nothing here treats "the
    // counterpoint" as a synonym for "the upper voice".
    let below = species("| d3/1 | d3/1 | c3/1", "| c4/1 | g3/1 | c4/1");
    let found = departed(&read(&below, "fourth-below.musa", AnalysisProfile::Species1));
    assert!(
        found.contains(&"species_consonance"),
        "the fourth is measured from the voice underneath, whichever line that is: {found:?}"
    );

    // Its inversion is not the dissonant one: a fifth over the bass stands,
    // which is what makes this a rule about the fourth and not about a number.
    let fifth = species("| c4/1 | a3/1 | c4/1", "| c3/1 | d3/1 | c3/1");
    let found = departed(&read(&fifth, "fifth.musa", AnalysisProfile::Species1));
    assert!(
        !found.contains(&"species_consonance"),
        "a fifth and a sixth over the bass are consonances: {found:?}"
    );
}

/// Analysing never raises a diagnostic. A style rule stops a build in exactly
/// one place, and that place is an `assert` the composer wrote.
///
/// The two see different things, and that is the design rather than an
/// accident: an assertion sees the passage inside its own braces, so it can
/// only judge what sounds there, while an analysis reads the whole score and
/// every voice in it.
#[test]
fn analysis_observes_and_assertion_asserts() {
    let flawed = satb(["| c3/1 | d3/1", "| g3/1 | a3/1", "| c4/1 | d4/1", "| e4/1 | f4/1"]);
    let compiled = compile(&SourceDocument::new(&flawed, "flawed.musa"), &CompileOptions::default());
    assert!(!compiled.has_errors(), "parallel fifths are not a compile error");
    let report = read(&flawed, "flawed.musa", AnalysisProfile::Satb);
    assert!(
        departed(&report).contains(&"satb_parallel_perfects"),
        "the analysis sees what the compiler declined to judge"
    );

    // The same motion inside an assertion, written where the assertion can see
    // it: one passage whose sonorities are chords.
    let asserted = ASSERTED.replace(
        "                    | [c3 g3 c4 e4]/1\n                    | [g2 g3 b3 d4]/1",
        "                    | [c3 g3 c4 e4]/1\n                    | [d3 a3 d4 f4]/1",
    );
    assert_ne!(asserted, ASSERTED, "the substitution must have found its passage");
    let compiled = compile(
        &SourceDocument::new(&asserted, "asserted.musa"),
        &CompileOptions::default(),
    );
    assert!(
        compiled.has_errors(),
        "parallel fifths inside an assertion must fail the build"
    );

    // And the fixture as committed passes, which is what makes it a claim
    // rather than a comment.
    let compiled = compile(
        &SourceDocument::new(ASSERTED, "asserted-voicings.musa"),
        &CompileOptions::default(),
    );
    assert!(!compiled.has_errors(), "the asserted fixture must compile");
}

// ------------------------------------------------------------------- the laws

/// The same sonorities spelled as four lanes, which is what an analysis reads.
fn as_lanes(harmony: &[&str], bars: &[[&str; 4]]) -> String {
    let line = |position: usize| -> String {
        bars.iter().fold(String::new(), |mut text, bar| {
            let _ = write!(text, "| {}/1 ", bar[position]);
            text
        })
    };
    let (bass, tenor, alto, soprano) = (line(0), line(1), line(2), line(3));
    let voices = [bass.as_str(), tenor.as_str(), alto.as_str(), soprano.as_str()];
    if harmony.is_empty() {
        satb(voices)
    } else {
        jazz(harmony, voices)
    }
}

/// The same sonorities spelled as one lane of chords under a claim, which is
/// what an assertion reads.
///
/// The two spellings are the same music: the notes sound at the same instants
/// at the same pitches, and only the lanes they are written in differ.
fn as_claim(harmony: &[&str], bars: &[[&str; 4]], rule: &str) -> String {
    let music = bars.iter().fold(String::new(), |mut text, bar| {
        let _ = writeln!(
            text,
            "                    [{} {} {} {}]/1",
            bar[0], bar[1], bar[2], bar[3]
        );
        text
    });
    let marks = harmony
        .iter()
        .enumerate()
        .fold(String::new(), |mut text, (bar, symbol)| {
            let _ = writeln!(text, "            at {}:1 {symbol};", bar.saturating_add(1));
            text
        });
    let lane = if harmony.is_empty() {
        String::new()
    } else {
        format!("        harmony {{\n{marks}        }}\n\n")
    };
    format!(
        "piece \"case\" {{\n    composer \"musa\";\n\n    tempo 1/4 = 60;\n    meter 4/4;\n    key c major;\n\n\
             score {{\n{lane}        part combo {{\n            clef treble;\n\n            voice v {{\n\
         \x20               assert follows({rule}) {{\n{music}                }}\n            }}\n        }}\n    }}\n}}\n"
    )
}

/// One assertable rule, with music that departs from it and music that does
/// not, written as sonorities so both spellings can be generated from it.
struct Agreement {
    rule: &'static str,
    profile: AnalysisProfile,
    /// The chord symbols the jazz profile segments on; empty for SATB, which
    /// segments on attacks and reads no symbol.
    harmony: &'static [&'static str],
    departs: &'static [[&'static str; 4]],
    silent: &'static [[&'static str; 4]],
}

/// The five rules `assert follows(…)` admits, which is exactly the set
/// decidable from a passage's own sonorities.
fn agreements() -> Vec<Agreement> {
    vec![
        Agreement {
            rule: "satb_parallel_perfects",
            profile: AnalysisProfile::Satb,
            harmony: &[],
            departs: &[["c3", "g3", "c4", "e4"], ["d3", "a3", "d4", "f4"]],
            silent: &[["c3", "g3", "c4", "e4"], ["g2", "g3", "b3", "d4"]],
        },
        Agreement {
            rule: "satb_spacing",
            profile: AnalysisProfile::Satb,
            harmony: &[],
            departs: &[["c2", "g3", "c4", "d5"]],
            silent: &[["c2", "c4", "e4", "g4"]],
        },
        Agreement {
            rule: "satb_overlap",
            profile: AnalysisProfile::Satb,
            harmony: &[],
            departs: &[["c3", "g3", "c4", "f4"], ["c3", "g3", "a3", "b3"]],
            silent: &[["c3", "g3", "c4", "f4"], ["c3", "a3", "f4", "a4"]],
        },
        Agreement {
            rule: "jazz_small_motion",
            profile: AnalysisProfile::JazzVoiceLeading,
            harmony: &["dm7", "g7"],
            departs: &[["d3", "f3", "c4", "e4"], ["g2", "f3", "b3", "d5"]],
            silent: &[["d3", "f3", "c4", "e4"], ["g2", "f3", "b3", "d4"]],
        },
        Agreement {
            rule: "jazz_spacing",
            profile: AnalysisProfile::JazzVoiceLeading,
            harmony: &["dm7"],
            departs: &[["c3", "d3", "e3", "f3"]],
            silent: &[["d3", "a3", "c4", "f4"]],
        },
    ]
}

/// A rule id names one rule, and the two things that check it must agree.
///
/// `crates/musa-compiler/src/analysis/motion.rs` states each rule once and is
/// traversed twice — over a snapshot's voices for an analysis, over a
/// passage's sonorities for an assertion — so this is a law rather than a
/// coincidence, and a rule that drifted apart in the two traversals would be
/// two rules sharing a name. The music is generated once and spelled both
/// ways, because an agreement test that wrote the two passages out separately
/// would be asserting that two hand-copied sources say the same thing.
#[test]
fn a_rule_reads_the_same_from_an_assertion_and_from_an_analysis() {
    for row in agreements() {
        let seen = departed(&read(&as_lanes(row.harmony, row.departs), "departs.musa", row.profile));
        assert!(
            seen.contains(&row.rule),
            "{}: the analysis did not report the music the assertion refuses; got {seen:?}",
            row.rule
        );
        let claim = as_claim(row.harmony, row.departs, row.rule);
        let compiled = compile(&SourceDocument::new(&claim, "departs.musa"), &CompileOptions::default());
        assert!(
            compiled.has_errors(),
            "{}: the assertion accepted the music the analysis reports",
            row.rule
        );

        let seen = departed(&read(&as_lanes(row.harmony, row.silent), "silent.musa", row.profile));
        assert!(
            !seen.contains(&row.rule),
            "{}: the analysis reported music that keeps the rule; got {seen:?}",
            row.rule
        );
        let claim = as_claim(row.harmony, row.silent, row.rule);
        let compiled = compile(&SourceDocument::new(&claim, "silent.musa"), &CompileOptions::default());
        assert!(
            !compiled.has_errors(),
            "{}: the assertion refused music that keeps the rule",
            row.rule
        );
    }
}

/// The rule ids anchored to a register rather than to an interval.
///
/// These two are the whole exception to the law below, and they are exceptions
/// by design: OMT `022` writes the four voices' ranges as absolute pitches,
/// and OMT `076`'s muddiness is a fact about the bottom of the piano and not
/// about the size of a second.
const ANCHORED: [&str; 2] = ["satb_ranges", "jazz_spacing"];

/// The same music an octave higher, spelled the same way.
///
/// Lines beginning `at` are left alone: a chord symbol is not a pitch, and
/// `g7` is a dominant seventh rather than a note in the seventh octave.
fn octave_up(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("at ") {
                line.to_owned()
            } else {
                raise(line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// One line with every pitch token in it raised an octave.
///
/// A pitch token is a letter `a`–`g` that opens a word, any accidental, and an
/// octave digit. Requiring the letter to open a word is what keeps `bass`,
/// `cantus`, and `rest` out of it.
fn raise(line: &str) -> String {
    let text: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut at = 0;
    while let Some(&ch) = text.get(at) {
        let opens = at
            .checked_sub(1)
            .is_none_or(|before| matches!(text.get(before), Some(' ' | '[' | '|')));
        if !opens || !matches!(ch, 'a'..='g') {
            out.push(ch);
            at = at.saturating_add(1);
            continue;
        }
        let mut ahead = at.saturating_add(1);
        while matches!(text.get(ahead), Some('#' | 's' | 'f' | 'b' | 'x')) {
            ahead = ahead.saturating_add(1);
        }
        match text.get(ahead).and_then(|digit| digit.to_digit(10)) {
            Some(octave) => {
                out.extend(text.get(at..ahead).unwrap_or_default());
                out.push(char::from_digit(octave.saturating_add(1), 10).unwrap_or('9'));
                at = ahead.saturating_add(1);
            }
            None => {
                out.push(ch);
                at = at.saturating_add(1);
            }
        }
    }
    out
}

/// Transposition is a group acting on pitch, and every rule about *motion* is
/// invariant under it: a fifth is a fifth wherever it is sung, and two voices
/// moving in parallel keep moving in parallel when the whole passage moves.
///
/// So a departure set may not change when a passage is raised an octave —
/// except for the two rules that are about register rather than interval, and
/// naming them is the point. A rule that silently depended on absolute pitch
/// would be claiming that music means something different when a choir
/// transposes it, and this is the test that would catch it.
#[test]
fn raising_a_passage_an_octave_changes_only_the_register_rules() {
    for case in cases() {
        let passages = std::iter::once(&case.departs).chain(case.silent.iter());
        for (source, why) in passages {
            let read = |source: &str| -> Vec<&'static str> {
                let mut found = departed(&read(source, "transposed.musa", case.profile));
                found.retain(|rule| !ANCHORED.contains(rule));
                found.sort_unstable();
                found
            };
            assert_eq!(
                read(source),
                read(&octave_up(source)),
                "{}: raising the passage an octave changed what was reported ({why})",
                case.rule
            );
        }
    }
}

/// Reading a score twice gives the same report and leaves the score alone.
///
/// `docs/rules/language/07-analysis.md` §4 requires it of every kind: a report
/// nobody can diff against yesterday's is a report nobody can act on. The two
/// style kinds are tested here rather than beside the other four because they
/// are the ones that need a profile, and the counterpoint profiles need a
/// two-voice score with a cantus named — which is what this corpus is.
#[test]
fn a_style_reading_repeats_exactly_and_changes_nothing() {
    for (source, name, profile) in corpus() {
        let snapshot = score(source, name);
        let copy = snapshot.clone();
        let mut request = AnalysisRequest::new(profile.kind()).under(profile);
        if profile.needs_cantus() {
            request = request.designating("cantus");
        }
        let first = analyze(&snapshot, &request).expect("a well-formed request");
        let second = analyze(&snapshot, &request).expect("a well-formed request");
        assert_eq!(first, second, "{name} was not read the same way twice");
        assert_eq!(snapshot, copy, "reading {name} changed the score");
    }
}
