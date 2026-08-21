//! The measured bar: which meter is in force where, which events open a
//! beat group, and the width a bar drawn to scale takes.
//!
//! One concern of the `formatter` module; see its docs for the rules.

use super::nodes::one_line;
use super::{Layout, MEASURE};
use crate::language::SyntaxNode;
use crate::meter::beat_groups;
use crate::{SyntaxElement, SyntaxKind};
use std::collections::{HashMap, HashSet};
use text_size::TextRange;

/// Columns to the whole note.
///
/// Sixteen to the quarter. 64 divides by 2, 4, 8, 16 and 32 exactly and by 3
/// closely enough that a triplet lands back on the grid where it ends.
const COLUMNS: u64 = 64;

/// The column an event at `elapsed` whole notes into the bar wants.
///
/// Linear, and that is a proof rather than a taste. If the width given to an
/// event were some function `W` of its own duration and columns accumulated,
/// then "equal elapsed time, equal column" applied to `1/4 + 1/4` against `1/2`
/// forces `W(1/2) = 2·W(1/4)` — and applied at every dyadic split, forces `W`
/// linear. Any concave curve, which is what an engraver actually uses, provably
/// puts beat 3 in a different column on a line of quarters than on a line of
/// halves, and that is the one thing this layout exists to prevent. Real
/// engraving escapes the argument by solving one spacing problem across a whole
/// system; here that would make an edit in bar 5 relay out bars 1 to 8, and
/// diff locality is what a text formatter must not spend.
///
/// Integers throughout, deliberately: `log2` is not bit-identical across libm
/// implementations, and a formatter whose output depended on the platform's
/// would make `musa format --check` fail in CI on a machine other than the one
/// that wrote the file.
fn grid(elapsed: Beat) -> Option<usize> {
    let columns = elapsed
        .numerator
        .checked_mul(COLUMNS)?
        .checked_div(elapsed.denominator)?;
    usize::try_from(columns).ok()
}

/// A `|` bar with every event at the column its onset asks for, or `None` when
/// it cannot be drawn that way.
///
/// Two conditions. The bar must be measurable, which is the same question the
/// beat groups ask and the same answer. And it must be a `|` bar: a named bar's
/// head is its address and is as long as its name, so its events start
/// somewhere no other bar's do, and a column that means one thing per line is
/// no column at all.
///
/// The recurrence is `col(k) = max(grid(t_k), col(k-1) + len(k-1) + gap(k))` —
/// the onset's column, or the compact layout's own gap past the previous event
/// when the music is denser than the grid. Because the second term is always
/// taken into account, a drawn bar is never *narrower* than the compact one,
/// which is what makes the setting safe: turning it on can never push a bar
/// over the line that was not already over it.
///
/// `gap(k)` is the compact writer's gap and not a constant one, which is the
/// whole of that guarantee: a beat-group boundary is written two spaces wide
/// there, so a grid that reserved one would draw a *narrower* line than the
/// compact layout at every group start dense enough to be pushed off its
/// column.
pub(super) fn drawn_to_scale(bar: &SyntaxNode, layout: &Layout) -> Option<String> {
    if bar.children().any(|child| child.kind() == SyntaxKind::Block) {
        return None;
    }
    let measured = measured_bar(bar, &layout.meters)?;
    let groups = beat_group_starts(bar, &layout.meters);
    let mut body = String::new();
    let mut elapsed = Beat::ZERO;
    let mut next = 0_usize;
    for (item, length) in measured.items {
        let gap = usize::from(groups.contains(&item.text_range()));
        let column = grid(elapsed)?.max(next.saturating_add(gap));
        while body.chars().count() < column {
            body.push(' ');
        }
        let text = one_line(&item, layout);
        next = column.saturating_add(text.chars().count()).saturating_add(1);
        body.push_str(&text);
        elapsed = elapsed.plus(length)?;
    }
    Some(format!("| {body}"))
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
pub(super) fn wrap_at_beat_groups(line: &str, indent: usize) -> Vec<String> {
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

/// The meter in force at each bar, by where the bar stands in the source.
///
/// Keyed by range because that is the one thing a bar has that is unique to it
/// and stable across the walk. The map holds only bars under a stated meter
/// that is a fraction: `meter none;` and a piece that never says one leave the
/// bar out, and a bar that is not in here is not grouped.
pub(super) type Meters = HashMap<TextRange, (u32, u32)>;

/// Which meter governs each bar — a pre-walk, because one pass cannot know.
///
/// A part may state its meter *after* the voices it governs (`part_decl`
/// accepts its declarations in any order), so a left-to-right reader would
/// space a part's first bars by the piece's meter and its last ones by the
/// part's. Inside a voice the order does mean what it looks like: a `meter`
/// written among the events is a change written where it happens, and it
/// governs from there.
pub(super) fn meters_in_force(root: &SyntaxNode) -> Meters {
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
pub(super) fn beat_group_starts(bar: &SyntaxNode, meters: &Meters) -> HashSet<TextRange> {
    let none = HashSet::new();
    let Some(measured) = measured_bar(bar, meters) else {
        return none;
    };
    let (numerator, denominator) = measured.meter;
    let items = measured.items;
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
    for &(ref item, length) in &items {
        let range = item.text_range();
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

/// A bar the formatter can read: its meter, and its items with their lengths.
struct MeasuredBar {
    meter: (u32, u32),
    items: Vec<(SyntaxNode, Beat)>,
}

/// The one reader of "can this bar be laid out to the beat".
///
/// Both layouts ask it — the beat groups of every file, and the grid of a
/// project that has asked for one — so a bar that groups is exactly a bar that
/// can be drawn to scale, and neither can drift into measuring something the
/// other does not.
fn measured_bar(bar: &SyntaxNode, meters: &Meters) -> Option<MeasuredBar> {
    if bar.kind() != SyntaxKind::BarStmt {
        return None;
    }
    let &(numerator, denominator) = meters.get(&bar.text_range())?;
    let items = measurable_items(bar)?;
    let total = items.iter().try_fold(Beat::ZERO, |sum, item| sum.plus(item.1))?;
    (total == Beat::new(u64::from(numerator), u64::from(denominator))).then_some(MeasuredBar {
        meter: (numerator, denominator),
        items,
    })
}

/// The bar's items with their written lengths, or `None` if it holds anything
/// the formatter cannot measure.
fn measurable_items(bar: &SyntaxNode) -> Option<Vec<(SyntaxNode, Beat)>> {
    // `bar name { … }` holds a block and `| …` holds its items directly —
    // the same two shapes `ast::voice_items` reads, and the same answer for
    // both, because a name does not change how a bar is counted.
    let body = bar.children().find(|child| child.kind() == SyntaxKind::Block);
    let items = body.as_ref().unwrap_or(bar).children();
    items
        .filter(|item| item.kind() != SyntaxKind::Block)
        .map(|item| {
            let kind = item.kind();
            // A note, a rest, a chord and a stacked chord fill the time they
            // write; a dynamic
            // and a mark fill none. Everything else — a `use`, a tuplet, an
            // `improvise`, a construct added next year — has no length this
            // crate can read, and the whitelist is what makes that the
            // default rather than a case somebody has to remember.
            let length = if matches!(
                kind,
                SyntaxKind::NoteStmt | SyntaxKind::RestStmt | SyntaxKind::ChordStmt | SyntaxKind::StackStmt
            ) {
                Beat::parse(&crate::ast::Duration::of(&item)?.value()?)?
            } else if matches!(kind, SyntaxKind::DynamicStmt | SyntaxKind::MarkStmt) {
                Beat::ZERO
            } else {
                return None;
            };
            Some((item, length))
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
