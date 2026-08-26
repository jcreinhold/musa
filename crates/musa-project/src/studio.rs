//! The studio, as the Sound and Mix workspaces read and edit it
//! (roadmap §14.4).
//!
//! Two halves, and they are deliberately the same two halves the score has.
//! [`StudioFacts`] is what the interface *displays*: patches, their stages,
//! every parameter's exact value and unit, the buses, the sends, and which
//! patch realizes each part. The UI may project a rational onto a control,
//! but it never becomes the authority. [`StudioEdit`] is what a knob *does*:
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

use musa_dsp::{SoundUnit, StudioExecution, StudioGraphProjection};
use musa_syntax::ast::AstNode as _;
use serde::Serialize;

use crate::command::TextEdit;
use crate::diagnostic::Span;
use crate::error::ProjectError;
use crate::facts::Fraction;

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
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageFacts {
    /// The stage's index within its container — the key an edit names it by.
    pub index: usize,
    /// The processor, written the way the language writes it: `lowpass`.
    pub processor: String,
    /// Plain catalogue sentence used as the visible description and
    /// accessible explanation.
    pub summary: String,
    /// Typed public call shape.
    pub signature: String,
    /// Where this processor comes from. Built-ins are explicit rather than
    /// looking like project declarations.
    pub origin: String,
    /// The name it was bound to (`lfo = oscillator(...)`), when it has one.
    /// This is what a `modulate` path addresses.
    pub label: Option<String>,
    /// Its parameters, in declaration order.
    pub params: Vec<ParamFacts>,
}

/// One parameter of one stage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamFacts {
    /// The name it is written with: `cutoff`.
    pub name: String,
    /// Plain catalogue sentence for labels, hover, and accessibility.
    pub summary: String,
    /// Its exact value in the unit's base — seconds, hertz, decibels, or a
    /// plain ratio. Written `30 ms` reads back as `3/100` seconds.
    pub value: Fraction,
    /// How the unit is written (`Hz`, `dB`, `s`), or empty for a plain ratio.
    pub unit: String,
    /// The smallest value a control may write, in the unit above.
    pub minimum: Fraction,
    /// The largest.
    pub maximum: Fraction,
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
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
    /// The effective source declaration, including the edition default.
    pub instrument: String,
    /// The effective source performance profile, including the edition default.
    pub profile: String,
    /// Whether source made the instrument choice explicit.
    pub explicit: bool,
}

/// `send violin -> hall at -18 dB;`
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendFacts {
    /// The part or bus sending.
    pub source: String,
    /// The bus receiving.
    pub bus: String,
    /// The level in decibels, whichever unit it was written in. One scale for
    /// every fader is the point: a mixer that showed some sends in dB and
    /// others as ratios would be asking the composer to convert.
    pub decibels: Fraction,
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

/// One named recorded-media source, collapsed from its checked occurrences.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSourceFacts {
    /// Source declaration name used by routes and sends.
    pub name: String,
    /// Verified logical asset address supplied by the source declaration.
    pub asset: String,
    /// `musical-clip` or `fixed-media-cue`.
    pub kind: String,
    /// `crop`, `loop`, or `rate` for a musical clip.
    pub fit: Option<String>,
    /// Number of checked occurrences in this score.
    pub occurrences: usize,
}

/// Everything the Sound and Mix workspaces display.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
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
    /// Named recorded-media sources in first-occurrence order.
    pub media: Vec<MediaSourceFacts>,
}

impl StudioFacts {
    /// Derive the facts from a compiled studio and the score it belongs to.
    ///
    /// The score is needed for one thing only: the list of parts, so a part
    /// the studio never mentions still gets a row.
    pub(crate) fn derive(
        studio: &StudioExecution,
        spans: &musa_compiler::StudioSpans,
        score: &musa_score::ScoreSnapshot,
        parts: &[String],
    ) -> Self {
        let mut media: Vec<MediaSourceFacts> = Vec::new();
        for occurrence in score.annotations().media() {
            if let Some(existing) = media.iter_mut().find(|source| source.name == occurrence.name) {
                existing.occurrences = existing.occurrences.saturating_add(1);
                continue;
            }
            let (kind, fit) = match occurrence.kind {
                musa_score::score::MediaKind::MusicalClip(fit) => (
                    "musical-clip".to_owned(),
                    Some(
                        match fit {
                            musa_score::score::MediaFit::Crop => "crop",
                            musa_score::score::MediaFit::Loop => "loop",
                            musa_score::score::MediaFit::Rate => "rate",
                        }
                        .to_owned(),
                    ),
                ),
                musa_score::score::MediaKind::FixedMediaCue => ("fixed-media-cue".to_owned(), None),
            };
            media.push(MediaSourceFacts {
                name: occurrence.name.clone(),
                asset: occurrence.asset.clone(),
                kind,
                fit,
                occurrences: 1,
            });
        }
        Self {
            declared: studio.declared(),
            patches: containers(studio, spans, ContainerKind::Patch),
            buses: containers(studio, spans, ContainerKind::Bus),
            signals: containers(studio, spans, ContainerKind::Signal),
            assignments: parts
                .iter()
                .map(|part| {
                    let written = studio
                        .assignments()
                        .find(|assignment| assignment.part() == part)
                        .map(|assignment| assignment.instrument().to_owned());
                    AssignmentFacts {
                        part: part.clone(),
                        instrument: written.clone().unwrap_or_else(|| "std.sound.basic_sine@1".to_owned()),
                        profile: score
                            .profiles()
                            .name_for_part(part)
                            .unwrap_or("std.performance.neutral")
                            .to_owned(),
                        explicit: written.is_some(),
                        patch: written,
                    }
                })
                .collect(),
            sends: studio
                .sends()
                .map(|send| SendFacts {
                    source: send.source().to_owned(),
                    bus: send.bus().to_owned(),
                    decibels: Fraction::from_ratio(*send.level().magnitude()),
                    span: spans.resolve(send.level_anchor()).map(span),
                })
                .collect(),
            routes: studio
                .routes()
                .map(|route| RouteFacts {
                    source: route.source().to_owned(),
                    destination: route.destination().to_owned(),
                })
                .collect(),
            media,
        }
    }
}

fn containers(
    studio: &StudioExecution,
    spans: &musa_compiler::StudioSpans,
    kind: ContainerKind,
) -> Vec<ContainerFacts> {
    let declared: Vec<&StudioGraphProjection> = match kind {
        ContainerKind::Patch => studio.patches().collect(),
        ContainerKind::Bus => studio.buses().collect(),
        ContainerKind::Signal => studio.signals().collect(),
    };
    declared
        .into_iter()
        .map(|graph| ContainerFacts {
            kind,
            name: graph.name().to_owned(),
            stages: graph
                .nodes()
                .enumerate()
                .map(|(index, node)| {
                    let catalogue = crate::standard_studio_vocabulary()
                        .ok()
                        .and_then(|vocabulary| vocabulary.processor(node.processor()));
                    StageFacts {
                        index,
                        processor: node.processor().to_owned(),
                        summary: catalogue.map_or_else(String::new, |doc| doc.summary().to_owned()),
                        signature: catalogue.map_or_else(String::new, |doc| doc.signature().to_owned()),
                        origin: "bundled Musa source + registered primitive".to_owned(),
                        label: node.label().map(ToOwned::to_owned),
                        params: node
                            .parameters()
                            .enumerate()
                            .map(|(at, (name, _dsp_name, value))| {
                                let source_parameter = catalogue.and_then(|processor| processor.parameters().get(at));
                                let written_anchor = node.written_parameters().nth(at).flatten();
                                ParamFacts {
                                    name: name.to_owned(),
                                    summary: source_parameter
                                        .map_or_else(String::new, |parameter| parameter.summary().to_owned()),
                                    value: Fraction::from_ratio(*value.magnitude()),
                                    unit: value.unit().spelling().unwrap_or_default().to_owned(),
                                    minimum: Fraction::from_ratio(source_parameter.map_or_else(
                                        || *value.magnitude(),
                                        |parameter| *parameter.minimum().magnitude(),
                                    )),
                                    maximum: Fraction::from_ratio(source_parameter.map_or_else(
                                        || *value.magnitude(),
                                        |parameter| *parameter.maximum().magnitude(),
                                    )),
                                    written: written_anchor.is_some(),
                                    span: written_anchor.and_then(|anchor| spans.resolve(anchor)).map(span),
                                    modulated_by: modulator(studio, graph.name(), index, name),
                                }
                            })
                            .collect(),
                    }
                })
                .collect(),
        })
        .collect()
}

/// The signal modulating one parameter, if any. Modulation is addressed by
/// patch name, node, and parameter, which is exactly this triple.
fn modulator(studio: &StudioExecution, container: &str, node: usize, param: &str) -> Option<String> {
    studio
        .modulations()
        .find(|modulation| {
            modulation.instrument() == container && modulation.node() == node && modulation.parameter() == param
        })
        .map(|modulation| modulation.source().to_owned())
}

fn span(source: musa_score::SourceSpan) -> Span {
    Span {
        start: source.start,
        end: source.end,
    }
}

/// A structured studio edit (roadmap §11's `AssignPatch`, and the parameter
/// changes §14.4's workspaces make).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum StudioEdit {
    /// Choose or replace both halves of a part's musician-facing sound sentence.
    ChooseSound {
        /// The part, as the score declares it.
        part: String,
        /// The source instrument declaration or qualified standard name.
        instrument: String,
        /// The source performance-profile declaration or qualified standard name.
        profile: String,
    },
    /// Write the edition defaults explicitly for a part that currently inherits them.
    MakeSoundExplicit {
        /// The part, as the score declares it.
        part: String,
    },
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
        StudioEdit::ChooseSound {
            ref part,
            ref instrument,
            ref profile,
        } => format!("choosing {instrument} using {profile} for {part}"),
        StudioEdit::MakeSoundExplicit { ref part } => {
            format!("making {part}'s default sound explicit")
        }
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
pub(crate) fn edits_for(
    studio: &StudioExecution,
    spans: &musa_compiler::StudioSpans,
    source: &str,
    edit: &StudioEdit,
) -> Result<Vec<TextEdit>, ProjectError> {
    match *edit {
        StudioEdit::ChooseSound {
            ref part,
            ref instrument,
            ref profile,
        } => choose_sound(studio, source, part, instrument, profile),
        StudioEdit::MakeSoundExplicit { ref part } => make_sound_explicit(studio, source, part),
        StudioEdit::AssignPatch {
            ref part, ref patch, ..
        } => assign_patch(studio, spans, source, part, patch),
        StudioEdit::SetParam {
            kind,
            ref container,
            stage,
            ref param,
            value,
        } => set_param(studio, spans, source, kind, container, stage, param, value),
        StudioEdit::SetSendLevel {
            source: ref sender,
            ref bus,
            decibels,
        } => set_send_level(studio, spans, source, sender, bus, decibels),
    }
}

fn make_sound_explicit(studio: &StudioExecution, source: &str, part: &str) -> Result<Vec<TextEdit>, ProjectError> {
    let parsed = musa_syntax::parse(source);
    let root = parsed.syntax();
    let piece = musa_syntax::ast::PieceDecl::from_root(&root)
        .ok_or_else(|| missing("this document has no piece to choose a sound for"))?;
    let profile = piece
        .score()
        .into_iter()
        .flat_map(|score| score.parts())
        .find(|candidate| candidate.name().as_deref() == Some(part))
        .and_then(|written| written.profile())
        .and_then(|written| written.name())
        .unwrap_or_else(|| "std::performance::neutral".to_owned());
    choose_sound(studio, source, part, "std::sound::instrument::basic_sine", &profile)
}

fn choose_sound(
    studio: &StudioExecution,
    source: &str,
    part: &str,
    instrument: &str,
    profile: &str,
) -> Result<Vec<TextEdit>, ProjectError> {
    let is_default = matches!(
        instrument,
        "basic_sine" | "std::sound::basic_sine" | "std::sound::instrument::basic_sine"
    );
    if !is_default && !studio.patches().any(|candidate| candidate.name() == instrument) {
        return Err(missing(format!("there is no instrument named `{instrument}`")));
    }
    let parsed = musa_syntax::parse(source);
    let root = parsed.syntax();
    let piece = musa_syntax::ast::PieceDecl::from_root(&root)
        .ok_or_else(|| missing("this document has no piece to choose a sound for"))?;
    let written = piece
        .score()
        .into_iter()
        .flat_map(|score| score.parts())
        .find(|candidate| candidate.name().as_deref() == Some(part))
        .ok_or_else(|| missing(format!("there is no part named `{part}`")))?;
    if let Some(sound) = written.sound() {
        let range = sound.syntax().text_range();
        return Ok(vec![TextEdit::new(
            Span {
                start: u32::from(range.start()),
                end: u32::from(range.end()),
            },
            format!("sound {instrument} using {profile};"),
        )]);
    }
    if let Some(selected_profile) = written.profile() {
        let range = selected_profile.syntax().text_range();
        return Ok(vec![TextEdit::new(
            Span {
                start: u32::from(range.start()),
                end: u32::from(range.end()),
            },
            format!("sound {instrument} using {profile};"),
        )]);
    }
    let range = written.syntax().text_range();
    let start = usize::from(range.start());
    let end = usize::from(range.end());
    let part_source = source
        .get(start..end)
        .ok_or_else(|| missing("the part's source span is outside this document"))?;
    let brace = part_source
        .find('{')
        .ok_or_else(|| missing("this part block is unfinished"))?;
    let at = u32::try_from(start.saturating_add(brace).saturating_add(1)).unwrap_or(u32::MAX);
    Ok(vec![TextEdit::new(
        Span { start: at, end: at },
        format!("\n            sound {instrument} using {profile};"),
    )])
}

fn missing(what: impl Into<String>) -> ProjectError {
    ProjectError::Uneditable(what.into())
}

fn assign_patch(
    studio: &StudioExecution,
    spans: &musa_compiler::StudioSpans,
    source: &str,
    part: &str,
    patch: &str,
) -> Result<Vec<TextEdit>, ProjectError> {
    if !studio.patches().any(|candidate| candidate.name() == patch) {
        return Err(missing(format!("there is no patch named `{patch}`")));
    }
    match studio
        .assignments()
        .find(|assignment| assignment.part() == part)
        .and_then(|assignment| spans.resolve(assignment.anchor()))
    {
        // The part already has an assignment: replace the name it points at,
        // leaving the statement — and any comment on it — alone.
        Some(written) => Ok(vec![TextEdit::new(span(written), patch)]),
        None => {
            let block = spans
                .resolve(studio.anchor())
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
    studio: &StudioExecution,
    spans: &musa_compiler::StudioSpans,
    source: &str,
    kind: ContainerKind,
    container: &str,
    stage: usize,
    param: &str,
    value: f64,
) -> Result<Vec<TextEdit>, ProjectError> {
    let found = match kind {
        ContainerKind::Patch => studio.patches().find(|graph| graph.name() == container),
        ContainerKind::Bus => studio.buses().find(|graph| graph.name() == container),
        ContainerKind::Signal => studio.signals().find(|graph| graph.name() == container),
    };
    let found = found.ok_or_else(|| missing(format!("there is no `{container}` in the studio")))?;
    let node = found
        .nodes()
        .nth(stage)
        .ok_or_else(|| missing(format!("`{container}` has no stage {stage}")))?;
    let (at, (_, _, quantity)) = node
        .parameters()
        .enumerate()
        .find(|(_, (name, _, _))| *name == param)
        .ok_or_else(|| missing(format!("`{}` has no parameter `{param}`", node.processor())))?;
    let written = node.written_parameters().nth(at).flatten();

    match written.and_then(|anchor| spans.resolve(anchor)) {
        // Written: replace the literal in the scale it was written in, so a
        // patch that says `30 ms` keeps saying milliseconds.
        Some(written) => {
            let range = span(written);
            let text = slice(source, range).unwrap_or_default();
            Ok(vec![TextEdit::new(range, rewritten(text, quantity.unit(), value))])
        }
        // Never written: add it, in the unit it is declared in.
        None => {
            let call = spans
                .resolve(node.anchor())
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
            let literal = written_value(quantity.unit(), value);
            let at = call.start.saturating_add(u32::try_from(close).unwrap_or(0));
            Ok(vec![TextEdit::new(
                Span { start: at, end: at },
                format!("{separator}{param}: {literal}"),
            )])
        }
    }
}

fn set_send_level(
    studio: &StudioExecution,
    spans: &musa_compiler::StudioSpans,
    source: &str,
    sender: &str,
    bus: &str,
    decibels: f64,
) -> Result<Vec<TextEdit>, ProjectError> {
    let send = studio
        .sends()
        .find(|send| send.source() == sender && send.bus() == bus)
        .ok_or_else(|| missing(format!("there is no send from `{sender}` to `{bus}`")))?;
    let written = spans
        .resolve(send.level_anchor())
        .ok_or_else(|| missing("that send's level was not written in the source"))?;
    let range = span(written);
    let text = slice(source, range).unwrap_or_default();
    // A send written as a plain ratio keeps being one; the fader's decibels
    // convert back rather than rewriting the composer's choice of scale.
    let value = match send.level().unit() {
        SoundUnit::Decibels => decibels,
        SoundUnit::Hertz | SoundUnit::Linear | SoundUnit::Seconds => {
            if decibels.is_finite() {
                10f64.powf(decibels / 20.0)
            } else {
                0.0
            }
        }
    };
    Ok(vec![TextEdit::new(range, rewritten(text, send.level().unit(), value))])
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
fn rewritten(existing: &str, unit: SoundUnit, value: f64) -> String {
    if unit == SoundUnit::Seconds && existing.trim_end().ends_with("ms") {
        return format!("{} ms", number(value * 1000.0));
    }
    written_value(unit, value)
}

/// A literal in the unit's canonical spelling.
fn written_value(unit: SoundUnit, value: f64) -> String {
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
