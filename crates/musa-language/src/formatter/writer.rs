//! The output buffer: indentation, line breaks, gaps, comments, and the
//! chain and list state a layout decision reads.
//!
//! One concern of the `formatter` module; see its docs for the rules.

use crate::SyntaxKind;
use crate::language::SyntaxNode;
use std::collections::HashSet;
use text_size::TextRange;

/// Layout state for the formatter.
pub(super) struct Writer {
    pub(super) out: String,
    pub(super) indent: usize,
    pub(super) at_line_start: bool,
    /// A line break is owed before the next token (after `;`, `{`, `}`,
    /// or a comment). Deferred so a trailing comment can join the line first.
    pub(super) need_newline: bool,
    /// Newlines seen in the whitespace run just passed.
    pub(super) pending_newlines: usize,
    /// Kind of the last significant token written.
    pub(super) prev: Option<SyntaxKind>,
    /// One entry per enclosing stacked chain: whether its continuation
    /// indent has been applied yet.
    pub(super) chains: Vec<bool>,
    /// One entry per enclosing comma-separated list: whether it is written
    /// one item per line. Innermost last.
    pub(super) lists: Vec<bool>,
    /// Set while rendering a horizontal run, which is the only time newlines
    /// are swallowed and the only time a gap is ever wider than one space.
    pub(super) run: Option<Run>,
}

/// What a horizontal run needs to know about itself while it is written.
pub(super) struct Run {
    /// The items that open a beat group, by range. The gap in front of one is
    /// two spaces wide instead of one.
    pub(super) beat_groups: HashSet<TextRange>,
    /// The next space written is a beat-group gap.
    pub(super) wide_gap: bool,
}

impl Writer {
    pub(super) fn new() -> Self {
        Self {
            out: String::new(),
            indent: 0,
            at_line_start: true,
            need_newline: false,
            pending_newlines: 0,
            prev: None,
            chains: Vec::new(),
            lists: Vec::new(),
            run: None,
        }
    }

    /// A writer that lays its input out on one line, widening the gap in front
    /// of every item in `beat_groups`.
    pub(super) fn one_line(beat_groups: HashSet<TextRange>) -> Self {
        Self {
            at_line_start: false,
            run: Some(Run {
                beat_groups,
                wide_gap: false,
            }),
            ..Self::new()
        }
    }

    /// Whether this item opens a beat group, and so takes the wider gap.
    pub(super) fn starts_a_beat_group(&self, node: &SyntaxNode) -> bool {
        self.run
            .as_ref()
            .is_some_and(|run| run.beat_groups.contains(&node.text_range()))
    }

    pub(super) fn widen_next_gap(&mut self) {
        if let Some(run) = self.run.as_mut() {
            run.wide_gap = true;
        }
    }

    /// Enter a chain that is written one stage per line.
    pub(super) fn open_chain(&mut self) {
        self.chains.push(false);
    }

    pub(super) fn close_chain(&mut self) {
        if self.chains.pop() == Some(true) {
            self.indent_less();
        }
    }

    pub(super) fn in_wrapped_chain(&self) -> bool {
        !self.chains.is_empty()
    }

    /// Whether this writer is laying its input out on one line.
    pub(super) fn in_run(&self) -> bool {
        self.run.is_some()
    }

    /// How much has been written, in columns.
    pub(super) fn written(&self) -> usize {
        self.out.chars().count()
    }

    /// Whether the last thing written asked for the line to end.
    pub(super) fn line_ended(&self) -> bool {
        self.need_newline
    }

    /// Enter a comma-separated list, recording whether it stacks. Entered for
    /// every list and not only the stacked ones: a short list inside a long
    /// one stays on its line, and the stack is what says so.
    pub(super) fn open_list(&mut self, broken: bool) {
        self.lists.push(broken);
    }

    pub(super) fn close_list(&mut self) {
        self.lists.pop();
    }

    /// Whether the innermost enclosing list is written one item per line.
    pub(super) fn list_breaks(&self) -> bool {
        self.lists.last().copied().unwrap_or_default()
    }

    /// Whether a stacked list about to close still owes its trailing comma:
    /// it has a last item, and that item is not already followed by one.
    ///
    /// An opener as the previous token is the empty list — nothing to end —
    /// and a line already broken is a list whose comma has been written and
    /// whose newline has been spent.
    pub(super) fn wants_trailing_comma(&self) -> bool {
        !self.at_line_start
            && !matches!(
                self.prev,
                None | Some(SyntaxKind::Comma | SyntaxKind::LParen | SyntaxKind::LBracket)
            )
    }

    /// A token the layout chose not to write. The whitespace in front of it is
    /// spent with it; `prev` stays what it was, because nothing was written and
    /// so what the next token joins is still the token before this one.
    pub(super) fn skip_token(&mut self) {
        self.pending_newlines = 0;
    }

    /// Indent the continuation lines — once per chain, and only when the
    /// first `|>` proves there will be any. The chain's head stays on the
    /// line its statement started.
    pub(super) fn indent_continuations(&mut self) {
        if let Some(indented) = self.chains.last_mut()
            && !*indented
        {
            *indented = true;
            self.indent_more();
        }
    }

    pub(super) fn finish(mut self) -> String {
        if self.run.is_some() {
            return self.out;
        }
        while self.out.ends_with('\n') {
            self.out.pop();
        }
        self.out.push('\n');
        self.out
    }

    /// Emit a deferred line break, if one is owed.
    pub(super) fn prep_line(&mut self) {
        if self.need_newline {
            self.newline();
            self.need_newline = false;
        }
    }

    pub(super) fn end_line(&mut self) {
        self.need_newline = true;
    }

    /// Put the closing brace of a stacked block on its own line.
    ///
    /// Nothing on a one-line run, where the brace joins its neighbour with a
    /// space, and nothing at the start of a line, which is already what this
    /// is for.
    pub(super) fn break_before_close(&mut self) {
        if self.run.is_some() || self.at_line_start {
            return;
        }
        self.newline();
    }

    /// Where the next thing written would land on the current line.
    ///
    /// The indent when the line has not been started — or has been ended and
    /// not yet broken — and the width written so far when it has.
    pub(super) fn column(&self) -> usize {
        if self.at_line_start || self.need_newline {
            return self.indent;
        }
        self.out
            .rsplit('\n')
            .next()
            .map_or(self.indent, |line| line.chars().count())
    }

    pub(super) fn write(&mut self, text: &str) {
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
    pub(super) fn write_word(&mut self, kind: SyntaxKind, text: &str, joining: bool) {
        self.prep_line();
        if !joining && self.needs_word_space() {
            self.space();
        }
        self.write(text);
        self.after_significant(kind);
    }

    /// Write a whole construct that was rendered elsewhere, as its own line.
    /// A run written where the line goes on afterwards.
    pub(super) fn write_run(&mut self, text: &str) {
        self.prep_line();
        if self.needs_word_space() {
            self.space();
        }
        self.write(text);
        self.after_significant(SyntaxKind::RBrace);
    }

    pub(super) fn write_line(&mut self, text: &str) {
        self.prep_line();
        if self.needs_word_space() {
            self.space();
        }
        self.write(text);
        self.end_line();
        self.after_significant(SyntaxKind::RBrace);
    }

    /// One space, or two in front of a beat group — and never one more than
    /// that. On a one-line run the gap between two items can be asked for
    /// twice, by the statement that ended and by the token that follows, and
    /// what is wanted is the gap rather than the count of requests.
    pub(super) fn space(&mut self) {
        if self.at_line_start {
            return;
        }
        let wide = self.run.as_mut().is_some_and(|run| std::mem::take(&mut run.wide_gap));
        let written = usize::from(self.out.ends_with(' ')).saturating_add(usize::from(self.out.ends_with("  ")));
        for _ in written..if wide { 2 } else { 1 } {
            self.out.push(' ');
        }
    }

    pub(super) fn newline(&mut self) {
        if self.run.is_some() {
            // One line has no newlines in it, and a statement boundary that
            // wanted one is the space between two items.
            self.space();
            return;
        }
        if !self.at_line_start {
            self.out.push('\n');
            self.at_line_start = true;
        }
    }

    /// A comment that trailed code in the original (no newline before it)
    /// stays trailing; an own-line comment writes at the current indent.
    pub(super) fn comment(&mut self, text: &str) {
        if self.pending_newlines == 0 {
            self.space();
        } else {
            self.prep_line();
        }
        self.pending_newlines = 0;
        self.write(text);
        self.end_line();
    }

    pub(super) fn note_whitespace(&mut self, text: &str) {
        self.pending_newlines = text.matches('\n').count();
    }

    pub(super) fn blank_line_if_pending(&mut self) {
        if self.run.is_some() {
            return;
        }
        let starts_line = self.at_line_start || self.need_newline;
        if starts_line && self.pending_newlines >= 2 && !self.out.is_empty() {
            self.prep_line();
            self.out.push('\n');
        }
        // Note: pending_newlines is *not* reset here — the comment logic
        // still needs it; real tokens reset it in after_significant.
    }

    /// Whether the next word-like token needs a space before it.
    ///
    /// The listed kinds are the ones nothing follows with a gap: an opener
    /// (`(`, `[`, `<`), a separator that already wrote its own space, or a
    /// sign that belongs to the number after it. `<` earns its place the way
    /// `[` does — it opens a parameter, and `Option<Pitch>` is one word for
    /// the same reason `[c4, d4]` starts tight.
    pub(super) fn needs_word_space(&self) -> bool {
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
                | SyntaxKind::Slash
                | SyntaxKind::Less
                // The operator spellings that appear nowhere but a
                // `BinaryExpr`, which writes its own space on both sides.
                | SyntaxKind::EqualsEquals
                | SyntaxKind::Plus
                | SyntaxKind::Star
        )
    }

    pub(super) fn after_significant(&mut self, kind: SyntaxKind) {
        self.prev = Some(kind);
        self.pending_newlines = 0;
    }

    pub(super) fn indent_more(&mut self) {
        self.indent = self.indent.saturating_add(4);
    }

    pub(super) fn indent_less(&mut self) {
        self.indent = self.indent.saturating_sub(4);
    }
}
