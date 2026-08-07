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

use musa_compiler::{ArticulationMark, Clef, DynamicMark, Mode, WrittenPitch};
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
    xml.open("identification", &[])?;
    xml.open("encoding", &[])?;
    // No version and no encoding date: an export of one source is the same
    // bytes on every machine and every build (§17.5's determinism rule).
    xml.leaf("software", &[], "musa")?;
    xml.close("encoding")?;
    xml.close("identification")?;

    xml.open("part-list", &[])?;
    for (index, staff) in plan.staves().iter().enumerate() {
        xml.open("score-part", &[("id", &part_id(index))])?;
        xml.leaf("part-name", &[], staff.name())?;
        xml.close("score-part")?;
    }
    xml.close("part-list")?;

    for (index, staff) in plan.staves().iter().enumerate() {
        write_part(&mut xml, &part_id(index), staff, divisions)?;
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
        divisions = widen(divisions, measure_length(staff))?;
        for measure in staff.measures() {
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
fn measure_length(staff: &StaffPlan) -> Ratio<i64> {
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

/// `MusicXML`'s articulation element for one mark. Every one of these is a
/// standard child of `<articulations>`.
fn articulation_element(mark: ArticulationMark) -> &'static str {
    match mark {
        ArticulationMark::Staccato => "staccato",
        ArticulationMark::Staccatissimo => "staccatissimo",
        ArticulationMark::Tenuto => "tenuto",
        ArticulationMark::Accent => "accent",
        ArticulationMark::Marcato => "strong-accent",
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

fn write_part(xml: &mut Xml, id: &str, staff: &StaffPlan, divisions: i64) -> Result<(), RenderError> {
    xml.open("part", &[("id", id)])?;
    // Slur numbers are per voice and nest like brackets, so the open ones are
    // a stack: the number a slur takes is the lowest one free when it starts.
    let mut open_slurs: HashMap<usize, Vec<u32>> = HashMap::new();
    for (index, measure) in staff.measures().iter().enumerate() {
        let number = measure.number().to_string();
        xml.open("measure", &[("number", &number)])?;
        if index == 0 {
            write_attributes(xml, staff, divisions)?;
        }
        let lanes = measure.lanes();
        let full = ticks(measure_length(staff), divisions);
        for (lane_index, lane) in lanes.iter().enumerate() {
            let consumed = write_lane(
                xml,
                lane,
                lane_index.saturating_add(1),
                divisions,
                full,
                open_slurs.entry(lane_index).or_default(),
            )?;
            // Every voice starts at the barline, so all but the last rewinds.
            if lane_index.saturating_add(1) < lanes.len() && consumed > 0 {
                xml.open("backup", &[])?;
                xml.leaf("duration", &[], &consumed.to_string())?;
                xml.close("backup")?;
            }
        }
        xml.close("measure")?;
    }
    xml.close("part")
}

fn write_attributes(xml: &mut Xml, staff: &StaffPlan, divisions: i64) -> Result<(), RenderError> {
    xml.open("attributes", &[])?;
    xml.leaf("divisions", &[], &divisions.to_string())?;
    if let Some(key) = staff.key_signature() {
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
        xml.close("key")?;
    }
    let (count, unit) = staff.time_signature();
    xml.open("time", &[])?;
    xml.leaf("beats", &[], &count.to_string())?;
    xml.leaf("beat-type", &[], &unit.to_string())?;
    xml.close("time")?;
    if let Some(clef) = staff.clef() {
        let (sign, line) = clef_sign_line(clef);
        xml.open("clef", &[])?;
        xml.leaf("sign", &[], sign)?;
        xml.leaf("line", &[], line)?;
        xml.close("clef")?;
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
    let mut cursor = Ratio::ZERO;
    for (index, item) in items.iter().enumerate() {
        let onset = item.onset_in_measure().as_ratio();
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
        if let Some(dynamic) = item.dynamic() {
            write_dynamic(xml, dynamic)?;
        }
        write_item(xml, item, &beams(items, index), voice, divisions, open_slurs)?;
        cursor += sounding(item);
    }
    Ok(ticks(cursor, divisions))
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
        if !item.articulations().is_empty() {
            xml.open("articulations", &[])?;
            for mark in item.articulations() {
                xml.empty(
                    articulation_element(*mark),
                    &[("placement", place(ARTICULATION_PLACEMENT))],
                )?;
            }
            xml.close("articulations")?;
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
