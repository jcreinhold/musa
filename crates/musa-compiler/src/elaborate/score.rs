//! Elaborating a piece into the snapshot the projection reads.
//!
//! One concern of the `elaborate` module; see its docs for the semantic path.

use super::bars::{check_keys, part_bars, resolve_meters};
use super::fact::{FactKind, VoiceTrack};
use super::place::{MediaDeclarations, placed};
use crate::compile::Compilation;
use crate::resolve::{self, Resolver};
use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::SourceSpan;
use musa_score::scope::Scope;
use musa_score::score::{Meter, Part, PartId, ScoreSnapshot, Voice, VoiceId};
use musa_score::time::MusicalTime;
use musa_syntax::ast::AstNode as _;
use musa_syntax::ast::PieceDecl;

/// The piece, read and evaluated: everything it claims proved, everything it
/// sounds projected into `snapshot`, and the identity of the whole.
///
/// One compilation, one temporal object
/// (docs/rules/events/06-surface-elaboration.md). Part and voice identity live
/// in `Scope` rather than in a timeline per voice, which is why nothing here
/// holds a lane: the projection buckets the piece's own occurrences by the scope
/// each fact was constructed at.
///
/// The order below is forced, and worth saying because three of the steps could
/// look independent and are not. The piece is evaluated before the barlines,
/// because the meters it states are occurrences *in* it. The barlines come
/// before the claims, because "this bar is a quarter short" is a sentence
/// measured in bars. And the markers come after the barlines and before the
/// hash, because `at bar 9` is a coordinate the meters decide, and because the
/// piece's identity is the identity of the whole piece, markers included.
pub(super) fn elaborate_score(
    resolver: &mut Resolver,
    elaborated: &mut crate::document::Document,
    piece: &PieceDecl,
    media: &MediaDeclarations,
    snapshot: &mut ScoreSnapshot,
) -> musa_events::SemanticHash {
    let Some(score) = piece.score() else {
        return musa_events::SemanticHash::default();
    };
    // Named bars join the namespace before any voice is read, so a bar in the
    // cello can be answered by the violin above it — or refused, if the answer
    // comes first. Either way the name exists.
    resolve::register_bars(resolver, snapshot, &score);
    let Some(read) = elaborated.piece(resolver, piece.syntax()) else {
        return musa_events::SemanticHash::default();
    };
    let Some(sounding) = evaluated(resolver, elaborated, &read.track) else {
        return musa_events::SemanticHash::default();
    };
    for part in &read.parts {
        if !part.name.is_empty()
            && let Some(span) = part.name_span
        {
            resolver
                .references
                .declare(crate::resolve::NameKind::Part, &part.name, span);
        }
        // A part's own meter is what the barlines in *this* part are counted
        // against, and the claims below are proved before any projection exists,
        // so the resolver carries it rather than reading it back off the score.
        if let Some(meter) = part.meter {
            resolver.part_meters.insert(part.id, meter);
        }
        assign_profile(resolver, snapshot, part);
        for voice in &part.voices {
            if !voice.name.is_empty()
                && let Some(span) = voice.name_span
            {
                resolver
                    .references
                    .declare(crate::resolve::NameKind::Voice, &voice.name, span);
            }
        }
    }
    // The meters first: every check below is measured against the barlines, and
    // where the barlines fall is what the meters decide.
    let bars = resolve_meters(resolver, stated(&sounding, meter_of));
    check_keys(resolver, &bars, stated(&sounding, key_of));
    prove(resolver, elaborated, &read, &bars);
    if resolver.track_sink.is_some() {
        // Measurement only, and the one place a voice is wanted on its own; the
        // piece itself is one term and was evaluated once, above.
        let lanes: Vec<VoiceTrack> = read
            .parts
            .iter()
            .flat_map(|part| part.voices.iter())
            .filter_map(|voice| elaborated.track(&voice.track).ok())
            .collect();
        if let Some(sink) = &mut resolver.track_sink {
            sink.extend(lanes);
        }
    }
    let whole = musa_events::together(vec![
        placed(resolver, media, &score, &bars, sounding.duration()),
        sounding,
    ]);
    // The piece's identity, taken where the piece exists as one temporal object
    // and nowhere else: after this line the timeline is a projection, and a hash
    // of the projection would be a hash of a view.
    let identity = whole.semantic_hash();
    let projection = crate::project::project(resolver, &whole);
    snapshot.set_contexts(projection.contexts);
    let mut projected = projection.voices;
    for part in &read.parts {
        let id = PartId(part.id);
        let mut names = indexmap::IndexMap::with_capacity(part.voices.len());
        let mut voices = indexmap::IndexMap::with_capacity(part.voices.len());
        for voice in &part.voices {
            let held = VoiceId(voice.id);
            names.insert(held, voice.name.clone());
            voices.insert(
                held,
                projected
                    .swap_remove(&(part.id, voice.id))
                    .unwrap_or_else(|| Voice::new(Vec::new())),
            );
        }
        snapshot
            .parts_mut()
            .insert(id, Part::new(id, part.name.clone(), voices, names));
    }
    identity
}

/// Record which profile realizes `part`, or say the piece declares no such
/// profile.
///
/// Here rather than in the reading that walked the part, because this is the
/// one place both halves are in hand: [`crate::lower::piece::Part::profile`] is
/// what the part says, and `snapshot` is what the piece declares. A profile
/// changes no note and no barline — it is how the marks a note carries become
/// numbers, which is exact gesture lowering's question and nothing the notation
/// asks.
fn assign_profile(resolver: &mut Resolver, snapshot: &mut ScoreSnapshot, part: &crate::lower::piece::Part) {
    let Some((profile, span)) = &part.profile else {
        return;
    };
    // The edition default is an ordinary standard-library declaration. Its
    // explicit spelling has exactly the same checked policy as omission, so
    // there is no piece-local profile record to install.
    if matches!(profile.as_str(), "neutral" | "std::performance::neutral") {
        return;
    }
    if snapshot.profiles().declares(profile) {
        snapshot.profiles_mut().assign(&part.name, profile.clone());
        return;
    }
    let known: Vec<&str> = snapshot.profiles().names().collect();
    let help = resolve::suggest(profile, &known, "profiles");
    resolver.report(
        Diagnostic::error(Code::UnknownName, format!("cannot find profile `{profile}`"))
            .at(*span, "not declared in this piece")
            .help(help),
    );
}

/// `raw` as the track it denotes, with any refusal restated where it was
/// written.
///
/// [`None`] rather than an empty track, because a piece that did not evaluate is
/// not a piece that sounds nothing: every check below it would then be run
/// against silence and report a second complaint about the first one.
pub(super) fn evaluated(
    resolver: &mut Resolver,
    elaborated: &crate::document::Document,
    raw: &musa_calculus::Raw,
) -> Option<VoiceTrack> {
    match elaborated.track(raw) {
        Ok(track) => Some(track),
        Err(error) => {
            resolver.report(crate::lower::refusals::restate(elaborated.sites(), &error));
            None
        }
    }
}

/// Prove every claim the piece writes, each against the barlines its own part
/// counts by.
///
/// Per part rather than per piece because `Meter` inherits by `Override`
/// (`musa_score::scope`): a part in 7/8 does not hear the piece's changes at all, so
/// a bar written in it is a measure nothing else in the score agrees about.
///
/// The claims are placed by [`crate::document::Document::passage`] rather than
/// by a cursor this walk keeps, which is the whole of what a fold buys: where a
/// passage begins is how long the music before it lasts, and both are exact
/// rational arithmetic on terms the reading already recorded.
fn prove(
    resolver: &mut Resolver,
    elaborated: &crate::document::Document,
    read: &crate::lower::piece::Piece,
    bars: &musa_score::BarLines,
) {
    for part in &read.parts {
        let here = part_bars(resolver, Scope::Part { part: part.id });
        for claimed in part.voices.iter().flat_map(|voice| voice.claims.iter()) {
            let (claim, passage) = match elaborated.passage(claimed) {
                Ok(placed) => placed,
                Err(error) => {
                    resolver.report(crate::lower::refusals::restate(elaborated.sites(), &error));
                    continue;
                }
            };
            let settled = musa_score::assert::Settled {
                bars: here.as_ref().unwrap_or(bars),
                meter_written: resolver.meter_written,
            };
            if let Some(failure) = musa_score::assert::check(&claim, &passage, &settled) {
                let (diagnostic, rest_fill) = failure.into_parts();
                let diagnostic = match rest_fill {
                    Some(fill) => {
                        let written = musa_score::assert::fraction(fill.duration.as_ratio());
                        let rest = format!("rest{}", musa_syntax::spell_duration(&written));
                        let filled = diagnostic.help(format!("add `{rest}`, or lengthen one of the durations"));
                        match fill.content_end {
                            Some(at) => {
                                filled.fix(format!("add `{rest}`"), SourceSpan::new(at, at), format!(" {rest}"))
                            }
                            None => filled,
                        }
                    }
                    None => diagnostic,
                };
                resolver.report(diagnostic);
            }
        }
    }
}

/// Every change of one kind of context the evaluated piece states, in the order
/// the barlines are folded in.
///
/// A **change** is a fact that begins somewhere, and that is what tells the two
/// kinds of statement apart here: [`crate::lower::notation`] writes a `meter` or
/// a `key` among a voice's items as a *point*, at the instant the fold had
/// reached, while [`crate::lower::piece`] writes a header's over the region it
/// governs. A header says what is in force, not what changes, so a fold that
/// counted it would start the piece over at its own first barline.
///
/// A part's own meter is not a change either, and needs no test: it is
/// constructed at `Scope::Part` and read through [`part_bars`], because `Meter`
/// inherits by `Override` and a part that states one does not hear the piece's.
pub(super) fn stated<T>(
    sounding: &VoiceTrack,
    select: impl Fn(&FactKind) -> Option<T>,
) -> Vec<(MusicalTime, T, SourceSpan)> {
    let mut changes: Vec<(MusicalTime, T, SourceSpan)> = sounding
        .occurrences()
        .iter()
        .filter(|occurrence| {
            occurrence.payload().scope == Scope::Piece && occurrence.span().start() == occurrence.span().end()
        })
        .filter_map(|occurrence| {
            let fact = occurrence.payload();
            Some((
                MusicalTime::new(occurrence.span().start().as_ratio()),
                select(&fact.kind)?,
                fact.origin.source_span,
            ))
        })
        .collect();
    changes.sort_by_key(|(at, _, span)| (*at, span.start));
    changes
}

/// The meter a fact states, when it states one.
pub(super) fn meter_of(kind: &FactKind) -> Option<Meter> {
    if let FactKind::Meter { numerator, denominator } = *kind {
        return Some(Meter::new(numerator, denominator));
    }
    None
}

/// The key a fact states, when it states one.
fn key_of(kind: &FactKind) -> Option<musa_score::Key> {
    if let FactKind::Key { tonic, mode } = *kind {
        return Some(musa_score::Key::new(tonic, mode));
    }
    None
}

/// Register everything the imported libraries declare, before the piece's
/// own declarations, so a collision is reported against the library that
/// caused it (roadmap §16).
///
/// A file with no piece declares and does not sound, so there is no score to
/// build and the absence of one is not a failure — that is the whole
/// difference between this path and the piece path, and it is why
/// [`Compilation::kind`] exists. What it *does* do is everything a check is
/// for: resolve what the file builds on, register its declarations so a
/// duplicate or a malformed motif is reported, and hold its `studio` to the
/// rules.
///
/// The file's own studio arrives as an *imported* block rather than as the
/// document's. That is not a trick: the rules for such a studio are the rules
/// that apply to it wherever it is read, and a file that could wire itself to
/// a score when opened directly and not when imported would compile two
/// different ways.
pub(super) fn elaborate_material(
    resolver: &mut Resolver,
    library: &musa_syntax::ast::Document,
    name: &str,
    options: &crate::CompileOptions,
) -> Compilation {
    let mut snapshot = ScoreSnapshot::default();
    let libraries = crate::imports::load(resolver, name, &library.imports(), &options.imports);
    // A library exports these declarations even though it places no cues of
    // its own, so malformed or duplicate media must fail when the library is
    // checked directly as well as when a piece imports it.
    let _media = super::media_declarations(resolver, &libraries, library, None);
    // The one checker, here as everywhere: the library and its imports as a
    // document, declared by musa-calculus. A document that came back is not yet a
    // library that checks — `elaborate` answers `Some` beside refusals it
    // reported — so the diagnostics decide.
    let sources: Vec<crate::document::Source> = libraries
        .each()
        .map(|(from, imported)| crate::document::Source::imported(imported.syntax(), from))
        .chain(std::iter::once(crate::document::Source::own(library.syntax())))
        .collect();
    let machines = crate::document::elaborate(resolver, &sources).map_or_else(Vec::new, |document| document.machines());
    if resolver
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == musa_score::diagnose::Severity::Error)
    {
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics)).into_material();
    }
    elaborate_libraries(resolver, &libraries, &mut snapshot);
    if let Some(performance) = library.performance() {
        let profiles = resolve::parse_profiles(resolver, &performance);
        let span = resolve::span_of(performance.syntax());
        resolve::merge_profiles(resolver, &mut snapshot, &profiles, span, None);
    }
    resolve::register_motifs(resolver, &mut snapshot, &library.motifs(), None);
    resolve::register_fragments(resolver, &mut snapshot, &library.fragments(), None);
    let studios: Vec<musa_syntax::ast::StudioDecl> = libraries
        .each()
        .filter_map(|(_, imported)| imported.studio())
        .chain(library.studio())
        .collect();
    let instruments: Vec<musa_syntax::ast::InstrumentDecl> = libraries
        .each()
        .flat_map(|(_, imported)| imported.instruments())
        .chain(library.instruments())
        .collect();
    let mut references = std::mem::take(&mut resolver.references);
    crate::studio::resolve(
        None,
        &studios,
        &instruments,
        &[],
        &[],
        &[],
        &mut references,
        &mut resolver.diagnostics,
    );
    Compilation::new(None, std::mem::take(&mut resolver.diagnostics))
        .with_machines(machines)
        .into_material()
}

pub(super) fn elaborate_libraries(
    resolver: &mut Resolver,
    libraries: &crate::imports::Libraries,
    snapshot: &mut ScoreSnapshot,
) {
    for (from, library) in libraries.each() {
        let path = from.path;
        if let Some(performance) = library.performance() {
            let profiles = resolve::parse_profiles(resolver, &performance);
            resolve::merge_profiles(
                resolver,
                snapshot,
                &profiles,
                resolve::span_of(performance.syntax()),
                Some(path),
            );
        }
        resolve::register_motifs(resolver, snapshot, &library.motifs(), Some(path));
        resolve::register_fragments(resolver, snapshot, &library.fragments(), Some(path));
    }
}
