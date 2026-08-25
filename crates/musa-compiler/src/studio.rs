//! Resolve surface `studio` declarations into [`StudioSpec`].
//!
//! The editable vocabulary belongs to `musa-dsp`; this module is only the
//! compiler pass that reads source and constructs one value of that vocabulary.

use indexmap::IndexMap;
use musa_dsp::{Assignment, Modulation, NodeIndex, Patch, Processor, Route, Send, StudioNode, StudioSpec, Unit, Value};
use musa_score::origin::SourceSpan;
use musa_syntax::ast::{
    Arg, AstNode as _, BusDecl, CallExpr, PatchDecl, SendStmt, SignalChain, SignalStage, StudioDecl, StudioItem,
};

use crate::resolve::{span_of, trimmed_span};
use musa_score::diagnose::{Code, Diagnostic};

/// Read a written number and unit suffix into a [`Value`].
///
/// `ms` is folded into seconds here, which is why the spec carries no
/// millisecond unit: the dimension is time, and the suffix is a scale.
fn parse_value(number: &str, suffix: Option<&str>) -> Option<Value> {
    let magnitude: f64 = number.parse().ok()?;
    let unit = match suffix {
        None => Unit::Linear,
        Some("Hz") => Unit::Hz,
        Some("dB") => Unit::Decibels,
        Some("s") => Unit::Seconds,
        Some("ms") => {
            return Some(Value {
                magnitude: magnitude / 1000.0,
                unit: Unit::Seconds,
            });
        }
        Some(_) => return None,
    };
    Some(Value { magnitude, unit })
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

/// Resolve a `studio` block into a [`StudioSpec`], reporting every unresolved
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
    parts: &[String],
    references: &mut crate::resolve::ReferenceIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> StudioSpec {
    let mut spec = StudioSpec::default();
    spec.set_span(decl.map(|decl| span_of(decl.syntax())));
    let mut items: Vec<StudioItem> = Vec::new();
    for library in imported {
        for item in library.items() {
            match item {
                StudioItem::Patch(_) | StudioItem::Signal(_) => items.push(item),
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

    // Two passes: patches, buses, and signals first, so the bindings that
    // follow can be checked against them regardless of writing order. A
    // studio reads top-down, but it does not have to be written that way.
    for (index, item) in items.iter().enumerate() {
        match item {
            StudioItem::Patch(patch) => declare_patch(patch, &mut spec, diagnostics),
            StudioItem::Bus(bus) => declare_bus(bus, &mut spec, diagnostics),
            StudioItem::Signal(signal) => {
                let name = signal.name().unwrap_or_default();
                let mut built = Patch::default();
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
                    spec.assign(part, Assignment { patch, patch_span });
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
                    spec.push_route(Route { source, destination });
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
            StudioItem::Patch(_) | StudioItem::Bus(_) | StudioItem::Signal(_) => {}
        }
    }
    spec
}

fn resolve_send(
    send: &SendStmt,
    spec: &mut StudioSpec,
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
    if level.unit != Unit::Decibels {
        diagnostics
            .push(Diagnostic::error(Code::NotAValue, "a send level is written in `dB`").maybe_at(span, "missing `dB`"));
        return;
    }
    let level_span = send.level().map(|literal| trimmed_span(literal.syntax()));
    spec.push_send(Send {
        source,
        bus,
        level,
        level_span,
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
    spec: &StudioSpec,
    span: Option<musa_score::origin::SourceSpan>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Modulation> {
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
    let matches: Vec<NodeIndex> = patch
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
    Some(Modulation {
        source: source.to_owned(),
        patch: patch_name.clone(),
        node: *node,
        param: declared.name,
    })
}

fn declare_patch(decl: &PatchDecl, spec: &mut StudioSpec, diagnostics: &mut Vec<Diagnostic>) {
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

fn declare_bus(decl: &BusDecl, spec: &mut StudioSpec, diagnostics: &mut Vec<Diagnostic>) {
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

/// The shared body of a patch or a bus: named signals, then chains, with the
/// names visible to the chains that follow them.
fn build_container(
    signals: &[musa_syntax::ast::SignalBinding],
    chains: &[musa_syntax::ast::ChainStmt],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Patch> {
    let mut patch = Patch::default();
    let mut locals: IndexMap<String, NodeIndex> = IndexMap::new();
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
fn lower_chain(chain: &SignalChain, patch: &mut Patch, diagnostics: &mut Vec<Diagnostic>) -> Option<NodeIndex> {
    let locals = IndexMap::new();
    let node = lower_chain_into(chain, patch, &locals, diagnostics)?;
    patch.set_output(node);
    Some(node)
}

/// Flatten `a |> b |> c` into nodes: each stage takes the previous stage's
/// node as its input, and `output` marks rather than adds one.
fn lower_chain_into(
    chain: &SignalChain,
    patch: &mut Patch,
    locals: &IndexMap<String, NodeIndex>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<NodeIndex> {
    let mut previous: Option<NodeIndex> = None;
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
    upstream: Option<NodeIndex>,
    patch: &mut Patch,
    locals: &IndexMap<String, NodeIndex>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<NodeIndex> {
    let span = Some(span_of(call.syntax()));
    let written = call.callee().unwrap_or_default();
    let Some(processor) = Processor::from_name(&written) else {
        diagnostics.push(
            Diagnostic::error(Code::UnknownWord, format!("unknown processor `{written}`"))
                .maybe_at(span, "musa has no processor by this name"),
        );
        return None;
    };
    let mut params: Vec<Value> = processor
        .params()
        .iter()
        .map(|declared| Value {
            magnitude: declared.default,
            unit: declared.unit,
        })
        .collect();
    let mut param_spans: Vec<Option<SourceSpan>> = vec![None; params.len()];
    let mut inputs: Vec<NodeIndex> = upstream.into_iter().collect();
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

    Some(patch.push(StudioNode {
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
    processor: Processor,
    params: &mut [Value],
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
            .note("the public range is fixed by the built-in processor schema"),
        );
        return;
    }
    if let Some(slot) = params.get_mut(index) {
        *slot = value;
    }
    if let Some(slot) = spans.get_mut(index) {
        // The literal's own range, unit included: replacing it replaces what
        // was written, so `1400 Hz` becomes `900 Hz` and nothing else moves.
        *slot = Some(trimmed_span(literal.syntax()));
    }
}
