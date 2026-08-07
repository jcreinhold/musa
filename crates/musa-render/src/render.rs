//! The `render_notation` facade (roadmap §15.4): one entry point for every
//! notation backend. Backends consume the shared `NotationPlan`; they never
//! re-derive measures themselves (§12.1).

use musa_compiler::ScoreSnapshot;

use crate::RenderError;
use crate::plan::{NotationOptions, plan_notation};

/// The notation output formats (extended at prompts 14, 27, 32).
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
    let plan = plan_notation(score, options)?;
    let text = match target {
        NotationTarget::Mei => crate::mei::render_mei(&plan)?,
        NotationTarget::LilyPond => crate::ly::render_lilypond(&plan)?,
        NotationTarget::MusicXml => crate::musicxml::render_musicxml(&plan)?,
    };
    Ok(RenderedNotation { target, text })
}
