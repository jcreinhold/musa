//! The compiler's side of the interchange format (prompt 48): a piece printed
//! as kernel text, and kernel text read back and checked.
//!
//! Both directions live here rather than in `musa-project` because the payload
//! is `ScoreFact`, which is crate-private and stays that way — the kernel is
//! generic in the payload, the compiler owns the payload, and only the
//! compiler can name the pair.
//!
//! **Kernel text is not `.musa` and never becomes it.** The source is
//! canonical (AGENTS.md); a kernel file is a projection, and reading one back
//! yields a term to check and evaluate, not a document to edit.

use crate::compile::SourceDocument;
use crate::elaborate::{ScoreFact, piece_term};

/// What `--check` learned about a kernel file.
///
/// Small on purpose: a checker reports that the file is well-formed and what
/// it denotes, and anything more would be an inspector with no caller.
#[derive(Clone, Debug)]
pub struct KernelCheck {
    /// The piece name the file declares.
    pub name: String,
    /// Which reading of the work this file projects, verbatim from its header
    /// (`docs/kernel/11-realization.md`). `None` for a file written before
    /// realizations existed, which is a file whose realization is unknown
    /// rather than a file that has none.
    pub realization: Option<String>,
    /// How many occurrences the term evaluates to.
    pub occurrences: usize,
    /// The evaluated timeline's extent, as an exact rational.
    pub extent: String,
}

/// A piece as kernel text, structure preserved.
///
/// Returns `None` when the source does not elaborate cleanly — the same
/// condition as [`crate::kernel_normal_form`], and for the same reason: there
/// is no term to print for a document that does not have one.
#[doc(hidden)]
pub fn kernel_text(source: &SourceDocument, realization: &crate::Realization) -> Option<String> {
    let (name, term, decisions) = piece_term(source, realization)?;
    Some(musa_kernel::print(&name, &term, &notes(realization, &decisions)))
}

/// What the file says about the reading of the work it projects.
///
/// Nothing at all when the piece decided nothing: every realization produces
/// that file, so naming one would be a claim the file does not need and would
/// make a determinate piece's export depend on a seed it never read. Otherwise
/// the seed — because a file that leaves a decision open and does not name its
/// realization cannot be reproduced — and every decision taken
/// (`docs/kernel/11-realization.md`, consumer obligation 1).
fn notes(realization: &crate::Realization, decisions: &[crate::DecisionRecord]) -> Vec<String> {
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
/// They are different serializations with different jobs, and prompt 48
/// assumed they would coincide — see "Repairs" in the prompt. N5
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
pub fn kernel_normalized_text(source: &SourceDocument, realization: &crate::Realization) -> Option<String> {
    let (name, term, decisions) = piece_term(source, realization)?;
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
    let (_, term) = musa_kernel::parse::<ScoreFact>(text).map_err(|error| error.to_string())?;
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
    let (name, term) = musa_kernel::parse::<ScoreFact>(text).map_err(|error| error.to_string())?;
    term.check().map_err(|error| error.to_string())?;
    let value = musa_kernel::evaluate_marked(term, crate::elaborate::instantiate);
    Ok(KernelCheck {
        name,
        realization: musa_kernel::notes(text)
            .find_map(|note| note.strip_prefix("realization "))
            .map(str::to_owned),
        occurrences: value.occurrences().len(),
        extent: value.extent().as_ratio().to_string(),
    })
}
