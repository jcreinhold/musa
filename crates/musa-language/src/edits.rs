//! Syntax-aware text edits (roadmap §11): the project's canonical mutation
//! mechanism. Score-editing commands (prompt 25) resolve to these.
//!
//! There is one entry point, [`compute_edits`]. A caller says what it wants
//! done to the music in [`EditIntent`] terms and gets back the text edits
//! that do it; it never reasons about tokens, indentation, or where a
//! statement ends. That keeps the syntax knowledge in the crate that owns the
//! syntax, and keeps the project layer free to be about provenance.

use text_size::{TextRange, TextSize};

use crate::SyntaxKind;
use crate::ast::{AstNode, PieceDecl, ScoreDecl, VoiceDecl};
use crate::language::{SyntaxElement, SyntaxNode};
use crate::parser::parse;

/// A replacement of one source range with new text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    /// Byte range of the original text to replace.
    pub range: TextRange,
    /// Text to put in its place (empty for a deletion).
    pub replacement: String,
}

impl TextEdit {
    /// Create an edit replacing `range` with `replacement`.
    pub fn new(range: TextRange, replacement: impl Into<String>) -> Self {
        Self {
            range,
            replacement: replacement.into(),
        }
    }
}

/// Apply `edits` to `source`, returning the new text.
///
/// Edits may be given in any order; they are applied in source order.
/// Overlapping or out-of-bounds edits are skipped — computing edits so they
/// cannot overlap is the caller's job (an edit command is transactional).
pub fn apply_edits(source: &str, edits: &[TextEdit]) -> String {
    let mut sorted: Vec<&TextEdit> = edits.iter().collect();
    sorted.sort_by_key(|edit| usize::from(edit.range.start()));
    let mut out = String::with_capacity(source.len());
    let mut cursor = 0usize;
    for edit in sorted {
        let start = usize::from(edit.range.start());
        let end = usize::from(edit.range.end());
        if start < cursor || start > end || end > source.len() {
            continue;
        }
        if let Some(before) = source.get(cursor..start) {
            out.push_str(before);
            out.push_str(&edit.replacement);
            cursor = end;
        }
    }
    if let Some(rest) = source.get(cursor..) {
        out.push_str(rest);
    }
    out
}

/// A statement to write into a voice, in the language's own terms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    /// `gs4 1/8;`
    Note {
        /// Written pitch, as it is spelled (`gs4`, `bf3`).
        pitch: String,
        /// Notated duration (`1/8`, `3/8`, `1`).
        duration: String,
    },
    /// `rest 1/4;`
    Rest {
        /// Notated duration.
        duration: String,
    },
    /// `chord [a3, c4, e4] 1/2;`
    Chord {
        /// Written pitches, low to high as the composer entered them.
        pitches: Vec<String>,
        /// Notated duration.
        duration: String,
    },
}

impl Statement {
    /// The statement's source text, without indentation or a line break.
    fn text(&self) -> String {
        match *self {
            Self::Note {
                ref pitch,
                ref duration,
            } => format!("{pitch} {duration};"),
            Self::Rest { ref duration } => format!("rest {duration};"),
            Self::Chord {
                ref pitches,
                ref duration,
            } => format!("chord [{}] {duration};", pitches.join(", ")),
        }
    }
}

/// Where a new statement goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Anchor {
    /// Immediately before the statement starting at this byte offset.
    Before {
        /// Byte offset of the statement's first significant token.
        at: u32,
    },
    /// Immediately after the statement starting at this byte offset.
    After {
        /// Byte offset of the statement's first significant token.
        at: u32,
    },
    /// At the end of a named voice — where entry lands when nothing is
    /// selected.
    EndOfVoice {
        /// The part's name.
        part: String,
        /// The voice's name within that part.
        voice: String,
    },
}

/// What an editing command wants done to the source (roadmap §11).
///
/// Offsets are byte offsets of a statement's first significant token, which
/// is exactly what an event's origin span reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditIntent {
    /// Respell the pitch of the note statement at `at`.
    SetPitch {
        /// The note statement's offset.
        at: u32,
        /// The new written pitch.
        pitch: String,
    },
    /// Renotate the duration of the note, rest, or chord at `at`.
    SetDuration {
        /// The statement's offset.
        at: u32,
        /// The new notated duration.
        duration: String,
    },
    /// Write a new statement into a voice.
    Insert {
        /// Where it goes.
        anchor: Anchor,
        /// What to write.
        statement: Statement,
    },
    /// Lift the statements from `first` through `last` into a new `motif`
    /// declaration, leaving a `use` in their place.
    ExtractMotif {
        /// Offset of the first statement to lift.
        first: u32,
        /// Offset of the last statement to lift (may equal `first`).
        last: u32,
        /// The motif's name.
        name: String,
    },
}

/// Why an intent could not be turned into edits.
///
/// Every variant names a fact about the source rather than an internal
/// failure, because the project layer reports these to a composer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    /// Nothing in the source starts at that offset.
    NoStatement {
        /// The offset that matched no statement.
        at: u32,
    },
    /// The statement there is not a note, so it has no pitch to respell.
    NotANote {
        /// The offset of the statement that was found instead.
        at: u32,
    },
    /// The statement there carries no duration token to replace.
    NoDuration {
        /// The offset of the statement.
        at: u32,
    },
    /// No such voice in the score.
    NoVoice {
        /// The part that was looked for.
        part: String,
        /// The voice that was looked for.
        voice: String,
    },
    /// The two ends of an extraction are not statements of the same block.
    NotSiblings,
    /// The document has no `score` block to place a motif before.
    NoScore,
}

impl core::fmt::Display for EditError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NoStatement { at } => write!(formatter, "no statement at byte {at}"),
            Self::NotANote { at } => write!(formatter, "the statement at byte {at} is not a note"),
            Self::NoDuration { at } => write!(formatter, "the statement at byte {at} has no duration"),
            Self::NoVoice { ref part, ref voice } => write!(formatter, "no voice `{voice}` in part `{part}`"),
            Self::NotSiblings => write!(formatter, "an extraction must be one run of statements in one block"),
            Self::NoScore => write!(formatter, "the piece has no `score` block"),
        }
    }
}

impl core::error::Error for EditError {}

/// One indent level, matching the formatter.
const INDENT: &str = "    ";

/// Compute the text edits that carry out `intent` against `source`.
///
/// The edits are non-overlapping and may be applied in any order with
/// [`apply_edits`]. Nothing is applied here: the caller decides whether the
/// result is acceptable, which is what makes an edit command transactional.
///
/// # Errors
///
/// Returns [`EditError`] when the intent does not describe something the
/// source contains — an offset that is not a statement, a pitch change aimed
/// at a rest, a voice that does not exist.
pub fn compute_edits(source: &str, intent: &EditIntent) -> Result<Vec<TextEdit>, EditError> {
    let document = parse(source);
    let root = document.syntax();
    match *intent {
        EditIntent::SetPitch { at, ref pitch } => set_pitch(&root, at, pitch),
        EditIntent::SetDuration { at, ref duration } => set_duration(&root, at, duration),
        EditIntent::Insert {
            ref anchor,
            ref statement,
        } => insert(&root, source, anchor, statement),
        EditIntent::ExtractMotif { first, last, ref name } => extract_motif(&root, source, first, last, name),
    }
}

/// The kinds that count as one statement of a voice or motif body.
fn is_statement(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::NoteStmt
            | SyntaxKind::RestStmt
            | SyntaxKind::ChordStmt
            | SyntaxKind::UseStmt
            | SyntaxKind::TransposeStmt
            | SyntaxKind::RepeatStmt
    )
}

/// The innermost statement covering `at`.
///
/// `at` is a statement's first significant byte, so the token there belongs
/// to it; taking the innermost ancestor keeps a `use` inside a `transpose`
/// block resolving to the `use`.
fn statement_at(root: &SyntaxNode, at: u32) -> Result<SyntaxNode, EditError> {
    let offset = TextSize::new(at);
    if !root.text_range().contains(offset) {
        return Err(EditError::NoStatement { at });
    }
    root.token_at_offset(offset)
        .right_biased()
        .and_then(|token| {
            token
                .parent_ancestors()
                .find(|node| is_statement(node.kind()) && node.text_range().contains(offset))
        })
        .ok_or(EditError::NoStatement { at })
}

/// The range of a node's significant content, skipping leading and trailing
/// trivia so an edit does not swallow the blank line before a statement.
fn trimmed(node: &SyntaxNode) -> TextRange {
    let significant: Vec<_> = node
        .children_with_tokens()
        .filter(|element| !element.kind().is_trivia())
        .collect();
    match (significant.first(), significant.last()) {
        (Some(first), Some(last)) => TextRange::new(first.text_range().start(), last.text_range().end()),
        _ => node.text_range(),
    }
}

/// The first token of `kinds` directly under `node`.
fn token_of(node: &SyntaxNode, kinds: &[SyntaxKind]) -> Option<TextRange> {
    node.children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|token| kinds.contains(&token.kind()))
        .map(|token| token.text_range())
}

fn set_pitch(root: &SyntaxNode, at: u32, pitch: &str) -> Result<Vec<TextEdit>, EditError> {
    let statement = statement_at(root, at)?;
    if statement.kind() != SyntaxKind::NoteStmt {
        return Err(EditError::NotANote { at });
    }
    // A note names its pitch either literally (`gs4`) or through a motif
    // parameter (`root`); both are the token to replace.
    let range =
        token_of(&statement, &[SyntaxKind::PitchLiteral, SyntaxKind::Identifier]).ok_or(EditError::NotANote { at })?;
    Ok(vec![TextEdit::new(range, pitch)])
}

fn set_duration(root: &SyntaxNode, at: u32, duration: &str) -> Result<Vec<TextEdit>, EditError> {
    let statement = statement_at(root, at)?;
    if !matches!(
        statement.kind(),
        SyntaxKind::NoteStmt | SyntaxKind::RestStmt | SyntaxKind::ChordStmt
    ) {
        return Err(EditError::NoDuration { at });
    }
    let range =
        token_of(&statement, &[SyntaxKind::Rational, SyntaxKind::Integer]).ok_or(EditError::NoDuration { at })?;
    Ok(vec![TextEdit::new(range, duration)])
}

/// The whitespace a line starts with, which a new line of it should match.
fn indent_at(source: &str, offset: u32) -> String {
    let offset = usize::try_from(offset).unwrap_or(source.len()).min(source.len());
    let line_start = source
        .get(..offset)
        .and_then(|before| before.rfind('\n').map(|index| index.saturating_add(1)))
        .unwrap_or(0);
    source
        .get(line_start..offset)
        .unwrap_or("")
        .chars()
        .take_while(|character| *character == ' ' || *character == '\t')
        .collect()
}

fn insert(root: &SyntaxNode, source: &str, anchor: &Anchor, statement: &Statement) -> Result<Vec<TextEdit>, EditError> {
    let text = statement.text();
    match *anchor {
        Anchor::Before { at } => {
            let range = trimmed(&statement_at(root, at)?);
            let indent = indent_at(source, u32::from(range.start()));
            let point = TextRange::empty(range.start());
            Ok(vec![TextEdit::new(point, format!("{text}\n{indent}"))])
        }
        Anchor::After { at } => {
            let range = trimmed(&statement_at(root, at)?);
            let indent = indent_at(source, u32::from(range.start()));
            let point = TextRange::empty(range.end());
            Ok(vec![TextEdit::new(point, format!("\n{indent}{text}"))])
        }
        Anchor::EndOfVoice { ref part, ref voice } => {
            let block = find_voice(root, part, voice).ok_or_else(|| EditError::NoVoice {
                part: part.clone(),
                voice: voice.clone(),
            })?;
            match block.items().last().map(|item| trimmed(item_syntax(item))) {
                // A voice that already has music: the new statement joins the
                // line after its last one, at the same indent.
                Some(range) => {
                    let indent = indent_at(source, u32::from(range.start()));
                    Ok(vec![TextEdit::new(
                        TextRange::empty(range.end()),
                        format!("\n{indent}{text}"),
                    )])
                }
                // An empty voice: one level in from the `voice` keyword.
                None => {
                    let node = block.syntax();
                    let brace = node
                        .descendants_with_tokens()
                        .filter_map(SyntaxElement::into_token)
                        .find(|token| token.kind() == SyntaxKind::LBrace)
                        .map_or_else(|| TextRange::empty(node.text_range().end()), |token| token.text_range());
                    let indent = indent_at(source, u32::from(node.text_range().start()));
                    Ok(vec![TextEdit::new(
                        TextRange::empty(brace.end()),
                        format!("\n{indent}{INDENT}{text}"),
                    )])
                }
            }
        }
    }
}

/// The untyped node behind a voice item.
fn item_syntax(item: &crate::ast::VoiceItem) -> &SyntaxNode {
    match *item {
        crate::ast::VoiceItem::Note(ref it) => it.syntax(),
        crate::ast::VoiceItem::Rest(ref it) => it.syntax(),
        crate::ast::VoiceItem::Chord(ref it) => it.syntax(),
        crate::ast::VoiceItem::Use(ref it) => it.syntax(),
        crate::ast::VoiceItem::Transpose(ref it) => it.syntax(),
        crate::ast::VoiceItem::Repeat(ref it) => it.syntax(),
        crate::ast::VoiceItem::Slur(ref it) => it.syntax(),
        crate::ast::VoiceItem::Dynamic(ref it) => it.syntax(),
        crate::ast::VoiceItem::Tuplet(ref it) => it.syntax(),
    }
}

fn find_voice(root: &SyntaxNode, part: &str, voice: &str) -> Option<VoiceDecl> {
    let piece = PieceDecl::from_root(root)?;
    let score: ScoreDecl = piece.score()?;
    score
        .parts()
        .into_iter()
        .find(|declared| declared.name().as_deref() == Some(part))?
        .voices()
        .into_iter()
        .find(|declared| declared.name().as_deref() == Some(voice))
}

fn extract_motif(
    root: &SyntaxNode,
    source: &str,
    first: u32,
    last: u32,
    name: &str,
) -> Result<Vec<TextEdit>, EditError> {
    let head = statement_at(root, first)?;
    let tail = statement_at(root, last)?;
    if head.parent() != tail.parent() {
        return Err(EditError::NotSiblings);
    }
    let head = trimmed(&head);
    let tail = trimmed(&tail);
    if head.start() > tail.start() {
        return Err(EditError::NotSiblings);
    }
    let lifted = TextRange::new(head.start(), tail.end());
    let body = source
        .get(usize::from(lifted.start())..usize::from(lifted.end()))
        .ok_or(EditError::NotSiblings)?;

    let piece = PieceDecl::from_root(root).ok_or(EditError::NoScore)?;
    let score = piece.score().ok_or(EditError::NoScore)?;
    let at = trimmed(score.syntax()).start();
    if at > lifted.start() {
        return Err(EditError::NotSiblings);
    }
    let outer = indent_at(source, u32::from(at));
    let inner = format!("{outer}{INDENT}");

    // The lifted statements were indented for wherever they sat; inside the
    // motif they are one level in from the declaration, and their relative
    // shape is kept by re-indenting whole lines rather than reflowing.
    let was = indent_at(source, u32::from(head.start()));
    let mut declaration = format!("motif {name}() {{\n");
    for line in body.lines() {
        let stripped = line.strip_prefix(was.as_str()).unwrap_or_else(|| line.trim_start());
        declaration.push_str(&inner);
        declaration.push_str(stripped);
        declaration.push('\n');
    }
    declaration.push_str(&outer);
    declaration.push_str("}\n\n");
    declaration.push_str(&outer);

    Ok(vec![
        TextEdit::new(TextRange::empty(at), declaration),
        TextEdit::new(lifted, format!("use {name}();")),
    ])
}
