//! What an editor says about a declaration.
//!
//! One record per checked declaration, produced by the pass that already knows
//! the answer. The type is the *checked* type, not a re-reading of the
//! annotation; the parameters are the ones the checker lowered; the summary is
//! the comment block written above the declaration, read off the lossless tree
//! rather than scraped out of a line-oriented scan of the file.
//!
//! Why it exists at all: hover, completion, signature help, and the outline
//! each need the same five sentences about a name, and before this they either
//! did without them or re-derived them from text. Re-derivation is the failure
//! mode this record removes — a second reader of Musa source, living in a
//! shell, disagreeing with the compiler about what a name is.
//!
//! What it deliberately does not carry: the checked body, the environment, the
//! closure, or the module table. A record is what a *reader* is told, and a
//! reader is told a name, a kind, a signature, where it is written, and
//! whether they may edit it.
//!
//! The record is shaped for the declaration kinds the language grows into.
//! [`NameKind`] is the one vocabulary for what a name names — extending it is
//! how prompt 169's instruments, controls, processors, assets, and packages
//! arrive, and every consumer's match is exhaustive, so they arrive loudly.

use musa_syntax::SyntaxNode;

use crate::resolve::NameKind;
use musa_score::origin::SourceSpan;

/// Where a documented declaration is written, and whether it may be edited.
///
/// Identity, not merely a location: `(uri, span)` names the declaration's own
/// name token in the document that declares it, which stays the same thing
/// across an edit elsewhere in the workspace. Two records with the same source
/// are the same declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemSource {
    /// The document, when it is not the one being compiled: an import's
    /// resolved path, or a bundled module's `musa-stdlib:` URI. `None` means
    /// "here", and only then does [`ItemSource::span`] index the open text.
    pub uri: Option<String>,
    /// The declaration's name token, in that document.
    pub span: SourceSpan,
    /// Whether an editor may write to it. Bundled modules are read-only:
    /// their text is compiled into the binary, so a virtual document is the
    /// only thing to navigate to and an edit has nowhere to land.
    pub read_only: bool,
}

/// A type as a reader meets it: the name, and the one line that distinguishes
/// it from the type it is most often confused with.
///
/// The distinction is the point. `NoteName` and `Pc12` are both "a pitch
/// class" in ordinary speech and are different objects in this language —
/// spelled versus modulo twelve — and the same is true of `Key` against
/// `Scale` and `ChordClass` against `Voicing`. The sentences come from
/// `musa_syntax::BASE_TYPES`, which is the vocabulary the compiler
/// reads a type from, so a hover cannot describe a type the language lacks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeNote {
    /// As it is spelled in source: `NoteName`, `List<Pitch>`,
    /// `Ratio -> EventTrack<WrittenTime>`.
    pub name: String,
    /// The one line for the *head* of the type, when the head is a primitive.
    /// A `List<Pitch>` is distinguished by being a list, so it has none.
    pub distinction: Option<&'static str>,
}

impl TypeNote {
    /// Read the distinction, if the whole spelling names one primitive.
    pub(crate) fn new(name: String) -> Self {
        let distinction = musa_syntax::BASE_TYPES
            .iter()
            .find(|(base, _)| *base == name)
            .map(|(_, line)| *line);
        Self { name, distinction }
    }
}

/// One parameter of a callable declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterDoc {
    /// The name it is called by — which is also the name it is passed by,
    /// since Musa's arguments may be named.
    pub name: String,
    /// `first: Interval` — the substring of the signature this parameter
    /// occupies, which is what signature help highlights.
    pub label: String,
    /// Its type.
    pub ty: TypeNote,
}

/// Everything an editor says about one declaration.
///
/// # Invariants
///
/// - `signature` begins with the word a reader would write to declare this
///   thing (`let`, `fn`, `motif`, …) and mentions `name` exactly once, as the
///   declared name.
/// - each `label` in `parameters` appears verbatim in `signature`. The list is
///   empty for a value *and* for a nullary callable, which the signature tells
///   apart by writing the empty parameter list a caller must also write.
/// - `result` is `Some` exactly when the declaration names a value.
/// - `source.uri` is `None` exactly when `source.span` indexes the compiled
///   document, so a consumer never mistakes one document's offsets for
///   another's.
/// - `source.read_only` implies `source.uri` is `Some`: the open document is
///   never read-only.
/// - Records are unique by `(name, kind)` within one compilation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemDoc {
    /// The name as it is reached: `perfect_fifth`, or `Harmony.triad` when an
    /// import was qualified.
    pub name: String,
    /// What it names.
    pub kind: NameKind,
    /// Where it is written, and whether it may be edited.
    pub source: ItemSource,
    /// The comment block written directly above the declaration, joined into
    /// one paragraph. `None` when nothing was written there.
    pub summary: Option<String>,
    /// The declaration line, without its body: `fn triad(root: NoteName) -> ChordClass`.
    pub signature: String,
    /// What it evaluates to — the result type for a callable, the value's own
    /// type otherwise. `None` for a declaration that is static structure and
    /// names no value: a `signature`, a `structure`, and whatever later
    /// registries file under the same rule.
    pub result: Option<TypeNote>,
    /// Its parameters, in order. Empty for a value.
    pub parameters: Vec<ParameterDoc>,
    /// What to write instead, when the summary's first line says
    /// `deprecated: …`. Editors strike the name through and say this.
    pub deprecation: Option<String>,
}

/// The comment block written directly above `node`, as one paragraph.
///
/// "Directly above" is a run of `//` lines with no blank line between them and
/// the declaration: a blank line ends a thought, and a comment separated by
/// one is about the section rather than the name. Read from the lossless tree,
/// so a comment inside a brace, at the end of a line, or between the
/// declaration's own tokens is not mistaken for a summary.
pub(crate) fn summary_above(node: &SyntaxNode) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();
    // From the first token that is *not* trivia: the parser may already have
    // attached the comment block to the declaration as leading trivia, and
    // walking back from the node's own first token would then start inside
    // the block and miss it.
    let first = node
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .find(|token| {
            !matches!(
                token.kind(),
                musa_syntax::SyntaxKind::Whitespace
                    | musa_syntax::SyntaxKind::LineComment
                    | musa_syntax::SyntaxKind::BlockComment
            )
        })?;
    let mut token = first.prev_token();
    while let Some(current) = token {
        if current.kind() == musa_syntax::SyntaxKind::Whitespace {
            // One newline separates a comment from what it documents; two end
            // the block.
            if current.text().matches('\n').count() > 1 {
                break;
            }
        } else if current.kind() == musa_syntax::SyntaxKind::LineComment {
            lines.push(current.text().trim_start_matches('/').trim().to_owned());
        } else {
            // Code, and code above a declaration belongs to the declaration
            // before it rather than to this one.
            break;
        }
        token = current.prev_token();
    }
    if lines.is_empty() {
        return None;
    }
    lines.reverse();
    Some(lines.join(" "))
}

/// The replacement a `deprecated: …` first line names.
///
/// A convention rather than a keyword: a deprecation is documentation, and
/// documentation is where this language writes it. The line is kept in the
/// summary as well — a reader scrolling past the hover should see it in
/// place, not only in the editor's strike-through.
pub(crate) fn deprecation_in(summary: &str) -> Option<String> {
    let rest = summary.strip_prefix("deprecated:")?;
    Some(rest.trim().to_owned())
}

/// Every documented declaration of one compilation, unique by name and kind.
///
/// Insertion order is checking order, which is import order followed by the
/// document's own: a reader offered two names sees the one they wrote last.
#[derive(Default)]
pub(crate) struct DocIndex {
    items: Vec<ItemDoc>,
}

impl DocIndex {
    /// Record one declaration, or keep the record already held for it.
    pub(crate) fn document(&mut self, item: ItemDoc) {
        if self
            .items
            .iter()
            .any(|held| held.kind == item.kind && held.name == item.name)
        {
            return;
        }
        self.items.push(item);
    }

    pub(crate) fn items(&self) -> &[ItemDoc] {
        &self.items
    }
}
