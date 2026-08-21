//! The compiler's side of the interchange format: a piece printed
//! as kernel text, and kernel text read back, checked, and compiled.
//!
//! Every direction lives here rather than in `musa-project` because the
//! payload is `ScoreFact`, which is crate-private and stays that way — the
//! kernel is generic in the payload, the compiler owns the payload, and only
//! the compiler can name the pair.
//!
//! **A kernel file is a document, and it is not `.musa`.** Those are two
//! statements and both hold. It is a document because the toolchain opens,
//! checks, formats, and serves it like any other source file
//! (`docs/rules/language/01-surface.md` §7). It is not `.musa` because nothing
//! rewrites it into surface syntax: a kernel file declares one composition,
//! it compiles to one track, and the round trip through this module never
//! produces a `piece` block. The source is still canonical (AGENTS.md) —
//! which source it is has simply stopped being a question with one answer.

use musa_kernel::TextPayload as _;

use crate::compile::{Compilation, SourceDocument};
use crate::elaborate::{ScoreFact, piece_term};
use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::SourceSpan;

/// What `--check` learned about a kernel file.
///
/// Small on purpose: a checker reports that the file is well-formed and what
/// it denotes, and anything more would be an inspector with no caller.
#[derive(Clone, Debug)]
pub struct KernelCheck {
    /// The piece name the file declares.
    pub name: String,
    /// Which reading of the work this file projects, verbatim from its header
    /// (`docs/rules/kernel/11-realization.md`). `None` for a file written before
    /// realizations existed, which is a file whose realization is unknown
    /// rather than a file that has none.
    pub realization: Option<String>,
    /// How many occurrences the term evaluates to.
    pub occurrences: usize,
    /// The evaluated track's duration, as an exact rational.
    pub duration: String,
}

/// A piece as kernel text, structure preserved.
///
/// Returns `None` when the source does not elaborate cleanly — the same
/// condition as [`crate::kernel_normal_form`], and for the same reason: there
/// is no term to print for a document that does not have one.
#[doc(hidden)]
pub fn kernel_text(
    source: &SourceDocument,
    realization: &musa_score::Realization,
    imports: &crate::imports::ImportSources,
) -> Option<String> {
    let (name, term, decisions) = piece_term(source, realization, imports)?;
    Some(musa_kernel::print(&name, &term, &notes(realization, &decisions)))
}

/// What the file says about the reading of the work it projects.
///
/// Nothing at all when the piece decided nothing: every realization produces
/// that file, so naming one would be a claim the file does not need and would
/// make a determinate piece's export depend on a seed it never read. Otherwise
/// the seed — because a file that leaves a decision open and does not name its
/// realization cannot be reproduced — and every decision taken
/// (`docs/rules/kernel/11-realization.md`, consumer obligation 1).
fn notes(realization: &musa_score::Realization, decisions: &[musa_score::DecisionRecord]) -> Vec<String> {
    if decisions.is_empty() {
        return Vec::new();
    }
    let mut notes = vec![format!(
        "realization seed={} pins={}",
        realization.seed(),
        realization.taken().count()
    )];
    notes.extend(
        decisions
            .iter()
            .map(|record| format!("decision {} {}", record.path(), record.decision())),
    );
    notes
}

/// A piece as kernel text, normalized: the term evaluated to a value and
/// printed as a single literal.
///
/// This is the *interchange* spelling of the normal form, not N5's bytes.
/// They are different serializations with different jobs. N5
/// ([`crate::kernel_normal_form`]) is the **equality** serialization: its
/// payload text is the canonical key, which deliberately omits the definition
/// span and the declaration id because two facts differing only in those are
/// the same fact (N3). That quotient is what makes the semantic hash
/// semantic — and it is exactly what an interchange file must not do, because
/// a consumer that reads one cannot reconstruct what was thrown away.
///
/// So both exist, neither changed the other, and the useful consequence is
/// that `--normalized` output is parseable: `--check` accepts it, which N5
/// bytes never could.
#[doc(hidden)]
pub fn kernel_normalized_text(
    source: &SourceDocument,
    realization: &musa_score::Realization,
    imports: &crate::imports::ImportSources,
) -> Option<String> {
    let (name, term, decisions) = piece_term(source, realization, imports)?;
    let value = musa_kernel::evaluate_marked(term, crate::elaborate::instantiate);
    Some(musa_kernel::print(
        &name,
        &musa_kernel::Term::literal(value),
        &notes(realization, &decisions),
    ))
}

/// What a kernel file *means*: its normalized form (N5) and its semantic
/// hash (N6).
///
/// This is the conformance operation. A second implementation of the kernel
/// is validated by reading a file from `examples/kernel/`, computing this
/// pair, and comparing — which is exactly what Q6 was waiting for, and why
/// the corpus rather than the CLI is the deliverable.
///
/// # Errors
///
/// As [`check_kernel_text`].
#[doc(hidden)]
pub fn kernel_text_meaning(text: &str) -> Result<(String, musa_kernel::SemanticHash), String> {
    let term = musa_kernel::parse::<musa_kernel::WrittenTime, ScoreFact>(text)
        .map_err(|error| error.to_string())?
        .into_term();
    term.check().map_err(|error| error.to_string())?;
    let value = musa_kernel::evaluate_marked(term, crate::elaborate::instantiate);
    let hash = value.semantic_hash();
    Ok((value.to_string(), hash))
}

/// Parse, check, and evaluate kernel text.
///
/// # Errors
///
/// The parse error's message, positioned in the input, or the well-formedness
/// violation [`musa_kernel::Term::check`] found (K7).
#[doc(hidden)]
pub fn check_kernel_text(text: &str) -> Result<KernelCheck, String> {
    let document =
        musa_kernel::parse::<musa_kernel::WrittenTime, ScoreFact>(text).map_err(|error| error.to_string())?;
    document.term().check().map_err(|error| error.to_string())?;
    let realization = realization_note(&document);
    let name = document.name().to_owned();
    let value = musa_kernel::evaluate_marked(document.into_term(), crate::elaborate::instantiate);
    Ok(KernelCheck {
        name,
        realization,
        occurrences: value.occurrences().len(),
        duration: value.duration().as_ratio().to_string(),
    })
}

/// Which reading of the work a file projects, if it says.
fn realization_note<C: musa_kernel::Coordinate, A>(document: &musa_kernel::Document<C, A>) -> Option<String> {
    document
        .notes()
        .iter()
        .find_map(|note| note.strip_prefix("realization "))
        .map(str::to_owned)
}

/// Compile a kernel interchange document (`docs/rules/language/01-surface.md` §7).
///
/// The other half of [`crate::compile`], reached when the document's first
/// line is the kernel marker. There is no surface parse, no resolution, no
/// name binding and no elaboration: a kernel file *is* the term, so the whole
/// of the work is read it, check it, evaluate it, and project the result the
/// same way the surface path projects its own track.
///
/// That last clause is the reason this is worth doing at all. A kernel
/// document reaching the same [`crate::project::project`] as a `.musa` piece
/// is what makes the subset claim operational rather than aspirational: one
/// projection, one snapshot type, and every backend downstream unable to tell
/// which door the track came through.
pub(crate) fn compile_kernel(source: &SourceDocument) -> Compilation {
    let text = source.text();
    // Read first, always, and decoded only afterwards. A file whose payload
    // type this build cannot decode is still a file the toolchain must be
    // able to show, and finding that out costs one erased pass.
    let document = match musa_kernel::read::<musa_kernel::WrittenTime>(text) {
        Ok(document) => document,
        Err(error) => return refused(text, &error),
    };
    if let Err(error) = document.term().check() {
        return refused(text, &error);
    }
    if document.payload_type() != ScoreFact::type_name() {
        return Compilation::new(None, vec![unsupported_payload(text, document.payload_type())]).into_kernel();
    }
    // Parsed a second time, now typed. The erased pass established that the
    // grammar is satisfied; this one decodes the payloads, and it reports at
    // the offset of the payload that failed rather than at the file, which is
    // why it is a parse rather than a walk over what was already read.
    let typed = match musa_kernel::parse::<musa_kernel::WrittenTime, ScoreFact>(text) {
        Ok(typed) => typed,
        Err(error) => return refused(text, &error),
    };
    let name = typed.name().to_owned();
    let value = musa_kernel::evaluate_marked(typed.into_term(), crate::elaborate::instantiate);
    let identity = value.semantic_hash();
    let mut resolver = crate::resolve::Resolver::new();
    let projection = crate::project::project(&mut resolver, &value);
    let mut snapshot = musa_score::score::ScoreSnapshot::default();
    snapshot.set_title(name);
    snapshot.set_contexts(projection.contexts);
    gather_parts(&mut snapshot, projection.voices);
    snapshot.set_annotations(std::mem::take(&mut resolver.annotations));
    Compilation::new(Some(snapshot), std::mem::take(&mut resolver.diagnostics))
        .into_kernel()
        .with_identity(identity)
}

/// Assemble the projected voices into parts.
///
/// The interchange format carries scope *identity* and not scope *names*: a
/// `ScoreFact` says it belongs to part 2 voice 0, and nothing in the file says
/// that part 2 was called "viola". So the names are made up here, visibly, in
/// the one place that has to make them up — a backend needs something to print
/// and a reader deserves to see immediately that the file did not say. Losing
/// the names is not a defect of the projection but of what a projection *is*:
/// a name is surface metadata, and this format carries the track.
///
/// Ordering is by identity rather than by first appearance, so a file whose
/// second part happens to start first still scores as parts 1, 2.
fn gather_parts(snapshot: &mut musa_score::score::ScoreSnapshot, voices: crate::project::Voices) {
    let mut lanes: Vec<((u32, u32), musa_score::score::Voice)> = voices.into_iter().collect();
    lanes.sort_by_key(|(key, _)| *key);
    let mut current: Option<(musa_score::score::PartId, musa_score::score::Part)> = None;
    for ((part, voice), lane) in lanes {
        let (part, voice) = (musa_score::score::PartId(part), musa_score::score::VoiceId(voice));
        if current.as_ref().is_none_or(|(open, _)| *open != part) {
            if let Some((open, built)) = current.take() {
                snapshot.parts_mut().insert(open, built);
            }
            current = Some((
                part,
                musa_score::score::Part::new(
                    part,
                    format!("part {}", part.0.saturating_add(1)),
                    indexmap::IndexMap::new(),
                    indexmap::IndexMap::new(),
                ),
            ));
        }
        if let Some((_, built)) = current.as_mut() {
            built.add_voice(voice, format!("voice {}", voice.0.saturating_add(1)), lane);
        }
    }
    if let Some((open, built)) = current {
        snapshot.parts_mut().insert(open, built);
    }
}

/// A kernel document the reader refused, as a compilation that produced no
/// score and said why.
fn refused(text: &str, error: &musa_kernel::KernelError) -> Compilation {
    // A parse error knows where it stopped reading. A well-formedness
    // violation is about the term as a whole — an occurrence outside its
    // track, a name bound twice — and pointing at one line of it would be
    // a guess dressed as a location, so those point at the document.
    let span = match error {
        musa_kernel::KernelError::Parse { offset, .. } => rest_of_line(text, *offset),
        musa_kernel::KernelError::InvalidSpan { .. }
        | musa_kernel::KernelError::NegativeDuration { .. }
        | musa_kernel::KernelError::OccurrenceOutOfBounds { .. }
        | musa_kernel::KernelError::EmptyComposition { .. }
        | musa_kernel::KernelError::FreeName { .. }
        | musa_kernel::KernelError::ShadowedName { .. }
        | musa_kernel::KernelError::NonPositiveScale { .. } => rest_of_line(text, 0),
    };
    Compilation::new(
        None,
        vec![Diagnostic::error(Code::Syntax, "this kernel file is not well formed").at(span, error.to_string())],
    )
    .into_kernel()
}

/// The diagnostic for a file this build cannot decode.
///
/// An error rather than a warning, and specifically *not* a syntax error: the
/// file is well formed and the reader said so. What is missing is on this side
/// — a payload type nothing here implements — and a diagnostic that blamed the
/// document would send its author to fix a file that is already correct.
fn unsupported_payload(text: &str, payload_type: &str) -> Diagnostic {
    Diagnostic::error(
        Code::UnsupportedPayload,
        format!("this build cannot read `{payload_type}` payloads"),
    )
    .at(
        payload_annotation(text, payload_type),
        format!("the file's occurrences are `{payload_type}`"),
    )
    .note(format!(
        "it parses, and `musa format` will lay it out; only its meaning is out of reach — \
         this build understands `{}`",
        ScoreFact::type_name()
    ))
}

/// Where the composition's `EventTrack[…]` annotation stands.
///
/// The coordinate is `WrittenTime` by the time this is reached: a file in any
/// other one was refused by the reader, which is typed in the coordinate it
/// expects.
fn payload_annotation(text: &str, payload_type: &str) -> SourceSpan {
    let needle = format!("EventTrack[WrittenTime, {payload_type}]");
    let start = text.find(&needle).unwrap_or(0);
    SourceSpan::new(clamp(start), clamp(start.saturating_add(needle.len())))
}

/// From `offset` to the end of the line it falls in, so a reader sees the
/// construct the cursor stopped inside rather than a caret between two
/// characters. An offset at the very end of the file underlines its last
/// character, because a span of nothing draws as nothing.
fn rest_of_line(text: &str, offset: usize) -> SourceSpan {
    let offset = offset.min(text.len());
    let rest = text.get(offset..).unwrap_or_default();
    let line = rest.find('\n').unwrap_or(rest.len());
    let end = offset.saturating_add(line.max(1)).min(text.len());
    SourceSpan::new(clamp(offset.min(end.saturating_sub(1))), clamp(end))
}

/// Byte offsets in a source span are `u32`; a file long enough to overflow one
/// is a file no editor will open either.
fn clamp(offset: usize) -> u32 {
    u32::try_from(offset).unwrap_or(u32::MAX)
}

/// A kernel document laid out canonically, or `None` when it cannot be read.
///
/// Formatting is defined for a file this build cannot *evaluate*, which is the
/// whole reason it goes through the erased reader: an unsupported payload
/// changes what a file means to us and not how it is written down.
pub(crate) fn format_kernel(text: &str) -> Option<String> {
    musa_kernel::read::<musa_kernel::WrittenTime>(text)
        .ok()
        .map(|document| document.to_text())
}
