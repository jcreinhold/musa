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

use musa_compiler::{ArticulationMark, ChordQuality, ChordSymbol, Clef, EventId, Mode, Seventh, WrittenPitch};

use crate::RenderError;
use crate::plan::{
    ARTICULATION_PLACEMENT, NotatedItem, NotatedKind, NotationPlan, Placement, PositionedMark, StaffPlan, VoiceLane,
};

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
    /// `\header { … }` — the piece's front matter, in `LilyPond`'s own slots.
    header: Vec<(&'static str, String)>,
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
    if !plan.harmony().is_empty() {
        let measure_len = measure_length(plan);
        variables.push(("chords".to_owned(), chord_names(plan.harmony(), measure_len)));
        score_children.push(LyNode::Command("\\new ChordNames \\chords".to_owned()));
    }
    for (index, staff) in plan.staves().iter().enumerate() {
        let variable = sanitize(staff.name());
        // Form markers are score-wide, so they are written once, in the
        // topmost staff: `\mark` is a Score-level event and LilyPond prints it
        // above the system however many staves the system has.
        let marks = if index == 0 { score_marks(plan) } else { Vec::new() };
        // Repeat barlines are the same kind of score-wide fact, and
        // `Score.repeatCommands` is a score property: written once, in the
        // topmost staff, or every staff would set it again.
        let measures = u32::try_from(staff.measures().len()).unwrap_or(u32::MAX);
        let repeats = if index == 0 {
            repeat_commands(plan, measures)
        } else {
            Commands::default()
        };
        let body = staff_body(staff, &marks, &repeats)?;
        score_children.push(LyNode::Command(format!("\\new Staff \\{variable}")));
        variables.push((variable, body));
    }
    let document = LyDocument {
        version: "2.24.0",
        header: header_fields(plan),
        variables,
        score: LyNode::Simultaneous(score_children),
    };
    Ok(print_document(&document))
}

/// The `\header` fields the piece names, in the order `LilyPond` prints them.
///
/// Absent lines are absent rather than empty: `LilyPond` reserves vertical space
/// for a field it is given, so an empty `composer` is a blank line above the
/// first system.
fn header_fields(plan: &NotationPlan) -> Vec<(&'static str, String)> {
    let front = plan.front();
    let mut fields = vec![("title", front.title.clone())];
    for (name, value) in [
        ("subtitle", &front.subtitle),
        ("composer", &front.composer),
        ("arranger", &front.arranger),
        ("copyright", &front.copyright),
    ] {
        if let Some(text) = value.as_deref() {
            fields.push((name, text.to_owned()));
        }
    }
    fields
}

/// `\set Score.repeatCommands` settings, by the barline they stand at.
///
/// `LilyPond` also has `\repeat volta 2 { … } \alternative { … }`, which needs
/// the music to be *nested* — and the plan is a flat stream of measures,
/// because that is what a score is once the barlines are decided.
/// `repeatCommands` is the same notation said at the measure boundaries, which
/// is where this backend already stands.
///
/// One entry per barline, because `repeatCommands` is a
/// *setting*: two of them at one boundary is the second one, not both. The
/// bracket that closes and the bracket that opens at the same barline are one
/// command, `(volta #f) end-repeat (volta "2.")`, or the repeat sign is lost.
#[derive(Default)]
struct Commands {
    /// The setting to write before a measure.
    before: std::collections::HashMap<u32, String>,
    /// The setting to write after the last measure, closing a bracket that
    /// runs to the end of the piece.
    trailing: Option<String>,
}

fn repeat_commands(plan: &NotationPlan, measures: u32) -> Commands {
    let mut parts: std::collections::BTreeMap<u32, Vec<String>> = std::collections::BTreeMap::new();
    let mut at = |barline: u32, command: String| parts.entry(barline).or_default().push(command);
    for repeat in plan.repeats() {
        let Some((last, earlier)) = repeat.endings.split_last() else {
            at(repeat.from, "start-repeat".to_owned());
            at(repeat.to.saturating_add(1), "end-repeat".to_owned());
            continue;
        };
        at(repeat.from, "start-repeat".to_owned());
        for volta in earlier {
            at(volta.from, format!("(volta \"{}.\")", volta.label()));
            at(volta.to.saturating_add(1), "(volta #f)".to_owned());
            at(volta.to.saturating_add(1), "end-repeat".to_owned());
        }
        at(last.from, format!("(volta \"{}.\")", last.label()));
        at(last.to.saturating_add(1), "(volta #f)".to_owned());
    }
    let mut commands = Commands::default();
    for (barline, body) in parts {
        let setting = format!("\\set Score.repeatCommands = #'({})", body.join(" "));
        if barline > measures {
            commands.trailing = Some(setting);
        } else {
            commands.before.insert(barline, setting);
        }
    }
    commands
}

/// One staff's music: header commands, then measures; multi-voice staves
/// become simultaneous voice blocks.
fn staff_body(
    staff: &StaffPlan,
    sections: &[PositionedMark<ScoreMark>],
    repeats: &Commands,
) -> Result<LyNode, RenderError> {
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
                let here: Vec<&PositionedMark<ScoreMark>> = if lane_index == 0 {
                    sections
                        .iter()
                        .filter(|mark| mark.measure == measure.number())
                        .collect()
                } else {
                    Vec::new()
                };
                if lane_index == 0
                    && let Some(command) = repeats.before.get(&measure.number())
                {
                    nodes.push(LyNode::Command(command.clone()));
                }
                nodes.extend(lane_body(lane, count, unit, &here)?);
                nodes.push(LyNode::BarCheck);
            }
        }
    }
    if let Some(command) = repeats.trailing.as_ref()
        && let Some(nodes) = lanes.first_mut()
    {
        nodes.push(LyNode::Command(command.clone()));
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
fn lane_body(
    lane: &VoiceLane,
    count: u32,
    unit: u32,
    sections: &[&PositionedMark<ScoreMark>],
) -> Result<Vec<LyNode>, RenderError> {
    if lane.items().is_empty() {
        // An uncovered measure of this voice renders as spacer skips — a
        // notation decision (docs/kernel/02): the kernel stored nothing.
        let mut nodes = Vec::new();
        for mark in sections {
            nodes.push(mark_node(&mark.what));
        }
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
    let mut pending = sections.iter();
    let mut next_mark = pending.next();
    for item in lane.items() {
        // A form marker prints where it falls, which in LilyPond means before
        // the note that follows it; a marker past the last note of the measure
        // lands after them all, below.
        while let Some(mark) = next_mark.filter(|mark| mark.onset_in_measure <= item.onset_in_measure()) {
            nodes.push(mark_node(&mark.what));
            next_mark = pending.next();
        }
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
    while let Some(mark) = next_mark {
        nodes.push(mark_node(&mark.what));
        next_mark = pending.next();
    }
    Ok(nodes)
}

/// A symbol the whole score reads, written into the topmost staff at the
/// place it falls.
enum ScoreMark {
    /// A form marker.
    Section(String),
    /// A tempo mark.
    Tempo(crate::plan::TempoText),
}

/// The score-level marks in playing order: at one place, the tempo is read
/// before the section name, the way a conductor reads the top of a page.
fn score_marks(plan: &NotationPlan) -> Vec<PositionedMark<ScoreMark>> {
    let mut marks: Vec<PositionedMark<ScoreMark>> = Vec::new();
    for tempo in plan.tempos() {
        marks.push(PositionedMark {
            measure: tempo.measure,
            onset_in_measure: tempo.onset_in_measure,
            what: ScoreMark::Tempo(tempo.what),
        });
    }
    for section in plan.sections() {
        marks.push(PositionedMark {
            measure: section.measure,
            onset_in_measure: section.onset_in_measure,
            what: ScoreMark::Section(section.what.clone()),
        });
    }
    marks.sort_by_key(|mark| (mark.measure, mark.onset_in_measure.as_ratio()));
    marks
}

/// A score-level mark as `LilyPond` writes it: a rehearsal mark carrying the
/// name rather than the automatic letter, or a metronome mark.
fn mark_node(mark: &ScoreMark) -> LyNode {
    match mark {
        ScoreMark::Section(name) => {
            let escaped = name.replace('\\', "").replace('"', "'");
            LyNode::Command(format!("\\mark \\markup {{ \\bold \"{escaped}\" }}"))
        }
        ScoreMark::Tempo(tempo) => {
            let unit = spell_duration(*tempo.beat.numer(), *tempo.beat.denom()).unwrap_or_else(|| "4".to_owned());
            LyNode::Command(format!("\\tempo {unit} = {}", tempo.bpm))
        }
    }
}

/// The harmony lane as a `\chordmode` sequence: skips up to each symbol, then
/// the symbol itself lasting until the next one.
///
/// `ChordNames` is what `LilyPond` renders best — the symbols sit in their own
/// line above the system and are spaced against the music — and it is what a
/// `LilyPond` user would write by hand. It costs one thing markup would not:
/// the chord has to be spelled in `chordmode`'s vocabulary rather than printed
/// verbatim, which is exactly what parsing the symbol bought.
fn chord_names(harmony: &[PositionedMark<ChordSymbol>], measure_len: num_rational::Ratio<i64>) -> LyNode {
    let mut nodes = Vec::new();
    let mut at = num_rational::Ratio::ZERO;
    for (index, chord) in harmony.iter().enumerate() {
        let start = measure_len * num_rational::Ratio::from_integer(i64::from(chord.measure.saturating_sub(1)))
            + chord.onset_in_measure.as_ratio();
        for piece in spell_ratio(start - at) {
            nodes.push(LyNode::Note {
                body: format!("s{piece}"),
                event: EventId(u64::MAX),
            });
        }
        let next = harmony.get(index.saturating_add(1)).map_or_else(
            // The last symbol holds to the end of its measure; nothing after
            // it disagrees, and a chord with no length prints nothing.
            || (start / measure_len).floor() * measure_len + measure_len,
            |next| {
                measure_len * num_rational::Ratio::from_integer(i64::from(next.measure.saturating_sub(1)))
                    + next.onset_in_measure.as_ratio()
            },
        );
        let held = if next > start { next - start } else { measure_len };
        let modifier = chordmode_modifier(&chord.what);
        for (piece_index, piece) in spell_ratio(held).into_iter().enumerate() {
            let body = if piece_index == 0 {
                format!("{}{piece}{modifier}", chordmode_root(&chord.what))
            } else {
                // A held symbol is one chord in LilyPond, tied across the
                // pieces its length decomposes into.
                format!("~ {}{piece}{modifier}", chordmode_root(&chord.what))
            };
            nodes.push(LyNode::Note {
                body,
                event: EventId(u64::MAX),
            });
        }
        at = next;
    }
    LyNode::Sequential(vec![
        LyNode::Command("\\chordmode".to_owned()),
        LyNode::Sequential(nodes),
    ])
}

/// The chord root in `LilyPond`'s english note names.
fn chordmode_root(chord: &ChordSymbol) -> String {
    pitch_name(WrittenPitch {
        letter: chord.letter,
        accidental: chord.accidental,
        octave: 3,
    })
}

/// The `:modifier` a chord symbol becomes in `chordmode`.
///
/// `LilyPond` spells a raised seventh as `7+`, which is how a minor-major
/// seventh (`cmmaj7`) is written; the word modifiers (`sus4`, `dim`, `aug`)
/// join a step with a dot, while `m` and `maj` prefix it directly, because
/// that is the spelling `LilyPond`'s own documentation uses.
fn chordmode_modifier(chord: &ChordSymbol) -> String {
    let base = match chord.quality {
        ChordQuality::Major => "",
        ChordQuality::Minor => "m",
        ChordQuality::Diminished => "dim",
        ChordQuality::Augmented => "aug",
        ChordQuality::Suspended2 => "sus2",
        ChordQuality::Suspended4 => "sus4",
    };
    let Some(seventh) = chord.seventh else {
        return match chord.extension {
            Some(step) => {
                if base.is_empty() {
                    format!(":{step}")
                } else {
                    format!(":{base}.{step}")
                }
            }
            None => {
                if base.is_empty() {
                    String::new()
                } else {
                    format!(":{base}")
                }
            }
        };
    };
    let step = chord.extension.unwrap_or(7);
    match (seventh, base) {
        (Seventh::Major, "") => format!(":maj{step}"),
        (Seventh::Major, base) => format!(":{base}{step}+"),
        (Seventh::Minor | Seventh::Diminished, "") => format!(":{step}"),
        (Seventh::Minor | Seventh::Diminished, "m") => format!(":m{step}"),
        (Seventh::Minor | Seventh::Diminished, base) => format!(":{base}{step}"),
    }
}

/// One measure's length in whole notes, from the first staff's meter.
fn measure_length(plan: &NotationPlan) -> num_rational::Ratio<i64> {
    plan.staves().first().map_or(num_rational::Ratio::ONE, |staff| {
        let (count, unit) = staff.time_signature();
        num_rational::Ratio::new(i64::from(count), i64::from(unit.max(1)))
    })
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
    // LilyPond has no phrase bracket that does not need an engraver added to
    // the layout, so a phrase prints as its name above its first note. The
    // name is the part a reader needs; the bracket is what the source already
    // shows.
    if let Some(phrase) = item.phrase().filter(|phrase| phrase.start) {
        let escaped = phrase.name.replace('\\', "").replace('"', "'");
        let markup = format!("^\\markup {{ \\italic \"{escaped}\" }}");
        body.push_str(&markup);
    }
    // A wedge opens on its first note and closes on the mark it arrives at,
    // which is exactly how it is written by hand: `c\< d e f\f`.
    if let Some(hairpin) = item.hairpin() {
        if hairpin.start {
            body.push_str(if hairpin.grows { "\\<" } else { "\\>" });
        }
        if hairpin.stop {
            body.push('\\');
            body.push_str(hairpin.target.name());
        }
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
    spell_ratio(num_rational::Ratio::new(count, unit))
}

/// The same, for a length already in whole notes.
fn spell_ratio(length: num_rational::Ratio<i64>) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut remaining = length;
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
    out.push_str("\\header {\n");
    for (field, value) in &document.header {
        out.push_str("  ");
        out.push_str(field);
        out.push_str(" = \"");
        out.push_str(&value.replace('\\', "\\\\").replace('"', "\\\""));
        out.push_str("\"\n");
    }
    out.push_str("}\n\n");
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
