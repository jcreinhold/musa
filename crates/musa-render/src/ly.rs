//! The `LilyPond` backend (roadmap §12.3): `NotationPlan` → typed `LyDocument`
//! (private `LyNode` tree) → deterministic pretty-printer → `.ly` text.
//! Export only: no `LilyPond` import, no binary invocation, no layout knobs —
//! the plan is semantic; engraving belongs to `LilyPond`.
//!
//! Source map: every note/chord/rest is preceded by a `% event:<hex>`
//! comment carrying its `EventId` (comments do not disturb layout). Tie
//! pieces of one event share the id; the `-t2` suffixing is an MEI contract,
//! not needed here because `LilyPond` ties are syntax (`~`), not identity.
//!
//! Duration spelling is derived from each piece's rational value, not the
//! source spelling: decomposition gives plan pieces the *event's* spelling,
//! so a whole note split into halves must not print `1` twice. For
//! un-decomposed notes the value-derived spelling coincides with the source
//! spelling (`1/4` → `4`, `3/8` → `4.`).

// Rational duration arithmetic is exact and total for musa's magnitudes (see
// musa-compiler/src/time.rs); the workspace arithmetic lint is allowed at
// module scope for that reason.
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{ArticulationMark, Clef, EventId, Mode, WrittenPitch};

use crate::RenderError;
use crate::plan::{ARTICULATION_PLACEMENT, NotatedItem, NotatedKind, NotationPlan, Placement, StaffPlan, VoiceLane};

/// The typed document node tree (§12.3). Private to the backend.
enum LyNode {
    /// `{ a b c }` — music in sequence.
    Sequential(Vec<Self>),
    /// `<< { a } \\ { b } >>` — simultaneous voices.
    Simultaneous(Vec<Self>),
    /// One note/chord/rest token with its event for the source map.
    Note { body: String, event: EventId },
    /// `\tuplet n/d { ... }` — a tuplet bracket over its items.
    Tuplet { num: u32, den: u32, body: Vec<Self> },
    /// A bar check `|` between measures.
    BarCheck,
    /// A lilypond command line (`\clef "treble"`, `\time 4/4`, `\key a \minor`).
    Command(String),
}

/// A complete `.ly` document.
struct LyDocument {
    version: &'static str,
    /// `(variable name, music)` in deterministic declaration order.
    variables: Vec<(String, LyNode)>,
    /// The `\score` block body.
    score: LyNode,
}

/// Render the plan to deterministic `LilyPond` text.
///
/// # Errors
/// [`RenderError::Unsupported`] when a piece's duration cannot be spelled as
/// a plain or dotted `LilyPond` duration (cannot happen after prompt 07's
/// decomposition; the error exists per §7.2, not for control flow).
pub(crate) fn render_lilypond(plan: &NotationPlan) -> Result<String, RenderError> {
    let mut variables = Vec::new();
    let mut score_children = Vec::new();
    for staff in plan.staves() {
        let variable = sanitize(staff.name());
        let body = staff_body(staff)?;
        score_children.push(LyNode::Command(format!("\\new Staff \\{variable}")));
        variables.push((variable, body));
    }
    let document = LyDocument {
        version: "2.24.0",
        variables,
        score: LyNode::Simultaneous(score_children),
    };
    Ok(print_document(&document))
}

/// One staff's music: header commands, then measures; multi-voice staves
/// become simultaneous voice blocks.
fn staff_body(staff: &StaffPlan) -> Result<LyNode, RenderError> {
    let mut head = Vec::new();
    if let Some(clef) = staff.clef() {
        let name = match clef {
            Clef::Treble => "treble",
            Clef::Bass => "bass",
            Clef::Alto => "alto",
            Clef::Tenor => "tenor",
        };
        head.push(LyNode::Command(format!("\\clef \"{name}\"")));
    }
    let (count, unit) = staff.time_signature();
    head.push(LyNode::Command(format!("\\time {count}/{unit}")));
    if let Some(key) = staff.key_signature() {
        let tonic = tonic_name(key.fifths, key.mode);
        let mode = match key.mode {
            Mode::Major => "\\major",
            Mode::Minor => "\\minor",
        };
        head.push(LyNode::Command(format!("\\key {tonic} {mode}")));
    }

    let lane_count = staff.measures().first().map_or(0, |m| m.lanes().len());
    let mut lanes: Vec<Vec<LyNode>> = (0..lane_count).map(|_| Vec::new()).collect();
    for measure in staff.measures() {
        for (lane_index, lane) in measure.lanes().iter().enumerate() {
            let nodes = lanes.get_mut(lane_index);
            if let Some(nodes) = nodes {
                nodes.extend(lane_body(lane, count, unit)?);
                nodes.push(LyNode::BarCheck);
            }
        }
    }
    let music = if lanes.len() <= 1 {
        LyNode::Sequential(lanes.into_iter().next().unwrap_or_default())
    } else {
        LyNode::Simultaneous(lanes.into_iter().map(LyNode::Sequential).collect())
    };
    head.push(music);
    Ok(LyNode::Sequential(head))
}

/// One lane's music for one measure.
fn lane_body(lane: &VoiceLane, count: u32, unit: u32) -> Result<Vec<LyNode>, RenderError> {
    if lane.items().is_empty() {
        // An uncovered measure of this voice renders as spacer skips — a
        // notation decision (docs/kernel/02): the kernel stored nothing.
        let mut nodes = Vec::new();
        for piece in spell_pieces(i64::from(count), i64::from(unit)) {
            nodes.push(LyNode::Note {
                body: format!("s{piece}"),
                event: EventId(u64::MAX),
            });
        }
        return Ok(nodes);
    }
    // A tuplet bracket wraps the items it covers, so the lane is built as a
    // stack: items go to the innermost open bracket.
    let mut nodes = Vec::new();
    let mut open: Option<(u32, u32, Vec<LyNode>)> = None;
    for item in lane.items() {
        if let Some(tuplet) = item.tuplet()
            && tuplet.start
            && open.is_none()
        {
            open = Some((tuplet.num, tuplet.den, Vec::new()));
        }
        let node = item_node(item)?;
        match open.as_mut() {
            Some((_, _, body)) => body.push(node),
            None => nodes.push(node),
        }
        if item.tuplet().is_some_and(|tuplet| tuplet.stop)
            && let Some((num, den, body)) = open.take()
        {
            nodes.push(LyNode::Tuplet { num, den, body });
        }
    }
    // A bracket the measure did not close (the compiler forbids it, but the
    // printer must still produce a document): close it here.
    if let Some((num, den, body)) = open.take() {
        nodes.push(LyNode::Tuplet { num, den, body });
    }
    Ok(nodes)
}

/// One plan item as a `LilyPond` note/chord/rest token, with the marks that
/// attach to it in `LilyPond`'s own order: duration, tie, articulations,
/// dynamic, slur.
fn item_node(item: &NotatedItem) -> Result<LyNode, RenderError> {
    let value = item.duration().value.as_ratio();
    let Some(duration) = spell_duration(*value.numer(), *value.denom()) else {
        return Err(RenderError::Unsupported {
            event: item.event(),
            what: format!("cannot spell duration {value} as a LilyPond duration"),
        });
    };
    let tie = if item.tie_start() { "~" } else { "" };
    let head = match item.kind() {
        NotatedKind::Rest => format!("r{duration}{tie}"),
        NotatedKind::Note { pitch } => format!("{}{duration}{tie}", pitch_name(*pitch)),
        NotatedKind::Chord { pitches } => {
            let tones: Vec<String> = pitches.iter().map(|pitch| pitch_name(*pitch)).collect();
            format!("<{}>{duration}{tie}", tones.join(" "))
        }
    };
    let mut body = head;
    for mark in item.articulations() {
        body.push_str(articulation_script(*mark));
    }
    if let Some(dynamic) = item.dynamic() {
        body.push('\\');
        body.push_str(dynamic.name());
    }
    if item.slur_start() {
        body.push('(');
    }
    if item.slur_stop() {
        body.push(')');
    }
    Ok(LyNode::Note {
        body,
        event: item.event(),
    })
}

/// `LilyPond`'s articulation scripts, with the direction the plan chose.
fn articulation_script(mark: ArticulationMark) -> &'static str {
    let side = match ARTICULATION_PLACEMENT {
        Placement::Above => '^',
        Placement::Below => '_',
    };
    match (mark, side) {
        (ArticulationMark::Staccato, '^') => "^.",
        (ArticulationMark::Staccato, _) => "_.",
        (ArticulationMark::Staccatissimo, '^') => "^!",
        (ArticulationMark::Staccatissimo, _) => "_!",
        (ArticulationMark::Tenuto, '^') => "^-",
        (ArticulationMark::Tenuto, _) => "_-",
        (ArticulationMark::Accent, '^') => "^>",
        (ArticulationMark::Accent, _) => "_>",
        (ArticulationMark::Marcato, '^') => "^^",
        (ArticulationMark::Marcato, _) => "_^",
    }
}

/// Absolute-octave English note name (`cs'`, `eff,`).
fn pitch_name(pitch: WrittenPitch) -> String {
    let letter = match pitch.letter {
        musa_compiler::Letter::C => "c",
        musa_compiler::Letter::D => "d",
        musa_compiler::Letter::E => "e",
        musa_compiler::Letter::F => "f",
        musa_compiler::Letter::G => "g",
        musa_compiler::Letter::A => "a",
        musa_compiler::Letter::B => "b",
    };
    let accidental = match pitch.accidental.0 {
        1 => "s",
        2 => "ss",
        -1 => "f",
        -2 => "ff",
        _ => "",
    };
    // LilyPond: c' is middle C (our octave 4); each ' raises, each , lowers.
    let marks: String = if pitch.octave >= 3 {
        "'".repeat(usize::try_from(pitch.octave.saturating_sub(3)).unwrap_or(usize::MAX))
    } else {
        ",".repeat(usize::try_from(3i8.saturating_sub(pitch.octave)).unwrap_or(usize::MAX))
    };
    format!("{letter}{accidental}{marks}")
}

/// The key-signature tonic for a fifths count and mode (circle of fifths).
fn tonic_name(fifths: i8, mode: Mode) -> &'static str {
    const MAJOR: [&str; 15] = [
        "cf", "gf", "df", "af", "ef", "bf", "f", "c", "g", "d", "a", "e", "b", "fs", "cs",
    ];
    const MINOR: [&str; 15] = [
        "af", "ef", "bf", "f", "c", "g", "d", "a", "e", "b", "fs", "cs", "gs", "ds", "as",
    ];
    let index = usize::try_from(fifths.saturating_add(7)).unwrap_or(7).min(14);
    match mode {
        Mode::Major => MAJOR.get(index).copied().unwrap_or("c"),
        Mode::Minor => MINOR.get(index).copied().unwrap_or("a"),
    }
}

/// A variable name from a part name: `part` plus camelCase words (`LilyPond`
/// identifiers are letters only — no hyphens or underscores).
fn sanitize(name: &str) -> String {
    let mut out = String::from("part");
    let mut new_word = true;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if new_word {
                out.extend(ch.to_uppercase());
                new_word = false;
            } else {
                out.push(ch.to_ascii_lowercase());
            }
        } else {
            new_word = true;
        }
    }
    out
}

/// The `LilyPond` duration token for a plain (`1/2^k`) or dotted (`3/2^(k+1)`)
/// value, or `None` when unspellable.
fn spell_duration(numerator: i64, denominator: i64) -> Option<String> {
    if numerator == 1 {
        Some(denominator.to_string())
    } else if numerator == 3 && denominator % 2 == 0 {
        Some(format!("{}.", denominator / 2))
    } else {
        None
    }
}

/// Greedy plain/dotted spelling of an arbitrary `count/unit` measure length
/// (for spacer skips), largest pieces first.
fn spell_pieces(count: i64, unit: i64) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut remaining = num_rational::Ratio::new(count, unit);
    while remaining > num_rational::Ratio::ZERO {
        let mut plain = num_rational::Ratio::from_integer(1);
        while plain > remaining {
            plain /= 2;
        }
        let dotted = plain * num_rational::Ratio::new(3, 2);
        let piece = if dotted <= remaining { dotted } else { plain };
        if let Some(token) = spell_duration(*piece.numer(), *piece.denom()) {
            pieces.push(token);
        }
        remaining -= piece;
    }
    pieces
}

// --- Deterministic pretty-printer -------------------------------------------

fn print_document(document: &LyDocument) -> String {
    let mut out = String::new();
    out.push_str("\\version \"");
    out.push_str(document.version);
    out.push_str("\"\n\\language \"english\"\n\n");
    for (name, body) in &document.variables {
        out.push_str(name);
        out.push_str(" = ");
        print_node(&mut out, body, 0);
        out.push('\n');
    }
    out.push_str("\\score {\n  ");
    match &document.score {
        LyNode::Simultaneous(children) => {
            out.push_str("<<\n");
            for child in children {
                out.push_str("    ");
                print_node(&mut out, child, 4);
                out.push('\n');
            }
            out.push_str("  >>\n");
        }
        single @ (LyNode::Sequential(_)
        | LyNode::Note { .. }
        | LyNode::Tuplet { .. }
        | LyNode::BarCheck
        | LyNode::Command(_)) => {
            print_node(&mut out, single, 2);
            out.push('\n');
        }
    }
    out.push_str("}\n");
    out
}

/// `{ ... }` with one child per line at `indent + 1`.
fn print_sequence(out: &mut String, children: &[LyNode], indent: usize) {
    let pad = "  ".repeat(indent);
    out.push_str("{\n");
    for child in children {
        out.push_str(&pad);
        out.push_str("  ");
        print_node(out, child, indent.saturating_add(1));
        out.push('\n');
    }
    out.push_str(&pad);
    out.push('}');
}

fn print_node(out: &mut String, node: &LyNode, indent: usize) {
    let pad = "  ".repeat(indent);
    match node {
        LyNode::Sequential(children) => print_sequence(out, children, indent),
        LyNode::Simultaneous(children) => {
            out.push_str("<<\n");
            let last = children.len().saturating_sub(1);
            for (index, child) in children.iter().enumerate() {
                out.push_str(&pad);
                out.push_str("  ");
                print_node(out, child, indent.saturating_add(1));
                if index < last {
                    out.push_str(" \\\\");
                }
                out.push('\n');
            }
            out.push_str(&pad);
            out.push_str(">>");
        }
        LyNode::Note { body, event } => {
            out.push_str(body);
            if *event != EventId(u64::MAX) {
                let comment = format!(" % event:{:x}", event.0);
                out.push_str(&comment);
            }
        }
        LyNode::Tuplet { num, den, body } => {
            let bracket = format!("\\tuplet {num}/{den} ");
            out.push_str(&bracket);
            print_sequence(out, body, indent);
        }
        LyNode::BarCheck => out.push('|'),
        LyNode::Command(text) => out.push_str(text),
    }
}
