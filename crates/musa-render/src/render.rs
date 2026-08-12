//! The `render_notation` facade (roadmap §15.4): one entry point for every
//! notation backend. Backends consume the shared `NotationPlan`; they never
//! re-derive measures themselves (§12.1).

use musa_compiler::ScoreSnapshot;

use crate::RenderError;
use crate::plan::{NotationOptions, plan_notation};

/// The notation output formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotationTarget {
    /// MEI 5 XML (Verovio-compatible, `xml:id` event mapping).
    Mei,
    /// `LilyPond` source (export only, §12.3).
    LilyPond,
    /// `MusicXML` 4.0 `score-partwise` (export only, §12.4) — the
    /// interchange format, without the event provenance MEI carries.
    MusicXml,
}

/// The rendered output of one backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedNotation {
    target: NotationTarget,
    text: String,
    warnings: Vec<String>,
}

impl RenderedNotation {
    /// Which backend produced this.
    pub fn target(&self) -> NotationTarget {
        self.target
    }

    /// The serialized document text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// What this target could not say, once per kind.
    ///
    /// A backend that has no element for a written freedom carries the
    /// realized music plus a text direction, which is a reading of the piece
    /// rather than the piece. That is a fact about the format, so it is
    /// reported here rather than discovered by whoever opens the file
    /// (`docs/rules/kernel/07-backend-contract.md`).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

/// What `target` cannot say about `plan`, once per kind.
fn losses(plan: &crate::plan::NotationPlan, target: NotationTarget) -> Vec<String> {
    let format = match target {
        NotationTarget::Mei => "MEI",
        NotationTarget::LilyPond => "LilyPond",
        NotationTarget::MusicXml => "MusicXML",
    };
    let mut losses = Vec::new();
    if plan
        .open()
        .iter()
        .any(|region| region.kind == crate::plan::OpenShape::Mobile)
    {
        losses.push(format!(
            "{format} has no mobile form: the fragments are exported in the order this reading chose, \
             under a text direction saying they may be played in any"
        ));
    }
    if plan
        .open()
        .iter()
        .any(|region| region.kind == crate::plan::OpenShape::Improvise)
    {
        losses.push(format!(
            "{format} has no improvised region: the frame is exported as its length of rests, \
             under a text direction saying to improvise"
        ));
    }
    if plan
        .open()
        .iter()
        .any(|region| region.kind == crate::plan::OpenShape::Passes)
    {
        losses.push(format!(
            "{format} has no ranged repeat: the body is exported with the number of passes this reading \
             took, under a text direction saying how many the piece allows"
        ));
    }
    // A mark whose column is empty is a mark this format has no way to say.
    // One line per mark rather than one per occurrence: the fact is about the
    // format, and a page of pedal marks would otherwise report itself once per
    // measure (`docs/rules/kernel/07-backend-contract.md`).
    let spelled = |mark: musa_compiler::Mark| match target {
        NotationTarget::Mei => mark.def().mei,
        NotationTarget::LilyPond => mark.def().lilypond,
        NotationTarget::MusicXml => mark.def().musicxml,
    };
    let mut silent: Vec<&'static str> = Vec::new();
    for staff in plan.staves() {
        for measure in staff.measures() {
            for lane in measure.lanes() {
                let written = lane
                    .items()
                    .iter()
                    .flat_map(|item| item.articulations().iter().copied())
                    .chain(lane.points().iter().map(|point| point.mark))
                    .chain(lane.marks().iter().map(|span| span.mark));
                for mark in written {
                    if spelled(mark).is_none() && !silent.contains(&mark.name()) {
                        silent.push(mark.name());
                    }
                }
            }
        }
    }
    silent.sort_unstable();
    for name in silent {
        losses.push(format!("{format} has no `{name}`: the mark is not exported"));
    }
    // Whether the staves are counted differently from each other, and whether
    // the bars they draw actually diverge. The second is the one that costs
    // something: 6/8 against 3/4 is the same barline grid beamed two ways,
    // and only different *lengths* make a measure mean two things.
    let first = plan.staves().first();
    let polymetric = plan
        .staves()
        .iter()
        .any(|staff| Some(staff.time_signature()) != first.map(crate::plan::StaffPlan::time_signature));
    let ragged = plan
        .staves()
        .iter()
        .any(|staff| Some(staff.measures().len()) != first.map(|staff| staff.measures().len()));
    if polymetric && ragged && target == NotationTarget::Mei {
        losses.push(
            "MEI numbers measures for the score, not for each staff: these staves are in different meters and \
             their barlines diverge, so a measure of this document holds whatever each staff had \
             reached by then rather than one measure of each"
                .to_owned(),
        );
    }
    if plan.staves().iter().any(|staff| !staff.tempos().is_empty()) {
        losses.push(format!(
            "{format} has no second conductor: a part at its own tempo is exported with its marking \
             attached to its own staff, and whether a reader's software follows it is that \
             software's answer"
        ));
    }
    if !plan.holds().is_empty() {
        losses.push(format!(
            "{format} has no free-duration bracket: each held note is exported at its written value, \
             under a text direction saying how far it may be held"
        ));
    }
    losses
}

/// Plan and render a score to a notation backend.
///
/// # Errors
/// [`RenderError::Notation`] when planning fails; [`RenderError::Xml`] on
/// writer failures.
pub fn render_notation(
    score: &ScoreSnapshot,
    target: NotationTarget,
    options: &NotationOptions,
) -> Result<RenderedNotation, RenderError> {
    let span = tracing::info_span!("render_notation", target = ?target);
    let _entered = span.enter();
    let plan = plan_notation(score, options)?;
    let text = match target {
        NotationTarget::Mei => crate::mei::render_mei(&plan)?,
        NotationTarget::LilyPond => crate::ly::render_lilypond(&plan)?,
        NotationTarget::MusicXml => crate::musicxml::render_musicxml(&plan)?,
    };
    let warnings = losses(&plan, target);
    // A backend that cannot say something the plan holds drops it silently
    // into `warnings`, which a caller may or may not print. The count is the
    // one number that says a rendering was lossy at all.
    tracing::debug!(bytes = text.len(), warnings = warnings.len(), "rendered");
    Ok(RenderedNotation { target, text, warnings })
}
