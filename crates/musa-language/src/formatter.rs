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
//! - A bar that fits the source measure is written on one line. The one
//!   exception, and see [`MEASURE`] for why it earns itself.
//! - Inside a bar, the gap between beat groups is two spaces instead of one.
//!   A beam is how notation shows which beats a player hears together, and
//!   horizontal space is the only thing text has to draw one with. Which
//!   notes group is [`beat_groups`]' answer, the same one the engraver beams
//!   by, so the two cannot disagree — and a bar the formatter cannot measure
//!   is written with single spaces rather than a guess.
//!
//! Laws (tested as properties): `format` is idempotent, and
//! `parse(format(parse(source)))` equals `parse(source)` up to whitespace.

use std::collections::{HashMap, HashSet};

use text_size::TextRange;

use crate::language::SyntaxNode;
use crate::meter::beat_groups;
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
    let root = document.syntax();
    let meters = meters_in_force(&root);
    let mut writer = Writer::new();
    format_node(&root, &mut writer, &meters);
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

fn format_node(node: &SyntaxNode, writer: &mut Writer, meters: &Meters) {
    // Set once the first token of a one-word construct has been written, so
    // the rest of it joins on without a space.
    let mut tight = false;
    for element in node.children_with_tokens() {
        match element {
            SyntaxElement::Node(child) => {
                writer.blank_line_if_pending();
                if writer.starts_a_beat_group(&child) {
                    writer.widen_next_gap();
                }
                if let Some(lines) = inline_run(&child, writer.indent, meters) {
                    for line in lines {
                        writer.write_line(&line);
                    }
                    continue;
                }
                let wrap = wraps_across_lines(&child);
                if wrap {
                    writer.open_chain();
                }
                format_node(&child, writer, meters);
                if wrap {
                    writer.close_chain();
                }
                // An event is self-delimiting, so there is no `;` inside it
                // to end its line the way a context statement's does. Where a
                // bar has not already claimed the line, the events are the
                // lines — which is the layout every unbarred voice has now.
                if matches!(
                    child.kind(),
                    SyntaxKind::NoteStmt | SyntaxKind::RestStmt | SyntaxKind::ChordStmt | SyntaxKind::BarStmt
                ) {
                    writer.end_line();
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
                // number is, a chord symbol is `fmaj7` however many tokens it
                // happens to lex as, and a modulation target is one path with
                // dots in it. Their insides take no spaces.
                if matches!(
                    node.kind(),
                    SyntaxKind::Position | SyntaxKind::ChordSymbol | SyntaxKind::ParamPath | SyntaxKind::PitchClass
                ) {
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
        // On its own line the `}` needs no space in front of it, and
        // `needs_word_space` already knows that: `prep_line` has just put the
        // writer at the start of a line. On a one-line run there is no line
        // start to rely on, and `grace { c5 d5 }` wants its space.
        if writer.needs_word_space() {
            writer.space();
        }
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
    } else if matches!(
        kind,
        SyntaxKind::Slash | SyntaxKind::Dot | SyntaxKind::Greater | SyntaxKind::Caret
    ) {
        // A short-form duration is part of the note's word: `c4/4.` is one
        // note written one way, not a pitch beside a fraction beside a dot.
        // An accent or a marcato is drawn on its notehead, so it is written
        // on its note: `c4/4>`, never `c4/4 >`.
        writer.write(text);
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
    /// Set while rendering a horizontal run, which is the only time newlines
    /// are swallowed and the only time a gap is ever wider than one space.
    run: Option<Run>,
}

/// What a horizontal run needs to know about itself while it is written.
struct Run {
    /// The items that open a beat group, by range. The gap in front of one is
    /// two spaces wide instead of one.
    beat_groups: HashSet<TextRange>,
    /// The next space written is a beat-group gap.
    wide_gap: bool,
}

/// A run of events written horizontally, as the lines to write for it.
///
/// A bar and a grace group are the two constructs that are *horizontal*: each
/// holds nothing but events, read left to right, and neither ends its items
/// with anything — so stacking them puts one word on each of several lines and
/// leaves the closing brace stranded after the last.
///
/// The run is rendered by the same [`Writer`] that writes everything else,
/// with line breaks suppressed. There is no second copy of the spacing rules
/// to keep in step: a token that closes up to its neighbour does so once, in
/// [`format_token`], and both layouts read it there.
///
/// Returns `None` for anything else, for a run carrying a comment — a comment
/// wants a line of its own and one line has nowhere to put it — and for a
/// *braced* run too wide for [`MEASURE`], which falls back to the way every
/// other block breaks. A `|` bar has no brace to break at, so an over-wide one
/// wraps at its beat groups instead; with no groups to wrap at, the line runs
/// long, because a long line is better than a wrong one.
fn inline_run(node: &SyntaxNode, indent: usize, meters: &Meters) -> Option<Vec<String>> {
    if !matches!(node.kind(), SyntaxKind::BarStmt | SyntaxKind::GraceStmt) {
        return None;
    }
    if node
        .descendants_with_tokens()
        .any(|element| matches!(element.kind(), SyntaxKind::LineComment | SyntaxKind::BlockComment))
    {
        return None;
    }
    let mut writer = Writer::one_line(beat_group_starts(node, meters));
    format_node(node, &mut writer, meters);
    let line = writer.finish();
    if indent.saturating_add(line.chars().count()) <= MEASURE {
        return Some(vec![line]);
    }
    // A braced run has somewhere to break — its `{` and `}` take the lines a
    // stacked block would give them. A `|` bar has neither, so it wraps.
    let braced = node.children().any(|child| child.kind() == SyntaxKind::Block);
    (!braced).then(|| wrap_at_beat_groups(&line, indent))
}

/// How far a continuation line is pushed past its bar's own indent.
///
/// Two, which is the width of `| `: a bar that wraps reads as one bar because
/// every line after the first starts under the first event rather than under
/// the barline.
const CONTINUATION: usize = 2;

/// Break an over-wide bar at the gaps between its beat groups.
///
/// The gaps are the only places a bar may be broken — a group is what a player
/// reads at once, and splitting one puts half a beat on the next line. So a
/// bar with no groups, or one group too wide to fit, runs long instead.
///
/// Groups that stay on one line keep the wide gap between them: the wrap
/// decides where the lines end, not what a gap means.
fn wrap_at_beat_groups(line: &str, indent: usize) -> Vec<String> {
    const GAP: &str = "  ";
    let mut lines: Vec<String> = Vec::new();
    for group in line.split(GAP) {
        let room = MEASURE
            .saturating_sub(indent)
            .saturating_sub(if lines.is_empty() { 0 } else { CONTINUATION });
        let joined = |last: &String| {
            last.chars()
                .count()
                .saturating_add(GAP.len())
                .saturating_add(group.chars().count())
        };
        match lines.last_mut() {
            Some(last) if joined(last) <= room => {
                last.push_str(GAP);
                last.push_str(group);
            }
            _ => {
                let mut next = String::new();
                if !lines.is_empty() {
                    next.push_str(&" ".repeat(CONTINUATION));
                }
                next.push_str(group);
                lines.push(next);
            }
        }
    }
    lines
}

// --- Beat groups -----------------------------------------------------------

/// The meter in force at each bar, by where the bar stands in the source.
///
/// Keyed by range because that is the one thing a bar has that is unique to it
/// and stable across the walk. The map holds only bars under a stated meter
/// that is a fraction: `meter none;` and a piece that never says one leave the
/// bar out, and a bar that is not in here is not grouped.
type Meters = HashMap<TextRange, (u32, u32)>;

/// Which meter governs each bar — a pre-walk, because one pass cannot know.
///
/// A part may state its meter *after* the voices it governs (`part_decl`
/// accepts its declarations in any order), so a left-to-right reader would
/// space a part's first bars by the piece's meter and its last ones by the
/// part's. Inside a voice the order does mean what it looks like: a `meter`
/// written among the events is a change written where it happens, and it
/// governs from there.
fn meters_in_force(root: &SyntaxNode) -> Meters {
    let mut meters = Meters::new();
    scan_meters(root, None, &mut meters);
    meters
}

fn scan_meters(node: &SyntaxNode, inherited: Option<(u32, u32)>, meters: &mut Meters) {
    // A part's own meter governs all of it, including the voices written
    // above the line that states it.
    let mut current = if node.kind() == SyntaxKind::PartDecl {
        node.children()
            .find(|child| child.kind() == SyntaxKind::MeterStmt)
            .map_or(inherited, |stated| meter_of(&stated))
    } else {
        inherited
    };
    for child in node.children() {
        if child.kind() == SyntaxKind::MeterStmt {
            current = meter_of(&child);
        }
        if child.kind() == SyntaxKind::BarStmt
            && let Some(meter) = current
        {
            meters.insert(child.text_range(), meter);
        }
        scan_meters(&child, current, meters);
    }
}

/// The fraction a `meter` statement states, or `None` for `meter none;`.
fn meter_of(statement: &SyntaxNode) -> Option<(u32, u32)> {
    let written = statement
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|token| token.kind() == SyntaxKind::Rational)?;
    let (numerator, denominator) = written.text().split_once('/')?;
    Some((numerator.parse().ok()?, denominator.parse().ok()?))
}

/// The items of a bar that begin a new beat group.
///
/// This is the whole of the layout decision, and it is a whitelist so that a
/// statement kind added next year falls back rather than guesses: a bar is
/// *measurable* when every item is a note, rest or chord contributing its
/// written duration, or a dynamic or a mark contributing nothing; when every
/// duration is a literal; when a meter is in force; and when the durations sum
/// to exactly one measure. Anything else — a `use`, a tuplet, an `improvise`,
/// a duration that is a parameter, a bar that does not add up — is spaced with
/// single spaces throughout, because the grouping would be a claim about music
/// the formatter cannot read.
///
/// One further condition, which is what keeps an ordinary bar ordinary: some
/// group must hold more than one event. A wide gap says *these belong
/// together*, and in a bar of four quarters in 4/4 every group holds one note,
/// so the gaps would separate nothing. A beam that beams one note is not a
/// beam.
fn beat_group_starts(bar: &SyntaxNode, meters: &Meters) -> HashSet<TextRange> {
    let none = HashSet::new();
    if bar.kind() != SyntaxKind::BarStmt {
        return none;
    }
    let Some(&(numerator, denominator)) = meters.get(&bar.text_range()) else {
        return none;
    };
    let Some(items) = measurable_items(bar) else {
        return none;
    };
    let measure = Beat::new(u64::from(numerator), u64::from(denominator));
    let Some(total) = items.iter().try_fold(Beat::ZERO, |sum, item| sum.plus(item.1)) else {
        return none;
    };
    if total != measure {
        return none;
    }
    let mut boundaries = Vec::new();
    let mut counted = 0_u32;
    for group in beat_groups(numerator, denominator) {
        counted = counted.saturating_add(group);
        boundaries.push(Beat::new(u64::from(counted), u64::from(denominator)));
    }
    // The downbeat and the barline are boundaries too, and neither is a gap.
    boundaries.pop();

    let mut starts = HashSet::new();
    let mut elapsed = Beat::ZERO;
    let mut next = 0_usize;
    let mut widest = 0_usize;
    let mut in_group = 0_usize;
    for &(range, length) in &items {
        while boundaries.get(next).is_some_and(|boundary| *boundary < elapsed) {
            next = next.saturating_add(1);
        }
        if boundaries.get(next) == Some(&elapsed) {
            next = next.saturating_add(1);
            starts.insert(range);
            widest = widest.max(in_group);
            in_group = 0;
        }
        in_group = in_group.saturating_add(1);
        elapsed = match elapsed.plus(length) {
            Some(sum) => sum,
            None => return HashSet::new(),
        };
    }
    if widest.max(in_group) > 1 {
        starts
    } else {
        HashSet::new()
    }
}

/// The bar's items with their written lengths, or `None` if it holds anything
/// the formatter cannot measure.
fn measurable_items(bar: &SyntaxNode) -> Option<Vec<(TextRange, Beat)>> {
    // `bar name { … }` holds a block and `| …` holds its items directly —
    // the same two shapes `ast::voice_items` reads, and the same answer for
    // both, because a name does not change how a bar is counted.
    let body = bar.children().find(|child| child.kind() == SyntaxKind::Block);
    let items = body.as_ref().unwrap_or(bar).children();
    items
        .filter(|item| item.kind() != SyntaxKind::Block)
        .map(|item| {
            let kind = item.kind();
            // A note, a rest and a chord fill the time they write; a dynamic
            // and a mark fill none. Everything else — a `use`, a tuplet, an
            // `improvise`, a construct added next year — has no length this
            // crate can read, and the whitelist is what makes that the
            // default rather than a case somebody has to remember.
            let length = if matches!(
                kind,
                SyntaxKind::NoteStmt | SyntaxKind::RestStmt | SyntaxKind::ChordStmt
            ) {
                Beat::parse(&crate::ast::Duration::of(&item)?.value()?)?
            } else if matches!(kind, SyntaxKind::DynamicStmt | SyntaxKind::MarkStmt) {
                Beat::ZERO
            } else {
                return None;
            };
            Some((item.text_range(), length))
        })
        .collect()
}

/// A length in whole notes, exactly.
///
/// Reduced on construction so that equality is equality of the fraction, which
/// is what a boundary test needs: 2/8 and 1/4 are the same instant.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Beat {
    numerator: u64,
    denominator: u64,
}

impl Beat {
    const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    fn new(numerator: u64, denominator: u64) -> Self {
        if denominator == 0 {
            return Self::ZERO;
        }
        let divisor = gcd(numerator, denominator).max(1);
        Self {
            numerator: numerator.checked_div(divisor).unwrap_or(0),
            denominator: denominator.checked_div(divisor).unwrap_or(1),
        }
    }

    /// A duration as `Duration::value` spells it: `3/8`, or `1` for a whole.
    fn parse(spelling: &str) -> Option<Self> {
        let (numerator, denominator) = spelling.split_once('/').unwrap_or((spelling, "1"));
        Some(Self::new(numerator.parse().ok()?, denominator.parse().ok()?))
    }

    /// `None` on overflow, which reads as "not measurable" rather than as a
    /// wrong answer.
    fn plus(self, other: Self) -> Option<Self> {
        let left = self.numerator.checked_mul(other.denominator)?;
        let right = other.numerator.checked_mul(self.denominator)?;
        Some(Self::new(
            left.checked_add(right)?,
            self.denominator.checked_mul(other.denominator)?,
        ))
    }
}

impl PartialOrd for Beat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let left = self.numerator.checked_mul(other.denominator)?;
        let right = other.numerator.checked_mul(self.denominator)?;
        Some(left.cmp(&right))
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let rest = a.checked_rem(b).unwrap_or(0);
        a = b;
        b = rest;
    }
    a
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
            run: None,
        }
    }

    /// A writer that lays its input out on one line, widening the gap in front
    /// of every item in `beat_groups`.
    fn one_line(beat_groups: HashSet<TextRange>) -> Self {
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
    fn starts_a_beat_group(&self, node: &SyntaxNode) -> bool {
        self.run
            .as_ref()
            .is_some_and(|run| run.beat_groups.contains(&node.text_range()))
    }

    fn widen_next_gap(&mut self) {
        if let Some(run) = self.run.as_mut() {
            run.wide_gap = true;
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
        if self.needs_word_space() {
            self.space();
        }
        self.write(text);
        self.end_line();
        self.after_significant(SyntaxKind::RBrace);
    }

    fn space(&mut self) {
        if self.at_line_start {
            return;
        }
        self.out.push(' ');
        if self.run.as_mut().is_some_and(|run| std::mem::take(&mut run.wide_gap)) {
            self.out.push(' ');
        }
    }

    fn newline(&mut self) {
        if self.run.is_some() {
            return;
        }
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
                | SyntaxKind::Slash
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
