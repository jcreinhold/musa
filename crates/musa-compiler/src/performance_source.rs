//! Checked source interpretation of finite performance request batches.
//!
//! Rust transcribes notation facts and already-parsed profile declarations;
//! `std::performance` alone decides what those declarations mean. One checked
//! artifact per part keeps work linear and gives the track bridge an exact,
//! schema-versioned answer rather than evaluator state or a Rust enum mirror.

use std::fmt::Write as _;

use musa_score::{Diagnostic, PerformanceProfile, PerformanceRequests, ScoreSnapshot};
use num_rational::Ratio;

use crate::{CompileOptions, SourceDocument, checked_source_value};

/// Failure of the checked source-to-track performance bridge.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PerformanceBridgeError {
    /// Source performance policy did not check or normalize.
    #[error("source performance interpretation failed: {0}")]
    Source(String),
    /// Mechanical coordinate/provenance construction failed.
    #[error(transparent)]
    Track(#[from] musa_score::PerformanceError),
}

/// Interpret notation through checked `std::performance` policy and construct
/// exact performed gesture tracks.
///
/// This is the production facade. Rust supplies no musical fallback: source
/// failure or source↔projection disagreement aborts the complete plan.
///
/// # Errors
///
/// Returns [`PerformanceBridgeError`] when source checking/evaluation or the
/// exact provenance/coordinate bridge fails.
pub fn lower_gestures(score: &ScoreSnapshot) -> Result<musa_score::GesturePlan, PerformanceBridgeError> {
    let artifacts = checked_performance_interpretations(score).map_err(|diagnostics| {
        PerformanceBridgeError::Source(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>()
                .join("; "),
        )
    })?;
    musa_score::lower_gestures_from_checked(score, &artifacts).map_err(Into::into)
}

/// Check and evaluate every part's source performance request batch.
///
/// Each returned artifact corresponds to the same-index part from
/// [`musa_score::performance_requests`]. The artifact is ordinary normalized
/// source data and has no public constructor; downstream mechanical lowering
/// can therefore trust that every result passed the general elaborator,
/// pattern unifier, evaluator, and storability check.
///
/// # Errors
///
/// Returns import, elaboration, resource, normalization, or schema diagnostics
/// from the ordinary checked-source path. No partial batch is published.
pub fn checked_performance_interpretations(
    score: &ScoreSnapshot,
) -> Result<Vec<musa_calculus::CheckedSource>, Vec<Diagnostic>> {
    let batches = musa_score::performance_requests(score);
    let mut artifacts = Vec::with_capacity(batches.len());
    let mut diagnostics = Vec::new();
    for batch in &batches {
        match checked_batch(batch) {
            Ok(artifact) => artifacts.push(artifact),
            Err(mut errors) => diagnostics.append(&mut errors),
        }
    }
    if diagnostics.is_empty() {
        Ok(artifacts)
    } else {
        Err(diagnostics)
    }
}

fn checked_batch(batch: &PerformanceRequests) -> Result<musa_calculus::CheckedSource, Vec<Diagnostic>> {
    let mut source = String::from("import std::performance;\n");
    let _ = writeln!(
        source,
        "let selected_profile: PerformanceProfile = {};",
        profile(batch.profile.as_ref())
    );
    source.push_str("let performance_requests: List(InterpretationRequest) = [\n");
    for view in &batch.views {
        let marks = view
            .marks
            .iter()
            .map(|mark| quoted(mark.name()))
            .collect::<Vec<_>>()
            .join(", ");
        let dynamic = view
            .dynamic
            .map_or_else(|| "None".to_owned(), |mark| format!("Some({})", quoted(mark.name())));
        let hairpin = view.hairpin.map_or_else(
            || "None".to_owned(),
            |(target, reached)| {
                format!(
                    "Some(HairpinView {{ target = {}, reached = {} }})",
                    quoted(target.name()),
                    ratio(reached)
                )
            },
        );
        let _ = writeln!(
            source,
            "    InterpretationRequest {{ policy = selected_profile, view = NotationView {{ instance = GestureId {{ value = {} }}, written_pitch = {}, dynamic_mark = {}, marks = [{}], hairpin = {}, connection = None, techniques = [] }} }},",
            view.instance, view.pitch, dynamic, marks, hairpin
        );
    }
    source.push_str(
        "];\nlet performance_interpretation: PerformanceInterpretationArtifact = PerformanceInterpretationArtifact {\n    schema_version = 1,\n    results = interpret_all(performance_requests)\n};\npiece \"Performance bridge\" { meter 4/4; key c major; score { part proof { voice observed { rest/1 } } } }\n",
    );
    checked_source_value(
        &SourceDocument::new(source, format!("musa-performance:/part-{}.musa", batch.part.0)),
        &CompileOptions::default(),
        "performance_interpretation",
        &musa_calculus::SourceSchema::new(
            "std.performance.PerformanceInterpretationArtifact",
            "PerformanceInterpretationArtifact",
            1,
        ),
    )
}

fn profile(profile: Option<&PerformanceProfile>) -> String {
    let Some(profile) = profile else {
        return "neutral".to_owned();
    };
    let dynamics = profile
        .dynamic_rules()
        .map(|(mark, level)| {
            format!(
                "DynamicRule {{ name = {}, expression = {} }}",
                quoted(mark.name()),
                ratio(level)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let timings = profile
        .mark_rules()
        .map(|(mark, rule)| {
            format!(
                "LegacyMarkTiming {{ name = {}, gate = {}, attack_seconds = {}, hold = {} }}",
                quoted(mark.name()),
                ratio(rule.gate),
                ratio(rule.attack),
                ratio(rule.hold)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("profile_from_legacy([{dynamics}], [{timings}])")
}

fn ratio(value: Ratio<i64>) -> String {
    format!("{}/{}", value.numer(), value.denom())
}

fn quoted(value: &str) -> String {
    format!("{value:?}")
}
