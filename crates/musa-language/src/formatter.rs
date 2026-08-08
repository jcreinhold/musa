//! The lossless formatter (roadmap §11).
//!
//! Operates on the concrete syntax tree — never on any semantic model — and
//! rewrites only whitespace. Rules (encoded and reviewed as insta snapshots):
//!
//! - 4-space indent per block level; `{` stays on the declaration line; `}`
//!   on its own line.
//! - One statement per line, terminated by `;`.
//! - Blank lines from the original are preserved, capped at one.
//! - A comment that trailed code on its line stays trailing; an own-line
//!   comment stays attached above the construct it precedes.
//! - A `bar` that fits the source measure is written on one line. The one
//!   exception, and see [`MEASURE`] for why it earns itself.
//!
//! Laws (tested as properties): `format` is idempotent, and
//! `parse(format(parse(source)))` equals `parse(source)` up to whitespace.

use crate::language::SyntaxNode;
use crate::{ParsedDocument, SyntaxElement, SyntaxKind};

/// The result of formatting a document.
pub struct FormattedSource {
    text: String,
}

impl FormattedSource {
    /// The formatted text, ending in exactly one newline.
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl std::fmt::Display for FormattedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

/// Format a parsed document. Lossless: every comment and token survives;
/// only whitespace trivia is normalized.
pub fn format(document: &ParsedDocument) -> FormattedSource {
    let mut writer = Writer::new();
    format_node(&document.syntax(), &mut writer);
    FormattedSource { text: writer.finish() }
}

/// How wide a line a bar may keep, indent included.
///
/// Everything else in musa is a short statement on its own line. A bar is the
/// one statement that is naturally horizontal, because that is the direction
/// music is read in, and a bar you can select with a double-click and drag
/// into the next voice is the difference between the brace being worth typing
/// and not. So a bar keeps its line, and breaks like any other block only when
/// it is genuinely long.
///
/// 96, not the source column's 48-character measure
/// (`01-visual-language.md` §8). A bar sits four levels in, so sixteen
/// characters are spent before the word `bar`, and a measure of four quarter
/// notes is forty more; budgeting a bar to the column would break every bar
/// there is, which is the same as not having the rule. What 96 buys is that a
/// bar of eight eighths — an ordinary bar — stays one line, and a bar long
/// enough to need scrolling to read is a bar long enough to stack. The cost is
/// that a barred piece has lines past the column, and the column scrolls them,
/// which is what it already does for comment prose and long signal chains.
const MEASURE: usize = 96;

fn format_node(node: &SyntaxNode, writer: &mut Writer) {
    // Set once the first token of a one-word construct has been written, so
    // the rest of it joins on without a space.
    let mut tight = false;
    for element in node.children_with_tokens() {
        match element {
            SyntaxElement::Node(child) => {
                writer.blank_line_if_pending();
                if let Some(line) = inline_bar(&child, writer.indent) {
                    writer.write_line(&line);
                    continue;
                }
                let wrap = wraps_across_lines(&child);
                if wrap {
                    writer.open_chain();
                }
                format_node(&child, writer);
                if wrap {
                    writer.close_chain();
                }
            }
            SyntaxElement::Token(token) => {
                let kind = token.kind();
                if kind == SyntaxKind::Whitespace {
                    writer.note_whitespace(token.text());
                    continue;
                }
                writer.blank_line_if_pending();
                // Some constructs are one word with punctuation in them: a
                // `measure:beat` coordinate is written `3:1` the way a bar
                // number is, and a chord symbol is `fmaj7` however many
                // tokens it happens to lex as. Their insides take no spaces.
                if matches!(node.kind(), SyntaxKind::Position | SyntaxKind::ChordSymbol) {
                    writer.write_word(kind, token.text(), tight);
                    tight = true;
                    continue;
                }
                format_token(kind, token.text(), writer);
            }
        }
    }
}

fn format_token(kind: SyntaxKind, text: &str, writer: &mut Writer) {
    if kind == SyntaxKind::LineComment || kind == SyntaxKind::BlockComment {
        writer.comment(text);
        return;
    }
    writer.prep_line();
    if kind == SyntaxKind::LBrace {
        writer.space();
        writer.write("{");
        writer.indent_more();
        writer.end_line();
    } else if kind == SyntaxKind::RBrace {
        writer.indent_less();
        writer.write("}");
        writer.end_line();
    } else if kind == SyntaxKind::Semicolon {
        writer.write(";");
        writer.end_line();
    } else if kind == SyntaxKind::Comma {
        writer.write(",");
        writer.space();
    } else if kind == SyntaxKind::Equals {
        writer.space();
        writer.write("=");
        writer.space();
    } else if kind == SyntaxKind::Colon {
        writer.write(":");
        writer.space();
    } else if kind == SyntaxKind::PipeForward && writer.in_wrapped_chain() {
        // A long chain reads as a stack of stages, which is how the roadmap
        // writes it and how a patch is actually thought about.
        writer.indent_continuations();
        writer.newline();
        writer.write("|>");
        writer.space();
    } else if kind == SyntaxKind::Arrow || kind == SyntaxKind::PipeForward {
        writer.space();
        writer.write(text);
        writer.space();
    } else if kind == SyntaxKind::LBracket {
        // `chord [` takes a space; `use sigh(` does not.
        if writer.needs_word_space() {
            writer.space();
        }
        writer.write(text);
    } else if kind == SyntaxKind::LParen || kind == SyntaxKind::RBracket || kind == SyntaxKind::RParen {
        writer.write(text);
    } else if kind == SyntaxKind::Dot {
        // A modulation path is one word with dots in it, not three words.
        writer.write(".");
    } else {
        if writer.needs_word_space() {
            writer.space();
        }
        writer.write(text);
    }
    writer.after_significant(kind);
}

/// Layout state for the formatter.
struct Writer {
    out: String,
    indent: usize,
    at_line_start: bool,
    /// A line break is owed before the next token (after `;`, `{`, `}`,
    /// or a comment). Deferred so a trailing comment can join the line first.
    need_newline: bool,
    /// Newlines seen in the whitespace run just passed.
    pending_newlines: usize,
    /// Kind of the last significant token written.
    prev: Option<SyntaxKind>,
    /// One entry per enclosing stacked chain: whether its continuation
    /// indent has been applied yet.
    chains: Vec<bool>,
}

/// A bar written on one line, when it is a bar and the line fits.
///
/// Returns `None` for anything that is not a `bar`, for a bar carrying a
/// comment — a comment wants a line of its own and one line has nowhere to put
/// it — and for a bar too wide for [`MEASURE`] at this indent, which falls back
/// to the way every other block breaks.
fn inline_bar(node: &SyntaxNode, indent: usize) -> Option<String> {
    if node.kind() != SyntaxKind::BarStmt {
        return None;
    }
    let mut line = String::new();
    let mut prev: Option<SyntaxKind> = None;
    for element in node.descendants_with_tokens() {
        let SyntaxElement::Token(token) = element else { continue };
        let kind = token.kind();
        if kind == SyntaxKind::Whitespace {
            continue;
        }
        if kind == SyntaxKind::LineComment || kind == SyntaxKind::BlockComment {
            return None;
        }
        if spaced_before(kind, prev) {
            line.push(' ');
        }
        line.push_str(token.text());
        prev = Some(kind);
    }
    (indent.saturating_add(line.chars().count()) <= MEASURE).then_some(line)
}

/// Whether a token takes a space in front of it on a bar's one line.
///
/// The same spacing `format_token` writes, stated as one rule instead of as a
/// sequence of writes: some tokens close up to what is before them, and some
/// tokens close up whatever comes after.
fn spaced_before(kind: SyntaxKind, prev: Option<SyntaxKind>) -> bool {
    let Some(prev) = prev else { return false };
    let closes_left = matches!(
        kind,
        SyntaxKind::Semicolon
            | SyntaxKind::Comma
            | SyntaxKind::LParen
            | SyntaxKind::RParen
            | SyntaxKind::RBracket
            | SyntaxKind::Dot
    );
    let closes_right = matches!(
        prev,
        SyntaxKind::LParen | SyntaxKind::LBracket | SyntaxKind::Minus | SyntaxKind::Dot
    );
    !closes_left && !closes_right
}

/// Whether a signal chain is long enough to be worth stacking.
///
/// Two stages (`oscillator(sine) |> gain(-15 dB)`) read fine on one line;
/// three or more is where a patch stops being a phrase and starts being a
/// signal path, and the roadmap's §7.1 example writes those stacked.
fn wraps_across_lines(node: &SyntaxNode) -> bool {
    node.kind() == SyntaxKind::SignalChain
        && node
            .children_with_tokens()
            .filter(|element| element.kind() == SyntaxKind::PipeForward)
            .count()
            >= 2
}

impl Writer {
    fn new() -> Self {
        Self {
            out: String::new(),
            indent: 0,
            at_line_start: true,
            need_newline: false,
            pending_newlines: 0,
            prev: None,
            chains: Vec::new(),
        }
    }

    /// Enter a chain that is written one stage per line.
    fn open_chain(&mut self) {
        self.chains.push(false);
    }

    fn close_chain(&mut self) {
        if self.chains.pop() == Some(true) {
            self.indent_less();
        }
    }

    fn in_wrapped_chain(&self) -> bool {
        !self.chains.is_empty()
    }

    /// Indent the continuation lines — once per chain, and only when the
    /// first `|>` proves there will be any. The chain's head stays on the
    /// line its statement started.
    fn indent_continuations(&mut self) {
        if let Some(indented) = self.chains.last_mut()
            && !*indented
        {
            *indented = true;
            self.indent_more();
        }
    }

    fn finish(mut self) -> String {
        while self.out.ends_with('\n') {
            self.out.pop();
        }
        self.out.push('\n');
        self.out
    }

    /// Emit a deferred line break, if one is owed.
    fn prep_line(&mut self) {
        if self.need_newline {
            self.newline();
            self.need_newline = false;
        }
    }

    fn end_line(&mut self) {
        self.need_newline = true;
    }

    fn write(&mut self, text: &str) {
        if self.at_line_start {
            for _ in 0..self.indent {
                self.out.push(' ');
            }
            self.at_line_start = false;
        }
        self.out.push_str(text);
    }

    /// Write one token of a construct that is spelled as a single word:
    /// spaced from what came before unless it is joining a word already
    /// started.
    fn write_word(&mut self, kind: SyntaxKind, text: &str, joining: bool) {
        self.prep_line();
        if !joining && self.needs_word_space() {
            self.space();
        }
        self.write(text);
        self.after_significant(kind);
    }

    /// Write a whole construct that was rendered elsewhere, as its own line.
    fn write_line(&mut self, text: &str) {
        self.prep_line();
        self.write(text);
        self.end_line();
        self.after_significant(SyntaxKind::RBrace);
    }

    fn space(&mut self) {
        if !self.at_line_start {
            self.out.push(' ');
        }
    }

    fn newline(&mut self) {
        if !self.at_line_start {
            self.out.push('\n');
            self.at_line_start = true;
        }
    }

    /// A comment that trailed code in the original (no newline before it)
    /// stays trailing; an own-line comment writes at the current indent.
    fn comment(&mut self, text: &str) {
        if self.pending_newlines == 0 {
            self.space();
        } else {
            self.prep_line();
        }
        self.pending_newlines = 0;
        self.write(text);
        self.end_line();
    }

    fn note_whitespace(&mut self, text: &str) {
        self.pending_newlines = text.matches('\n').count();
    }

    fn blank_line_if_pending(&mut self) {
        let starts_line = self.at_line_start || self.need_newline;
        if starts_line && self.pending_newlines >= 2 && !self.out.is_empty() {
            self.prep_line();
            self.out.push('\n');
        }
        // Note: pending_newlines is *not* reset here — the comment logic
        // still needs it; real tokens reset it in after_significant.
    }

    /// Whether the next word-like token needs a space before it.
    fn needs_word_space(&self) -> bool {
        let Some(prev) = self.prev else {
            return false;
        };
        if self.at_line_start {
            return false;
        }
        !matches!(
            prev,
            SyntaxKind::LParen
                | SyntaxKind::LBracket
                | SyntaxKind::Minus
                | SyntaxKind::Comma
                | SyntaxKind::Colon
                | SyntaxKind::Equals
                | SyntaxKind::Arrow
                | SyntaxKind::PipeForward
                | SyntaxKind::Dot
        )
    }

    fn after_significant(&mut self, kind: SyntaxKind) {
        self.prev = Some(kind);
        self.pending_newlines = 0;
    }

    fn indent_more(&mut self) {
        self.indent = self.indent.saturating_add(4);
    }

    fn indent_less(&mut self) {
        self.indent = self.indent.saturating_sub(4);
    }
}
