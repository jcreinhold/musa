//! The checked notation proposal (prompt 205c's facade).
//!
//! One captured MIDI take, given a clock, a destination meter, and a key, is
//! composed into one immutable [`NotationProposal`]: the exact score facts per
//! voice, the chord/rest structure and its shape alternatives, the top ranked
//! rhythm candidate, a complete derivation to captured event ids, the facts the
//! pipeline declared as losses, and — when every written end is a binary
//! subdivision — a canonical Musa source preview that has been parsed and
//! compiled under the destination context.
//!
//! The proposal is an explanation, not an answer. It names what it read and
//! what it dropped, and it never hands a musician a source preview that cannot
//! be compiled: an uncheckable preview is a [`ProposalError`], and a duration
//! this prompt cannot yet spell (a tuplet, tie, dot, grace, or rest) is a
//! declared deferral to prompt 205ca, not a silently wrong source.

// Note, end, and onset slices are all built from `0..completed.notes.len()` and
// remain position-aligned by construction; reading them by index cannot leave
// the slice.
#![allow(clippy::indexing_slicing)]

use std::collections::BTreeMap;
use std::fmt::Write as _;

use musa_compiler::{CompileOptions, DocumentKind, SemanticHash, SourceDocument, compile, format_document};
use musa_score::Key;
use musa_syntax::BarSpacing;

use crate::command::Revision;
use crate::midi::CapturedMidiEvent;
use crate::transcription_pairing::{self, GroupShape};
use crate::transcription_search::{Candidate, Refusal, RhythmEvent, Take, TakeClock};
use crate::transcription_spell;
use crate::transcription_voice::{self, VoiceConstraint, VoiceRefusal};

/// Why a proposal could not be composed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProposalError {
    /// The rhythm search refused the take under the named policy.
    SearchRefused(Refusal),
    /// One onset group held more simultaneous notes than the four-voice
    /// ceiling can voice.
    VoiceRefused(VoiceRefusal),
    /// The generated source preview did not parse and compile; this is an
    /// internal error, never something a reviewer is asked to repair.
    UncheckableSource(String),
    /// A policy-loading or name failure.
    Policy(String),
}

/// One note's composed score facts in the proposal. Indexed like the captured
/// completed-note list; the [`Candidate`] `event_ids` name the same indices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalNote {
    /// The written pitch, with explicit accidental, e.g. `c#4`.
    pub pitch: String,
    /// Enharmonic readings of a chromatic note, empty when the key already
    /// spells it unambiguously.
    pub alternatives: Vec<String>,
    /// The assigned voice, 0..4.
    pub voice: u8,
    /// The written end in grid ticks (24 ticks = one quarter).
    pub written_end_ticks: u32,
    /// Sound continues past the key release under a sustain pedal; the written
    /// end stays at the key release.
    pub pedal_extended: bool,
    /// Too short for the ordinary tier (a grace candidate), never silently
    /// shrunk.
    pub grace: bool,
    /// The raw capture event ids this note derived from (note-on, note-off).
    pub derivation: [u64; 2],
}

/// One onset group's structure and the chord shapes it still offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalGroup {
    pub onset_micros: u64,
    /// Completed-note indices in the group, onset-ascending.
    pub notes: Vec<usize>,
    /// The chord-shape readings the group offers; empty for a single note.
    pub shapes: Vec<GroupShape>,
}

/// A captured fact the pipeline could not fold into a written note, declared
/// rather than silently dropped or invented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProposalLoss {
    /// A note-on that never found a note-off.
    UnpairedNoteOn { event_id: u64 },
    /// A note-off that never found a note-on.
    UnpairedNoteOff { event_id: u64 },
    /// Notes whose sound was still pedal-held when the capture ended.
    HeldAtEnd { count: usize },
    /// A written end not on this prompt's binary grid, deferred to 205ca.
    DeferredDuration { note_index: usize },
}

/// A compiled source preview: the canonical, formatted source and the
/// compilation identity it was proven to carry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalSource {
    source: String,
    kind: DocumentKind,
    identity: SemanticHash,
}

impl ProposalSource {
    /// The canonical formatted source, a complete `piece`.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The compiled document kind of the preview.
    pub const fn kind(&self) -> DocumentKind {
        self.kind
    }

    /// The stable semantic identity of the compiled preview.
    pub const fn identity(&self) -> SemanticHash {
        self.identity
    }
}

/// One immutable, versioned notation proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotationProposal {
    proposal_version: u64,
    report_version: u64,
    take_name: String,
    revision: Revision,
    policy_name: String,
    cost_version: u64,
    cost_fields: Vec<String>,
    meter: String,
    key: Option<Key>,
    notes: Vec<ProposalNote>,
    groups: Vec<ProposalGroup>,
    voice_count: usize,
    candidate: Candidate,
    losses: Vec<ProposalLoss>,
    source: Option<ProposalSource>,
    peak_states: usize,
    peak_storage_bytes: usize,
}

/// The proposal facade version; bump when this public shape changes.
pub const PROPOSAL_VERSION: u64 = 1;

/// Binary grid durations in ticks and the source denominator that spells each
/// (`96/denominator` ticks). Everything else is a deferral to 205ca.
const BINARY_DENOMINATORS: [(u32, u32); 6] = [
    (96, 1), // whole
    (48, 2), // half
    (24, 4), // quarter
    (12, 8), // eighth
    (6, 16), // sixteenth
    (3, 32), // thirty-second
];

impl NotationProposal {
    /// The proposal facade version.
    pub const fn proposal_version(&self) -> u64 {
        self.proposal_version
    }

    /// The rhythm report version this proposal read.
    pub const fn report_version(&self) -> u64 {
        self.report_version
    }

    /// The take's stable identity.
    pub fn take_name(&self) -> &str {
        &self.take_name
    }

    /// The session revision the proposal was computed at.
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    /// The checked policy the search ran under.
    pub fn policy_name(&self) -> &str {
        &self.policy_name
    }

    /// The policy's published cost-record version.
    pub const fn cost_version(&self) -> u64 {
        self.cost_version
    }

    /// The policy's published cost-field order.
    pub fn cost_fields(&self) -> &[String] {
        &self.cost_fields
    }

    /// The destination meter the proposal was written against (e.g. `4/4`).
    pub fn meter(&self) -> &str {
        &self.meter
    }

    /// The key the spelling read; `None` means no key was claimed.
    pub const fn key(&self) -> Option<Key> {
        self.key
    }

    /// One composed note per completed note, in captured onset order.
    pub fn notes(&self) -> &[ProposalNote] {
        &self.notes
    }

    /// The onset groups and their chord-shape alternatives.
    pub fn groups(&self) -> &[ProposalGroup] {
        &self.groups
    }

    /// Number of distinct voices the proposal uses, at most four.
    pub const fn voice_count(&self) -> usize {
        self.voice_count
    }

    /// The top ranked rhythm candidate this proposal composes.
    pub fn rhythm_candidate(&self) -> &Candidate {
        &self.candidate
    }

    /// The facts declared as losses, never silently dropped.
    pub fn losses(&self) -> &[ProposalLoss] {
        &self.losses
    }

    /// The compiled source preview, when every written end was spellable here.
    pub fn source(&self) -> Option<&ProposalSource> {
        self.source.as_ref()
    }

    /// The largest surviving rhythm-search beam.
    pub const fn peak_states(&self) -> usize {
        self.peak_states
    }

    /// The largest compacted candidate/back-pointer storage the search used.
    pub const fn peak_storage_bytes(&self) -> usize {
        self.peak_storage_bytes
    }
}

/// Compose one captured take into a notation proposal.
pub(crate) fn propose(
    revision: Revision,
    take_name: &str,
    events: &[CapturedMidiEvent],
    clock: TakeClock,
    bar_ticks: u32,
    meter: &str,
    key: Option<Key>,
    policy_name: &str,
    voice_pins: &[VoiceConstraint],
    spacing: BarSpacing,
) -> Result<NotationProposal, ProposalError> {
    let completed = transcription_pairing::complete(events);
    let quarter = match clock {
        TakeClock::Known { quarter_micros, .. } => quarter_micros,
        TakeClock::Free | TakeClock::Unmeasured => 500_000,
    };

    // The rhythm half: the same search the report runs, so note indices and the
    // candidate's event ids line up with the completed-note order.
    let rhythm_events = completed
        .notes
        .iter()
        .enumerate()
        .map(|(index, note)| RhythmEvent {
            id: u32::try_from(index).unwrap_or(u32::MAX),
            onset_micros: note.onset_micros,
            release_micros: note.key_release_micros,
            sounding_end_micros: note.sounding_end_micros,
        })
        .collect::<Vec<_>>();
    let take = Take {
        name: take_name.to_owned(),
        clock,
        bar_ticks,
        events: rhythm_events,
        pins: Vec::new(),
    };
    let report = crate::rhythm::RhythmTranscriptionReport::transcribe(revision, &take, policy_name);
    if let Some(reason) = report.refusal() {
        return Err(ProposalError::SearchRefused(reason));
    }
    if let Some(message) = report.policy_error() {
        return Err(ProposalError::Policy(message.to_owned()));
    }
    let Some(candidates) = report.candidates() else {
        return Err(ProposalError::Policy("no candidates were produced".to_owned()));
    };
    let Some(candidate) = candidates.first() else {
        return Err(ProposalError::Policy("the rank is empty".to_owned()));
    };

    let assignment = transcription_voice::assign_voices(&completed.notes, quarter, voice_pins)
        .map_err(ProposalError::VoiceRefused)?;
    let spellings = transcription_spell::spell_pitches(&completed.notes, key);
    let ends = transcription_spell::infer_written_ends(&completed.notes, &candidate.durations);

    let notes = (0..completed.notes.len())
        .map(|index| ProposalNote {
            pitch: spellings[index].spelled.clone(),
            alternatives: spellings[index].alternatives.clone(),
            voice: assignment.voices[index],
            written_end_ticks: ends[index].duration_ticks,
            pedal_extended: completed.notes[index].pedal_extended,
            grace: ends[index].grace,
            derivation: completed.notes[index].derivation,
        })
        .collect::<Vec<_>>();

    let groups = transcription_pairing::group_notes(&completed.notes, quarter)
        .into_iter()
        .map(|group| ProposalGroup {
            onset_micros: group.onset_micros,
            notes: group.notes.clone(),
            shapes: transcription_pairing::shape_alternatives(&group),
        })
        .collect::<Vec<_>>();

    let mut losses = Vec::new();
    losses.extend(
        completed
            .unpaired_note_ons
            .iter()
            .copied()
            .map(|event_id| ProposalLoss::UnpairedNoteOn { event_id }),
    );
    losses.extend(
        completed
            .unpaired_note_offs
            .iter()
            .copied()
            .map(|event_id| ProposalLoss::UnpairedNoteOff { event_id }),
    );
    if completed.held_at_end > 0 {
        losses.push(ProposalLoss::HeldAtEnd {
            count: completed.held_at_end,
        });
    }

    let source = build_source(
        take_name,
        meter,
        quarter,
        candidate,
        &completed.notes,
        &notes,
        &mut losses,
        spacing,
    )?;

    Ok(NotationProposal {
        proposal_version: PROPOSAL_VERSION,
        report_version: report.report_version(),
        take_name: take_name.to_owned(),
        revision,
        policy_name: policy_name.to_owned(),
        cost_version: report.cost_version(),
        cost_fields: report.cost_fields().to_vec(),
        meter: meter.to_owned(),
        key,
        notes,
        groups,
        voice_count: assignment.voice_count,
        candidate: candidate.clone(),
        losses,
        source,
        peak_states: report.peak_states(),
        peak_storage_bytes: report.peak_storage_bytes(),
    })
}

/// Build, format, and compile the source preview. Returns `Ok(None)` with a
/// deferral loss when a written end or an inter-note gap is not a binary
/// subdivision (prompt 205ca spells those). Never returns a source that does
/// not compile: an uncheckable preview is an error.
fn build_source(
    take_name: &str,
    meter: &str,
    quarter_micros: u64,
    candidate: &Candidate,
    completed: &[crate::transcription_pairing::CompletedNote],
    notes: &[ProposalNote],
    losses: &mut Vec<ProposalLoss>,
    spacing: BarSpacing,
) -> Result<Option<ProposalSource>, ProposalError> {
    if completed.is_empty() {
        return Ok(None);
    }

    // A voice holds one pitch at a time; lay each one out in onset order with
    // explicit rests for the gaps. Defer if any duration or gap is non-binary.
    let mut by_voice: BTreeMap<u8, Vec<usize>> = BTreeMap::new();
    for (index, note) in notes.iter().enumerate() {
        by_voice.entry(note.voice).or_default().push(index);
    }

    let mut voice_lines = Vec::new();
    for (voice_id, indices) in by_voice {
        let mut line = String::new();
        let mut cursor = 0_u32;
        for &index in &indices {
            let onset = candidate.onsets[index];
            let written_end = notes[index].written_end_ticks;
            let Some(denominator) = binary_denominator(written_end) else {
                losses.push(ProposalLoss::DeferredDuration { note_index: index });
                return Ok(None);
            };
            if onset > cursor {
                let Some(rest_denominators) = decompose_gap(onset.saturating_sub(cursor)) else {
                    losses.push(ProposalLoss::DeferredDuration { note_index: index });
                    return Ok(None);
                };
                for rest_denominator in rest_denominators {
                    let _ = write!(line, "rest/{rest_denominator} ");
                }
            } else if onset < cursor {
                // Overlap within a voice is a tie, not an ordinary note; 205ca
                // spells it.
                losses.push(ProposalLoss::DeferredDuration { note_index: index });
                return Ok(None);
            }
            let _ = write!(line, "{}/{denominator} ", notes[index].pitch);
            cursor = onset.saturating_add(written_end);
        }
        let line = line.trim_end();
        if !line.is_empty() {
            voice_lines.push(format!("            voice voice{voice_id} {{ | {line} }}"));
        }
    }

    if voice_lines.is_empty() {
        return Ok(None);
    }

    let bpm = 60_000_000_u64.checked_div(quarter_micros.max(1)).unwrap_or(120);
    let mut text = String::new();
    let _ = writeln!(text, "piece \"{take_name}\" {{");
    let _ = writeln!(text, "    tempo 1/4 = {bpm};");
    let _ = writeln!(text, "    meter {meter};");
    text.push_str("    score {\n");
    text.push_str("        part keyboard {\n");
    for line in voice_lines {
        let _ = writeln!(text, "{line}");
    }
    text.push_str("        }\n");
    text.push_str("    }\n");
    text.push_str("}\n");

    let Some(formatted) = format_document(&text, spacing) else {
        return Err(ProposalError::UncheckableSource(
            "the generated source did not format".to_owned(),
        ));
    };

    let document = SourceDocument::new(formatted.clone(), format!("{take_name} preview"));
    let compilation = compile(&document, &CompileOptions::default());
    if compilation.has_errors() {
        let detail = compilation
            .diagnostics()
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(ProposalError::UncheckableSource(detail));
    }

    Ok(Some(ProposalSource {
        source: formatted,
        kind: compilation.kind(),
        identity: compilation.identity(),
    }))
}

/// The denominator whose `note/denominator` equals the tick length, for the
/// binary grid only.
fn binary_denominator(ticks: u32) -> Option<u32> {
    BINARY_DENOMINATORS
        .iter()
        .find_map(|(tick, denominator)| (*tick == ticks).then_some(*denominator))
}

/// Decompose a gap into binary rests, largest first; `None` when the gap is not
/// a sum of binary subdivisions (a ternary/tuplet gap, 205ca's territory).
fn decompose_gap(ticks: u32) -> Option<Vec<u32>> {
    if ticks == 0 {
        return Some(Vec::new());
    }
    if !ticks.is_multiple_of(3) {
        return None;
    }
    let mut remaining = ticks;
    let mut parts = Vec::new();
    for (tick, denominator) in BINARY_DENOMINATORS {
        while remaining >= tick {
            remaining = remaining.saturating_sub(tick);
            parts.push(denominator);
        }
    }
    (remaining == 0).then_some(parts)
}
