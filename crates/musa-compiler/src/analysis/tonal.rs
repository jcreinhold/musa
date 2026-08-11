//! The `tonal` kind: what the chords are doing, and in which key.
//!
//! **Abstract domain.** Three families, all indexed by a key:
//!
//! - a **key region**: a stretch of slices together with the set of keys whose
//!   collection accounts for every class sounding in it;
//! - a **numeral**: a (key, slice) pair together with the Roman numeral of a
//!   diatonic or applied chord standing in one of the four relations of
//!   [`Fit`] to the slice's classes;
//! - a **change of tonic**: a boundary between two key regions, read both as a
//!   tonicization and as a modulation, with the OMT `051` criteria each
//!   reading satisfies attached.
//!
//! **Abstraction map.** α reduces each slice to its spelled classes (as
//! `chords` does), scans the slice sequence accumulating classes, and closes a
//! region when no key accounts for the accumulation. Inside a region it stacks
//! the collection's own thirds on each degree (`Scale::stacked`, so a numeral
//! carries no quality of its own — the collection supplies it) and, for each
//! degree that can be tonicized, the applied dominant and applied leading-tone
//! chords of that degree. Minor keys are read against the natural *and*
//! harmonic collections, because a minor key routinely sounds its raised
//! seventh and a reading that refused it would find no dominant in any minor
//! piece (OMT `014`).
//!
//! **Soundness.** α does not determine a key. A key region is a fact exactly
//! when one key survives and the passage is one region; otherwise every
//! surviving key is a candidate and they are all in the report. A numeral is a
//! fact only when its region's key is a fact, its fit is exact, and it is the
//! only numeral the key offers for that slice — three conditions that fail
//! constantly, which is the honest result: a numeral is a reading of a chord
//! under a key, and both halves are readings.
//!
//! γ of a change-of-tonic finding is every score in which the notes change the
//! way the boundary says. That set contains scores a musician would hear as a
//! passing tonicization and scores they would hear as a modulation, and α
//! cannot separate them — the criteria that would (a confirming cadence, the
//! change persisting, the new key's notes recurring) are stated as
//! [`Ground`]s and the reader decides. **Duration alone never decides it**:
//! nothing here compares the length of a region to a threshold.
//!
//! **Recursion.** There is none, and that is deliberate. Peyton Jones (1987)
//! §22.3 warns that the obvious treatments of recursion in an abstract
//! interpretation are wrong; the region scan is a single left-to-right pass
//! over a finite slice sequence with a monotonically shrinking key set, so it
//! needs no fixed point and has none to get wrong. A reading that iterated —
//! keys informing segmentation informing keys — would owe that argument, and
//! is not what this does.

// Rational arithmetic on musical time is exact mathematical arithmetic, not
// raw integer ops; clippy::arithmetic_side_effects does not apply to it.
#![allow(clippy::arithmetic_side_effects)]

use super::chords::fit_of;
use super::segment::Slice;
use super::{AnalysisFinding, Approach, Evidence, Fit, Ground, Lane, Observation, Standing};
use crate::chord::{ChordClass, ChordType};
use crate::pitch::{Accidental, Letter, PitchClass};
use crate::scale::{Collection, Degree, Scale, signature_scale};
use crate::score::{Key, Mode, ScoreSnapshot};
use crate::time::MusicalTime;

/// A stretch of slices one or more keys account for.
struct Region {
    from: usize,
    to: usize,
    keys: Vec<Key>,
}

pub(super) fn observe(
    snapshot: &ScoreSnapshot,
    lanes: &[Lane],
    slices: &[Slice],
    assumed: Option<Key>,
) -> Vec<AnalysisFinding> {
    let written = written_key(snapshot, lanes);
    let regions = match assumed {
        // An assumed key is the reader saying "read this the way I hear it".
        // It narrows the reading and does not make it true, which is why it
        // produces one region and changes nothing about how a numeral's
        // standing is decided below.
        Some(key) => vec![Region {
            from: 0,
            to: slices.len().saturating_sub(1),
            keys: vec![key],
        }],
        None => scan(slices, written),
    };
    let mut found = Vec::new();
    for region in &regions {
        let Some((first, last)) = bounds(slices, region) else {
            continue;
        };
        let standing = if regions.len() == 1 && region.keys.len() == 1 {
            Standing::Fact
        } else {
            Standing::Candidate
        };
        found.extend(region.keys.iter().map(|key| {
            AnalysisFinding::judged(
                "key-region",
                standing,
                Observation::KeyRegion {
                    key: *key,
                    from: first,
                    to: last,
                },
                passage(slices, region),
                vec![
                    Ground {
                        criterion: "every sounding class is in the key's collection",
                        satisfied: true,
                        cites: "OMT 013",
                    },
                    Ground {
                        criterion: "the key is the one the source writes",
                        satisfied: written == Some(*key),
                        cites: "OMT 051",
                    },
                ],
            )
        }));
        found.extend(numerals(slices, region, regions.len()));
    }
    found.extend(changes(slices, &regions));
    found
}

/// The key the source writes, when it writes one.
fn written_key(snapshot: &ScoreSnapshot, lanes: &[Lane]) -> Option<Key> {
    lanes
        .first()
        .and_then(|lane| snapshot.key_at(lane.scope(), MusicalTime::ZERO))
        .or_else(|| snapshot.key_at(crate::scope::Scope::Piece, MusicalTime::ZERO))
}

/// The keys this reading proposes for a passage, one per region, in the order
/// the regions arrive.
///
/// Shared with `cadence`, which has to ask a cadence question in *some* key
/// and must ask it in the same keys the tonal reading proposes — two kinds
/// disagreeing about what key a passage is in would be two analyses, not one
/// library.
pub(super) fn proposed(snapshot: &ScoreSnapshot, lanes: &[Lane], slices: &[Slice], assumed: Option<Key>) -> Vec<Key> {
    if let Some(key) = assumed {
        return vec![key];
    }
    let mut out: Vec<Key> = Vec::new();
    for region in scan(slices, written_key(snapshot, lanes)) {
        if let Some(key) = region.keys.first()
            && !out.contains(key)
        {
            out.push(*key);
        }
    }
    out
}

/// Which degree of the key the slice's chord is built on, when the key names
/// one.
pub(super) fn degree_of(key: Key, slice: &Slice) -> Option<i64> {
    chord_at(key, slice).map(|(ordinal, _)| ordinal)
}

/// Whether the slice puts the root of that chord in the bass (OMT `019`).
pub(super) fn in_root_position(key: Key, slice: &Slice) -> bool {
    chord_at(key, slice).is_some_and(|(_, chord)| slice.bass() == Some(chord.root()))
}

/// The diatonic chord of the key the slice fits best, with its degree.
///
/// Best, not only: a cadence question is asked of a chord that may be
/// incomplete, and refusing to answer unless the fit is exact would find no
/// cadence in any texture that omits a fifth (OMT `020`).
fn chord_at(key: Key, slice: &Slice) -> Option<(i64, ChordClass)> {
    let classes = slice.classes();
    let mut best: Option<(u8, i64, ChordClass)> = None;
    for collection in collections(key) {
        let scale = Scale::new(key.tonic(), collection);
        for ordinal in 1..=7 {
            for members in [3, 4] {
                let Some(chord) = scale.stacked(Degree::new(ordinal), members) else {
                    continue;
                };
                let Some(fit) = fit_of(chord, &classes) else {
                    continue;
                };
                let rank = strength(fit);
                if best.is_none_or(|(seen, _, _)| rank > seen) {
                    best = Some((rank, ordinal, chord));
                }
            }
        }
    }
    best.map(|(_, ordinal, chord)| (ordinal, chord))
}

/// Where a region begins and ends in time.
fn bounds(slices: &[Slice], region: &Region) -> Option<(MusicalTime, MusicalTime)> {
    let first = slices.get(region.from)?;
    let last = slices.get(region.to)?;
    Some((first.onset, last.onset + last.extent))
}

/// The notes a region's finding points at.
fn passage(slices: &[Slice], region: &Region) -> Evidence {
    let notes = slices
        .get(region.from..=region.to)
        .unwrap_or_default()
        .iter()
        .flat_map(Slice::refs)
        .collect();
    let (from, to) = bounds(slices, region).unwrap_or((MusicalTime::ZERO, MusicalTime::ZERO));
    Evidence::Passage { from, to, notes }
}

/// Cut the slice sequence into regions, closing one when no key accounts for
/// everything sounding since it opened.
///
/// One left-to-right pass, with the surviving key set shrinking inside a
/// region and reset at each boundary. The written key does not gate the scan —
/// it only orders the survivors — because a piece that modulates has one
/// written key and several regions, and a scan that assumed otherwise could
/// not find the second one.
fn scan(slices: &[Slice], written: Option<Key>) -> Vec<Region> {
    let all = keys(written);
    let mut regions = Vec::new();
    let mut start = 0;
    let mut classes: Vec<PitchClass> = Vec::new();
    let mut surviving = all.clone();
    for (at, slice) in slices.iter().enumerate() {
        let mut next = classes.clone();
        for class in slice.classes() {
            if !next.contains(&class) {
                next.push(class);
            }
        }
        let still: Vec<Key> = surviving.iter().copied().filter(|key| covers(*key, &next)).collect();
        if still.is_empty() && at > start {
            regions.push(Region {
                from: start,
                to: at.saturating_sub(1),
                keys: surviving,
            });
            start = at;
            classes = slice.classes();
            surviving = all.iter().copied().filter(|key| covers(*key, &classes)).collect();
        } else {
            classes = next;
            surviving = still;
        }
    }
    if !slices.is_empty() {
        regions.push(Region {
            from: start,
            to: slices.len().saturating_sub(1),
            keys: surviving,
        });
    }
    regions
}

/// Every key a reading may propose, in the order a report prints them.
///
/// The order is stated and deterministic, never a probability: the written key
/// first, then the keys nearest it on the circle of fifths, then major before
/// minor, then by spelling. Keys past seven accidentals are not enumerated —
/// they are the same sounds under spellings no signature writes.
fn keys(written: Option<Key>) -> Vec<Key> {
    let mut all: Vec<Key> = Vec::new();
    for steps in 0..7 {
        let Some(letter) = Letter::from_steps(steps) else {
            continue;
        };
        for accidental in [Accidental::FLAT, Accidental::NATURAL, Accidental::SHARP] {
            for mode in [Mode::Major, Mode::Minor] {
                let key = Key::new(PitchClass { letter, accidental }, mode);
                if key.fifths().abs() <= 7 {
                    all.push(key);
                }
            }
        }
    }
    all.sort_by_key(|key| {
        (
            u8::from(written != Some(*key)),
            key.fifths().abs(),
            key.fifths(),
            match key.mode() {
                Mode::Major => 0_u8,
                Mode::Minor => 1,
            },
            key.tonic().letter.steps(),
            key.tonic().accidental.0,
        )
    });
    all
}

/// Whether a key's collections contain every class.
///
/// A minor key is read through all three of its collections, because the
/// raised sixth and seventh are not outside the key — they are what the key
/// does at a cadence (OMT `014`). A major key is read through its one.
fn covers(key: Key, classes: &[PitchClass]) -> bool {
    let palette = palette(key);
    classes.iter().all(|class| palette.contains(class))
}

/// Every class a key's collections spell.
fn palette(key: Key) -> Vec<PitchClass> {
    let mut classes = Vec::new();
    for collection in collections(key) {
        let scale = Scale::new(key.tonic(), collection);
        for ordinal in 1..=7 {
            if let Some(class) = scale.class(Degree::new(ordinal))
                && !classes.contains(&class)
            {
                classes.push(class);
            }
        }
    }
    classes
}

/// The collections a key is read through.
///
/// Natural and harmonic minor, and deliberately not melodic: the raised
/// seventh is what a minor key does at every cadence, so a reading without it
/// finds no dominant anywhere, while the raised sixth appears ascending in a
/// line and is weak evidence about the key (OMT `014`). Admitting it would
/// give every minor key nine classes, and a nine-class key accounts for
/// passages it has no business accounting for — a piece that plainly modulates
/// would come back as one region in some minor key nobody hears.
fn collections(key: Key) -> Vec<Collection> {
    match key.mode() {
        Mode::Major => vec![Collection::Major],
        Mode::Minor => vec![Collection::NaturalMinor, Collection::HarmonicMinor],
    }
}

/// The numerals a region's keys offer for each of its slices.
fn numerals(slices: &[Slice], region: &Region, regions: usize) -> Vec<AnalysisFinding> {
    let mut found = Vec::new();
    for at in region.from..=region.to {
        let Some(slice) = slices.get(at) else {
            continue;
        };
        let classes = slice.classes();
        for key in &region.keys {
            let readings = readings(*key, &classes, slice.bass());
            let alone = readings.len() == 1 && regions == 1 && region.keys.len() == 1;
            found.extend(read(slice, *key, alone, None));
            found.extend(tonicizations(slices, at, *key, &readings));
        }
    }
    found
}

/// One slice's numerals in one key, with an optional extra criterion saying
/// why this key was asked about at all.
fn read(slice: &Slice, key: Key, alone: bool, why: Option<Ground>) -> Vec<AnalysisFinding> {
    readings(key, &slice.classes(), slice.bass())
        .into_iter()
        .map(|(numeral, fit, _)| {
            let mut grounds = vec![Ground {
                criterion: "the chord the numeral names is what sounds",
                satisfied: fit == Fit::Exact,
                cites: "OMT 026",
            }];
            grounds.extend(why);
            AnalysisFinding::judged(
                "numeral",
                if alone && fit == Fit::Exact {
                    Standing::Fact
                } else {
                    Standing::Candidate
                },
                Observation::Numeral {
                    numeral,
                    key,
                    fit,
                    onset: slice.onset,
                    extent: slice.extent,
                },
                Evidence::Passage {
                    from: slice.onset,
                    to: slice.onset + slice.extent,
                    notes: slice.refs(),
                },
                grounds,
            )
        })
        .collect()
}

/// Every numeral a key offers for one slice, at the strongest fit achieved.
///
/// The third element of each reading is the degree the numeral tonicizes, when
/// it is an applied chord — kept so a tonicization can be recognized without
/// parsing the numeral text back apart.
fn readings(key: Key, classes: &[PitchClass], bass: Option<PitchClass>) -> Vec<(String, Fit, Option<i64>)> {
    let mut out: Vec<(String, Fit, Option<i64>)> = Vec::new();
    let mut push = |text: String, fit: Fit, target: Option<i64>| {
        if !out.iter().any(|(seen, _, _)| *seen == text) {
            out.push((text, fit, target));
        }
    };
    for collection in collections(key) {
        let scale = Scale::new(key.tonic(), collection);
        for ordinal in 1..=7 {
            for members in [3, 4] {
                let Some(chord) = scale.stacked(Degree::new(ordinal), members) else {
                    continue;
                };
                let Some(fit) = fit_of(chord, classes) else {
                    continue;
                };
                push(spell(ordinal, chord, members, bass), fit, None);
            }
        }
    }
    for target in 2..=6 {
        for (text, chord) in applied(key, target) {
            let Some(fit) = fit_of(chord, classes) else {
                continue;
            };
            push(text, fit, Some(target));
        }
    }
    let Some(best) = out.iter().map(|(_, fit, _)| strength(*fit)).max() else {
        return out;
    };
    out.retain(|(_, fit, _)| strength(*fit) == best);
    out.sort_by(|left, right| left.0.cmp(&right.0));
    out
}

/// How strong a relation is, for choosing which tier to report.
fn strength(fit: Fit) -> u8 {
    match fit {
        Fit::Exact => 3,
        Fit::Incomplete => 2,
        Fit::WithExtraTones => 1,
        Fit::Partial => 0,
    }
}

/// The applied chords of one degree: its dominant and its leading-tone chords
/// (OMT `050`).
///
/// Reported only when the chord sounds a class the home key does not spell.
/// `V/IV` in C major is a C major triad, which is `I`; offering both names for
/// the same three notes would be noise rather than a reading.
fn applied(key: Key, target: i64) -> Vec<(String, ChordClass)> {
    let home = signature_scale(key);
    let Some(local) = home.class(Degree::new(target)) else {
        return Vec::new();
    };
    let Some(quality) = home.stacked(Degree::new(target), 3) else {
        return Vec::new();
    };
    // A diminished degree is not tonicized: it is not a key, and `V/vii°` is
    // not something the notation writes (OMT 050).
    if quality.kind() == ChordType::Diminished {
        return Vec::new();
    }
    let of = symbol(target, quality);
    let major = Scale::new(local, Collection::Major);
    let (Some(dominant), Some(leading)) = (major.class(Degree::new(5)), major.class(Degree::new(7))) else {
        return Vec::new();
    };
    let outside = palette(key);
    [
        (format!("V/{of}"), ChordClass::new(dominant, ChordType::Major)),
        (format!("V7/{of}"), ChordClass::new(dominant, ChordType::Dominant7)),
        (format!("vii°/{of}"), ChordClass::new(leading, ChordType::Diminished)),
        (format!("vii°7/{of}"), ChordClass::new(leading, ChordType::Diminished7)),
    ]
    .into_iter()
    .filter(|(_, chord)| {
        (0..chord.size())
            .filter_map(|at| chord.member_class(at))
            .any(|class| !outside.contains(&class))
    })
    .collect()
}

/// The numeral text for a diatonic chord: the symbol, the quality marker, and
/// the figured bass its inversion writes.
fn spell(ordinal: i64, chord: ChordClass, members: usize, bass: Option<PitchClass>) -> String {
    let position = bass.and_then(|class| chord.position_of(class)).unwrap_or(0);
    format!("{}{}", symbol(ordinal, chord), figure(members, position))
}

/// The Roman symbol, cased and marked by what the collection stacked.
///
/// The case is not chosen and then applied to a degree: it is read off the
/// chord the collection's own notes make, which is why `ii` is minor in major
/// and `II` is major in Dorian without either being stipulated (`crate::roman`
/// opens with this).
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "lowercase is what a numeral takes by default, so a chord type added later inherits the right answer; spelling out nineteen variants would say the same thing and go stale"
)]
fn symbol(ordinal: i64, chord: ChordClass) -> String {
    let numeral = match ordinal {
        1 => "I",
        2 => "II",
        3 => "III",
        4 => "IV",
        5 => "V",
        6 => "VI",
        _ => "VII",
    };
    match chord.kind() {
        ChordType::Major | ChordType::Dominant7 | ChordType::Major7 => numeral.to_owned(),
        ChordType::Augmented => format!("{numeral}+"),
        ChordType::Diminished | ChordType::Diminished7 => format!("{}°", numeral.to_lowercase()),
        ChordType::HalfDiminished7 => format!("{}ø", numeral.to_lowercase()),
        // Everything else a collection can stack is minor-flavoured, and a
        // numeral says so by being lowercase and nothing more. A chord type
        // whose numeral needs its own marker gets an arm above.
        _ => numeral.to_lowercase(),
    }
}

/// The figured bass an inversion writes (OMT `019`).
fn figure(members: usize, position: usize) -> &'static str {
    match (members, position) {
        (4, 0) => "7",
        (4, 1) => "65",
        (4, 2) => "43",
        (4, 3) => "42",
        (_, 1) => "6",
        (_, 2) => "64",
        _ => "",
    }
}

/// A tonicization: an applied chord followed by the degree it applies to.
///
/// The resolution is required. An applied dominant that goes somewhere else is
/// a chromatic chord and possibly a deceptive one, and calling it a
/// tonicization would be reporting an intention rather than a sound.
fn tonicizations(
    slices: &[Slice],
    at: usize,
    key: Key,
    here_now: &[(String, Fit, Option<i64>)],
) -> Vec<AnalysisFinding> {
    let Some(next) = slices.get(at.saturating_add(1)) else {
        return Vec::new();
    };
    let Some(here) = slices.get(at) else {
        return Vec::new();
    };
    let after = readings(key, &next.classes(), next.bass());
    here_now
        .iter()
        .filter_map(|(_, _, target)| *target)
        .filter_map(|target| {
            let home = signature_scale(key);
            let quality = home.stacked(Degree::new(target), 3)?;
            let of = symbol(target, quality);
            let resolves = after
                .iter()
                .any(|(text, _, applied)| applied.is_none() && text.starts_with(of.as_str()));
            resolves.then(|| {
                let mut notes = here.refs();
                notes.extend(next.refs());
                AnalysisFinding::judged(
                    "tonicization",
                    Standing::Candidate,
                    Observation::Tonicization {
                        target: of.clone(),
                        key,
                        from: here.onset,
                        to: next.onset + next.extent,
                    },
                    Evidence::Passage {
                        from: here.onset,
                        to: next.onset + next.extent,
                        notes,
                    },
                    vec![
                        Ground {
                            criterion: "an applied chord of the degree sounds",
                            satisfied: true,
                            cites: "OMT 050",
                        },
                        Ground {
                            criterion: "the degree it applies to follows it",
                            satisfied: true,
                            cites: "OMT 050",
                        },
                        Ground {
                            criterion: "the home key still accounts for the passage",
                            satisfied: true,
                            cites: "OMT 051",
                        },
                    ],
                )
            })
        })
        .collect()
}

/// Every boundary between two key regions, reported as both readings.
///
/// A boundary is a place where no key accounts for what came before and what
/// comes after together. Whether a listener hears that as a passing
/// tonicization or as a modulation is exactly what the criteria in OMT `051`
/// are for, and where they do not settle it, both readings are candidates in
/// the same report. Nothing here compares the length of a region to a
/// threshold: duration alone does not decide this.
fn changes(slices: &[Slice], regions: &[Region]) -> Vec<AnalysisFinding> {
    let mut found = Vec::new();
    for pair in regions.windows(2) {
        let (Some(before), Some(after)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        let (Some(from_key), Some(to_key)) = (before.keys.first().copied(), after.keys.first().copied()) else {
            continue;
        };
        let Some(at) = slices.get(after.from).map(|slice| slice.onset) else {
            continue;
        };
        let pivot = slices
            .get(before.to)
            .map(|slice| slice.classes())
            .is_some_and(|classes| covers(from_key, &classes) && covers(to_key, &classes));
        let last = slices.get(after.to);
        let ends_the_passage = after.to.saturating_add(1) >= slices.len();
        let notes = slices
            .get(before.to..=after.to)
            .unwrap_or_default()
            .iter()
            .flat_map(Slice::refs)
            .collect();
        let evidence = Evidence::Passage {
            from: slices.get(before.to).map_or(at, |slice| slice.onset),
            to: last.map_or(at, |slice| slice.onset + slice.extent),
            notes,
        };
        let grounds = vec![
            Ground {
                criterion: "a chord diatonic in both keys prepares the change",
                satisfied: pivot,
                cites: "OMT 051",
            },
            Ground {
                criterion: "the new key holds to the end of the passage",
                satisfied: ends_the_passage,
                cites: "OMT 051",
            },
            Ground {
                criterion: "a cadence in the new key confirms it",
                satisfied: false,
                cites: "OMT 051",
            },
        ];
        // The two chords the boundary falls between are exactly the ones a
        // musician argues about, so each is read in *both* keys: the last
        // chord of the old region under the new key (that is what makes it a
        // pivot at all), and the first chord of the new region under the old
        // key (which is where an applied chord shows up as `V/V` rather than
        // as `V`). Without these two readings the report would name a pivot
        // and then never show one.
        if let Some(slice) = slices.get(before.to) {
            found.extend(read(
                slice,
                to_key,
                false,
                Some(Ground {
                    criterion: "the chord is diatonic in the key before it as well",
                    satisfied: pivot,
                    cites: "OMT 051",
                }),
            ));
        }
        if let Some(slice) = slices.get(after.from) {
            found.extend(read(
                slice,
                from_key,
                false,
                Some(Ground {
                    criterion: "the chord is read in the key the passage is leaving",
                    satisfied: true,
                    cites: "OMT 050",
                }),
            ));
        }
        found.push(AnalysisFinding::judged(
            "modulation",
            Standing::Candidate,
            Observation::Modulation {
                from_key,
                to_key,
                how: if pivot { Approach::Pivot } else { Approach::Direct },
                at,
            },
            evidence.clone(),
            grounds.clone(),
        ));
        let home = signature_scale(from_key);
        let target = (1..=7).find(|ordinal| home.class(Degree::new(*ordinal)) == Some(to_key.tonic()));
        if let Some(of) = target
            .and_then(|ordinal| Some((ordinal, home.stacked(Degree::new(ordinal), 3)?)))
            .map(|(ordinal, quality)| symbol(ordinal, quality))
        {
            found.push(AnalysisFinding::judged(
                "tonicization",
                Standing::Candidate,
                Observation::Tonicization {
                    target: of,
                    key: from_key,
                    from: at,
                    to: last.map_or(at, |slice| slice.onset + slice.extent),
                },
                evidence,
                grounds,
            ));
        }
    }
    found
}
