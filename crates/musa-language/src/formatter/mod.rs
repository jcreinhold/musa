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
//! - A comma-separated list — a call's arguments, a constructor's fields —
//!   is written on one line when it fits [`MEASURE`], and one item per line
//!   when it does not. See [`breakable_list`]. Its trailing comma follows
//!   that decision rather than the source: present when the list is written
//!   down the page, absent when it is on one line. See [`ends_its_list`].
//! - Inside a bar, the gap between beat groups is two spaces instead of one.
//!   A beam is how notation shows which beats a player hears together, and
//!   horizontal space is the only thing text has to draw one with. Which
//!   notes group is [`beat_groups`]' answer, the same one the engraver beams
//!   by, so the two cannot disagree — and a bar the formatter cannot measure
//!   is written with single spaces rather than a guess.
//!
//! Laws (tested as properties): `format` is idempotent, and
//! `parse(format(parse(source)))` equals `parse(source)` up to whitespace.
//!
//! # The rules, by file
//!
//! - [`writer`] — the output buffer every rule writes through.
//! - [`nodes`] — how a node lays out its children.
//! - [`tokens`] — how one token is spaced against the one before it.
//! - [`lists`] — whether a comma-separated list goes across or down.
//! - [`bars`] — meter, beat groups, and the width of a bar drawn to scale.

use crate::ParsedDocument;

mod bars;
mod lists;
mod nodes;
mod tokens;
mod writer;

use bars::{Meters, meters_in_force};
use nodes::format_node;
use writer::Writer;

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

/// How a bar's events are spaced across its line.
///
/// The default is computed rather than asked for. Every file written so far is
/// compact, so defaulting to [`BarSpacing::Proportional`] would rewrite a
/// corpus on upgrade; and compact is the layout that is never actively wrong,
/// because it never claims an alignment. So there is no unset state and no
/// `Option<BarSpacing>` anywhere: a caller that has not been told chooses
/// `Compact`, and that is an answer rather than a gap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarSpacing {
    /// One space between events, and two between beat groups.
    #[default]
    Compact,
    /// Every event at a column proportional to when it sounds, so beat *n*
    /// falls in the same column on every line of a voice and the page can be
    /// read down as well as across.
    Proportional,
}

/// Format a parsed document.
///
/// Lossless: every comment survives, and every token that says something
/// survives. Whitespace trivia is normalized, and so is the one token that is
/// punctuation for a layout rather than a word of the program — a list's own
/// trailing comma, which [`ends_its_list`] explains and which is written or
/// dropped to match the layout the list was given.
///
/// `bars` is a required parameter and not an overload on purpose. Leaving a
/// one-argument `format` in the API would leave a trap that silently means
/// "compact", so the next call site added — a preview, a build step, a doc
/// test — would quietly ignore the project's setting. Making it mandatory is
/// the type system enforcing what the crate graph cannot: this crate cannot
/// read a manifest, so every caller answers which layout this is.
pub fn format(document: &ParsedDocument, bars: BarSpacing) -> FormattedSource {
    let root = document.syntax();
    let layout = Layout {
        meters: meters_in_force(&root),
        bars,
    };
    let mut writer = Writer::new();
    format_node(&root, &mut writer, &layout);
    FormattedSource { text: writer.finish() }
}

/// What the layout of a bar depends on beyond the tree in front of it.
struct Layout {
    meters: Meters,
    bars: BarSpacing,
}

/// How wide a line a bar — or a comma-separated list — may keep, indent
/// included.
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
///
/// A list is budgeted by the same number for the same reason: a list is
/// horizontal until it is long, and one number is one rule to remember.
const MEASURE: usize = 96;
