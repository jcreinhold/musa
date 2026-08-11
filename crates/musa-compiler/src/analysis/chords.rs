//! The `chords` kind: what the notes sounding together add up to.
//!
//! **Abstract domain.** For each slice the request's segmentation produces:
//! the slice's set of spelled pitch classes together with its bass, and the
//! set of pairs *(chord class, relation)* where the relation is one of the
//! four in [`Fit`]. A finding is one such pair, or the slice itself, or the
//! comparison of a written symbol against the pair set at that instant.
//!
//! **Abstraction map.** α cuts the lanes into slices (`segment`), reduces each
//! slice to its spelled classes — losing register, doubling, and voicing,
//! which is what "chord" means at all (OMT `019-inversion.md`) — and pairs it
//! with every chord in the vocabulary below, rooted on every sounding class,
//! that stands in one of the four relations. Only the strongest relation any
//! chord achieves is reported, with every chord achieving it.
//!
//! **Soundness.** α is exact on the slice: two scores whose notes differ
//! inside a slice differ in its class set, so the `sonority` finding is a
//! fact. It is *not* exact on the naming: a class set is generally the content
//! of several chords, and the diminished seventh is the standing example — its
//! four classes are the content of four chords with four different roots, and
//! α cannot separate them. So a fit is a fact exactly when the vocabulary
//! contains one chord standing in the strongest achieved relation, and a
//! candidate otherwise. γ of a candidate set is every score whose slice has
//! that class content, which no single label describes.
//!
//! What no finding here licenses is a claim about *function*: this says which
//! chords the notes spell, not what any of them is doing. Function needs a
//! key, which is `tonal`'s question. Nor does anything here decide which notes
//! are embellishing (OMT `039-embellishing-tones.md`) — a fit that needs an
//! extra note explained says so through [`Fit::WithExtraTones`] and stops.

// Rational arithmetic on musical time is exact mathematical arithmetic, not
// raw integer ops; clippy::arithmetic_side_effects does not apply to it.
#![allow(clippy::arithmetic_side_effects)]

use super::segment::Slice;
use super::{AnalysisFinding, AnalysisScope, ChordName, Evidence, Fit, Ground, Observation, Standing, inside};
use crate::chord::{ChordClass, ChordType};
use crate::harmony::{ChordQuality, ChordSymbol, Seventh};
use crate::pitch::PitchClass;
use crate::score::ScoreSnapshot;
use crate::time::MusicalTime;

/// The chords this analysis is willing to name.
///
/// A policy of the reading and not of the language: `crate::chord` knows more
/// chord types than these, and a reading that offered every one of them as a
/// candidate for every slice would bury the two or three a musician would
/// actually argue about. These are the triads and sevenths of common-practice
/// harmony (OMT `017-triads.md`, `018-seventh-chords.md`) plus the augmented
/// sixths, which are the chromatic sonorities the notation names outright.
const VOCABULARY: [ChordType; 17] = [
    ChordType::Major,
    ChordType::Minor,
    ChordType::Diminished,
    ChordType::Augmented,
    ChordType::Major6,
    ChordType::Minor6,
    ChordType::Dominant7,
    ChordType::Major7,
    ChordType::Minor7,
    ChordType::MinorMajor7,
    ChordType::HalfDiminished7,
    ChordType::Diminished7,
    ChordType::Suspended2,
    ChordType::Suspended4,
    ChordType::ItalianSixth,
    ChordType::FrenchSixth,
    ChordType::GermanSixth,
];

pub(super) fn observe(
    snapshot: &ScoreSnapshot,
    slices: &[Slice],
    scope: &AnalysisScope,
    window: Option<(MusicalTime, MusicalTime)>,
) -> Vec<AnalysisFinding> {
    let mut found = Vec::new();
    for slice in slices {
        let notes = slice.refs();
        let passage = Evidence::Passage {
            from: slice.onset,
            to: slice.onset + slice.extent,
            notes,
        };
        found.push(AnalysisFinding::stated(
            "sonority",
            Observation::Sonority {
                pitches: slice.notes.iter().map(|voiced| voiced.pitch).collect(),
                onset: slice.onset,
                extent: slice.extent,
            },
            passage.clone(),
        ));
        let readings = readings(slice);
        let standing = standing(&readings);
        found.extend(readings.iter().map(|(chord, fit)| {
            AnalysisFinding::judged(
                "chord-fit",
                standing,
                Observation::ChordFit {
                    chord: ChordName::of(*chord),
                    fit: *fit,
                    onset: slice.onset,
                    extent: slice.extent,
                },
                passage.clone(),
                vec![Ground {
                    criterion: "the sounding classes are the chord's members",
                    satisfied: *fit == Fit::Exact,
                    cites: "OMT 019",
                }],
            )
        }));
    }
    // A written symbol is the piece's statement about the whole texture, so it
    // is compared only when the request is about the piece — the same rule the
    // `facts` kind reads symbols under, for the same reason.
    if matches!(*scope, AnalysisScope::Score) {
        found.extend(
            snapshot
                .annotations()
                .harmony()
                .iter()
                .filter(|mark| inside(window, mark.at))
                .map(|mark| compare(&mark.symbol, mark.origin.source_span, mark.at, slices)),
        );
    }
    found
}

/// Every chord the slice's classes stand in the strongest achieved relation
/// to, in a stated order: by root letter, then by accidental, then by the
/// vocabulary's own order. Not a probability, and not a ranking by quality —
/// a deterministic order so two runs print the same bytes.
fn readings(slice: &Slice) -> Vec<(ChordClass, Fit)> {
    let classes = slice.classes();
    let mut fits: Vec<(ChordClass, Fit)> = Vec::new();
    for root in &classes {
        for kind in VOCABULARY {
            let chord = ChordClass::new(*root, kind);
            if let Some(fit) = fit_of(chord, &classes) {
                let chord = match slice.bass() {
                    Some(bass) if bass != *root => chord.over(bass),
                    _ => chord,
                };
                fits.push((chord, fit));
            }
        }
    }
    let Some(best) = fits.iter().map(|(_, fit)| strength(*fit)).max() else {
        return fits;
    };
    fits.retain(|(_, fit)| strength(*fit) == best);
    fits.sort_by_key(|(chord, _)| {
        (
            chord.root().letter.steps(),
            chord.root().accidental.0,
            VOCABULARY.iter().position(|kind| *kind == chord.kind()).unwrap_or(0),
        )
    });
    fits
}

/// A fact when one chord in the vocabulary stands in the achieved relation and
/// that relation is exact; a candidate otherwise.
///
/// The two conditions are separate on purpose. Several exact fits is the
/// diminished seventh, where α genuinely cannot choose. One inexact fit is a
/// reading that had to explain a note away, and a reading that explains
/// something away is not a fact about the notes.
fn standing(readings: &[(ChordClass, Fit)]) -> Standing {
    match *readings {
        [(_, Fit::Exact)] => Standing::Fact,
        _ => Standing::Candidate,
    }
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

/// How `classes` stand to `chord`, or `None` when they stand too far away to
/// be worth reporting.
///
/// The bounds are the reading's own policy, stated here rather than tuned: at
/// most one member may be missing, because a triad with two members absent is
/// two notes and names nothing; and at most one class may be extra, because a
/// chord that needs two notes explained away is a worse account than the chord
/// those notes belong to.
pub(super) fn fit_of(chord: ChordClass, classes: &[PitchClass]) -> Option<Fit> {
    let members: Vec<PitchClass> = (0..chord.size()).filter_map(|at| chord.member_class(at)).collect();
    let missing = members.iter().filter(|member| !classes.contains(member)).count();
    let extra = classes.iter().filter(|class| !members.contains(class)).count();
    let present = members.len().saturating_sub(missing);
    match (missing, extra) {
        (0, 0) => Some(Fit::Exact),
        (0, _) if extra <= 1 => Some(Fit::WithExtraTones),
        (1, 0) if present >= 2 => Some(Fit::Incomplete),
        (1, 1) if present >= 3 => Some(Fit::Partial),
        _ => None,
    }
}

/// What a written symbol and the notes under it say about each other.
///
/// The compiler still derives nothing from a symbol; this is a reader holding
/// two things the source already said next to each other, which is why it is
/// an analysis finding and not a diagnostic.
fn compare(
    symbol: &ChordSymbol,
    span: crate::origin::SourceSpan,
    at: MusicalTime,
    slices: &[Slice],
) -> AnalysisFinding {
    let under = slices
        .iter()
        .find(|slice| slice.onset <= at && slice.onset + slice.extent > at);
    let classes = under.map(Slice::classes).unwrap_or_default();
    let named = spelled(symbol);
    let root_sounds = classes.contains(&symbol.root());
    let agrees = named.is_some_and(|chord| matches!(fit_of(chord, &classes), Some(Fit::Exact | Fit::Incomplete)));
    let sounding = under
        .map(readings)
        .and_then(|fits| fits.first().map(|(chord, _)| ChordName::of(*chord)));
    AnalysisFinding::judged(
        "symbol-reading",
        if agrees { Standing::Fact } else { Standing::Conflict },
        Observation::SymbolReading {
            symbol: symbol.clone(),
            sounding,
            at,
        },
        Evidence::Annotation { span },
        vec![
            Ground {
                criterion: "the symbol's root sounds",
                satisfied: root_sounds,
                cites: "OMT 019",
            },
            Ground {
                criterion: "the notes spell the chord the symbol names",
                satisfied: agrees,
                cites: "OMT 019",
            },
            Ground {
                criterion: "the symbol names no extension above the seventh",
                satisfied: symbol.extension().is_none(),
                cites: "OMT 018",
            },
        ],
    )
}

/// The chord a symbol names, when the vocabulary has it.
///
/// Absent for a symbol whose extensions reach past the seventh: `c13` names
/// notes this vocabulary does not stack, and answering with the triad
/// underneath would be comparing the notes against a chord the source did not
/// write.
fn spelled(symbol: &ChordSymbol) -> Option<ChordClass> {
    let kind = match (symbol.quality(), symbol.seventh()) {
        (ChordQuality::Major, None) => ChordType::Major,
        (ChordQuality::Major, Some(Seventh::Major)) => ChordType::Major7,
        (ChordQuality::Major, Some(Seventh::Minor)) => ChordType::Dominant7,
        (ChordQuality::Minor, None) => ChordType::Minor,
        (ChordQuality::Minor, Some(Seventh::Minor)) => ChordType::Minor7,
        (ChordQuality::Minor, Some(Seventh::Major)) => ChordType::MinorMajor7,
        (ChordQuality::Diminished, None) => ChordType::Diminished,
        (ChordQuality::Diminished, Some(Seventh::Minor)) => ChordType::HalfDiminished7,
        (ChordQuality::Diminished, Some(Seventh::Diminished)) => ChordType::Diminished7,
        (ChordQuality::Augmented, None) => ChordType::Augmented,
        (ChordQuality::Suspended2, None) => ChordType::Suspended2,
        (ChordQuality::Suspended4, None) => ChordType::Suspended4,
        _ => return None,
    };
    if symbol.extension().is_some() {
        return None;
    }
    Some(ChordClass::new(symbol.root(), kind))
}
