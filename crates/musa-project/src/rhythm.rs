//! The versioned rhythm-transcription report (prompt 204b's facade).
//!
//! One request over one completed-note take yields one immutable report. The
//! report names the take, revision, and policy it read, carries the checked
//! policy's published cost-field order and version, and projects the search
//! outcome without exposing any optimizer state or the checked-source
//! projection. Pairing raw note-ons/offs into completed notes is prompt 205's
//! step, so a report is built from [`Take`] data, not raw MIDI.

use crate::command::Revision;
use crate::transcription_policy::standard;
use crate::transcription_search::{Candidate, REPORT_VERSION, Refusal, ReviewRegion, SearchOutcome, Take};

/// One immutable, versioned rhythm-transcription report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhythmTranscriptionReport {
    report_version: u64,
    take_name: String,
    revision: Revision,
    policy_name: String,
    cost_fields: Vec<String>,
    cost_version: u64,
    outcome: Option<SearchOutcome>,
    policy_error: Option<String>,
    peak_states: usize,
    peak_storage_bytes: usize,
}

impl RhythmTranscriptionReport {
    /// Build a report by running the checked policy over one take.
    pub(crate) fn transcribe(revision: Revision, take: &Take, policy_name: &str) -> Self {
        let cost_fields;
        let cost_version;
        let outcome;
        let policy_error;
        let mut peak_states = 0;
        let mut peak_storage_bytes = 0;
        match standard() {
            Ok(policies) => {
                cost_fields = policies.cost_fields().to_vec();
                cost_version = policies.cost_version();
                match policies.policy(policy_name) {
                    Ok(policy) => {
                        debug_assert_eq!(policy.name(), policy_name, "lookup and declared name must agree");
                        let (result, measurements) = crate::transcription_search::search_take_measured(take, policy);
                        outcome = Some(result);
                        peak_states = measurements.peak_states;
                        peak_storage_bytes = measurements.peak_storage_bytes;
                        policy_error = None;
                    }
                    Err(error) => {
                        outcome = None;
                        policy_error = Some(error.to_string());
                    }
                }
            }
            Err(message) => {
                cost_fields = Vec::new();
                cost_version = 0;
                outcome = None;
                policy_error = Some(message.to_owned());
            }
        }
        Self {
            report_version: REPORT_VERSION,
            take_name: take.name.clone(),
            revision,
            policy_name: policy_name.to_owned(),
            cost_fields,
            cost_version,
            outcome,
            policy_error,
            peak_states,
            peak_storage_bytes,
        }
    }

    /// The report facade version; bump it when this public shape changes.
    pub const fn report_version(&self) -> u64 {
        self.report_version
    }

    /// The take's stable identity.
    pub fn take_name(&self) -> &str {
        &self.take_name
    }

    /// The session revision the report was computed at.
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    /// The checked policy the search ran under.
    pub fn policy_name(&self) -> &str {
        &self.policy_name
    }

    /// The policy's published cost-field order.
    pub fn cost_fields(&self) -> &[String] {
        &self.cost_fields
    }

    /// The policy's published cost-record version.
    pub const fn cost_version(&self) -> u64 {
        self.cost_version
    }

    /// The ranked candidates, in order, when one was produced.
    pub fn candidates(&self) -> Option<&[Candidate]> {
        match &self.outcome {
            Some(SearchOutcome::Ranked { candidates, .. }) => Some(candidates),
            _ => None,
        }
    }

    /// The local structural review regions, when a rank was produced.
    pub fn needs_review(&self) -> Option<&[ReviewRegion]> {
        match &self.outcome {
            Some(SearchOutcome::Ranked { needs_review, .. }) => Some(needs_review),
            _ => None,
        }
    }

    /// The refusal, when the take could not be turned into a grid.
    pub fn refusal(&self) -> Option<Refusal> {
        match &self.outcome {
            Some(SearchOutcome::Refused(reason)) => Some(*reason),
            _ => None,
        }
    }

    /// A policy-loading or name failure, when the search never ran.
    pub fn policy_error(&self) -> Option<&str> {
        self.policy_error.as_deref()
    }

    /// The largest surviving beam the search retained.
    pub const fn peak_states(&self) -> usize {
        self.peak_states
    }

    /// The largest compacted candidate/back-pointer storage the search used.
    pub const fn peak_storage_bytes(&self) -> usize {
        self.peak_storage_bytes
    }
}
