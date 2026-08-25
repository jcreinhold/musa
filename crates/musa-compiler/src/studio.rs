//! Desugar the compatibility `studio` spelling into checked `std::sound` data.
//!
//! [`SurfaceStudio`] is a private CST-resolution record: it retains names,
//! topology, exact written literals, and edit spans long enough to print the
//! ordinary source constructors. It is neither a public semantic value nor a
//! DSP input; the checked source artifact below is the production handoff.

use crate::studio_model::{
    SurfaceAssignment, SurfaceGraph, SurfaceModulation, SurfaceNode, SurfaceNodeIndex, SurfaceProcessor,
    SurfaceQuantity, SurfaceRoute, SurfaceSend, SurfaceStudio, SurfaceUnit,
};
use indexmap::IndexMap;
use musa_score::origin::SourceSpan;
use musa_syntax::ast::{
    Arg, AstNode as _, BusDecl, CallExpr, InstrumentDecl, PatchDecl, RoomDecl, SendStmt, SignalChain, SignalStage,
    SoundStmt, StudioDecl, StudioItem,
};
use num_rational::Ratio;

use crate::resolve::{span_of, trimmed_span};
use musa_score::diagnose::{Code, Diagnostic};
use std::fmt::Write as _;

/// Read an exact decimal or ratio and its unit into a [`SurfaceQuantity`].
///
/// `ms` is folded into seconds here, which is why the spec carries no
/// millisecond unit: the dimension is time, and the suffix is a scale.
fn parse_value(number: &str, suffix: Option<&str>) -> Option<SurfaceQuantity> {
    let magnitude = match number.split_once('/') {
        Some((numerator, denominator)) => {
            let denominator = denominator.parse().ok()?;
            if denominator == 0 {
                return None;
            }
            Ratio::new(numerator.parse().ok()?, denominator)
        }
        None => musa_score::profile::parse_decimal(number)?,
    };
    let unit = match suffix {
        None => SurfaceUnit::Linear,
        Some("Hz") => SurfaceUnit::Hz,
        Some("dB") => SurfaceUnit::Decibels,
        Some("s") => SurfaceUnit::Seconds,
        Some("ms") => {
            let magnitude =
                musa_score::time::exact_arithmetic(magnitude, Ratio::from_integer(1000), musa_score::time::Exact::Div)?;
            return Some(SurfaceQuantity::new(magnitude, SurfaceUnit::Seconds));
        }
        Some(_) => return None,
    };
    Some(SurfaceQuantity::new(magnitude, unit))
}

/// What a library's `studio` may not write.
fn complain(node: &musa_syntax::SyntaxNode, what: &str, diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.push(
        Diagnostic::error(
            Code::Misplaced,
            format!("a library's `studio` declares patches and signals; `{what}` belongs to the piece"),
        )
        .at(trimmed_span(node), "not allowed in a library"),
    );
}

/// Resolve a `studio` block into a [`SurfaceStudio`], reporting every unresolved
/// name and mis-united value against `diagnostics`.
///
/// `parts` is the set of part names the score declared: `assign` is the one
/// place the two layers meet, so it is the one place a studio name is checked
/// against a score name.
/// `imported` holds the `studio` block of each library the piece imports. A
/// library ships building blocks — patches and signals — and nothing that
/// wires them to a particular score, because it does not know the score;
/// anything else it writes is reported rather than silently applied.
pub(crate) fn resolve(
    decl: Option<&StudioDecl>,
    imported: &[StudioDecl],
    imported_instruments: &[InstrumentDecl],
    instruments: &[InstrumentDecl],
    sounds: &[(String, SoundStmt)],
    parts: &[String],
    references: &mut crate::resolve::ReferenceIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> SurfaceStudio {
    let mut spec = SurfaceStudio::default();
    spec.set_span(decl.map(|decl| span_of(decl.syntax())));
    let mut items: Vec<StudioItem> = Vec::new();
    for library in imported {
        for item in library.items() {
            match item {
                StudioItem::Patch(_) | StudioItem::Signal(_) => items.push(item),
                StudioItem::Room(ref node) => complain(node.syntax(), "room", diagnostics),
                StudioItem::Bus(ref node) => complain(node.syntax(), "bus", diagnostics),
                StudioItem::Modulate(ref node) => complain(node.syntax(), "modulate", diagnostics),
                StudioItem::Assign(ref node) => complain(node.syntax(), "assign", diagnostics),
                StudioItem::Route(ref node) => complain(node.syntax(), "route", diagnostics),
                StudioItem::Send(ref node) => complain(node.syntax(), "send", diagnostics),
            }
        }
    }
    // The compiled document's own items begin here; everything before is a
    // library's, whose spans name text the reference record does not cover.
    let main_start = items.len();
    items.extend(decl.map(StudioDecl::items).unwrap_or_default());

    for instrument in imported_instruments {
        declare_instrument(instrument, &mut spec, diagnostics);
    }
    for instrument in instruments {
        declare_instrument(instrument, &mut spec, diagnostics);
        let name = instrument.name().unwrap_or_default();
        if !name.is_empty()
            && spec.has_patch(&name)
            && let Some(span) = instrument.name_token().as_ref().map(crate::resolve::source_span_of)
        {
            references.declare(crate::resolve::NameKind::Patch, &name, span);
        }
    }

    // Two passes: patches, buses, and signals first, so the bindings that
    // follow can be checked against them regardless of writing order. A
    // studio reads top-down, but it does not have to be written that way.
    for (index, item) in items.iter().enumerate() {
        match item {
            StudioItem::Patch(patch) => {
                declare_patch(patch, &mut spec, diagnostics);
                if index >= main_start {
                    deprecate_patch(patch, diagnostics);
                }
            }
            StudioItem::Bus(bus) => declare_bus(bus, &mut spec, diagnostics),
            StudioItem::Room(room) => declare_room(room, &mut spec, diagnostics),
            StudioItem::Signal(signal) => {
                let name = signal.name().unwrap_or_default();
                let mut built = SurfaceGraph::default();
                let Some(chain) = signal.chain() else { continue };
                if lower_chain(&chain, &mut built, diagnostics).is_some() && !spec.insert_signal(name.clone(), built) {
                    diagnostics.push(
                        Diagnostic::error(Code::DuplicateName, format!("duplicate signal `{name}`"))
                            .at(trimmed_span(signal.syntax()), "declared again here"),
                    );
                }
            }
            StudioItem::Modulate(_) | StudioItem::Assign(_) | StudioItem::Route(_) | StudioItem::Send(_) => {}
        }
        // A patch that resolved is a declaration the record keeps — but only
        // the document's own; a library's patch is spelled in its own file.
        if index >= main_start
            && let StudioItem::Patch(patch) = item
        {
            let name = patch.name().unwrap_or_default();
            if !name.is_empty()
                && spec.has_patch(&name)
                && let Some(span) = crate::resolve::token_span(patch.syntax(), musa_syntax::SyntaxKind::Identifier)
            {
                references.declare(crate::resolve::NameKind::Patch, &name, span);
            }
        }
    }

    for (part, sound) in sounds {
        let instrument = sound.instrument().unwrap_or_default();
        let span = Some(span_of(sound.syntax()));
        if !parts.contains(part) {
            diagnostics.push(
                Diagnostic::error(Code::UnknownName, format!("unknown part `{part}`"))
                    .maybe_at(span, "no part with this name"),
            );
            continue;
        }
        let is_default = matches!(
            instrument.as_str(),
            "basic_sine" | "std::sound::basic_sine" | "std::sound::instrument::basic_sine"
        );
        if !is_default && !spec.has_patch(&instrument) {
            diagnostics.push(
                Diagnostic::error(Code::UnknownName, format!("missing sound `{instrument}`"))
                    .maybe_at(span, "no instrument declaration with this name")
                    .help("declare the instrument, import the package that owns it, or choose `std::sound::instrument::basic_sine`"),
            );
            continue;
        }
        let patch_span = sound
            .instrument_token()
            .map(|token| crate::resolve::source_span_of(&token));
        if !is_default && let Some(span) = patch_span {
            references.record_use(crate::resolve::NameKind::Patch, &instrument, span);
        }
        let selected = if is_default {
            "std.sound.basic_sine@1".to_owned()
        } else {
            instrument
        };
        spec.assign(
            part.clone(),
            SurfaceAssignment {
                patch: selected,
                patch_span,
            },
        );
        spec.push_route(SurfaceRoute {
            source: part.clone(),
            destination: "master".to_owned(),
            span,
        });
    }

    for item in &items {
        match item {
            StudioItem::Assign(assign) => {
                let (Some(part), Some(patch)) = (assign.source(), assign.destination()) else {
                    continue;
                };
                let span = Some(span_of(assign.syntax()));
                if !parts.contains(&part) {
                    diagnostics.push(
                        Diagnostic::error(Code::UnknownName, format!("unknown part `{part}`"))
                            .maybe_at(span, "no part with this name"),
                    );
                } else if !spec.has_patch(&patch) {
                    diagnostics.push(
                        Diagnostic::error(Code::UnknownName, format!("unknown patch `{patch}`"))
                            .maybe_at(span, "no patch with this name"),
                    );
                } else {
                    let patch_span = assign
                        .destination_token()
                        .map(|token| crate::resolve::source_span_of(&token));
                    // Both names resolved: the assign is a use of each.
                    if let Some(span) = assign
                        .source_token()
                        .map(|token| crate::resolve::source_span_of(&token))
                    {
                        references.record_use(crate::resolve::NameKind::Part, &part, span);
                    }
                    if let Some(span) = patch_span {
                        references.record_use(crate::resolve::NameKind::Patch, &patch, span);
                    }
                    if spec.assignment(&part).is_some() {
                        diagnostics.push(
                            Diagnostic::error(Code::DuplicateName, format!("part `{part}` already has an instrument"))
                                .maybe_at(span, "assigned again here"),
                        );
                    } else {
                        spec.assign(part, SurfaceAssignment { patch, patch_span });
                    }
                }
            }
            StudioItem::Route(route) => {
                let (Some(source), Some(destination)) = (route.source(), route.destination()) else {
                    continue;
                };
                let span = Some(span_of(route.syntax()));
                if !spec.is_routable(&source) {
                    diagnostics.push(
                        Diagnostic::error(
                            Code::UnknownName,
                            format!("`{source}` is not an assigned part or a bus"),
                        )
                        .maybe_at(span, "nothing sends from here"),
                    );
                } else if destination != "master" && !spec.has_bus(&destination) {
                    diagnostics.push(
                        Diagnostic::error(Code::UnknownName, format!("unknown destination `{destination}`"))
                            .maybe_at(span, "not a bus or `master`"),
                    );
                } else {
                    if parts.contains(&source)
                        && let Some(span) = route.source_token().map(|token| crate::resolve::source_span_of(&token))
                    {
                        references.record_use(crate::resolve::NameKind::Part, &source, span);
                    }
                    spec.push_route(SurfaceRoute {
                        source,
                        destination,
                        span,
                    });
                }
            }
            StudioItem::Send(send) => resolve_send(send, &mut spec, parts, references, diagnostics),
            StudioItem::Modulate(modulate) => {
                let Some(source) = modulate.source() else { continue };
                let span = Some(span_of(modulate.syntax()));
                if !spec.has_signal(&source) {
                    diagnostics.push(
                        Diagnostic::error(Code::UnknownName, format!("unknown signal `{source}`"))
                            .maybe_at(span, "not declared in this studio"),
                    );
                    continue;
                }
                if let Some(modulation) = resolve_target(&source, &modulate.target(), &spec, span, diagnostics) {
                    spec.push_modulation(modulation);
                }
            }
            StudioItem::Patch(_) | StudioItem::Bus(_) | StudioItem::Room(_) | StudioItem::Signal(_) => {}
        }
    }
    diagnose_bus_cycles(&spec, diagnostics);
    spec
}

/// Elaborate the resolved compatibility surface into the ordinary
/// `std::sound::studio` value consumed by production preparation.
pub(crate) fn checked_source(
    studio: &SurfaceStudio,
    declared: bool,
) -> Result<
    (
        musa_calculus::CheckedSource,
        Vec<Option<musa_score::origin::SourceSpan>>,
    ),
    Vec<Diagnostic>,
> {
    let mut anchors = Anchors::default();
    let mut body = String::from("import std::sound::production;\n\nlet compiled_studio: StudioExecutionArtifact = ");
    write_studio(&mut body, studio, declared, &mut anchors);
    body.push_str(";\n");
    let checked = crate::checked_source_value(
        &crate::SourceDocument::new(body, "musa-generated:/studio-cutover.musa"),
        &crate::CompileOptions::default(),
        "compiled_studio",
        &musa_calculus::SourceSchema::new(
            "std.sound.production.StudioExecutionArtifact",
            "StudioExecutionArtifact",
            1,
        ),
    )?;
    Ok((checked, anchors.spans))
}

#[derive(Default)]
struct Anchors {
    spans: Vec<Option<SourceSpan>>,
}

impl Anchors {
    fn push(&mut self, span: Option<SourceSpan>) -> u64 {
        let anchor = u64::try_from(self.spans.len()).unwrap_or(u64::MAX);
        self.spans.push(span);
        anchor
    }
}

fn write_studio(out: &mut String, studio: &SurfaceStudio, declared: bool, anchors: &mut Anchors) {
    let root = anchors.push(studio.span());
    let _ = write!(
        out,
        "StudioExecutionArtifact {{ schema_version = 1, anchor = {root}, declared = {}, patches = ",
        if declared { "true" } else { "false" }
    );
    write_graphs(out, studio.patches(), anchors);
    out.push_str(", buses = ");
    write_graphs(out, studio.buses(), anchors);
    out.push_str(", signals = ");
    write_graphs(out, studio.signals(), anchors);
    out.push_str(", assignments = [");
    for (index, (part, instrument)) in studio.assignments().enumerate() {
        separator(out, index);
        let anchor = anchors.push(studio.assignment(part).and_then(|assignment| assignment.patch_span));
        let _ = write!(
            out,
            "StudioAssignment {{ anchor = {anchor}, part_name = {}, instrument = {} }}",
            quoted(part),
            quoted(instrument)
        );
    }
    out.push_str("], routes = [");
    for (index, route) in studio.routes().iter().enumerate() {
        separator(out, index);
        let anchor = anchors.push(route.span);
        let _ = write!(
            out,
            "StudioRoute {{ anchor = {anchor}, source = {}, destination = {} }}",
            quoted(&route.source),
            quoted(&route.destination)
        );
    }
    out.push_str("], sends = [");
    for (index, send) in studio.sends().iter().enumerate() {
        separator(out, index);
        let anchor = anchors.push(send.span);
        let level_anchor = anchors.push(send.level_span);
        let _ = write!(
            out,
            "StudioSend {{ anchor = {anchor}, level_anchor = {level_anchor}, source = {}, destination_bus = {}, level = {} }}",
            quoted(&send.source),
            quoted(&send.bus),
            quantity(send.level)
        );
    }
    out.push_str("], modulations = [");
    for (index, modulation) in studio.modulations().iter().enumerate() {
        separator(out, index);
        let anchor = anchors.push(None);
        let _ = write!(
            out,
            "StudioModulation {{ anchor = {anchor}, source = {}, instrument = {}, node = {}, parameter = {} }}",
            quoted(&modulation.source),
            quoted(&modulation.patch),
            modulation.node,
            quoted(modulation.param)
        );
    }
    out.push_str("] }");
}

fn write_graphs<'a>(
    out: &mut String,
    graphs: impl Iterator<Item = (&'a str, &'a SurfaceGraph)>,
    anchors: &mut Anchors,
) {
    out.push('[');
    for (index, (name, graph)) in graphs.enumerate() {
        separator(out, index);
        let anchor = anchors.push(None);
        let _ = write!(
            out,
            "StudioGraph {{ anchor = {anchor}, name = {}, nodes = [",
            quoted(name)
        );
        for (node_index, node) in graph.nodes().iter().enumerate() {
            separator(out, node_index);
            let anchor = anchors.push(node.span);
            let label = node
                .label
                .as_deref()
                .map_or_else(|| "None".to_owned(), |name| format!("Some({})", quoted(name)));
            let _ = write!(
                out,
                "StudioNode {{ anchor = {anchor}, label = {label}, processor = {}, written_parameters = [",
                processor(node)
            );
            for (parameter_index, value) in node.params.iter().enumerate() {
                separator(out, parameter_index);
                match value {
                    Some(_) => {
                        let anchor = anchors.push(node.param_spans.get(parameter_index).copied().flatten());
                        let _ = write!(out, "Some({anchor})");
                    }
                    None => out.push_str("None"),
                }
            }
            out.push_str("], inputs = [");
            for (input_index, input) in node.inputs.iter().enumerate() {
                separator(out, input_index);
                let _ = write!(out, "{input}");
            }
            out.push_str("] }");
        }
        let _ = write!(out, "], output_node = {} }}", graph.output().unwrap_or(0));
    }
    out.push(']');
}

fn processor(node: &SurfaceNode) -> String {
    let arguments = node
        .params
        .iter()
        .map(|value| value.map_or_else(|| "None".to_owned(), |value| format!("Some({})", quantity(value))))
        .collect::<Vec<_>>()
        .join(", ");
    match node.processor {
        SurfaceProcessor::Mix => "studio_mix()".to_owned(),
        SurfaceProcessor::Oscillator => format!("studio_oscillator({arguments})"),
        SurfaceProcessor::Gain => format!("studio_gain({arguments})"),
        SurfaceProcessor::Envelope => format!("studio_envelope({arguments})"),
        SurfaceProcessor::Lowpass => format!("studio_lowpass({arguments})"),
        SurfaceProcessor::Highpass => format!("studio_highpass({arguments})"),
        SurfaceProcessor::Reverb => format!("studio_reverb({arguments})"),
        SurfaceProcessor::Delay => format!("studio_delay({arguments})"),
        SurfaceProcessor::Chorus => format!("studio_chorus({arguments})"),
        SurfaceProcessor::Scale => format!("studio_scale({arguments})"),
        SurfaceProcessor::Bias => format!("studio_bias({arguments})"),
        SurfaceProcessor::Clamp => format!("studio_clamp({arguments})"),
        SurfaceProcessor::Smoothing => format!("studio_smoothing({arguments})"),
    }
}

fn quantity(value: SurfaceQuantity) -> String {
    let magnitude = source_ratio(value.magnitude);
    match value.unit {
        SurfaceUnit::Hz => format!("Written(Frequency, {magnitude}, Hertz)"),
        SurfaceUnit::Linear => format!("Written(LinearAmplitude, {magnitude}, Linear)"),
        SurfaceUnit::Decibels => format!("Written(Level, {magnitude}, Decibels)"),
        SurfaceUnit::Seconds => format!("Written(Time, {magnitude}, Seconds)"),
    }
}

fn source_ratio(value: Ratio<i64>) -> String {
    if *value.numer() < 0 {
        format!("0/1 - {}/{}", value.numer().unsigned_abs(), value.denom())
    } else {
        format!("{}/{}", value.numer(), value.denom())
    }
}

fn quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len().saturating_add(2));
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn separator(out: &mut String, index: usize) {
    if index > 0 {
        out.push_str(", ");
    }
}

fn diagnose_bus_cycles(spec: &SurfaceStudio, diagnostics: &mut Vec<Diagnostic>) {
    let edges = spec
        .sends()
        .iter()
        .filter(|send| spec.has_bus(&send.source))
        .map(|send| (send.source.as_str(), send.bus.as_str(), send.span))
        .chain(
            spec.routes()
                .iter()
                .filter(|route| route.destination != "master" && spec.has_bus(&route.source))
                .map(|route| (route.source.as_str(), route.destination.as_str(), route.span)),
        )
        .collect::<Vec<_>>();
    for (source, destination, span) in &edges {
        let mut frontier = vec![*destination];
        let mut seen = std::collections::HashSet::new();
        let mut cyclic = source == destination;
        while !cyclic {
            let Some(bus) = frontier.pop() else { break };
            if !seen.insert(bus) {
                continue;
            }
            for (_, next, _) in edges.iter().filter(|(from, _, _)| *from == bus) {
                if next == source {
                    cyclic = true;
                    break;
                }
                frontier.push(*next);
            }
        }
        if cyclic {
            diagnostics.push(
                Diagnostic::error(
                    Code::DependencyCycle,
                    format!("bus binding `{source} -> {destination}` participates in a cycle"),
                )
                .maybe_at(*span, "the room signal would depend on itself"),
            );
        }
    }
}

fn resolve_send(
    send: &SendStmt,
    spec: &mut SurfaceStudio,
    parts: &[String],
    references: &mut crate::resolve::ReferenceIndex,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (Some(source), Some(bus)) = (send.source(), send.destination()) else {
        return;
    };
    let span = Some(span_of(send.syntax()));
    if !spec.is_routable(&source) {
        diagnostics.push(
            Diagnostic::error(
                Code::UnknownName,
                format!("`{source}` is not an assigned part or a bus"),
            )
            .maybe_at(span, "nothing sends from here"),
        );
        return;
    }
    if !spec.has_bus(&bus) {
        diagnostics.push(
            Diagnostic::error(Code::UnknownName, format!("unknown bus `{bus}`"))
                .maybe_at(span, "no bus with this name"),
        );
        return;
    }
    // The source is routable and names a part: a use of it, recorded.
    if parts.contains(&source)
        && let Some(span) = send.source_token().map(|token| crate::resolve::source_span_of(&token))
    {
        references.record_use(crate::resolve::NameKind::Part, &source, span);
    }
    let level = send
        .level()
        .and_then(|literal| parse_value(&literal.number()?, literal.unit().as_deref()));
    let Some(level) = level else {
        diagnostics.push(
            Diagnostic::error(Code::NotAValue, "a send level must be a number").maybe_at(span, "expected a number"),
        );
        return;
    };
    if level.unit != SurfaceUnit::Decibels {
        diagnostics
            .push(Diagnostic::error(Code::NotAValue, "a send level is written in `dB`").maybe_at(span, "missing `dB`"));
        return;
    }
    let level_span = send.level().map(|literal| trimmed_span(literal.syntax()));
    spec.push_send(SurfaceSend {
        source,
        bus,
        level,
        level_span,
        span,
    });
}

/// `glass_pad.lowpass.cutoff` → the node and parameter it names.
///
/// A stage is addressed by its binding name when it has one and by its
/// processor name otherwise. Two unnamed `lowpass` stages in one patch are
/// therefore ambiguous, and saying so is better than silently picking one.
fn resolve_target(
    source: &str,
    path: &[String],
    spec: &SurfaceStudio,
    span: Option<musa_score::origin::SourceSpan>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<SurfaceModulation> {
    let written = path.join(".");
    let [patch_name, stage, param] = path else {
        diagnostics.push(
            Diagnostic::error(
                Code::NotAValue,
                format!("`{written}` is not a `<patch>.<stage>.<parameter>` path"),
            )
            .maybe_at(span, "expected `<patch>.<stage>.<parameter>`"),
        );
        return None;
    };
    let Some(patch) = spec.patch(patch_name) else {
        diagnostics.push(
            Diagnostic::error(Code::UnknownName, format!("unknown patch `{patch_name}`"))
                .maybe_at(span, "no patch with this name"),
        );
        return None;
    };
    let matches: Vec<SurfaceNodeIndex> = patch
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, node)| node.label.as_deref() == Some(stage.as_str()) || node.processor.name() == stage)
        .map(|(index, _)| index)
        .collect();
    let [node] = matches.as_slice() else {
        let (message, label, help) = if matches.is_empty() {
            (
                format!("`{patch_name}` has no stage named `{stage}`"),
                "unknown stage",
                "name a stage in the patch's chain, or give this one a name with `<name> = …`",
            )
        } else {
            (
                format!("`{patch_name}` has more than one `{stage}`"),
                "which one?",
                "give the stage a name — `warm = lowpass(…)` — and address it by that",
            )
        };
        diagnostics.push(
            Diagnostic::error(Code::UnknownName, message)
                .maybe_at(span, label)
                .help(help),
        );
        return None;
    };
    let processor = patch.nodes().get(*node)?.processor;
    let Some(declared) = processor.param(param) else {
        diagnostics.push(
            Diagnostic::error(Code::UnknownName, format!("`{stage}` has no parameter `{param}`"))
                .maybe_at(span, "unknown parameter"),
        );
        return None;
    };
    Some(SurfaceModulation {
        source: source.to_owned(),
        patch: patch_name.clone(),
        node: *node,
        param: declared.name,
    })
}

fn declare_patch(decl: &PatchDecl, spec: &mut SurfaceStudio, diagnostics: &mut Vec<Diagnostic>) {
    let name = decl.name().unwrap_or_default();
    let Some(patch) = build_container(&decl.signals(), &decl.chains(), diagnostics) else {
        return;
    };
    if patch.output().is_none() {
        diagnostics.push(
            Diagnostic::error(Code::Studio, format!("patch `{name}` never reaches `output`"))
                .at(trimmed_span(decl.syntax()), "this chain ends nowhere"),
        );
        return;
    }
    if !spec.insert_patch(name.clone(), patch) {
        diagnostics.push(
            Diagnostic::error(Code::DuplicateName, format!("duplicate patch `{name}`"))
                .at(trimmed_span(decl.syntax()), "declared again here"),
        );
    }
}

fn declare_instrument(decl: &InstrumentDecl, spec: &mut SurfaceStudio, diagnostics: &mut Vec<Diagnostic>) {
    let name = decl.name().unwrap_or_default();
    let signature = decl.signature().unwrap_or_default();
    if !matches!(
        signature.as_str(),
        "note_instrument" | "std::sound::note_instrument" | "std::sound::instrument::note_instrument"
    ) {
        diagnostics.push(
            Diagnostic::error(
                Code::UnsupportedLanguageStage,
                format!("instrument `{name}` names a custom source signature that preparation cannot project yet"),
            )
            .at(trimmed_span(decl.syntax()), "the declaration is valid, but its contract projection is pending")
            .help("use the edition `std::sound::instrument::note_instrument` contract until custom signature projection lands"),
        );
        return;
    }
    let Some(implementation) = decl.implementation() else {
        if decl.asset().is_some() {
            diagnostics.push(
                Diagnostic::error(
                    Code::UnsupportedLanguageStage,
                    format!("instrument `{name}` names an asset adapter that is not installed yet"),
                )
                .at(
                    trimmed_span(decl.syntax()),
                    "the declaration is valid, but its adapter is pending",
                )
                .help("use an `implementation graph` instrument until the sample-adapter prompt lands"),
            );
        } else {
            diagnostics.push(
                Diagnostic::error(Code::Studio, format!("instrument `{name}` has no implementation"))
                    .at(trimmed_span(decl.syntax()), "a source contract alone produces no sound")
                    .help("supply `implementation graph { ... }` or an immutable asset adapter"),
            );
        }
        return;
    };
    let Some(graph) = build_container(&implementation.signals(), &implementation.chains(), diagnostics) else {
        return;
    };
    if graph.output().is_none() {
        diagnostics.push(
            Diagnostic::error(Code::Studio, format!("instrument `{name}` never reaches `output`"))
                .at(trimmed_span(decl.syntax()), "this implementation ends nowhere"),
        );
        return;
    }
    if !spec.insert_patch(name.clone(), graph) {
        diagnostics.push(
            Diagnostic::error(Code::DuplicateName, format!("duplicate instrument `{name}`"))
                .at(trimmed_span(decl.syntax()), "declared again here"),
        );
    }
}

fn deprecate_patch(decl: &PatchDecl, diagnostics: &mut Vec<Diagnostic>) {
    let written = decl.syntax().text().to_string();
    let Some(body) = written.find('{').and_then(|at| written.get(at..)) else {
        return;
    };
    let name = decl.name().unwrap_or_default();
    let replacement = format!("instrument {name} conforms note_instrument {{ implementation graph {body} }}");
    let span = trimmed_span(decl.syntax());
    diagnostics.push(
        Diagnostic::warning(Code::Syntax, "`patch` is the previous edition's instrument spelling")
            .at(span, "accepted during the compatibility window")
            .help("write an instrument declaration with a private graph implementation")
            .fix("convert this patch to an instrument", span, replacement),
    );
}

fn declare_bus(decl: &BusDecl, spec: &mut SurfaceStudio, diagnostics: &mut Vec<Diagnostic>) {
    let name = decl.name().unwrap_or_default();
    // A bus's input is whatever is sent to it, so its chain needs no
    // `output` terminal: the last stage *is* the output.
    let Some(mut bus) = build_container(&decl.signals(), &decl.chains(), diagnostics) else {
        return;
    };
    if bus.output().is_none() {
        let last = bus.nodes().len().checked_sub(1);
        match last {
            Some(index) => bus.set_output(index),
            None => {
                diagnostics.push(
                    Diagnostic::error(Code::Studio, format!("bus `{name}` is empty"))
                        .at(trimmed_span(decl.syntax()), "no processors"),
                );
                return;
            }
        }
    }
    if !spec.insert_bus(name.clone(), bus) {
        diagnostics.push(
            Diagnostic::error(Code::DuplicateName, format!("duplicate bus `{name}`"))
                .at(trimmed_span(decl.syntax()), "declared again here"),
        );
    }
}

fn declare_room(decl: &RoomDecl, spec: &mut SurfaceStudio, diagnostics: &mut Vec<Diagnostic>) {
    let name = decl.name().unwrap_or_default();
    let Some(mut room) = build_container(&decl.signals(), &decl.chains(), diagnostics) else {
        return;
    };
    if room.output().is_none() {
        let last = room.nodes().len().checked_sub(1);
        match last {
            Some(index) => room.set_output(index),
            None => {
                diagnostics.push(
                    Diagnostic::error(Code::Studio, format!("room `{name}` has no ambience effect"))
                        .at(trimmed_span(decl.syntax()), "empty room"),
                );
                return;
            }
        }
    }
    if !spec.insert_bus(name.clone(), room) {
        diagnostics.push(
            Diagnostic::error(Code::DuplicateName, format!("duplicate room or bus `{name}`"))
                .at(trimmed_span(decl.syntax()), "declared again here"),
        );
    }
}

/// The shared body of a patch or a bus: named signals, then chains, with the
/// names visible to the chains that follow them.
fn build_container(
    signals: &[musa_syntax::ast::SignalBinding],
    chains: &[musa_syntax::ast::ChainStmt],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<SurfaceGraph> {
    let mut patch = SurfaceGraph::default();
    let mut locals: IndexMap<String, SurfaceNodeIndex> = IndexMap::new();
    for binding in signals {
        let Some(chain) = binding.chain() else { continue };
        let name = binding.name().unwrap_or_default();
        if let Some(node) = lower_chain_into(&chain, &mut patch, &locals, diagnostics) {
            if let Some(entry) = patch.nodes_mut().get_mut(node) {
                entry.label = Some(name.clone());
            }
            locals.insert(name, node);
        }
    }
    for statement in chains {
        let Some(chain) = statement.chain() else { continue };
        lower_chain_into(&chain, &mut patch, &locals, diagnostics);
    }
    Some(patch)
}

/// A top-level signal chain, which has no enclosing patch's local names.
fn lower_chain(
    chain: &SignalChain,
    patch: &mut SurfaceGraph,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<SurfaceNodeIndex> {
    let locals = IndexMap::new();
    let node = lower_chain_into(chain, patch, &locals, diagnostics)?;
    patch.set_output(node);
    Some(node)
}

/// Flatten `a |> b |> c` into nodes: each stage takes the previous stage's
/// node as its input, and `output` marks rather than adds one.
fn lower_chain_into(
    chain: &SignalChain,
    patch: &mut SurfaceGraph,
    locals: &IndexMap<String, SurfaceNodeIndex>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<SurfaceNodeIndex> {
    let mut previous: Option<SurfaceNodeIndex> = None;
    for stage in chain.stages() {
        match &stage {
            SignalStage::Name(name) => {
                let text = name.text().unwrap_or_default();
                if text == "output" {
                    match previous {
                        Some(node) => patch.set_output(node),
                        None => diagnostics.push(
                            Diagnostic::error(Code::Studio, "`output` needs a signal before it")
                                .at(trimmed_span(stage.syntax()), "nothing reaches it"),
                        ),
                    }
                    continue;
                }
                match locals.get(&text) {
                    Some(node) => previous = Some(*node),
                    None => {
                        diagnostics.push(
                            Diagnostic::error(Code::UnknownName, format!("unknown signal `{text}`"))
                                .at(trimmed_span(stage.syntax()), "not declared in this studio"),
                        );
                        return None;
                    }
                }
            }
            SignalStage::Call(call) => {
                previous = Some(lower_call(call, previous, patch, locals, diagnostics)?);
            }
            SignalStage::Literal(_) => {
                diagnostics.push(
                    Diagnostic::error(Code::NotAValue, "a number is not a signal")
                        .at(trimmed_span(stage.syntax()), "expected a signal"),
                );
                return None;
            }
        }
    }
    previous
}

/// One `name(args)` construction, with its upstream stage already lowered.
fn lower_call(
    call: &CallExpr,
    upstream: Option<SurfaceNodeIndex>,
    patch: &mut SurfaceGraph,
    locals: &IndexMap<String, SurfaceNodeIndex>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<SurfaceNodeIndex> {
    let span = Some(span_of(call.syntax()));
    let written = call.callee().unwrap_or_default();
    let Some(processor) = SurfaceProcessor::from_name(&written) else {
        diagnostics.push(
            Diagnostic::error(Code::UnknownWord, format!("unknown processor `{written}`"))
                .maybe_at(span, "musa has no processor by this name"),
        );
        return None;
    };
    let mut params: Vec<Option<SurfaceQuantity>> = vec![None; processor.params().len()];
    let mut param_spans: Vec<Option<SourceSpan>> = vec![None; params.len()];
    let mut inputs: Vec<SurfaceNodeIndex> = upstream.into_iter().collect();
    let mut positional = 0usize;

    for arg in call.args() {
        match arg.value() {
            // `envelope(adsr(...))`: the inner construction's arguments are
            // the outer processor's, so it flattens rather than nesting. The
            // shape exists for readability, not for a second node.
            Some(SignalStage::Call(inner)) if is_argument_group(&inner) => {
                for nested in inner.args() {
                    bind_argument(
                        &nested,
                        processor,
                        &mut params,
                        &mut param_spans,
                        &mut positional,
                        diagnostics,
                    );
                }
            }
            Some(SignalStage::Call(inner)) => {
                if let Some(node) = lower_call(&inner, None, patch, locals, diagnostics) {
                    inputs.push(node);
                }
            }
            Some(SignalStage::Name(name)) => {
                let text = name.text().unwrap_or_default();
                match locals.get(&text) {
                    Some(node) => inputs.push(*node),
                    // A bare word that names no signal is a mode selector
                    // (`oscillator(sine)`), which today has one legal value.
                    None if text == "sine" => {}
                    None => diagnostics.push(
                        Diagnostic::error(Code::UnknownName, format!("unknown signal `{text}`"))
                            .at(trimmed_span(name.syntax()), "not declared in this studio"),
                    ),
                }
            }
            Some(SignalStage::Literal(_)) | None => {
                bind_argument(
                    &arg,
                    processor,
                    &mut params,
                    &mut param_spans,
                    &mut positional,
                    diagnostics,
                );
            }
        }
    }

    Some(patch.push(SurfaceNode {
        processor,
        label: None,
        params,
        param_spans,
        span,
        inputs,
    }))
}

/// Whether a construction is an argument group (`adsr(...)`) rather than a
/// processor. Argument groups exist to make a long parameter list readable.
fn is_argument_group(call: &CallExpr) -> bool {
    call.callee().as_deref() == Some("adsr")
}

/// Bind one written argument to a declared parameter, checking its unit.
fn bind_argument(
    arg: &Arg,
    processor: SurfaceProcessor,
    params: &mut [Option<SurfaceQuantity>],
    spans: &mut [Option<SourceSpan>],
    positional: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let span = Some(span_of(arg.syntax()));
    let index = match arg.name() {
        Some(name) => {
            let Some(found) = processor.params().iter().position(|param| param.name == name) else {
                if let Some(replacement) = processor
                    .params()
                    .iter()
                    .find(|param| param.former_names.contains(&name.as_str()))
                {
                    let mut diagnostic = Diagnostic::error(
                        Code::UnknownName,
                        format!(
                            "`{name}` is no longer a parameter of `{}`; write `{}`",
                            processor.name(),
                            replacement.name
                        ),
                    )
                    .maybe_at(span, "removed parameter spelling")
                    .note(replacement.summary);
                    if let Some(token) = arg.name_token() {
                        diagnostic = diagnostic.fix(
                            format!("replace `{name}` with `{}`", replacement.name),
                            crate::resolve::source_span_of(&token),
                            replacement.name,
                        );
                    }
                    diagnostics.push(diagnostic);
                    return;
                }
                diagnostics.push(
                    Diagnostic::error(
                        Code::UnknownName,
                        format!("`{}` has no parameter `{name}`", processor.name()),
                    )
                    .maybe_at(span, "unknown parameter"),
                );
                return;
            };
            found
        }
        None => {
            let index = *positional;
            *positional = positional.saturating_add(1);
            if index >= processor.params().len() {
                diagnostics.push(
                    Diagnostic::error(
                        Code::NotAValue,
                        format!("`{}` takes no argument in that position", processor.name()),
                    )
                    .maybe_at(span, "too many arguments"),
                );
                return;
            }
            index
        }
    };
    let Some(declared) = processor.params().get(index).copied() else {
        return;
    };
    let Some(SignalStage::Literal(literal)) = arg.value() else {
        diagnostics.push(
            Diagnostic::error(Code::NotAValue, format!("`{}` must be a number", declared.name))
                .maybe_at(span, "expected a number"),
        );
        return;
    };
    let Some(number) = literal.number() else { return };
    let Some(value) = parse_value(&number, literal.unit().as_deref()) else {
        diagnostics.push(
            Diagnostic::error(Code::NotAValue, format!("`{number}` is not a number"))
                .maybe_at(span, "expected a number"),
        );
        return;
    };
    if value.unit != declared.unit {
        // §7.2: units are syntax, so a missing one is a diagnostic and not a
        // guess. The message names the unit the parameter is declared in.
        let (message, label) = match declared.unit.spelling() {
            Some(expected) => (
                format!("`{}` is written in `{expected}`", declared.name),
                format!("expected `{expected}`"),
            ),
            None => (format!("`{}` takes no unit", declared.name), "drop the unit".to_owned()),
        };
        let mut diagnostic = Diagnostic::error(Code::NotAValue, message)
            .maybe_at(span, label)
            .note("units are part of the syntax, so musa never guesses one");
        // Writing the unit the parameter is declared in is the one repair
        // that changes no number, so it is a fix rather than a help line. A
        // *wrong* unit is not: `resonance: 2 Hz` might be a misplaced argument, and
        // guessing which is exactly what a fix must not do.
        if let (Some(expected), None, Some(at)) = (declared.unit.spelling(), literal.unit(), span) {
            let written = match arg.name() {
                Some(name) => format!("{name}: {number} {expected}"),
                None => format!("{number} {expected}"),
            };
            diagnostic = diagnostic.fix(format!("write `{number} {expected}`"), at, written);
        }
        diagnostics.push(diagnostic);
        return;
    }
    if !(declared.range.0..=declared.range.1).contains(&value.magnitude) {
        diagnostics.push(
            Diagnostic::error(
                Code::OutOfRange,
                format!(
                    "`{}` must be between {} and {}{}",
                    declared.name,
                    declared.range.0,
                    declared.range.1,
                    declared
                        .unit
                        .spelling()
                        .map_or(String::new(), |unit| format!(" {unit}")),
                ),
            )
            .maybe_at(span, "outside the writable range")
            .note("the public range is fixed by the standard-library processor declaration"),
        );
        return;
    }
    if let Some(slot) = params.get_mut(index) {
        *slot = Some(value);
    }
    if let Some(slot) = spans.get_mut(index) {
        // The literal's own range, unit included: replacing it replaces what
        // was written, so `1400 Hz` becomes `900 Hz` and nothing else moves.
        *slot = Some(trimmed_span(literal.syntax()));
    }
}
