//! The `MusicXML` backend (roadmap §12.4): deterministic `score-partwise` 4.0
//! from a `NotationPlan`, built through quick-xml's writer — never string
//! assembly.
//!
//! `MusicXML` is an **edge format**. It is what leaves the workbench for
//! Finale, Sibelius, `MuseScore`, or Dorico; nothing about it comes back in
//! (import is Phase 4) and no `MusicXML` convention is allowed to reach the
//! score model. The plan drives the document, the same plan MEI and
//! `LilyPond` read.
//!
//! **No event provenance, deliberately.** `MusicXML` has no standard note
//! identifier — `id` attributes exist but no consumer round-trips them — so
//! this backend embeds none rather than inventing a musa-only convention that
//! would look like interchange and behave like a private extension. That
//! asymmetry is exactly why MEI is the live format the editor talks to
//! (§12.2) and `MusicXML` is the one it hands over.
//!
//! **Divisions.** `MusicXML` measures time in integer divisions of a quarter
//! note, so one value has to represent every duration and every onset in the
//! score exactly. It is computed as the least common multiple of what each
//! rational demands, once for the whole document; a score that would need
//! more than [`MAX_DIVISIONS`] is refused with [`RenderError::Divisions`]
//! rather than rounded (§7.2 — no silent approximation at an edge).
//!
//! **Accidentals.** `<pitch>` carries `<step>`/`<alter>`/`<octave>`, which
//! determines the spelling completely (B-flat and A-sharp differ in step).
//! The printed `<accidental>` element is *not* written: it means "print this
//! symbol here", and musa has no cautionary-accidental model to justify
//! forcing one. Consumers derive the printed accidentals from the alteration,
//! the key signature, and the measure — which reproduces what the source
//! spelled.

// Rational duration arithmetic is exact and total for musa's magnitudes (see
// musa-compiler/src/time.rs); the workspace arithmetic lint is allowed at
// module scope for that reason.
#![allow(clippy::arithmetic_side_effects)]

use std::collections::HashMap;

use musa_compiler::{ChordQuality, ChordSymbol, Clef, DynamicMark, Mode, Seventh, Slot, WrittenPitch};
use num_rational::Ratio;
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};

use crate::RenderError;
use crate::plan::{
    ARTICULATION_PLACEMENT, DYNAMIC_PLACEMENT, NotatedItem, NotatedKind, NotationPlan, Placement, SLUR_PLACEMENT,
    StaffPlan, VoiceLane,
};

/// The largest divisions-per-quarter this backend will emit. Well past what
/// any tuplet nesting a human writes needs (a 7:5 inside a 5:3 inside a
/// triplet lands near 3 000), and small enough that every duration in the
/// document stays an ordinary integer for the consumer.
const MAX_DIVISIONS: i64 = 1 << 20;

/// How many slurs `MusicXML` can distinguish at one time.
const SLUR_NUMBERS: u32 = 6;

/// Render the plan to deterministic `MusicXML` text.
///
/// # Errors
/// [`RenderError::Divisions`] when no reasonable divisions value represents
/// the score's durations exactly, [`RenderError::Unsupported`] for a duration
/// with no `MusicXML` note type, [`RenderError::Xml`] on writer failures.
pub(crate) fn render_musicxml(plan: &NotationPlan) -> Result<String, RenderError> {
    let divisions = divisions_for(plan)?;
    let mut xml = Xml::new();
    xml.declaration()?;
    xml.doctype()?;
    xml.open("score-partwise", &[("version", "4.0")])?;

    // The front matter, in MusicXML's own slots. `<work>` precedes
    // `<identification>` in the partwise content model, and both precede
    // `<part-list>`; a reader that gets them out of order rejects the file.
    let front = plan.front();
    xml.open("work", &[])?;
    xml.leaf("work-title", &[], &front.title)?;
    xml.close("work")?;
    if let Some(subtitle) = front.subtitle.as_deref() {
        // MusicXML has no subtitle: it belongs to the movement, which is what
        // a subtitle names in a single-movement piece.
        xml.leaf("movement-title", &[], subtitle)?;
    }
    xml.open("identification", &[])?;
    if let Some(composer) = front.composer.as_deref() {
        xml.leaf("creator", &[("type", "composer")], composer)?;
    }
    if let Some(arranger) = front.arranger.as_deref() {
        xml.leaf("creator", &[("type", "arranger")], arranger)?;
    }
    if let Some(copyright) = front.copyright.as_deref() {
        xml.leaf("rights", &[], copyright)?;
    }
    xml.open("encoding", &[])?;
    // No version and no encoding date: an export of one source is the same
    // bytes on every machine and every build (§17.5's determinism rule).
    xml.leaf("software", &[], "musa")?;
    xml.close("encoding")?;
    // Which reading this is, in the one place `MusicXML` keeps notes about a
    // file rather than about the music. Absent for a determinate piece, which
    // has no reading to name.
    if let Some(performance) = front.performance {
        xml.open("miscellaneous", &[])?;
        xml.leaf(
            "miscellaneous-field",
            &[("name", "realization")],
            &format!("One realization of an open work \u{2014} performance {performance}."),
        )?;
        xml.close("miscellaneous")?;
    }
    xml.close("identification")?;

    xml.open("part-list", &[])?;
    for (index, staff) in plan.staves().iter().enumerate() {
        xml.open("score-part", &[("id", &part_id(index))])?;
        xml.leaf("part-name", &[], staff.name())?;
        xml.close("score-part")?;
    }
    xml.close("part-list")?;

    // Repeat barlines, unlike those symbols, belong to every part: a reader
    // that saw `:|` on one staff and not the others would have two scores.
    let marks = barlines(plan);
    for (index, staff) in plan.staves().iter().enumerate() {
        // Chord symbols and form markers belong to the score, not to a part,
        // and MusicXML has nowhere to put a score-wide symbol: they are
        // written in the first part, which is where a reader expects them.
        let annotations = if index == 0 { Some(plan) } else { None };
        write_part(&mut xml, &part_id(index), staff, divisions, annotations, &marks)?;
    }
    xml.close("score-partwise")?;
    xml.finish()
}

/// `P1`, `P2`, … — the conventional part identifier, and the only identity
/// this document carries.
fn part_id(index: usize) -> String {
    format!("P{}", index.saturating_add(1))
}

// --- Divisions ---------------------------------------------------------------

/// The sounding length of one item in whole notes. Inside a tuplet the symbol
/// and the sound differ, and `<duration>` is the sound (`<type>` is the
/// symbol).
fn sounding(item: &NotatedItem) -> Ratio<i64> {
    let symbol = item.duration().value.as_ratio();
    item.tuplet().map_or(symbol, |tuplet| {
        symbol * Ratio::new(i64::from(tuplet.den), i64::from(tuplet.num))
    })
}

/// How many divisions per quarter note this length needs to be an integer.
fn demand(length: Ratio<i64>) -> i64 {
    *(length * Ratio::from_integer(4)).denom()
}

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let next = a % b;
        a = b;
        b = next;
    }
    a.max(1)
}

/// Fold one more demand into the running divisions value.
fn widen(divisions: i64, length: Ratio<i64>) -> Result<i64, RenderError> {
    let needed = demand(length);
    let combined = divisions / gcd(divisions, needed);
    let combined = combined.checked_mul(needed).filter(|value| *value <= MAX_DIVISIONS);
    combined.ok_or_else(|| RenderError::Divisions {
        duration: length.to_string(),
        max: MAX_DIVISIONS,
    })
}

/// One divisions value for the whole document: every measure length, onset,
/// and sounding duration in it must land on an integer.
fn divisions_for(plan: &NotationPlan) -> Result<i64, RenderError> {
    let mut divisions = 1;
    for staff in plan.staves() {
        divisions = widen(divisions, measure_duration(staff))?;
        for measure in staff.measures() {
            divisions = widen(divisions, measure.duration().as_ratio())?;
            for lane in measure.lanes() {
                for item in lane.items() {
                    divisions = widen(divisions, item.onset_in_measure().as_ratio())?;
                    divisions = widen(divisions, sounding(item))?;
                }
            }
        }
    }
    Ok(divisions)
}

/// A measure's length in whole notes, from the time signature.
fn measure_duration(staff: &StaffPlan) -> Ratio<i64> {
    let (count, unit) = staff.time_signature();
    if unit == 0 {
        Ratio::ZERO
    } else {
        Ratio::new(i64::from(count), i64::from(unit))
    }
}

/// A length in whole notes as `MusicXML` divisions. Exact by construction:
/// `divisions_for` already folded in every length this is called with.
fn ticks(length: Ratio<i64>, divisions: i64) -> i64 {
    let scaled = length * Ratio::new(divisions * 4, 1);
    debug_assert_eq!(*scaled.denom(), 1, "divisions_for missed a duration");
    scaled.to_integer()
}

// --- Note spelling -----------------------------------------------------------

/// The `MusicXML` note type and dot count for a written value. Decomposition
/// guarantees a plain (`1/2^k`) or dotted (`3/2^(k+1)`) value.
fn note_type(symbol: Ratio<i64>) -> Option<(&'static str, u32)> {
    let (base, dots) = match *symbol.numer() {
        1 => (*symbol.denom(), 0),
        3 if symbol.denom() % 2 == 0 => (symbol.denom() / 2, 1),
        _ => return None,
    };
    let name = match base {
        1 => "whole",
        2 => "half",
        4 => "quarter",
        8 => "eighth",
        16 => "16th",
        32 => "32nd",
        64 => "64th",
        128 => "128th",
        256 => "256th",
        _ => return None,
    };
    Some((name, dots))
}

/// How many beams a written value carries: an eighth one, a sixteenth two,
/// and so on. Zero for anything a quarter or longer.
fn beam_levels(symbol: Ratio<i64>) -> u32 {
    let base = match *symbol.numer() {
        1 => *symbol.denom(),
        3 if symbol.denom() % 2 == 0 => symbol.denom() / 2,
        _ => return 0,
    };
    let mut level = 0;
    let mut value = 4;
    while value < base && level < 8 {
        value *= 2;
        level += 1;
    }
    level
}

/// `<beam>` levels for one item, read from its neighbors in the same group.
///
/// A level the item has and a neighbor does not is a hook, which is how
/// `MusicXML` spells the short side of a dotted-eighth/sixteenth pair.
fn beams(items: &[NotatedItem], index: usize) -> Vec<(u32, &'static str)> {
    let Some(item) = items.get(index) else {
        return Vec::new();
    };
    let Some(group) = item.beam() else {
        return Vec::new();
    };
    let levels_of = |at: Option<usize>| -> u32 {
        at.and_then(|at| items.get(at))
            .filter(|neighbor| neighbor.beam() == Some(group))
            .map_or(0, |neighbor| beam_levels(neighbor.duration().value.as_ratio()))
    };
    let before = levels_of(index.checked_sub(1));
    let after = levels_of(Some(index.saturating_add(1)));
    // A group of one is not a beam; the note keeps its flags.
    if before == 0 && after == 0 {
        return Vec::new();
    }
    let own = beam_levels(item.duration().value.as_ratio());
    (1..=own)
        .map(|level| {
            let kind = match (before >= level, after >= level) {
                (false, true) => "begin",
                (true, true) => "continue",
                (true, false) => "end",
                (false, false) if after > 0 => "forward hook",
                (false, false) => "backward hook",
            };
            (level, kind)
        })
        .collect()
}

/// The side of the staff a symbol takes, as `MusicXML` spells it.
fn place(placement: Placement) -> &'static str {
    match placement {
        Placement::Above => "above",
        Placement::Below => "below",
    }
}

/// The `<clef>` sign and line for a written clef.
fn clef_sign_line(clef: Clef) -> (&'static str, &'static str) {
    match clef {
        Clef::Treble => ("G", "2"),
        Clef::Bass => ("F", "4"),
        Clef::Alto => ("C", "3"),
        Clef::Tenor => ("C", "4"),
    }
}

fn step_of(pitch: WrittenPitch) -> &'static str {
    match pitch.letter {
        musa_compiler::Letter::C => "C",
        musa_compiler::Letter::D => "D",
        musa_compiler::Letter::E => "E",
        musa_compiler::Letter::F => "F",
        musa_compiler::Letter::G => "G",
        musa_compiler::Letter::A => "A",
        musa_compiler::Letter::B => "B",
    }
}

// --- Document ----------------------------------------------------------------

/// What a measure carries at its two barlines.
///
/// `MusicXML` writes these per part, so unlike MEI's measure-level attributes
/// they are repeated on every staff — the same fact, said once per part
/// because that is the shape of the file.
#[derive(Default)]
struct Barline {
    /// A forward repeat sign at the left barline.
    forward: bool,
    /// A volta bracket opening here, by its label.
    ending_start: Option<String>,
    /// A volta bracket closing here: its label, and whether it turns back
    /// (`stop`) or simply runs out (`discontinue`).
    ending_stop: Option<(String, &'static str)>,
    /// A backward repeat sign at the right barline.
    backward: bool,
}

fn barlines(plan: &NotationPlan) -> HashMap<u32, Barline> {
    let mut marks: HashMap<u32, Barline> = HashMap::new();
    for repeat in plan.repeats() {
        marks.entry(repeat.from).or_default().forward = true;
        let Some((last, earlier)) = repeat.endings.split_last() else {
            marks.entry(repeat.to).or_default().backward = true;
            continue;
        };
        for volta in earlier {
            marks.entry(volta.from).or_default().ending_start = Some(volta.label());
            let closing = marks.entry(volta.to).or_default();
            closing.ending_stop = Some((volta.label(), "stop"));
            closing.backward = true;
        }
        marks.entry(last.from).or_default().ending_start = Some(last.label());
        marks.entry(last.to).or_default().ending_stop = Some((last.label(), "discontinue"));
    }
    marks
}

/// One barline element, in the order the `MusicXML` DTD wants its children.
fn write_barline(
    xml: &mut Xml,
    location: &str,
    style: Option<&str>,
    ending: Option<(&str, &str)>,
    repeat: Option<&str>,
) -> Result<(), RenderError> {
    if style.is_none() && ending.is_none() && repeat.is_none() {
        return Ok(());
    }
    xml.open("barline", &[("location", location)])?;
    if let Some(style) = style {
        xml.leaf("bar-style", &[], style)?;
    }
    if let Some((number, kind)) = ending {
        xml.leaf("ending", &[("number", number), ("type", kind)], "")?;
    }
    if let Some(direction) = repeat {
        xml.leaf("repeat", &[("direction", direction)], "")?;
    }
    xml.close("barline")
}

fn write_part(
    xml: &mut Xml,
    id: &str,
    staff: &StaffPlan,
    divisions: i64,
    annotations: Option<&NotationPlan>,
    marks: &HashMap<u32, Barline>,
) -> Result<(), RenderError> {
    xml.open("part", &[("id", id)])?;
    // Slur numbers are per voice and nest like brackets, so the open ones are
    // a stack: the number a slur takes is the lowest one free when it starts.
    let mut open_slurs: HashMap<usize, Vec<u32>> = HashMap::new();
    for (index, measure) in staff.measures().iter().enumerate() {
        let number = measure.number().to_string();
        xml.open("measure", &[("number", &number)])?;
        if index == 0 {
            write_attributes(xml, staff, divisions)?;
        } else if measure.time_signature().is_some() || measure.key_signature().is_some() {
            // A measure that changes meter or key carries the change and
            // nothing else: divisions and the opening clef are still what the
            // first measure's `<attributes>` said. `MusicXML` fixes the child
            // order — key before time — so the two are written together.
            xml.open("attributes", &[])?;
            if let Some(key) = measure.key_signature() {
                write_key(xml, key)?;
            }
            if let Some((count, unit)) = measure.time_signature() {
                xml.open("time", &[])?;
                xml.leaf("beats", &[], &count.to_string())?;
                xml.leaf("beat-type", &[], &unit.to_string())?;
                xml.close("time")?;
            }
            xml.close("attributes")?;
        }
        let barline = marks.get(&measure.number());
        if let Some(barline) = barline {
            write_barline(
                xml,
                "left",
                barline.forward.then_some("heavy-light"),
                barline.ending_start.as_deref().map(|label| (label, "start")),
                barline.forward.then_some("forward"),
            )?;
        }
        // This staff's own tempo, over and above the piece's: polytempo, and
        // empty for every part that plays at the piece's speed.
        for tempo in staff.tempos().iter().filter(|mark| mark.measure == measure.number()) {
            write_tempo(xml, tempo, divisions)?;
        }
        if let Some(plan) = annotations {
            write_positioned(xml, plan, measure.number(), divisions)?;
        }
        let lanes = measure.lanes();
        let full = ticks(measure.duration().as_ratio(), divisions);
        for (lane_index, lane) in lanes.iter().enumerate() {
            let consumed = write_lane(
                xml,
                lane,
                lane_index.saturating_add(1),
                divisions,
                full,
                // `MusicXML` writes a mid-measure clef as `<attributes>`
                // among the notes, which puts it in one voice's stream; the
                // clef is the staff's, so it goes in the first.
                if lane_index == 0 { measure.clefs() } else { &[] },
                open_slurs.entry(lane_index).or_default(),
            )?;
            // Every voice starts at the barline, so all but the last rewinds.
            if lane_index.saturating_add(1) < lanes.len() && consumed > 0 {
                xml.open("backup", &[])?;
                xml.leaf("duration", &[], &consumed.to_string())?;
                xml.close("backup")?;
            }
        }
        if let Some(barline) = barline {
            write_barline(
                xml,
                "right",
                barline.backward.then_some("light-heavy"),
                barline
                    .ending_stop
                    .as_ref()
                    .map(|(label, kind)| (label.as_str(), *kind)),
                barline.backward.then_some("backward"),
            )?;
        } else if !measure.meter().is_measured() {
            // Unmeasured music: the measure is real and numbered, and the
            // line that would close it is not drawn.
            write_barline(xml, "right", Some("none"), None, None)?;
        }
        xml.close("measure")?;
    }
    xml.close("part")
}

fn write_key(xml: &mut Xml, key: crate::plan::KeySignature) -> Result<(), RenderError> {
    xml.open("key", &[])?;
    xml.leaf("fifths", &[], &key.fifths.to_string())?;
    xml.leaf(
        "mode",
        &[],
        match key.mode {
            Mode::Major => "major",
            Mode::Minor => "minor",
        },
    )?;
    xml.close("key")
}

/// `<clef>` as `MusicXML` writes one, in the head or among the notes.
fn write_clef(xml: &mut Xml, clef: Clef) -> Result<(), RenderError> {
    let (sign, line) = clef_sign_line(clef);
    xml.open("clef", &[])?;
    xml.leaf("sign", &[], sign)?;
    xml.leaf("line", &[], line)?;
    xml.close("clef")
}

fn write_attributes(xml: &mut Xml, staff: &StaffPlan, divisions: i64) -> Result<(), RenderError> {
    xml.open("attributes", &[])?;
    xml.leaf("divisions", &[], &divisions.to_string())?;
    if let Some(key) = staff.key_signature() {
        write_key(xml, key)?;
    }
    // A piece that opens unmeasured prints no time signature, because there
    // is none: `<time>` with no beats in it is a document no reader accepts.
    let (count, unit) = staff.time_signature();
    if count > 0 && unit > 0 {
        xml.open("time", &[])?;
        xml.leaf("beats", &[], &count.to_string())?;
        xml.leaf("beat-type", &[], &unit.to_string())?;
        xml.close("time")?;
    }
    if let Some(clef) = staff.clef() {
        write_clef(xml, clef)?;
    }
    xml.close("attributes")
}

/// Write one voice's measure and report how many divisions it consumed.
fn write_lane(
    xml: &mut Xml,
    lane: &VoiceLane,
    voice: usize,
    divisions: i64,
    full_measure: i64,
    clefs: &[crate::plan::ClefChange],
    open_slurs: &mut Vec<u32>,
) -> Result<i64, RenderError> {
    let items = lane.items();
    if items.is_empty() {
        // A voice that does not sound in this measure gets the whole-measure
        // rest `MusicXML` has for exactly that (MEI's `<mSpace>`).
        xml.open("note", &[])?;
        xml.empty("rest", &[("measure", "yes")])?;
        xml.leaf("duration", &[], &full_measure.to_string())?;
        xml.leaf("voice", &[], &voice.to_string())?;
        xml.close("note")?;
        return Ok(full_measure);
    }
    // Point marks are written at the head of the measure with an `<offset>`,
    // like chord symbols and form markers: `MusicXML`'s offset says where a
    // direction sounds independently of where it sits in the document, and a
    // measure whose voices backup and forward has no single place meaning
    // "here".
    for point in lane.points() {
        write_point(xml, point, divisions)?;
    }
    let mut cursor = Ratio::ZERO;
    let mut pending_clefs = clefs.iter();
    let mut next_clef = pending_clefs.next();
    for (index, item) in items.iter().enumerate() {
        let onset = item.onset_in_measure().as_ratio();
        while let Some(change) = next_clef.filter(|change| change.onset_in_measure.as_ratio() <= onset) {
            xml.open("attributes", &[])?;
            write_clef(xml, change.clef)?;
            xml.close("attributes")?;
            next_clef = pending_clefs.next();
        }
        // The plan can leave a voice silent between events without writing a
        // rest for it; `<forward>` is how a partwise document skips time.
        if onset > cursor {
            xml.open("forward", &[])?;
            xml.leaf("duration", &[], &ticks(onset - cursor, divisions).to_string())?;
            xml.close("forward")?;
        } else if onset < cursor {
            xml.open("backup", &[])?;
            xml.leaf("duration", &[], &ticks(cursor - onset, divisions).to_string())?;
            xml.close("backup")?;
        }
        cursor = onset;
        if let Some(phrase) = item.phrase().filter(|phrase| phrase.start) {
            write_phrase(xml, &phrase.name, true)?;
        }
        if let Some(dynamic) = item.dynamic() {
            write_dynamic(xml, dynamic)?;
        }
        if let Some(hairpin) = item.hairpin().filter(|hairpin| hairpin.start) {
            write_wedge(xml, if hairpin.grows { "crescendo" } else { "diminuendo" })?;
        }
        for span in item.spans().iter().filter(|span| span.start) {
            write_span(xml, span, true)?;
        }
        write_item(xml, item, &beams(items, index), voice, divisions, open_slurs)?;
        if let Some(hairpin) = item.hairpin().filter(|hairpin| hairpin.stop) {
            write_wedge(xml, "stop")?;
            // The wedge stops; the mark it stopped at is what the reader
            // plays, so it is printed too.
            write_dynamic(xml, hairpin.target)?;
        }
        for span in item.spans().iter().filter(|span| span.stop) {
            write_span(xml, span, false)?;
        }
        if let Some(phrase) = item.phrase().filter(|phrase| phrase.stop) {
            write_phrase(xml, &phrase.name, false)?;
        }
        cursor += sounding(item);
    }
    while let Some(change) = next_clef {
        xml.open("attributes", &[])?;
        write_clef(xml, change.clef)?;
        xml.close("attributes")?;
        next_clef = pending_clefs.next();
    }
    Ok(ticks(cursor, divisions))
}

/// One tempo marking as a `<direction>`.
///
/// A direction lives inside a part, which is what makes polytempo expressible
/// here at all: the piece's markings are written into the first part, and a
/// part at its own speed writes its own into itself.
fn write_tempo(
    xml: &mut Xml,
    tempo: &crate::plan::PositionedMark<crate::plan::TempoText>,
    divisions: i64,
) -> Result<(), RenderError> {
    xml.open("direction", &[("placement", "above")])?;
    // A tempo word is `<words>`, a metronome mark is `<metronome>`, and a
    // marking carrying both writes both direction-types inside the one
    // direction — which is what tells a reader they belong together
    // rather than being two instructions at the same instant.
    if let Some(text) = tempo.what.text.as_ref() {
        xml.open("direction-type", &[])?;
        xml.leaf("words", &[], text)?;
        xml.close("direction-type")?;
    }
    if let Some(mark) = tempo.what.metronome {
        xml.open("direction-type", &[])?;
        xml.open("metronome", &[])?;
        xml.leaf("beat-unit", &[], beat_unit_name(*mark.beat.denom()))?;
        xml.leaf("per-minute", &[], &mark.bpm.to_string())?;
        xml.close("metronome")?;
        xml.close("direction-type")?;
    }
    write_offset(xml, tempo.onset_in_measure.as_ratio(), divisions)?;
    // `<sound>` is the same fact for a player rather than a reader: the
    // tempo in quarter notes per minute, which is what playback uses. A
    // marking with no metronome moves no clock, so it gets none — a word
    // is not a speed, and guessing one here would be the collapse §2
    // forbids.
    if let Some(mark) = tempo.what.metronome {
        let quarters = Ratio::from_integer(i64::from(mark.bpm)) * mark.beat * Ratio::from_integer(4);
        xml.empty("sound", &[("tempo", &format_number(quarters))])?;
    }
    xml.close("direction")
}

/// The chord symbols and form markers falling in one measure.
///
/// Both are written at the head of the measure with an `<offset>` rather than
/// interleaved with the notes: `MusicXML`'s offset says where a symbol sounds
/// independently of where it sits in the document, and a measure whose voices
/// backup and forward has no single place that means "here".
fn write_positioned(xml: &mut Xml, plan: &NotationPlan, measure: u32, divisions: i64) -> Result<(), RenderError> {
    for tempo in plan.tempos().iter().filter(|mark| mark.measure == measure) {
        write_tempo(xml, tempo, divisions)?;
    }
    for section in plan.sections().iter().filter(|mark| mark.measure == measure) {
        xml.open("direction", &[("placement", "above")])?;
        xml.open("direction-type", &[])?;
        xml.leaf("rehearsal", &[], &section.what)?;
        xml.close("direction-type")?;
        write_offset(xml, section.onset_in_measure.as_ratio(), divisions)?;
        xml.close("direction")?;
    }
    for chord in plan.harmony().iter().filter(|mark| mark.measure == measure) {
        write_harmony(xml, &chord.what, chord.onset_in_measure.as_ratio(), divisions)?;
    }
    // An open region has no `MusicXML` element, so it is a word direction at
    // each end: the instruction where it opens, and where it closes so a
    // reader knows how far it reaches. Lossy, and said to be lossy in
    // `docs/rules/kernel/07-backend-contract.md`.
    for hold in plan.holds().iter().filter(|mark| mark.measure == measure) {
        let text = format!("hold to {}", hold.what.most.as_ratio());
        write_words(xml, &text, hold.onset_in_measure.as_ratio(), divisions)?;
    }
    for region in plan.open() {
        if region.from == measure {
            write_words(xml, &region.text, Ratio::ZERO, divisions)?;
        }
        if region.to == measure && region.to != region.from {
            write_words(xml, "end", Ratio::ZERO, divisions)?;
        }
    }
    Ok(())
}

/// A text direction above the staff: the one thing every backend can print.
fn write_words(xml: &mut Xml, text: &str, onset: Ratio<i64>, divisions: i64) -> Result<(), RenderError> {
    xml.open("direction", &[("placement", "above")])?;
    xml.open("direction-type", &[])?;
    xml.leaf("words", &[], text)?;
    xml.close("direction-type")?;
    write_offset(xml, onset, divisions)?;
    xml.close("direction")
}

/// `MusicXML` names its note values rather than numbering them.
fn beat_unit_name(denominator: i64) -> &'static str {
    match denominator {
        1 => "whole",
        2 => "half",
        8 => "eighth",
        16 => "16th",
        32 => "32nd",
        64 => "64th",
        _ => "quarter",
    }
}

/// A rational as a decimal `MusicXML` will accept, without a trailing `.0`
/// where the value is whole.
fn format_number(value: Ratio<i64>) -> String {
    if *value.denom() == 1 {
        return value.numer().to_string();
    }
    format!("{:.4}", *value.numer() as f64 / *value.denom() as f64)
}

/// `<offset>`, omitted at the start of a measure where it would say nothing.
fn write_offset(xml: &mut Xml, onset: Ratio<i64>, divisions: i64) -> Result<(), RenderError> {
    if onset == Ratio::ZERO {
        return Ok(());
    }
    xml.leaf("offset", &[], &ticks(onset, divisions).to_string())
}

/// One chord symbol as `<harmony>`.
///
/// `<kind>` carries musa's parsed reading of the symbol and its `text`
/// attribute carries what the composer wrote, so a consumer that understands
/// the kind gets the structure and one that does not still prints `fmaj7`.
fn write_harmony(xml: &mut Xml, chord: &ChordSymbol, onset: Ratio<i64>, divisions: i64) -> Result<(), RenderError> {
    xml.open("harmony", &[])?;
    xml.open("root", &[])?;
    xml.leaf("root-step", &[], chord_step(chord))?;
    xml.leaf("root-alter", &[], &chord.root().accidental.0.to_string())?;
    xml.close("root")?;
    xml.leaf("kind", &[("text", chord.text())], chord_kind(chord))?;
    write_offset(xml, onset, divisions)?;
    xml.close("harmony")
}

fn chord_step(chord: &ChordSymbol) -> &'static str {
    match chord.root().letter {
        musa_compiler::Letter::C => "C",
        musa_compiler::Letter::D => "D",
        musa_compiler::Letter::E => "E",
        musa_compiler::Letter::F => "F",
        musa_compiler::Letter::G => "G",
        musa_compiler::Letter::A => "A",
        musa_compiler::Letter::B => "B",
    }
}

/// `MusicXML`'s `kind` vocabulary for a parsed symbol.
///
/// The vocabulary is coarser than the symbols musa reads — it has no name for
/// a suspended chord with a seventh, and none for an augmented major seventh —
/// so those fall back to the nearest kind and rely on the `text` attribute for
/// the exact symbol. Nothing is invented: an unrepresentable shade is written
/// as the triad it is built on, never as a different chord.
fn chord_kind(chord: &ChordSymbol) -> &'static str {
    let triad = match chord.quality() {
        ChordQuality::Major => "major",
        ChordQuality::Minor => "minor",
        ChordQuality::Diminished => "diminished",
        ChordQuality::Augmented => "augmented",
        ChordQuality::Suspended2 => "suspended-second",
        ChordQuality::Suspended4 => "suspended-fourth",
    };
    let Some(seventh) = chord.seventh() else {
        return match (chord.extension(), chord.quality()) {
            (Some(6), ChordQuality::Major) => "major-sixth",
            (Some(6), ChordQuality::Minor) => "minor-sixth",
            (Some(_) | None, _) => triad,
        };
    };
    match (chord.quality(), seventh, chord.extension()) {
        (ChordQuality::Diminished, _, _) => "diminished-seventh",
        (ChordQuality::Major, Seventh::Major, None) => "major-seventh",
        (ChordQuality::Major, Seventh::Major, Some(9)) => "major-ninth",
        (ChordQuality::Major, Seventh::Major, Some(11)) => "major-11th",
        (ChordQuality::Major, Seventh::Major, Some(13)) => "major-13th",
        (ChordQuality::Major, Seventh::Minor | Seventh::Diminished, None) => "dominant",
        (ChordQuality::Major, Seventh::Minor | Seventh::Diminished, Some(9)) => "dominant-ninth",
        (ChordQuality::Major, Seventh::Minor | Seventh::Diminished, Some(11)) => "dominant-11th",
        (ChordQuality::Major, Seventh::Minor | Seventh::Diminished, Some(13)) => "dominant-13th",
        (ChordQuality::Minor, Seventh::Major, _) => "major-minor",
        (ChordQuality::Minor, Seventh::Minor | Seventh::Diminished, None) => "minor-seventh",
        (ChordQuality::Minor, Seventh::Minor | Seventh::Diminished, Some(9)) => "minor-ninth",
        (ChordQuality::Minor, Seventh::Minor | Seventh::Diminished, Some(11)) => "minor-11th",
        (ChordQuality::Minor, Seventh::Minor | Seventh::Diminished, Some(13)) => "minor-13th",
        (ChordQuality::Augmented, _, _) => "augmented-seventh",
        (ChordQuality::Major | ChordQuality::Minor, _, Some(_)) => triad,
        (ChordQuality::Suspended2 | ChordQuality::Suspended4, _, _) => triad,
    }
}

/// A phrase, as the bracket `MusicXML` spans a run of notes with, plus the
/// words that name it.
fn write_phrase(xml: &mut Xml, name: &str, start: bool) -> Result<(), RenderError> {
    xml.open("direction", &[("placement", "above")])?;
    xml.open("direction-type", &[])?;
    if start {
        xml.leaf("words", &[], name)?;
    }
    xml.close("direction-type")?;
    xml.open("direction-type", &[])?;
    xml.empty(
        "bracket",
        &[
            ("type", if start { "start" } else { "stop" }),
            ("number", "1"),
            ("line-end", "down"),
        ],
    )?;
    xml.close("direction-type")?;
    xml.close("direction")
}

/// One end of a span mark, as a `<direction>` at the note it happens on.
///
/// `MusicXML` gives `<pedal>` and `<octave-shift>` the same two-ended shape a
/// wedge has, so the row's spelling and a `start`/`stop` are the whole
/// emission. A row `MusicXML` cannot say writes nothing and is reported once
/// per export instead.
fn write_span(xml: &mut Xml, span: &crate::plan::SpanMark, start: bool) -> Result<(), RenderError> {
    let Some(name) = span.mark.def().musicxml else {
        return Ok(());
    };
    let kind = if start { "start" } else { "stop" };
    // An octave shift says how far in `@size`, where 8 is one octave; the
    // sign is already in `<direction>`'s own up/down type words.
    let size = match span.argument {
        Some(musa_compiler::MarkArgument::Number(shift)) if shift.abs() >= 2 => "15",
        _ => "8",
    };
    let kind = if name == "octave-shift" && start {
        match span.argument {
            Some(musa_compiler::MarkArgument::Number(shift)) if shift < 0 => "up",
            _ => "down",
        }
    } else {
        kind
    };
    xml.open("direction", &[("placement", "above")])?;
    xml.open("direction-type", &[])?;
    if name == "octave-shift" {
        xml.empty(name, &[("type", kind), ("size", size)])?;
    } else {
        xml.empty(name, &[("type", kind), ("line", "yes")])?;
    }
    xml.close("direction-type")?;
    xml.close("direction")
}

/// A mark standing at one place, as a `<direction>` with an `<offset>`.
fn write_point(xml: &mut Xml, point: &crate::plan::PointMark, divisions: i64) -> Result<(), RenderError> {
    let Some(name) = point.mark.def().musicxml else {
        return Ok(());
    };
    xml.open("direction", &[("placement", "above")])?;
    xml.open("direction-type", &[])?;
    match &point.argument {
        Some(argument) => xml.leaf(name, &[], &argument.to_string())?,
        None => xml.empty(name, &[])?,
    }
    xml.close("direction-type")?;
    write_offset(xml, point.onset_in_measure.as_ratio(), divisions)?;
    xml.close("direction")
}

/// One end of a hairpin: `<wedge>` opens with the shape and closes with
/// `stop`, which is how `MusicXML` spans one.
fn write_wedge(xml: &mut Xml, kind: &str) -> Result<(), RenderError> {
    xml.open("direction", &[("placement", place(DYNAMIC_PLACEMENT))])?;
    xml.open("direction-type", &[])?;
    xml.empty("wedge", &[("type", kind), ("number", "1")])?;
    xml.close("direction-type")?;
    xml.close("direction")
}

/// A dynamic marking, as the `<direction>` that precedes the note it marks.
fn write_dynamic(xml: &mut Xml, mark: DynamicMark) -> Result<(), RenderError> {
    xml.open("direction", &[("placement", place(DYNAMIC_PLACEMENT))])?;
    xml.open("direction-type", &[])?;
    xml.open("dynamics", &[])?;
    // Every `DynamicMark` name is a `MusicXML` dynamics element; the test
    // `dynamic_names_are_musicxml_elements` is what keeps that true.
    xml.empty(mark.name(), &[])?;
    xml.close("dynamics")?;
    xml.close("direction-type")?;
    xml.close("direction")?;
    Ok(())
}

/// What one `<note>` element needs to know beyond the plan item: which pitch
/// of a chord it is, and whether it is the one carrying the chord's marks.
struct NoteSpelling<'a> {
    item: &'a NotatedItem,
    /// `None` for a rest.
    pitch: Option<WrittenPitch>,
    /// A second-or-later pitch of a chord (`<chord/>`).
    stacked: bool,
    /// Carries the beams, slurs, tuplet, and articulations. False for the
    /// stacked pitches, which would otherwise print the marks once per tone.
    leads: bool,
    voice: usize,
    duration: i64,
    beams: &'a [(u32, &'static str)],
}

/// One plan item: a note, a rest, or a chord's worth of stacked notes.
fn write_item(
    xml: &mut Xml,
    item: &NotatedItem,
    beams: &[(u32, &'static str)],
    voice: usize,
    divisions: i64,
    open_slurs: &mut Vec<u32>,
) -> Result<(), RenderError> {
    let duration = ticks(sounding(item), divisions);
    write_graces(xml, item, voice)?;
    let mut spelling = NoteSpelling {
        item,
        pitch: None,
        stacked: false,
        leads: true,
        voice,
        duration,
        beams,
    };
    match item.kind() {
        NotatedKind::Rest => write_note(xml, &spelling, open_slurs),
        NotatedKind::Note { pitch } => {
            spelling.pitch = Some(*pitch);
            write_note(xml, &spelling, open_slurs)
        }
        NotatedKind::Chord { pitches } => {
            for (index, pitch) in pitches.iter().enumerate() {
                spelling.pitch = Some(*pitch);
                spelling.stacked = index > 0;
                spelling.leads = index == 0;
                write_note(xml, &spelling, open_slurs)?;
            }
            Ok(())
        }
    }
}

/// The grace notes leaning on an item, as `<note>` elements ahead of it.
///
/// A grace `<note>` carries **no `<duration>`** — the spec forbids it, and
/// musa has none to write: a grace is a point occurrence.
///
/// Two attributes are deliberately absent. `steal-time-previous` and
/// `steal-time-following` are `MusicXML` asking the *editor* to settle how the
/// grace is played, and that is the question roadmap §2 keeps out of the
/// notation: musa answers it per part, in the profile, from the same page. A
/// file without them says what was written and leaves the reading open.
///
/// `slash="yes"` is the one choice that has to be made, because `MusicXML` has
/// no way to decline it. It is the right one: the *unslashed* grace is the
/// form whose length is notated as a proportion of the principal, and musa's
/// grace has no written length at all. The slashed grace is the ornamental
/// one, which is what this is.
fn write_graces(xml: &mut Xml, item: &NotatedItem, voice: usize) -> Result<(), RenderError> {
    for grace in item.graces() {
        xml.open("note", &[])?;
        xml.empty("grace", &[("slash", "yes")])?;
        xml.open("pitch", &[])?;
        xml.leaf("step", &[], step_of(grace.pitch))?;
        if grace.pitch.accidental.0 != 0 {
            xml.leaf("alter", &[], &grace.pitch.accidental.0.to_string())?;
        }
        xml.leaf("octave", &[], &grace.pitch.octave.to_string())?;
        xml.close("pitch")?;
        xml.leaf("voice", &[], &voice.to_string())?;
        // An eighth is a drawing instruction, not a length: `<type>` is
        // required and a grace's stem is conventionally flagged.
        xml.leaf("type", &[], "eighth")?;
        let named: Vec<&str> = grace
            .articulations
            .iter()
            .filter(|mark| mark.slot() == Some(Slot::Articulation))
            .filter_map(|mark| mark.def().musicxml)
            .collect();
        if !named.is_empty() {
            xml.open("notations", &[])?;
            xml.open("articulations", &[])?;
            for name in named {
                xml.empty(name, &[("placement", place(ARTICULATION_PLACEMENT))])?;
            }
            xml.close("articulations")?;
            xml.close("notations")?;
        }
        xml.close("note")?;
    }
    Ok(())
}

fn write_note(xml: &mut Xml, note: &NoteSpelling<'_>, open_slurs: &mut Vec<u32>) -> Result<(), RenderError> {
    let item = note.item;
    let symbol = item.duration().value.as_ratio();
    let Some((kind, dots)) = note_type(symbol) else {
        return Err(RenderError::Unsupported {
            event: item.event(),
            what: format!("cannot spell duration {symbol} as a MusicXML note type"),
        });
    };
    xml.open("note", &[])?;
    if note.stacked {
        xml.empty("chord", &[])?;
    }
    match note.pitch {
        None => xml.empty("rest", &[])?,
        Some(pitch) => {
            xml.open("pitch", &[])?;
            xml.leaf("step", &[], step_of(pitch))?;
            if pitch.accidental.0 != 0 {
                xml.leaf("alter", &[], &pitch.accidental.0.to_string())?;
            }
            xml.leaf("octave", &[], &pitch.octave.to_string())?;
            xml.close("pitch")?;
        }
    }
    xml.leaf("duration", &[], &note.duration.to_string())?;
    // The `<tie>` elements are the sound of the tie; the `<tied>` elements in
    // `<notations>` are its printed slur. The spec wants both, and a consumer
    // that reads only one of them still gets a whole note (§12.4).
    if item.tie_stop() {
        xml.empty("tie", &[("type", "stop")])?;
    }
    if item.tie_start() {
        xml.empty("tie", &[("type", "start")])?;
    }
    xml.leaf("voice", &[], &note.voice.to_string())?;
    xml.leaf("type", &[], kind)?;
    for _ in 0..dots {
        xml.empty("dot", &[])?;
    }
    if let Some(tuplet) = item.tuplet() {
        xml.open("time-modification", &[])?;
        xml.leaf("actual-notes", &[], &tuplet.num.to_string())?;
        xml.leaf("normal-notes", &[], &tuplet.den.to_string())?;
        xml.close("time-modification")?;
    }
    if note.leads {
        for (level, kind) in note.beams {
            xml.leaf("beam", &[("number", &level.to_string())], kind)?;
        }
    }
    write_notations(xml, note, open_slurs)?;
    xml.close("note")
}

/// `<notations>`: the printed marks, written only when there are some.
fn write_notations(xml: &mut Xml, note: &NoteSpelling<'_>, open_slurs: &mut Vec<u32>) -> Result<(), RenderError> {
    let item = note.item;
    let ties = item.tie_start() || item.tie_stop();
    // A tuplet only marks its two ends: the notes between them carry the
    // `<time-modification>` that says how long they last and nothing printed.
    let bracket = item.tuplet().is_some_and(|tuplet| tuplet.start || tuplet.stop);
    let marks = note.leads && (item.slur_start() || item.slur_stop() || bracket || !item.articulations().is_empty());
    if !ties && !marks {
        return Ok(());
    }
    xml.open("notations", &[])?;
    if item.tie_stop() {
        xml.empty("tied", &[("type", "stop")])?;
    }
    if item.tie_start() {
        xml.empty("tied", &[("type", "start")])?;
    }
    if note.leads {
        // Closing first frees the number the next slur may take, which is
        // what makes a chain of slurs read as `1`, `1`, `1` rather than
        // climbing until it runs out of numbers.
        if item.slur_stop() {
            let number = open_slurs.pop().unwrap_or(1);
            xml.empty("slur", &[("type", "stop"), ("number", &number.to_string())])?;
        }
        if item.slur_start() {
            // `MusicXML` numbers concurrent slurs 1–6; a seventh open at once
            // is not a score anyone wrote, and reusing 1 keeps the document
            // valid rather than emitting a number no consumer accepts.
            let number = (1..=SLUR_NUMBERS)
                .find(|number| !open_slurs.contains(number))
                .unwrap_or(1);
            open_slurs.push(number);
            xml.empty(
                "slur",
                &[
                    ("type", "start"),
                    ("number", &number.to_string()),
                    ("placement", place(SLUR_PLACEMENT)),
                ],
            )?;
        }
        if let Some(tuplet) = item.tuplet() {
            if tuplet.start {
                xml.empty("tuplet", &[("type", "start"), ("bracket", "yes")])?;
            }
            if tuplet.stop {
                xml.empty("tuplet", &[("type", "stop")])?;
            }
        }
        // `<notations>` sorts its children into slots, and the slot is the
        // row's rather than this function's: a trill in `<articulations>` is
        // not valid `MusicXML`, and a fermata belongs to neither wrapper.
        for (slot, wrapper) in [
            (Slot::Articulation, Some("articulations")),
            (Slot::Ornament, Some("ornaments")),
            (Slot::Technical, Some("technical")),
            (Slot::Fermata, None),
        ] {
            let named: Vec<&str> = item
                .articulations()
                .iter()
                .filter(|mark| mark.slot() == Some(slot))
                .filter_map(|mark| mark.def().musicxml)
                .collect();
            if named.is_empty() {
                continue;
            }
            if let Some(wrapper) = wrapper {
                xml.open(wrapper, &[])?;
            }
            for name in named {
                xml.empty(name, &[("placement", place(ARTICULATION_PLACEMENT))])?;
            }
            if let Some(wrapper) = wrapper {
                xml.close(wrapper)?;
            }
        }
    }
    xml.close("notations")
}

// --- Writer ------------------------------------------------------------------

/// A thin, indenting XML writer. Element construction stays in one place so
/// the backend reads as the document it produces.
struct Xml {
    writer: Writer<Vec<u8>>,
}

impl Xml {
    fn new() -> Self {
        Self {
            writer: Writer::new_with_indent(Vec::new(), b' ', 2),
        }
    }

    fn write(&mut self, event: Event<'_>) -> Result<(), RenderError> {
        self.writer.write_event(event).map_err(|error| RenderError::xml(&error))
    }

    fn declaration(&mut self) -> Result<(), RenderError> {
        self.write(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("no"))))
    }

    /// The partwise doctype. Consumers that validate (Finale in particular)
    /// want it, and it names the format for anyone reading the file.
    fn doctype(&mut self) -> Result<(), RenderError> {
        self.write(Event::DocType(BytesText::from_escaped(
            "score-partwise PUBLIC \"-//Recordare//DTD MusicXML 4.0 Partwise//EN\" \
             \"http://www.musicxml.org/dtds/partwise.dtd\"",
        )))
    }

    fn tag<'a>(name: &str, attributes: &[(&str, &str)]) -> BytesStart<'a> {
        let mut element = BytesStart::new(name.to_string());
        for (key, value) in attributes {
            element.push_attribute((*key, *value));
        }
        element
    }

    fn open(&mut self, name: &str, attributes: &[(&str, &str)]) -> Result<(), RenderError> {
        self.write(Event::Start(Self::tag(name, attributes)))
    }

    fn close(&mut self, name: &str) -> Result<(), RenderError> {
        self.write(Event::End(BytesEnd::new(name.to_string())))
    }

    fn empty(&mut self, name: &str, attributes: &[(&str, &str)]) -> Result<(), RenderError> {
        self.write(Event::Empty(Self::tag(name, attributes)))
    }

    fn leaf(&mut self, name: &str, attributes: &[(&str, &str)], text: &str) -> Result<(), RenderError> {
        self.open(name, attributes)?;
        self.write(Event::Text(BytesText::new(text)))?;
        self.close(name)
    }

    fn finish(self) -> Result<String, RenderError> {
        String::from_utf8(self.writer.into_inner()).map_err(|error| RenderError::Xml(error.to_string()))
    }
}
