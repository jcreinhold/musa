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

use musa_compiler::{AnalysisFinding, AnalysisReport, Evidence, Observation, PartId, Scope, ScoreSnapshot, VoiceId};
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
    Event {
        /// The part's name.
        part: String,
        /// The voice's name.
        voice: String,
        /// The event id — `event-1f`, the MEI `xml:id`.
        event: String,
        /// The statement that spells it, as a byte range in the source.
        span: Span,
        /// 1-based line of that statement.
        line: u32,
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
    }
}

/// Resolve a finding's evidence into names, spans, and lines.
fn evidence(finding: &AnalysisFinding, score: &ScoreSnapshot, lines: &Lines<'_>) -> EvidenceFacts {
    match *finding.evidence() {
        Evidence::Event { part, voice, id, span } => EvidenceFacts::Event {
            part: part_name(score, part),
            voice: voice_name(score, part, voice),
            event: format!("event-{:x}", id.0),
            span: Span {
                start: span.start,
                end: span.end,
            },
            line: lines.at(span.start).line,
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
