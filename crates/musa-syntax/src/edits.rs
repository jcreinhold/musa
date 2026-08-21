//! Syntax-aware text edits (roadmap §11): the project's canonical mutation
//! mechanism. Score-editing commands resolve to these.
//!
//! There is one entry point, [`compute_edits`]. A caller says what it wants
//! done to the music in [`EditIntent`] terms and gets back the text edits
//! that do it; it never reasons about tokens, indentation, or where a
//! statement ends. That keeps the syntax knowledge in the crate that owns the
//! syntax, and keeps the project layer free to be about provenance.

use text_size::{TextRange, TextSize};

use crate::SyntaxKind;
use crate::ast::{AstNode, PieceDecl, ScoreDecl, VoiceDecl, quote, unquote};
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
    /// `g#4/8`
    Note {
        /// Written pitch, as it is spelled (`g#4`, `bb3`).
        pitch: String,
        /// Notated duration (`1/8`, `3/8`, `1`).
        duration: String,
    },
    /// `rest/4`
    Rest {
        /// Notated duration.
        duration: String,
    },
    /// `[a3 c4 e4]/2`
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
            } => format!("{pitch}{}", spell_duration(duration)),
            Self::Rest { ref duration } => format!("rest{}", spell_duration(duration)),
            Self::Chord {
                ref pitches,
                ref duration,
            } => format!("[{}]{}", pitches.join(" "), spell_duration(duration)),
        }
    }
}

/// A notated duration, written the way a composer writes it on a note.
///
/// `1/4` is `/4`, `3/8` is `/4.` and `7/16` is `/4..` — the short form says
/// the note value and the dots, which is what the notation says. A duration
/// the short form cannot spell is written long with a space in front of it,
/// so a tuplet member (`1/12`) and a value only a tie can write (`5/8`) still
/// come out as something the parser reads back.
///
/// The result carries whatever separates it from the pitch, so `format!
/// ("{pitch}{}", spell_duration(d))` is a note however the duration spells.
/// Public because the compiler's bar-length fix offers a filling rest, and
/// two spellings of one duration must not disagree.
pub fn spell_duration(written: &str) -> String {
    let Some((numerator, denominator)) = fraction(written) else {
        return format!(" {written}");
    };
    // `/N` with `d` dots is `(2^(d+1) - 1) / (N · 2^d)`, so the numerator says
    // how many dots there are and the denominator says which note value they
    // are written on.
    for dots in 0..=MAX_DOTS {
        let scale = 1u64 << dots;
        let expected = (2u64 << dots).saturating_sub(1);
        if numerator != expected {
            continue;
        }
        if denominator.checked_rem(scale) != Some(0) {
            continue;
        }
        let Some(value) = denominator.checked_div(scale) else {
            continue;
        };
        return format!("/{value}{}", ".".repeat(usize::from(dots)));
    }
    format!(" {written}")
}

/// How many augmentation dots the short form will write. Three is already
/// past what an engraver prints.
const MAX_DOTS: u8 = 3;

/// A written duration as a fraction, or `None` when it is a parameter's name
/// or something else the short form has no opinion about.
fn fraction(written: &str) -> Option<(u64, u64)> {
    match written.split_once('/') {
        Some((numerator, denominator)) => Some((numerator.trim().parse().ok()?, denominator.trim().parse().ok()?)),
        None => Some((written.trim().parse().ok()?, 1)),
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

/// One of the piece's own header statements.
///
/// These are the facts a piece states about itself rather than about its
/// music, and they are the ones the interface prints in places a composer can
/// point at: the frame's title, the engraved page's head and foot, the
/// transport band's tempo, key, and meter.
///
/// The order of the variants is the order the statements are written in, and
/// it is load-bearing: it is what decides where a statement the piece does
/// not have yet gets inserted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeaderField {
    /// The name in `piece "…"`. It is the only one that cannot be emptied.
    Title,
    /// `subtitle "…";`
    Subtitle,
    /// `composer "…";`
    Composer,
    /// `arranger "…";`
    Arranger,
    /// `copyright "…";`
    Copyright,
    /// `tempo quarter = 72;` — the value is everything after the keyword.
    Tempo,
    /// `meter 4/4;`
    Meter,
    /// `key a minor;`
    Key,
}

impl HeaderField {
    /// Every field, in the order they are written.
    pub const ALL: [Self; 8] = [
        Self::Title,
        Self::Subtitle,
        Self::Composer,
        Self::Arranger,
        Self::Copyright,
        Self::Tempo,
        Self::Meter,
        Self::Key,
    ];

    /// The keyword that opens the statement, and the name a message calls it.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Subtitle => "subtitle",
            Self::Composer => "composer",
            Self::Arranger => "arranger",
            Self::Copyright => "copyright",
            Self::Tempo => "tempo",
            Self::Meter => "meter",
            Self::Key => "key",
        }
    }

    /// Whether the value is written as a quoted string.
    ///
    /// The four front-matter roles carry prose a composer typed and are
    /// quoted; `tempo`, `meter`, and `key` carry musa's own notation and are
    /// written bare, which is also why an unparseable one has to come back as
    /// a compiler diagnostic rather than as a field-level complaint.
    #[must_use]
    pub fn is_quoted(self) -> bool {
        matches!(
            self,
            Self::Title | Self::Subtitle | Self::Composer | Self::Arranger | Self::Copyright
        )
    }

    /// The node kind the statement parses to, for the fields that have one.
    fn kind(self) -> Option<SyntaxKind> {
        match self {
            Self::Title => None,
            Self::Subtitle | Self::Composer | Self::Arranger | Self::Copyright => Some(SyntaxKind::FrontMatterStmt),
            Self::Tempo => Some(SyntaxKind::TempoStmt),
            Self::Meter => Some(SyntaxKind::MeterStmt),
            Self::Key => Some(SyntaxKind::KeyStmt),
        }
    }

    /// The keyword token that opens the statement, for the fields that have
    /// one — this is what tells two `FrontMatterStmt`s apart.
    fn keyword(self) -> Option<SyntaxKind> {
        match self {
            Self::Title => None,
            Self::Subtitle => Some(SyntaxKind::SubtitleKw),
            Self::Composer => Some(SyntaxKind::ComposerKw),
            Self::Arranger => Some(SyntaxKind::ArrangerKw),
            Self::Copyright => Some(SyntaxKind::CopyrightKw),
            Self::Tempo => Some(SyntaxKind::TempoKw),
            Self::Meter => Some(SyntaxKind::MeterKw),
            Self::Key => Some(SyntaxKind::KeyKw),
        }
    }
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
    /// Respell one note of one motif occurrence, by adding or merging a
    /// `with { note <n> = <pitch>; }` clause on the `use` at `at`
    /// (roadmap §9).
    Specialize {
        /// The `use` statement's offset.
        at: u32,
        /// Which note of that occurrence, counting from one — the same
        /// number the score inspector shows.
        position: u32,
        /// The new written pitch.
        pitch: String,
    },
    /// Set, add, or remove one of the piece's header statements.
    ///
    /// An empty `value` removes the statement — adding and removing a line of
    /// front matter are the same gesture, so they are the same intent.
    SetHeader {
        /// Which statement.
        field: HeaderField,
        /// Its new value, unquoted and unescaped as the composer typed it.
        value: String,
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
    /// The statement there is not a motif occurrence, so it has no notes of
    /// its own to specialize.
    NotAnOccurrence {
        /// The offset of the statement that was found instead.
        at: u32,
    },
    /// The two ends of an extraction are not statements of the same block.
    NotSiblings,
    /// The document has no `score` block to place a motif before.
    NoScore,
    /// The document has no `piece` declaration to carry a header statement.
    NoPiece,
    /// A piece must be called something.
    CannotEmptyTitle,
}

impl core::fmt::Display for EditError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NoStatement { at } => write!(formatter, "no statement at byte {at}"),
            Self::NotANote { at } => write!(formatter, "the statement at byte {at} is not a note"),
            Self::NoDuration { at } => write!(formatter, "the statement at byte {at} has no duration"),
            Self::NoVoice { ref part, ref voice } => write!(formatter, "no voice `{voice}` in part `{part}`"),
            Self::NotAnOccurrence { at } => {
                write!(formatter, "the statement at byte {at} is not a motif occurrence")
            }
            Self::NotSiblings => write!(formatter, "an extraction must be one run of statements in one block"),
            Self::NoScore => write!(formatter, "the piece has no `score` block"),
            Self::NoPiece => write!(formatter, "the document has no `piece`"),
            Self::CannotEmptyTitle => write!(formatter, "a piece has to be called something"),
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
        EditIntent::Specialize {
            at,
            position,
            ref pitch,
        } => specialize(&root, at, position, pitch),
        EditIntent::SetHeader { field, ref value } => set_header(&root, source, field, value),
        EditIntent::ExtractMotif { first, last, ref name } => extract_motif(&root, source, first, last, name),
    }
}

/// What the piece says for one header field, spelled the way it says it.
///
/// `None` when the piece has no such statement — which is a fact worth having
/// rather than a blank, because the interface shows a field for a role the
/// piece has not filled in and that empty row is where a composer discovers
/// they can fill it.
///
/// The spelling is the source's, not a rendering of it: `quarter = 72`, not
/// `♩ = 72`; `a minor`, not `A minor`. A field that read one dialect and wrote
/// another would be a second language to keep working, and the value here is
/// exactly what [`EditIntent::SetHeader`] takes back.
#[must_use]
pub fn read_header(source: &str, field: HeaderField) -> Option<String> {
    let document = parse(source);
    let piece = document
        .syntax()
        .children()
        .find(|node| node.kind() == SyntaxKind::PieceDecl)?;

    if field == HeaderField::Title {
        let range = token_of(&piece, &[SyntaxKind::String])?;
        return slice(source, range).map(unquote);
    }
    let statement = header_statement(&piece, field)?;
    let written = slice(source, header_value_range(&statement, field)?)?.trim();
    Some(if field.is_quoted() {
        unquote(written)
    } else {
        written.to_owned()
    })
}

fn slice(source: &str, range: TextRange) -> Option<&str> {
    source.get(usize::from(range.start())..usize::from(range.end()))
}

/// Add or merge one `note <n> = <pitch>;` override on the `use` at `at`.
///
/// Three cases, and the difference between them is only punctuation: a call
/// with no clause grows one in place of its `;`, a clause that already
/// respells this note has that pitch replaced, and any other clause gains an
/// override in position order — so a composer who specializes three notes
/// reads them back in the order they are played.
fn specialize(root: &SyntaxNode, at: u32, position: u32, pitch: &str) -> Result<Vec<TextEdit>, EditError> {
    let statement = statement_at(root, at)?;
    if statement.kind() != SyntaxKind::UseStmt {
        return Err(EditError::NotAnOccurrence { at });
    }
    let written = format!("note {position} = {pitch};");
    let Some(clause) = statement
        .children()
        .find(|child| child.kind() == SyntaxKind::WithClause)
    else {
        // `use sigh();` → `use sigh() with { note 2 = d5; }`. The call stops
        // being a statement and becomes a block, so its `;` goes with it.
        let semicolon = token_of(&statement, &[SyntaxKind::Semicolon])
            .unwrap_or_else(|| TextRange::empty(trimmed(&statement).end()));
        return Ok(vec![TextEdit::new(semicolon, format!(" with {{ {written} }}"))]);
    };

    let overrides: Vec<SyntaxNode> = clause
        .children()
        .filter(|child| child.kind() == SyntaxKind::OverrideStmt)
        .collect();
    let position_of = |node: &SyntaxNode| {
        node.children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Integer)
            .and_then(|token| token.text().parse::<u32>().ok())
    };
    if let Some(existing) = overrides.iter().find(|node| position_of(node) == Some(position)) {
        let range = token_of(existing, &[SyntaxKind::PitchLiteral]).ok_or(EditError::NotAnOccurrence { at })?;
        return Ok(vec![TextEdit::new(range, pitch.to_owned())]);
    }
    match overrides
        .iter()
        .find(|node| position_of(node).is_some_and(|existing| existing > position))
    {
        Some(later) => Ok(vec![TextEdit::new(
            TextRange::empty(trimmed(later).start()),
            format!("{written} "),
        )]),
        None => {
            let last = overrides.last().map(|node| trimmed(node).end());
            let brace = token_of(&clause, &[SyntaxKind::LBrace]).map(TextRange::end);
            let point = last.or(brace).ok_or(EditError::NotAnOccurrence { at })?;
            Ok(vec![TextEdit::new(TextRange::empty(point), format!(" {written}"))])
        }
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

/// Set, add, or remove one header statement.
///
/// Three cases, and which one applies is a fact about the source rather than
/// something the caller has to know: a statement the piece has is rewritten
/// in place, a statement it does not have is inserted where the order says it
/// goes, and an empty value removes it. That is what lets one field in the
/// interface both write a composer's name and take it back off the page.
///
/// Only the *value* is rewritten, never the whole line: a piece whose tempo
/// carries a trailing comment keeps its comment.
fn set_header(root: &SyntaxNode, source: &str, field: HeaderField, value: &str) -> Result<Vec<TextEdit>, EditError> {
    let piece = root
        .children()
        .find(|node| node.kind() == SyntaxKind::PieceDecl)
        .ok_or(EditError::NoPiece)?;

    if field == HeaderField::Title {
        if value.is_empty() {
            return Err(EditError::CannotEmptyTitle);
        }
        let range = token_of(&piece, &[SyntaxKind::String]).ok_or(EditError::NoPiece)?;
        return Ok(vec![TextEdit::new(range, quote(value))]);
    }

    match header_statement(&piece, field) {
        Some(statement) if value.is_empty() => Ok(vec![TextEdit::new(with_leading_blank(source, &statement), "")]),
        Some(statement) => {
            let range = header_value_range(&statement, field).ok_or(EditError::NoPiece)?;
            Ok(vec![TextEdit::new(range, written_value(field, value))])
        }
        // Removing what is not there is not an error, it is what the composer
        // asked for: the field is already empty.
        None if value.is_empty() => Ok(Vec::new()),
        None => Ok(vec![insert_header(&piece, source, field, value)]),
    }
}

/// The piece's statement for `field`, if it has one.
///
/// Written twice, the first wins here — the resolver already refuses the
/// document, and rewriting the first is what a composer looking at the page
/// would expect either way.
fn header_statement(piece: &SyntaxNode, field: HeaderField) -> Option<SyntaxNode> {
    let (kind, keyword) = (field.kind()?, field.keyword()?);
    piece
        .children()
        .filter(|node| node.kind() == kind)
        .find(|node| token_of(node, &[keyword]).is_some())
}

/// The part of a header statement an edit replaces: everything between its
/// keyword and its `;`, trimmed.
///
/// Stated as a span rather than as a token because `tempo quarter = 72` and
/// `key a minor` are several tokens and one value, and a field that could
/// only replace single tokens would have to know which fields those were.
fn header_value_range(statement: &SyntaxNode, field: HeaderField) -> Option<TextRange> {
    let keyword = token_of(statement, &[field.keyword()?])?;
    let end = token_of(statement, &[SyntaxKind::Semicolon]).map_or_else(|| trimmed(statement).end(), TextRange::start);
    let start = statement
        .children_with_tokens()
        .filter(|element| !element.kind().is_trivia())
        .map(|element| element.text_range())
        .find(|range| range.start() >= keyword.end())
        .map_or(end, TextRange::start);
    (start <= end).then(|| TextRange::new(start, end))
}

/// The text a value is written as: quoted for the prose roles, bare for
/// musa's own notation.
fn written_value(field: HeaderField, value: &str) -> String {
    if field.is_quoted() {
        quote(value)
    } else {
        value.to_owned()
    }
}

/// A statement's range together with the line it sits on, so deleting it
/// leaves no blank line behind.
fn with_leading_blank(source: &str, statement: &SyntaxNode) -> TextRange {
    let range = trimmed(statement);
    let start = usize::from(range.start());
    let line_start = source
        .get(..start)
        .and_then(|before| before.rfind('\n'))
        .map_or(0, |index| index.saturating_add(1));
    let indent_only = source
        .get(line_start..start)
        .is_some_and(|text| text.chars().all(char::is_whitespace));
    let from = if indent_only && line_start > 0 {
        line_start.saturating_sub(1)
    } else {
        start
    };
    TextRange::new(TextSize::new(u32::try_from(from).unwrap_or(0)), range.end())
}

/// Where a statement the piece does not have yet goes.
///
/// After the last header statement that comes before it, or — when the piece
/// states nothing at all yet — on the line after the opening brace. Order is
/// [`HeaderField::ALL`], which is the order the formatter leaves a piece in,
/// so a composer who adds a subtitle after a composer still reads them back
/// the way an edition prints them.
fn insert_header(piece: &SyntaxNode, source: &str, field: HeaderField, value: &str) -> TextEdit {
    let written = format!("{} {};", field.word(), written_value(field, value));
    let earlier = HeaderField::ALL
        .iter()
        .take_while(|candidate| **candidate < field)
        .filter_map(|candidate| header_statement(piece, *candidate))
        .map(|statement| trimmed(&statement))
        .max_by_key(|range| range.end());

    if let Some(range) = earlier {
        let indent = indent_at(source, u32::from(range.start()));
        return TextEdit::new(TextRange::empty(range.end()), format!("\n{indent}{written}"));
    }

    // Nothing to follow: the line after `{`, at one indent. The blank line
    // after it is what keeps the piece's own facts a block of their own, the
    // way every example in `examples/` sets them.
    let brace = token_of(piece, &[SyntaxKind::LBrace]).unwrap_or_else(|| TextRange::empty(trimmed(piece).start()));
    let indent = format!("{}{INDENT}", indent_at(source, u32::from(trimmed(piece).start())));
    TextEdit::new(TextRange::empty(brace.end()), format!("\n{indent}{written}\n"))
}

fn set_pitch(root: &SyntaxNode, at: u32, pitch: &str) -> Result<Vec<TextEdit>, EditError> {
    let statement = statement_at(root, at)?;
    if statement.kind() != SyntaxKind::NoteStmt {
        return Err(EditError::NotANote { at });
    }
    // A note names its pitch either literally (`g#4`) or through a motif
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
    let range = duration_range(&statement).ok_or(EditError::NoDuration { at })?;
    Ok(vec![TextEdit::new(range, spell_duration(duration))])
}

/// Where a statement's written duration is, so that setting it replaces the
/// whole of it and nothing else.
///
/// The range covers the value and stops at `to`: rewriting `1/4` in
/// `g4 1/4 to 2/1` must leave the performer's bound alone. Taking the range
/// from the `Duration` node rather than from the first numeral is what makes
/// the short form safe — the `4` in `c4/4` is a numeral like any other, and
/// a token search would find the one in the pitch.
///
/// It starts at the *end of the token before it*, so the space in `c4 1/4`
/// belongs to the edit. That is what lets one replacement write either form:
/// `/4` closes the gap and ` 5/8` opens one.
fn duration_range(statement: &SyntaxNode) -> Option<TextRange> {
    let node = statement
        .children()
        .find(|child| child.kind() == SyntaxKind::Duration)?;
    let mut end = None;
    for token in node.children_with_tokens().filter_map(SyntaxElement::into_token) {
        if token.kind() == SyntaxKind::ToKw {
            break;
        }
        if token.kind().is_trivia() {
            continue;
        }
        end = Some(token.text_range().end());
    }
    let mut start = node.text_range().start();
    let mut before = node.prev_sibling_or_token();
    while let Some(element) = before {
        if !element.kind().is_trivia() {
            break;
        }
        start = element.text_range().start();
        before = element.prev_sibling_or_token();
    }
    Some(TextRange::new(start, end?))
}

/// Whether a statement is written inside a bar, and so on a line shared with
/// the rest of that bar.
fn in_a_bar(statement: &SyntaxNode) -> bool {
    statement.parent().is_some_and(|parent| {
        // A `|` bar holds its items directly; a `bar { … }` holds a block.
        parent.kind() == SyntaxKind::BarStmt
            || (parent.kind() == SyntaxKind::Block
                && parent.parent().is_some_and(|above| above.kind() == SyntaxKind::BarStmt))
    })
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
            let statement = statement_at(root, at)?;
            let range = trimmed(&statement);
            let point = TextRange::empty(range.start());
            if in_a_bar(&statement) {
                return Ok(vec![TextEdit::new(point, format!("{text} "))]);
            }
            let indent = indent_at(source, u32::from(range.start()));
            Ok(vec![TextEdit::new(point, format!("{text}\n{indent}"))])
        }
        Anchor::After { at } => {
            let statement = statement_at(root, at)?;
            let range = trimmed(&statement);
            let point = TextRange::empty(range.end());
            // A bar is written on one line, so a note joins the line it
            // belongs to rather than splitting the bar in two.
            if in_a_bar(&statement) {
                return Ok(vec![TextEdit::new(point, format!(" {text}"))]);
            }
            let indent = indent_at(source, u32::from(range.start()));
            Ok(vec![TextEdit::new(point, format!("\n{indent}{text}"))])
        }
        Anchor::EndOfVoice { ref part, ref voice } => {
            let block = find_voice(root, part, voice).ok_or_else(|| EditError::NoVoice {
                part: part.clone(),
                voice: voice.clone(),
            })?;
            match block.items().last() {
                // A voice whose last thing is a bar: the note joins that bar,
                // after everything already written in it.
                Some(crate::ast::VoiceItem::Bar(bar)) => {
                    let end = bar
                        .content_end()
                        .map_or_else(|| trimmed(bar.syntax()).end(), TextSize::from);
                    Ok(vec![TextEdit::new(TextRange::empty(end), format!(" {text}"))])
                }
                // A voice that already has music: the new statement joins the
                // line after its last one, at the same indent.
                Some(item) => {
                    let range = trimmed(item_syntax(item));
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
        crate::ast::VoiceItem::InScale(ref it) => it.syntax(),
        crate::ast::VoiceItem::Stack(ref it) => it.syntax(),
        crate::ast::VoiceItem::Transpose(ref it) => it.syntax(),
        crate::ast::VoiceItem::Repeat(ref it) => it.syntax(),
        crate::ast::VoiceItem::Bar(ref it) => it.syntax(),
        crate::ast::VoiceItem::Assert(ref it) => it.syntax(),
        crate::ast::VoiceItem::Senza(ref it) => it.syntax(),
        crate::ast::VoiceItem::Ending(ref it) => it.syntax(),
        crate::ast::VoiceItem::Slur(ref it) => it.syntax(),
        crate::ast::VoiceItem::Dynamic(ref it) => it.syntax(),
        crate::ast::VoiceItem::Tuplet(ref it) => it.syntax(),
        crate::ast::VoiceItem::Stretch(ref it) => it.syntax(),
        crate::ast::VoiceItem::Retrograde(ref it) => it.syntax(),
        crate::ast::VoiceItem::Invert(ref it) => it.syntax(),
        crate::ast::VoiceItem::Phrase(ref it) => it.syntax(),
        crate::ast::VoiceItem::Mark(ref it) => it.syntax(),
        crate::ast::VoiceItem::Grace(ref it) => it.syntax(),
        crate::ast::VoiceItem::Hairpin(ref it) => it.syntax(),
        crate::ast::VoiceItem::Tempo(ref it) => it.syntax(),
        crate::ast::VoiceItem::Meter(ref it) => it.syntax(),
        crate::ast::VoiceItem::Key(ref it) => it.syntax(),
        crate::ast::VoiceItem::Clef(ref it) => it.syntax(),
        crate::ast::VoiceItem::Mobile(ref it) => it.syntax(),
        crate::ast::VoiceItem::Improvise(ref it) => it.syntax(),
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

/// What a statement belongs to, looking past the bar it is written in.
///
/// Two notes in different bars of one voice are siblings for the purpose of
/// lifting them into a motif: a barline is a mark inside the voice, not a
/// scope. Comparing parents directly would refuse the normal case the moment
/// the music has barlines at all.
fn enclosing_run(statement: &SyntaxNode) -> Option<SyntaxNode> {
    let mut node = statement.parent()?;
    loop {
        let in_a_bar = node.kind() == SyntaxKind::BarStmt
            || (node.kind() == SyntaxKind::Block
                && node.parent().is_some_and(|above| above.kind() == SyntaxKind::BarStmt));
        if !in_a_bar {
            return Some(node);
        }
        node = node.parent()?;
    }
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
    if enclosing_run(&head) != enclosing_run(&tail) {
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
