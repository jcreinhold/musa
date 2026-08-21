//! The studio, as the Sound and Mix workspaces read and edit it
//! (roadmap §14.4).
//!
//! Two halves, and they are deliberately the same two halves the score has.
//! [`StudioFacts`] is what the interface *displays*: patches, their stages,
//! every parameter's value and unit, the buses, the sends, and which patch
//! realizes each part — all display-ready, so a fader renders a number it was
//! handed rather than converting one. [`StudioEdit`] is what a knob *does*:
//! musical intent in, [`TextEdit`]s out.
//!
//! The rule that shapes both is roadmap §11: the `.musa` source is the one
//! authority. A slider does not own a value that the text mirrors — it reads
//! the compiled value and writes back the smallest text change that produces
//! the new one. Concretely, that means rewriting the literal the composer
//! wrote and nothing else: regenerating the whole call would silently
//! normalize `30 ms` to `0.03 s`, reorder named arguments, and drop any
//! comment inside the parentheses. So a parameter that *was* written is
//! rewritten in the unit it was written in, and only a parameter that was
//! never written is inserted, in the unit it is declared in.

use musa_compiler::{Modulation, NodeIndex, StudioSpec, Unit, Value};
use serde::Serialize;

use crate::command::TextEdit;
use crate::diagnostic::Span;
use crate::error::ProjectError;

/// Which of the studio's three namespaces a container lives in.
///
/// They are separate namespaces in the language, so a name alone does not
/// identify a container and an edit has to say which one it means.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ContainerKind {
    /// `patch glass_pad { … }` — an instrument.
    Patch,
    /// `bus hall { … }` — an effect bus.
    Bus,
    /// `signal lfo = …;` — a modulation source.
    Signal,
}

/// One processor stage inside a patch, as the Sound workspace draws it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageFacts {
    /// The stage's index within its container — the key an edit names it by.
    pub index: usize,
    /// The processor, written the way the language writes it: `lowpass`.
    pub processor: String,
    /// The name it was bound to (`lfo = oscillator(...)`), when it has one.
    /// This is what a `modulate` path addresses.
    pub label: Option<String>,
    /// Its parameters, in declaration order.
    pub params: Vec<ParamFacts>,
}

/// One parameter of one stage.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamFacts {
    /// The name it is written with: `cutoff`.
    pub name: String,
    /// Its value in the unit's base — seconds, hertz, decibels, or a plain
    /// ratio. Written `30 ms` reads back as `0.03`.
    pub value: f64,
    /// How the unit is written (`Hz`, `dB`, `s`), or empty for a plain ratio.
    pub unit: String,
    /// The smallest value a control may write, in the unit above.
    pub minimum: f64,
    /// The largest.
    pub maximum: f64,
    /// Whether the patch wrote this value. False means it is the default, and
    /// the interface should show it as inherited rather than as chosen.
    pub written: bool,
    /// Where the written value is, for revealing it in the source.
    pub span: Option<Span>,
    /// The signal modulating this parameter, when one does. A modulated
    /// parameter's written value is an offset the modulation moves around, so
    /// the interface must say so rather than showing a knob that appears to
    /// disagree with what is heard (§13.7).
    pub modulated_by: Option<String>,
}

/// A patch, bus, or signal, with its stages.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerFacts {
    /// Which namespace it lives in.
    pub kind: ContainerKind,
    /// Its declared name.
    pub name: String,
    /// Its stages, in the order the chain runs them.
    pub stages: Vec<StageFacts>,
}

/// `assign violin -> glass_pad;`, plus the parts that have no assignment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentFacts {
    /// The part's name, as the score declares it.
    pub part: String,
    /// The patch realizing it, or `None` for a part the studio does not
    /// mention — which keeps the built-in instrument rather than going silent
    /// (§14.8), and which the workspace shows as such.
    pub patch: Option<String>,
}

/// `send violin -> hall at -18 dB;`
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendFacts {
    /// The part or bus sending.
    pub source: String,
    /// The bus receiving.
    pub bus: String,
    /// The level in decibels, whichever unit it was written in. One scale for
    /// every fader is the point: a mixer that showed some sends in dB and
    /// others as ratios would be asking the composer to convert.
    pub decibels: f64,
    /// Where the written level is.
    pub span: Option<Span>,
}

/// `route violin -> master;`
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteFacts {
    /// The part or bus whose output is routed.
    pub source: String,
    /// Where it goes: a bus name, or `master`.
    pub destination: String,
}

/// Everything the Sound and Mix workspaces display.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioFacts {
    /// Whether the piece declares a `studio` block at all. False means every
    /// part sounds through the built-in instrument, and the workspaces say so
    /// rather than showing an empty rack.
    pub declared: bool,
    /// The patches, in source order.
    pub patches: Vec<ContainerFacts>,
    /// The buses, in source order.
    pub buses: Vec<ContainerFacts>,
    /// The modulation signals, in source order.
    pub signals: Vec<ContainerFacts>,
    /// Every part in the score, with the patch realizing it.
    pub assignments: Vec<AssignmentFacts>,
    /// The sends, in source order.
    pub sends: Vec<SendFacts>,
    /// The routes, in source order.
    pub routes: Vec<RouteFacts>,
}

impl StudioFacts {
    /// Derive the facts from a compiled studio and the score it belongs to.
    ///
    /// The score is needed for one thing only: the list of parts, so a part
    /// the studio never mentions still gets a row.
    pub(crate) fn derive(studio: &StudioSpec, parts: &[String]) -> Self {
        Self {
            declared: !studio.is_empty(),
            patches: containers(studio, ContainerKind::Patch),
            buses: containers(studio, ContainerKind::Bus),
            signals: containers(studio, ContainerKind::Signal),
            assignments: parts
                .iter()
                .map(|part| AssignmentFacts {
                    part: part.clone(),
                    patch: studio.patch_for_part(part).map(ToOwned::to_owned),
                })
                .collect(),
            sends: studio
                .sends()
                .iter()
                .map(|send| SendFacts {
                    source: send.source.clone(),
                    bus: send.bus.clone(),
                    decibels: level_decibels(send.level),
                    span: send.level_span.map(span),
                })
                .collect(),
            routes: studio
                .routes()
                .iter()
                .map(|route| RouteFacts {
                    source: route.source.clone(),
                    destination: route.destination.clone(),
                })
                .collect(),
        }
    }
}

fn containers(studio: &StudioSpec, kind: ContainerKind) -> Vec<ContainerFacts> {
    let declared: Vec<(&str, &musa_compiler::Patch)> = match kind {
        ContainerKind::Patch => studio.patches().collect(),
        ContainerKind::Bus => studio.buses().collect(),
        ContainerKind::Signal => studio.signals().collect(),
    };
    declared
        .into_iter()
        .map(|(name, patch)| ContainerFacts {
            kind,
            name: name.to_owned(),
            stages: patch
                .nodes()
                .iter()
                .enumerate()
                .map(|(index, node)| StageFacts {
                    index,
                    processor: node.processor.name().to_owned(),
                    label: node.label.clone(),
                    params: node
                        .processor
                        .params()
                        .iter()
                        .enumerate()
                        .map(|(at, declared)| ParamFacts {
                            name: declared.name.to_owned(),
                            value: node.params.get(at).map_or(declared.default, |value| value.magnitude),
                            unit: declared.unit.spelling().unwrap_or_default().to_owned(),
                            minimum: declared.range.0,
                            maximum: declared.range.1,
                            written: node.param_spans.get(at).copied().flatten().is_some(),
                            span: node.param_spans.get(at).copied().flatten().map(span),
                            modulated_by: modulator(studio, name, index, declared.name),
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

/// The signal modulating one parameter, if any. Modulation is addressed by
/// patch name, node, and parameter, which is exactly this triple.
fn modulator(studio: &StudioSpec, container: &str, node: NodeIndex, param: &'static str) -> Option<String> {
    studio
        .modulations()
        .iter()
        .find(|modulation: &&Modulation| {
            modulation.patch == container && modulation.node == node && modulation.param == param
        })
        .map(|modulation| modulation.source.clone())
}

fn span(source: musa_score::SourceSpan) -> Span {
    Span {
        start: source.start,
        end: source.end,
    }
}

/// A level in decibels, whatever unit it was written in.
fn level_decibels(level: Value) -> f64 {
    match level.unit {
        Unit::Decibels => level.magnitude,
        // `as_linear` is the identity for the other units, and a ratio of
        // zero or less has no decibel value: silence is the floor.
        Unit::Hz | Unit::Linear | Unit::Seconds => {
            let linear = level.as_linear();
            if linear > 0.0 {
                20.0 * linear.log10()
            } else {
                f64::NEG_INFINITY
            }
        }
    }
}

/// A structured studio edit (roadmap §11's `AssignPatch`, and the parameter
/// changes §14.4's workspaces make).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum StudioEdit {
    /// Point a part at a different patch. Rewrites the existing `assign`'s
    /// patch name, or writes a new `assign` if the part had none.
    AssignPatch {
        /// The part, as the score declares it.
        part: String,
        /// The patch to realize it with.
        patch: String,
    },
    /// Change one parameter of one stage — what turning a knob does.
    SetParam {
        /// Which namespace the container is in.
        kind: ContainerKind,
        /// The container's name.
        container: String,
        /// The stage's index, from [`StageFacts::index`].
        stage: usize,
        /// The parameter's name.
        param: String,
        /// The new value, in the unit's base — the same scale
        /// [`ParamFacts::value`] reports.
        value: f64,
    },
    /// Move one send's fader.
    SetSendLevel {
        /// The part or bus sending.
        source: String,
        /// The bus receiving.
        bus: String,
        /// The new level in decibels, the same scale [`SendFacts::decibels`]
        /// reports.
        decibels: f64,
    },
}

/// How a failed studio edit names itself.
pub(crate) fn describe(edit: &StudioEdit) -> String {
    match *edit {
        StudioEdit::AssignPatch {
            ref part, ref patch, ..
        } => format!("assigning {part} to {patch}"),
        StudioEdit::SetParam {
            ref container,
            ref param,
            value,
            ..
        } => format!("setting {container}'s {param} to {value}"),
        StudioEdit::SetSendLevel {
            ref source,
            ref bus,
            decibels,
        } => format!("sending {source} to {bus} at {decibels} dB"),
    }
}

/// Resolve a studio edit into text edits against `source`.
///
/// # Errors
/// [`ProjectError::Uneditable`] when the edit names something this revision
/// does not have, or something the source cannot express.
pub(crate) fn edits_for(studio: &StudioSpec, source: &str, edit: &StudioEdit) -> Result<Vec<TextEdit>, ProjectError> {
    match *edit {
        StudioEdit::AssignPatch {
            ref part, ref patch, ..
        } => assign_patch(studio, source, part, patch),
        StudioEdit::SetParam {
            kind,
            ref container,
            stage,
            ref param,
            value,
        } => set_param(studio, source, kind, container, stage, param, value),
        StudioEdit::SetSendLevel {
            source: ref sender,
            ref bus,
            decibels,
        } => set_send_level(studio, source, sender, bus, decibels),
    }
}

fn missing(what: impl Into<String>) -> ProjectError {
    ProjectError::Uneditable(what.into())
}

fn assign_patch(studio: &StudioSpec, source: &str, part: &str, patch: &str) -> Result<Vec<TextEdit>, ProjectError> {
    if studio.patch(patch).is_none() {
        return Err(missing(format!("there is no patch named `{patch}`")));
    }
    match studio.assignment(part).and_then(|assignment| assignment.patch_span) {
        // The part already has an assignment: replace the name it points at,
        // leaving the statement — and any comment on it — alone.
        Some(written) => Ok(vec![TextEdit::new(span(written), patch)]),
        None => {
            let block = studio
                .span()
                .ok_or_else(|| missing("this piece has no `studio` block to write into"))?;
            let at = closing_brace(source, span(block))
                .ok_or_else(|| missing("this piece's `studio` block is unfinished"))?;
            Ok(vec![TextEdit::new(
                Span { start: at, end: at },
                format!("    assign {part} -> {patch};\n"),
            )])
        }
    }
}

/// The offset of the statement-insertion point inside a block: the start of
/// the line its closing brace is on, so the new statement lands as the last
/// one rather than beside the brace.
fn closing_brace(source: &str, block: Span) -> Option<u32> {
    let end = usize::try_from(block.end).ok()?;
    let text = source.get(..end)?;
    let brace = text.rfind('}')?;
    let line = text.get(..brace)?.rfind('\n').map_or(brace, |at| at.saturating_add(1));
    u32::try_from(line).ok()
}

fn set_param(
    studio: &StudioSpec,
    source: &str,
    kind: ContainerKind,
    container: &str,
    stage: usize,
    param: &str,
    value: f64,
) -> Result<Vec<TextEdit>, ProjectError> {
    let found = match kind {
        ContainerKind::Patch => studio.patch(container),
        ContainerKind::Bus => studio.buses().find(|(name, _)| *name == container).map(|(_, it)| it),
        ContainerKind::Signal => studio.signals().find(|(name, _)| *name == container).map(|(_, it)| it),
    };
    let found = found.ok_or_else(|| missing(format!("there is no `{container}` in the studio")))?;
    let node = found
        .nodes()
        .get(stage)
        .ok_or_else(|| missing(format!("`{container}` has no stage {stage}")))?;
    let at = node
        .processor
        .params()
        .iter()
        .position(|declared| declared.name == param)
        .ok_or_else(|| missing(format!("`{}` has no parameter `{param}`", node.processor.name())))?;
    let declared = node
        .processor
        .params()
        .get(at)
        .copied()
        .ok_or_else(|| missing("that parameter went missing"))?;

    match node.param_spans.get(at).copied().flatten() {
        // Written: replace the literal in the scale it was written in, so a
        // patch that says `30 ms` keeps saying milliseconds.
        Some(written) => {
            let range = span(written);
            let text = slice(source, range).unwrap_or_default();
            Ok(vec![TextEdit::new(range, rewritten(text, declared.unit, value))])
        }
        // Never written: add it, in the unit it is declared in.
        None => {
            let call = node
                .span
                .ok_or_else(|| missing("that stage was not written in the source"))?;
            let call = span(call);
            let text = slice(source, call).unwrap_or_default();
            let close = text
                .rfind(')')
                .ok_or_else(|| missing("that stage takes no arguments to set"))?;
            let inside = text
                .get(..close)
                .and_then(|head| head.find('(').map(|open| open.saturating_add(1)));
            let empty = inside.is_none_or(|open| text.get(open..close).is_none_or(|args| args.trim().is_empty()));
            let separator = if empty { "" } else { ", " };
            let literal = written_value(declared.unit, value);
            let at = call.start.saturating_add(u32::try_from(close).unwrap_or(0));
            Ok(vec![TextEdit::new(
                Span { start: at, end: at },
                format!("{separator}{param}: {literal}"),
            )])
        }
    }
}

fn set_send_level(
    studio: &StudioSpec,
    source: &str,
    sender: &str,
    bus: &str,
    decibels: f64,
) -> Result<Vec<TextEdit>, ProjectError> {
    let send = studio
        .sends()
        .iter()
        .find(|send| send.source == sender && send.bus == bus)
        .ok_or_else(|| missing(format!("there is no send from `{sender}` to `{bus}`")))?;
    let written = send
        .level_span
        .ok_or_else(|| missing("that send's level was not written in the source"))?;
    let range = span(written);
    let text = slice(source, range).unwrap_or_default();
    // A send written as a plain ratio keeps being one; the fader's decibels
    // convert back rather than rewriting the composer's choice of scale.
    let value = match send.level.unit {
        Unit::Decibels => decibels,
        Unit::Hz | Unit::Linear | Unit::Seconds => {
            if decibels.is_finite() {
                10f64.powf(decibels / 20.0)
            } else {
                0.0
            }
        }
    };
    Ok(vec![TextEdit::new(range, rewritten(text, send.level.unit, value))])
}

fn slice(source: &str, range: Span) -> Option<&str> {
    let start = usize::try_from(range.start).ok()?;
    let end = usize::try_from(range.end).ok()?;
    source.get(start..end)
}

/// A replacement for an existing literal, in the scale that literal used.
///
/// The only scale the language has two spellings for is time, and `ms` is
/// common enough in a patch that normalizing it away would be a visible loss.
fn rewritten(existing: &str, unit: Unit, value: f64) -> String {
    if unit == Unit::Seconds && existing.trim_end().ends_with("ms") {
        return format!("{} ms", number(value * 1000.0));
    }
    written_value(unit, value)
}

/// A literal in the unit's canonical spelling.
fn written_value(unit: Unit, value: f64) -> String {
    match unit.spelling() {
        Some(spelling) => format!("{} {spelling}", number(value)),
        None => number(value),
    }
}

/// A number as a composer would write it: no exponent, no trailing zeros, and
/// no more precision than a knob can mean.
fn number(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_owned();
    }
    let text = format!("{value:.4}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text.is_empty() || text == "-" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}
