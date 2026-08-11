//! An analysis report, made display-ready.
//!
//! `musa-compiler`'s report is typed for a program: exact rationals, part ids,
//! byte offsets. Everything a person reads — the sentence, the bar number, the
//! line — is computed here, once, for the same reason [`crate::facts`] exists:
//! `docs/interface/03-interaction.md` §7 lists what a frontend may compute, and
//! nothing musical is on it.
//!
//! This is not a pass-through. The report arrives as ids and offsets and
//! leaves as names, bars, beats, lines, and sentences; a caller that wanted the
//! typed form would be reaching past the facade for something the interface has
//! no use for.

use musa_compiler::{
    AnalysisFinding, AnalysisReport, Evidence, NoteRef, Observation, PartId, Scope, ScoreSnapshot, VoiceId,
};
use serde::Serialize;

use crate::diagnostic::Span;
use crate::facts::Fraction;
use crate::position::Lines;

/// What one analysis saw, in words and numbers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisFacts {
    /// The analysis that ran, as it is written on a command line: `facts`.
    pub kind: String,
    /// One line saying what it does.
    pub method: String,
    /// The style whose rules it read by, for an analysis that needs one named.
    /// Absent for the kinds that read the notation alone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// What it took for granted, one sentence each. Never empty: an analysis
    /// that assumed nothing would be claiming something.
    pub assumptions: Vec<String>,
    /// Everything it saw, in the report's deterministic order.
    pub findings: Vec<FindingFacts>,
}

/// One thing an analysis saw.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingFacts {
    /// The stable code, for filtering without matching on prose:
    /// `sounding-pitch`, `key-in-force`.
    pub code: String,
    /// `fact`, `candidate`, or `conflict` — how firmly it is held, relative
    /// to the analysis's own abstraction rather than to a severity scale.
    pub standing: String,
    /// The finding as a sentence: `A♯4 sounds for 1/4`.
    pub summary: String,
    /// Where it is anchored, in whole notes from the piece start.
    pub at: Fraction,
    /// 1-based bar number, counted in the piece's barlines.
    pub bar: u32,
    /// 1-based beat within that bar, exact.
    pub beat: Fraction,
    /// Where in the score it can be seen.
    pub evidence: EvidenceFacts,
    /// What the finding was judged against, and what held. Empty for a finding
    /// that judged nothing.
    pub grounds: Vec<GroundFacts>,
    /// The style rule the finding is about, for a profile reading. Absent for
    /// every other kind, because a cadence is not a rule and a reader filtering
    /// on strength must not be handed one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<RuleFacts>,
}

/// One rule of a named profile, as a reader needs to see it.
///
/// `strength` is here and not folded into the sentence because it is the whole
/// of what a style adds: the motion is a fact about the notes either way, and
/// what changes between SATB, species counterpoint, and jazz is only how
/// firmly each holds the same rule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleFacts {
    /// Its id, the word a source writes in `assert follows(...)`.
    pub id: String,
    /// What it says, in one line.
    pub states: String,
    /// `definitional`, `hard in this exercise`, or `guideline`.
    pub strength: String,
    /// Where it is written down.
    pub cites: String,
}

/// One criterion a finding was judged against.
///
/// Carried through rather than folded into the sentence, because this is what
/// makes a reading arguable: a reader disagreeing with "imperfect authentic
/// cadence" is disagreeing with one of these lines, and an interface that only
/// had the sentence could not show them which.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundFacts {
    /// What was checked.
    pub criterion: String,
    /// Whether the music satisfies it.
    pub satisfied: bool,
    /// Where the criterion comes from: `OMT 036`.
    pub cites: String,
}

/// One note a finding points at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteFacts {
    /// The part's name.
    pub part: String,
    /// The voice's name.
    pub voice: String,
    /// The event id — `event-1f`, the MEI `xml:id`.
    pub event: String,
    /// The statement that spells it, as a byte range in the source.
    pub span: Span,
    /// 1-based line of that statement.
    pub line: u32,
}

/// Where a finding can be seen.
///
/// Tagged rather than flattened into optional fields, because a consumer's
/// first question is always which of the three it has: an event can be
/// selected, an annotation can be revealed, and a value in force can be
/// neither — it is a fact about a stretch of the piece and not about a place
/// in the file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EvidenceFacts {
    /// A score event, by the same identity the engraved page carries.
    Event(NoteFacts),
    /// A stretch of music and every note in it: a simultaneity, a cadence's
    /// two chords, a key region. The notes are named one by one rather than
    /// hulled into a span, because two notes of one chord can come from two
    /// different motifs and no single source range covers them.
    Passage {
        /// Where the stretch begins, in whole notes from the piece start.
        from: Fraction,
        /// Where it ends.
        to: Fraction,
        /// The notes in it, in the report's own order.
        notes: Vec<NoteFacts>,
    },
    /// Something written above the staff.
    Annotation {
        /// Where it is written, as a byte range in the source.
        span: Span,
        /// 1-based line.
        line: u32,
    },
    /// A value in force, with no statement to point at. The finding's own
    /// `at`/`bar`/`beat` is where it takes force, which is the whole answer.
    InForce,
}

impl AnalysisFacts {
    /// Resolve a report against the score it was read from.
    pub(crate) fn derive(report: &AnalysisReport, score: &ScoreSnapshot, source: &str) -> Self {
        let lines = Lines::new(source);
        let bars = score.bars(Scope::Piece);
        Self {
            kind: report.kind().as_str().to_owned(),
            method: report.method().to_owned(),
            profile: report.profile().map(|profile| profile.as_str().to_owned()),
            assumptions: report.assumptions().iter().map(|line| (*line).to_owned()).collect(),
            findings: report
                .findings()
                .iter()
                .map(|finding| {
                    let at = finding.observation().at();
                    let place = bars.at(at);
                    FindingFacts {
                        code: finding.code().to_owned(),
                        standing: finding.standing().as_str().to_owned(),
                        summary: summarize(finding.observation()),
                        at: Fraction::from_ratio(at.as_ratio()),
                        bar: place.measure,
                        beat: Fraction::from_ratio(place.beat),
                        evidence: evidence(finding, score, &lines),
                        grounds: finding
                            .grounds()
                            .iter()
                            .map(|ground| GroundFacts {
                                criterion: ground.criterion.to_owned(),
                                satisfied: ground.satisfied,
                                cites: ground.cites.to_owned(),
                            })
                            .collect(),
                        rule: rule_of(finding.observation()),
                    }
                })
                .collect(),
        }
    }
}

impl AnalysisFacts {
    /// The report as JSON text, the shape a script or an interface reads.
    ///
    /// Rendered here rather than by handing out a `serde_json::Value`: what
    /// crosses this boundary is the report, and a caller holding a JSON tree
    /// would be free to edit it and publish the result under the analysis's
    /// name.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_owned())
    }
}

/// The finding as a sentence.
fn summarize(observation: &Observation) -> String {
    match *observation {
        Observation::Sounding { pitch, extent, .. } => {
            format!("{} sounds for {}", crate::facts::written(pitch), extent.as_ratio())
        }
        Observation::Silence { extent, .. } => format!("nothing sounds for {}", extent.as_ratio()),
        Observation::Written { ref symbol, .. } => format!("`{}` is written above the staff", symbol.text()),
        Observation::KeyInForce { key, .. } => format!(
            "{} {} is in force",
            crate::facts::pitch_class(key.tonic()),
            crate::facts::mode(key.mode())
        ),
        Observation::MeterInForce { meter, .. } => {
            format!("{}/{} is in force", meter.numerator(), meter.denominator())
        }
        Observation::Sonority {
            ref pitches, extent, ..
        } => format!(
            "{} sound together for {}",
            pitches
                .iter()
                .map(|pitch| crate::facts::written(*pitch))
                .collect::<Vec<_>>()
                .join(", "),
            extent.as_ratio()
        ),
        Observation::ChordFit { chord, fit, .. } => {
            format!("the notes fit {} — {}", name(chord), fit.as_str())
        }
        Observation::SymbolReading {
            ref symbol, sounding, ..
        } => match sounding {
            Some(chord) => format!("`{}` is written; the notes spell {}", symbol.text(), name(chord)),
            None => format!("`{}` is written; the notes spell no chord", symbol.text()),
        },
        Observation::Numeral {
            ref numeral, key, fit, ..
        } => format!("{numeral} in {} — {} fit", key_name(key), fit.as_str()),
        Observation::KeyRegion { key, to, .. } => {
            format!("{} accounts for the music up to {}", key_name(key), to.as_ratio())
        }
        Observation::Cadence { cadence, key, .. } => {
            format!("{} in {}", cadence.name(), key_name(key))
        }
        Observation::Tonicization { ref target, key, .. } => {
            format!("{target} of {} is tonicized", key_name(key))
        }
        Observation::RuleInForce { rule, .. } => format!("{} is in force: {}", rule.id(), rule.states()),
        Observation::Departure {
            rule, ref voices, interval, ..
        } => {
            let who = match voices.split_last() {
                None => "the voicing".to_owned(),
                Some((last, [])) => last.clone(),
                Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
            };
            let verb = if voices.len() == 1 { "departs" } else { "depart" };
            let verb = if voices.is_empty() { "departs" } else { verb };
            match interval {
                Some(interval) => format!("{who} {verb} from {} — {interval}", rule.id()),
                None => format!("{who} {verb} from {}", rule.id()),
            }
        }
        Observation::Modulation {
            from_key, to_key, how, ..
        } => format!(
            "{} gives way to {} — {} approach",
            key_name(from_key),
            key_name(to_key),
            how.as_str()
        ),
    }
}

/// The rule a finding is about, for the two observations that have one.
fn rule_of(observation: &Observation) -> Option<RuleFacts> {
    let (Observation::RuleInForce { rule, .. } | Observation::Departure { rule, .. }) = *observation else {
        return None;
    };
    Some(RuleFacts {
        id: rule.id().to_owned(),
        states: rule.states().to_owned(),
        strength: rule.strength().as_str().to_owned(),
        cites: rule.cites().to_owned(),
    })
}

/// A chord as a reader writes it: `C major`, `F♯ dominant7/A♯`.
fn name(chord: musa_compiler::ChordName) -> String {
    let base = format!("{} {}", crate::facts::pitch_class(chord.root()), chord.quality());
    match chord.bass() {
        Some(bass) => format!("{base}/{}", crate::facts::pitch_class(bass)),
        None => base,
    }
}

/// A key as a reader writes it: `C major`.
fn key_name(key: musa_compiler::Key) -> String {
    format!(
        "{} {}",
        crate::facts::pitch_class(key.tonic()),
        crate::facts::mode(key.mode())
    )
}

/// Resolve a finding's evidence into names, spans, and lines.
fn evidence(finding: &AnalysisFinding, score: &ScoreSnapshot, lines: &Lines<'_>) -> EvidenceFacts {
    match *finding.evidence() {
        Evidence::Event(note) => EvidenceFacts::Event(note_facts(note, score, lines)),
        Evidence::Passage { from, to, ref notes } => EvidenceFacts::Passage {
            from: Fraction::from_ratio(from.as_ratio()),
            to: Fraction::from_ratio(to.as_ratio()),
            notes: notes.iter().map(|note| note_facts(*note, score, lines)).collect(),
        },
        Evidence::Annotation { span } => EvidenceFacts::Annotation {
            span: Span {
                start: span.start,
                end: span.end,
            },
            line: lines.at(span.start).line,
        },
        Evidence::InForce { .. } => EvidenceFacts::InForce,
    }
}

/// One note, resolved into names, a span, and a line.
fn note_facts(note: NoteRef, score: &ScoreSnapshot, lines: &Lines<'_>) -> NoteFacts {
    NoteFacts {
        part: part_name(score, note.part),
        voice: voice_name(score, note.part, note.voice),
        event: format!("event-{:x}", note.id.0),
        span: Span {
            start: note.span.start,
            end: note.span.end,
        },
        line: lines.at(note.span.start).line,
    }
}

/// The part's name, or its id when the snapshot has no part by that id — which
/// cannot happen for a report read from this same score, and is answered
/// rather than panicked over.
fn part_name(score: &ScoreSnapshot, part: PartId) -> String {
    score
        .parts()
        .get(part)
        .map_or_else(|| part.0.to_string(), |found| found.name().to_owned())
}

/// The voice's name, under the same rule.
fn voice_name(score: &ScoreSnapshot, part: PartId, voice: VoiceId) -> String {
    score
        .parts()
        .get(part)
        .and_then(|found| found.voice_name(voice))
        .map_or_else(|| voice.0.to_string(), ToOwned::to_owned)
}
