//! The MEI backend (roadmap §12.2): deterministic MEI 5 XML from a
//! `NotationPlan`, built through quick-xml's writer — never string assembly.
//!
//! **`xml:id` contract (load-bearing for the GUI, prompts 20/21):** every
//! note/chord/rest element carries `xml:id="event-<hex>"` where `<hex>` is the
//! lowercase hex of the score `EventId`. Tied pieces of one event share the
//! base id: the first piece is `event-<hex>`, later pieces are
//! `event-<hex>-t2`, `event-<hex>-t3`, … — stripping a `-tN` suffix yields
//! the `EventId`. Layers are `layer-<staff>-<lane>` (1-based) and are not
//! event-mapped. Nothing else uses the `event-` prefix.
//!
//! Ties render as `tie="i|m|t"` attributes on the pieces (Verovio-compatible).
//! Beams render as `<beam>` groups exactly as the plan assigns them. Key
//! signatures affect only `<scoreDef>`; pitch spelling passes through
//! verbatim (§6.3).

use musa_compiler::{Clef, EventId, Mode, WrittenPitch};
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};

use crate::RenderError;
use crate::plan::{NotatedItem, NotatedKind, NotationPlan, StaffPlan, VoiceLane};

/// An element builder with an independent lifetime so locally-computed
/// attribute values can be borrowed for the duration of one write.
fn element<'a>(name: &str) -> BytesStart<'a> {
    BytesStart::new(name.to_string())
}

/// Render the plan to deterministic MEI text.
///
/// # Errors
/// [`RenderError::Xml`] on XML writer failures (not expected in practice:
/// the plan is already validated).
pub(crate) fn render_mei(plan: &NotationPlan) -> Result<String, RenderError> {
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|error| RenderError::xml(&error))?;

    let mut mei = element("mei");
    mei.push_attribute(("xmlns", "http://www.music-encoding.org/ns/mei"));
    mei.push_attribute(("meiversion", "5.0"));
    writer
        .write_event(Event::Start(mei))
        .map_err(|error| RenderError::xml(&error))?;
    for wrapper in ["music", "body", "mdiv", "score"] {
        start(&mut writer, wrapper)?;
    }
    write_score_def(&mut writer, plan)?;
    start(&mut writer, "section")?;
    // Tie-piece counters are document-scoped: an event's pieces can span
    // measures, and the `-tN` suffixes must count across the whole chain.
    let mut piece_counts: std::collections::HashMap<EventId, u32> = std::collections::HashMap::new();
    let measure_count = plan.staves().first().map_or(0, |staff| staff.measures().len());
    for index in 0..measure_count {
        write_measure(&mut writer, plan, index, &mut piece_counts)?;
    }
    end(&mut writer, "section")?;
    for wrapper in ["score", "mdiv", "body", "music"] {
        end(&mut writer, wrapper)?;
    }
    end(&mut writer, "mei")?;

    String::from_utf8(writer.into_inner()).map_err(|error| RenderError::Xml(error.to_string()))
}

fn start(writer: &mut Writer<Vec<u8>>, name: &str) -> Result<(), RenderError> {
    writer
        .write_event(Event::Start(element(name)))
        .map_err(|error| RenderError::xml(&error))
}

fn end(writer: &mut Writer<Vec<u8>>, name: &str) -> Result<(), RenderError> {
    writer
        .write_event(Event::End(BytesEnd::new(name.to_string())))
        .map_err(|error| RenderError::xml(&error))
}

/// `<scoreDef>` with meter, key, and one `<staffDef>` per staff.
fn write_score_def(writer: &mut Writer<Vec<u8>>, plan: &NotationPlan) -> Result<(), RenderError> {
    let Some(first) = plan.staves().first() else {
        return Ok(());
    };
    let (count, unit) = first.time_signature();
    let count_text = count.to_string();
    let unit_text = unit.to_string();
    let mut score_def = element("scoreDef");
    score_def.push_attribute(("meter.count", count_text.as_str()));
    score_def.push_attribute(("meter.unit", unit_text.as_str()));
    if let Some(key) = first.key_signature() {
        let sig = match key.fifths.cmp(&0) {
            std::cmp::Ordering::Equal => "0".to_string(),
            std::cmp::Ordering::Greater => format!("{}s", key.fifths),
            std::cmp::Ordering::Less => format!("{}f", key.fifths.saturating_abs()),
        };
        let mode = match key.mode {
            Mode::Major => "major",
            Mode::Minor => "minor",
        };
        score_def.push_attribute(("key.sig", sig.as_str()));
        score_def.push_attribute(("key.mode", mode));
    }
    writer
        .write_event(Event::Start(score_def))
        .map_err(|error| RenderError::xml(&error))?;

    start(writer, "staffGrp")?;
    for (index, staff) in plan.staves().iter().enumerate() {
        write_staff_def(writer, index.saturating_add(1), staff)?;
    }
    end(writer, "staffGrp")?;
    end(writer, "scoreDef")
}

fn write_staff_def(writer: &mut Writer<Vec<u8>>, n: usize, staff: &StaffPlan) -> Result<(), RenderError> {
    let n_text = n.to_string();
    let mut staff_def = element("staffDef");
    staff_def.push_attribute(("n", n_text.as_str()));
    staff_def.push_attribute(("lines", "5"));
    if let Some(clef) = staff.clef() {
        let (shape, line) = clef_shape_line(clef);
        staff_def.push_attribute(("clef.shape", shape));
        staff_def.push_attribute(("clef.line", line));
    }
    writer
        .write_event(Event::Empty(staff_def))
        .map_err(|error| RenderError::xml(&error))
}

fn clef_shape_line(clef: Clef) -> (&'static str, &'static str) {
    match clef {
        Clef::Treble => ("G", "2"),
        Clef::Bass => ("F", "4"),
        Clef::Alto => ("C", "3"),
        Clef::Tenor => ("C", "4"),
    }
}

fn write_measure(
    writer: &mut Writer<Vec<u8>>,
    plan: &NotationPlan,
    index: usize,
    piece_counts: &mut std::collections::HashMap<EventId, u32>,
) -> Result<(), RenderError> {
    let n_text = index.saturating_add(1).to_string();
    let mut measure = element("measure");
    measure.push_attribute(("n", n_text.as_str()));
    writer
        .write_event(Event::Start(measure))
        .map_err(|error| RenderError::xml(&error))?;
    for (staff_index, staff) in plan.staves().iter().enumerate() {
        let Some(measure_plan) = staff.measures().get(index) else {
            continue;
        };
        let staff_n = staff_index.saturating_add(1).to_string();
        let mut staff_elem = element("staff");
        staff_elem.push_attribute(("n", staff_n.as_str()));
        writer
            .write_event(Event::Start(staff_elem))
            .map_err(|error| RenderError::xml(&error))?;
        for (lane_index, lane) in measure_plan.lanes().iter().enumerate() {
            write_layer(
                writer,
                staff_index.saturating_add(1),
                lane_index.saturating_add(1),
                lane,
                piece_counts,
            )?;
        }
        end(writer, "staff")?;
    }
    end(writer, "measure")
}

fn write_layer(
    writer: &mut Writer<Vec<u8>>,
    staff: usize,
    lane: usize,
    lane_plan: &VoiceLane,
    piece_counts: &mut std::collections::HashMap<EventId, u32>,
) -> Result<(), RenderError> {
    let layer_id = format!("layer-{staff}-{lane}");
    let n_text = lane.to_string();
    let mut layer = element("layer");
    layer.push_attribute(("xml:id", layer_id.as_str()));
    layer.push_attribute(("n", n_text.as_str()));
    writer
        .write_event(Event::Start(layer))
        .map_err(|error| RenderError::xml(&error))?;

    if lane_plan.items().is_empty() {
        writer
            .write_event(Event::Empty(element("mSpace")))
            .map_err(|error| RenderError::xml(&error))?;
    }
    let items = lane_plan.items();
    for (index, item) in items.iter().enumerate() {
        let beam = item.beam();
        let opens_beam = beam.is_some()
            && index
                .checked_sub(1)
                .is_none_or(|before| items.get(before).and_then(NotatedItem::beam) != beam);
        let closes_beam = beam.is_some() && items.get(index.saturating_add(1)).and_then(NotatedItem::beam) != beam;
        if opens_beam {
            start(writer, "beam")?;
        }
        write_item(writer, item, piece_counts)?;
        if closes_beam {
            end(writer, "beam")?;
        }
    }
    end(writer, "layer")
}

/// The `xml:id` for one item: base `event-<hex>`, later tie pieces suffixed
/// `-t2`, `-t3`, … (module-contract above).
fn item_id(item: &NotatedItem, piece_counts: &mut std::collections::HashMap<EventId, u32>) -> String {
    let count = piece_counts.entry(item.event()).or_insert(0);
    let base = format!("event-{:x}", item.event().0);
    *count = count.saturating_add(1);
    if item.tie_stop() {
        format!("{base}-t{count}")
    } else {
        base
    }
}

/// The tie attribute for an item, from its plan flags.
fn tie_attr(item: &NotatedItem) -> Option<&'static str> {
    match (item.tie_start(), item.tie_stop()) {
        (true, true) => Some("m"),
        (true, false) => Some("i"),
        (false, true) => Some("t"),
        (false, false) => None,
    }
}

/// MEI `dur`/`dots` for a plan piece. Decomposition guarantees each piece is
/// a plain power of two (`1/2^k` → `dur = 2^k`) or a dotted value
/// (`3/2^(k+1)` → `dur = 2^k`, `dots = 1`).
fn dur_attrs(item: &NotatedItem) -> (String, Option<&'static str>) {
    let value = item.duration().value.as_ratio();
    let numerator = *value.numer();
    let denominator = *value.denom();
    if numerator == 1 {
        (denominator.to_string(), None)
    } else {
        debug_assert_eq!(numerator, 3, "plan pieces are plain or dotted values");
        ((denominator / 2).to_string(), Some("1"))
    }
}

fn push_note_pitch(elem: &mut BytesStart<'_>, pitch: WrittenPitch, octave: &str) {
    let letter = match pitch.letter {
        musa_compiler::Letter::C => "c",
        musa_compiler::Letter::D => "d",
        musa_compiler::Letter::E => "e",
        musa_compiler::Letter::F => "f",
        musa_compiler::Letter::G => "g",
        musa_compiler::Letter::A => "a",
        musa_compiler::Letter::B => "b",
    };
    elem.push_attribute(("oct", octave));
    elem.push_attribute(("pname", letter));
    let accid = match pitch.accidental.0 {
        1 => Some("s"),
        2 => Some("x"),
        -1 => Some("f"),
        -2 => Some("ff"),
        _ => None,
    };
    if let Some(accid) = accid {
        elem.push_attribute(("accid", accid));
    }
}

fn write_item(
    writer: &mut Writer<Vec<u8>>,
    item: &NotatedItem,
    piece_counts: &mut std::collections::HashMap<EventId, u32>,
) -> Result<(), RenderError> {
    let id = item_id(item, piece_counts);
    let (dur, dots) = dur_attrs(item);
    match item.kind() {
        NotatedKind::Rest => {
            let mut rest = element("rest");
            rest.push_attribute(("xml:id", id.as_str()));
            rest.push_attribute(("dur", dur.as_str()));
            if let Some(dots) = dots {
                rest.push_attribute(("dots", dots));
            }
            writer
                .write_event(Event::Empty(rest))
                .map_err(|error| RenderError::xml(&error))?;
        }
        NotatedKind::Note { pitch } => {
            let mut note = element("note");
            note.push_attribute(("xml:id", id.as_str()));
            note.push_attribute(("dur", dur.as_str()));
            if let Some(dots) = dots {
                note.push_attribute(("dots", dots));
            }
            push_note_pitch(&mut note, *pitch, &pitch.octave.to_string());
            if let Some(tie) = tie_attr(item) {
                note.push_attribute(("tie", tie));
            }
            writer
                .write_event(Event::Empty(note))
                .map_err(|error| RenderError::xml(&error))?;
        }
        NotatedKind::Chord { pitches } => {
            let mut chord = element("chord");
            chord.push_attribute(("xml:id", id.as_str()));
            chord.push_attribute(("dur", dur.as_str()));
            if let Some(dots) = dots {
                chord.push_attribute(("dots", dots));
            }
            if let Some(tie) = tie_attr(item) {
                chord.push_attribute(("tie", tie));
            }
            writer
                .write_event(Event::Start(chord))
                .map_err(|error| RenderError::xml(&error))?;
            for pitch in pitches {
                let mut tone = element("note");
                push_note_pitch(&mut tone, *pitch, &pitch.octave.to_string());
                writer
                    .write_event(Event::Empty(tone))
                    .map_err(|error| RenderError::xml(&error))?;
            }
            end(writer, "chord")?;
        }
    }
    Ok(())
}
