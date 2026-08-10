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

use musa_compiler::{Clef, EventId, Mark, Mode, Slot, WrittenPitch};
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};

use crate::RenderError;
use crate::plan::{
    ARTICULATION_PLACEMENT, DYNAMIC_PLACEMENT, NotatedItem, NotatedKind, NotationPlan, Placement, SLUR_PLACEMENT,
    StaffPlan, VoiceLane,
};

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
    write_head(&mut writer, plan)?;
    for wrapper in ["music", "body", "mdiv", "score"] {
        start(&mut writer, wrapper)?;
    }
    write_score_def(&mut writer, plan)?;
    start(&mut writer, "section")?;
    // Tie-piece counters are document-scoped: an event's pieces can span
    // measures, and the `-tN` suffixes must count across the whole chain.
    let mut piece_counts: std::collections::HashMap<EventId, u32> = std::collections::HashMap::new();
    // The longest staff, not the first: under polymeter a 7/8 staff has more
    // measures than the 4/4 above it, and taking the first would drop the
    // tail of the piece rather than mis-space it.
    let measure_count = plan
        .staves()
        .iter()
        .map(|staff| staff.measures().len())
        .max()
        .unwrap_or(0);
    let barlines = Barlines::of(plan);
    // A volta is an element that *contains* measures in MEI, not an attribute
    // on them, so the bracket is opened and closed around the loop.
    let mut open_until: Option<u32> = None;
    for index in 0..measure_count {
        let number = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
        if let Some(volta) = barlines.ending_at(number) {
            let label = volta.label();
            let n = volta.passes.first().copied().unwrap_or(1).to_string();
            start_with(&mut writer, "ending", &[("n", n.as_str()), ("label", label.as_str())])?;
            open_until = Some(volta.to);
        }
        let last = index.saturating_add(1) == measure_count;
        write_measure(&mut writer, plan, index, last, &barlines, &mut piece_counts)?;
        if open_until == Some(number) {
            end(&mut writer, "ending")?;
            open_until = None;
        }
    }
    end(&mut writer, "section")?;
    for wrapper in ["score", "mdiv", "body", "music"] {
        end(&mut writer, wrapper)?;
    }
    end(&mut writer, "mei")?;

    String::from_utf8(writer.into_inner()).map_err(|error| RenderError::Xml(error.to_string()))
}

/// `<meiHead>`: the front matter, as facts rather than as layout.
///
/// This is the catalogue record: what a library, an archive, or another
/// notation program reads to learn whose piece this is. It is not what gets
/// printed — [`write_page_head`] writes that — and the two are deliberately
/// separate, because a fact about the piece and a line on a page are different
/// things even when they carry the same words. A score with no `<meiHead>` is
/// a score Verovio warns about and titles "Untitled".
fn write_head(writer: &mut Writer<Vec<u8>>, plan: &NotationPlan) -> Result<(), RenderError> {
    let front = plan.front();
    start(writer, "meiHead")?;
    start(writer, "fileDesc")?;

    start(writer, "titleStmt")?;
    text_element(writer, "title", &front.title, &[])?;
    if let Some(subtitle) = front.subtitle.as_deref() {
        text_element(writer, "title", subtitle, &[("type", "subtitle")])?;
    }
    if let Some(composer) = front.composer.as_deref() {
        text_element(writer, "composer", composer, &[])?;
    }
    if let Some(arranger) = front.arranger.as_deref() {
        text_element(writer, "arranger", arranger, &[])?;
    }
    end(writer, "titleStmt")?;

    // `<pubStmt>` is required inside `<fileDesc>` even when the piece says
    // nothing about publication, so an unpublished piece gets the empty
    // element rather than an empty pair of tags.
    if let Some(copyright) = front.copyright.as_deref() {
        start(writer, "pubStmt")?;
        // `analog` is absent on purpose: the notice is the piece's own words,
        // not a machine-readable licence musa is entitled to interpret.
        start(writer, "availability")?;
        text_element(writer, "useRestrict", copyright, &[])?;
        end(writer, "availability")?;
        end(writer, "pubStmt")?;
    } else {
        writer
            .write_event(Event::Empty(element("pubStmt")))
            .map_err(|error| RenderError::xml(&error))?;
    }

    // Which reading this is. A determinate piece has none and writes none:
    // every realization of it produces this file, so naming a seed would be a
    // claim the document does not need.
    if let Some(performance) = front.performance {
        start(writer, "notesStmt")?;
        text_element(
            writer,
            "annot",
            &format!("One realization of an open work \u{2014} performance {performance}."),
            &[("type", "realization")],
        )?;
        end(writer, "notesStmt")?;
    }

    end(writer, "fileDesc")?;
    end(writer, "meiHead")
}

/// `<name attr="…">text</name>`, escaped by the writer.
fn text_element(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    text: &str,
    attributes: &[(&str, &str)],
) -> Result<(), RenderError> {
    let mut node = element(name);
    for (key, value) in attributes {
        node.push_attribute((*key, *value));
    }
    writer
        .write_event(Event::Start(node))
        .map_err(|error| RenderError::xml(&error))?;
    writer
        .write_event(Event::Text(quick_xml::events::BytesText::new(text)))
        .map_err(|error| RenderError::xml(&error))?;
    end(writer, name)
}

fn start(writer: &mut Writer<Vec<u8>>, name: &str) -> Result<(), RenderError> {
    writer
        .write_event(Event::Start(element(name)))
        .map_err(|error| RenderError::xml(&error))
}

/// `<name attr="…">`, for an element whose children are written by hand.
fn start_with(writer: &mut Writer<Vec<u8>>, name: &str, attributes: &[(&str, &str)]) -> Result<(), RenderError> {
    let mut node = element(name);
    for (key, value) in attributes {
        node.push_attribute((*key, *value));
    }
    writer
        .write_event(Event::Start(node))
        .map_err(|error| RenderError::xml(&error))
}

/// Bare text between elements, escaped by the writer.
fn text(writer: &mut Writer<Vec<u8>>, text: &str) -> Result<(), RenderError> {
    writer
        .write_event(Event::Text(quick_xml::events::BytesText::new(text)))
        .map_err(|error| RenderError::xml(&error))
}

/// `<name/>`.
fn empty(writer: &mut Writer<Vec<u8>>, name: &str) -> Result<(), RenderError> {
    writer
        .write_event(Event::Empty(element(name)))
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
    // A piece that opens unmeasured states no meter. MEI reads a missing
    // `meter.count` as "none stated", which is the truth here, where
    // `meter.count="0"` would be a meter of no beats.
    if count > 0 && unit > 0 {
        score_def.push_attribute(("meter.count", count_text.as_str()));
        score_def.push_attribute(("meter.unit", unit_text.as_str()));
    }
    let sig = first.key_signature().map(|key| key_sig(key.fifths));
    if let (Some(sig), Some(key)) = (sig.as_deref(), first.key_signature()) {
        score_def.push_attribute(("key.sig", sig));
        score_def.push_attribute(("key.mode", key_mode(key.mode)));
    }
    writer
        .write_event(Event::Start(score_def))
        .map_err(|error| RenderError::xml(&error))?;

    write_page_head(writer, plan)?;
    write_page_foot(writer, plan)?;

    start(writer, "staffGrp")?;
    for (index, staff) in plan.staves().iter().enumerate() {
        // A staff in its own meter states it here; one that agrees with the
        // score says nothing, so a piece that is not polymetric writes the
        // document it wrote before.
        let own = (staff.time_signature() != (count, unit)).then(|| staff.time_signature());
        write_staff_def(writer, index.saturating_add(1), staff, own)?;
    }
    end(writer, "staffGrp")?;
    end(writer, "scoreDef")
}

/// `<pgHead>` and `<pgHead2>`: the front matter as printed, with ids.
///
/// Verovio will draw a head of its own from `<meiHead>` — and did, until this
/// prompt — but an automatic head is anonymous: every id in it is generated
/// per-render, so nothing on the page can be traced back to the statement that
/// put it there. Writing the head here means the title carries
/// [`FRONT_TITLE`] the way a notehead carries `event-<hex>`, and clicking the
/// piece's name is the same machinery as clicking one of its notes.
///
/// The layout vocabulary this uses — head or foot, centred or right — is
/// MEI's own way of saying which *region* a line belongs to, and is the whole
/// of what musa is allowed to say about place. No coordinate, no margin, no
/// rastral size, nothing per-page: those are the engraver's, and Verovio
/// decides every one of them from the two hints below.
///
/// `<pgHead2>` is the running head on every page after the first. Verovio's
/// automatic one is a centred page number, and losing it silently was the
/// cost of encoding the head, so it is written here too — the number itself
/// is `<num label="page"/>`, which Verovio fills in per page.
fn write_page_head(writer: &mut Writer<Vec<u8>>, plan: &NotationPlan) -> Result<(), RenderError> {
    let front = plan.front();
    start(writer, "pgHead")?;

    // The title block: the piece's name, and beneath it whatever it is for.
    start_with(writer, "rend", &[("halign", "center"), ("valign", "top")])?;
    text_element(
        writer,
        "rend",
        &front.title,
        &[("xml:id", FRONT_TITLE), ("fontsize", "x-large")],
    )?;
    if let Some(subtitle) = front.subtitle.as_deref() {
        empty(writer, "lb")?;
        text_element(
            writer,
            "rend",
            subtitle,
            &[("xml:id", FRONT_SUBTITLE), ("fontsize", "small")],
        )?;
    }
    end(writer, "rend")?;

    // The attribution block, right of the title block and level with its foot,
    // which is where two centuries of engraved editions have put it.
    if front.composer.is_some() || front.arranger.is_some() {
        start_with(writer, "rend", &[("halign", "right"), ("valign", "bottom")])?;
        if let Some(composer) = front.composer.as_deref() {
            text_element(writer, "rend", composer, &[("xml:id", FRONT_COMPOSER)])?;
        }
        if let Some(arranger) = front.arranger.as_deref() {
            // The two names are one right-hand column, so the break between
            // them belongs inside it — a second block would be a second cell.
            if front.composer.is_some() {
                empty(writer, "lb")?;
            }
            text_element(
                writer,
                "rend",
                arranger,
                &[("xml:id", FRONT_ARRANGER), ("fontsize", "small")],
            )?;
        }
        end(writer, "rend")?;
    }
    end(writer, "pgHead")?;

    start(writer, "pgHead2")?;
    start_with(
        writer,
        "rend",
        &[("halign", "center"), ("valign", "top"), ("fontsize", "small")],
    )?;
    // `#` is the placeholder Verovio substitutes the page number for; a `<num>`
    // with any other content, or none, is printed literally. The dashes around
    // it are the running head Verovio drew automatically before this prompt
    // encoded the one on page 1, and losing them would be a regression nobody
    // asked for.
    text(writer, "– ")?;
    text_element(writer, "num", "#", &[("label", "page")])?;
    text(writer, " –")?;
    end(writer, "rend")?;
    end(writer, "pgHead2")
}

/// The `xml:id` each printed line of front matter carries, so the interface
/// can name what the pointer is over. Mirrored in the webview's
/// `score/front-matter.ts`; the two lists are one contract.
pub(crate) const FRONT_TITLE: &str = "front-title";
pub(crate) const FRONT_SUBTITLE: &str = "front-subtitle";
pub(crate) const FRONT_COMPOSER: &str = "front-composer";
pub(crate) const FRONT_ARRANGER: &str = "front-arranger";
pub(crate) const FRONT_COPYRIGHT: &str = "front-copyright";

/// `<pgFoot>`: the copyright line at the foot of the first page.
///
/// The notice is already in `<meiHead>` as `<useRestrict>`, which is where an
/// archive looks for it — but that is catalogue metadata, and an engraver does
/// not read it onto the page. `<pgFoot>` is MEI's own way to say "this line
/// belongs at the foot", so writing it here is still musa naming a fact and
/// the engraver placing it. Verovio's alternative is its automatic footer,
/// which advertises Verovio rather than printing the piece's notice.
///
/// A piece with no copyright gets no footer at all rather than an empty one,
/// which is why this is the encoded footer and not the automatic one.
fn write_page_foot(writer: &mut Writer<Vec<u8>>, plan: &NotationPlan) -> Result<(), RenderError> {
    let Some(copyright) = plan.front().copyright.as_deref() else {
        return Ok(());
    };
    start(writer, "pgFoot")?;
    // Centred and small, because that is how every edition sets a copyright
    // line: unaligned it goes flush with the page edge rather than the
    // margin, and at the default size it competes with the composer's name.
    // `<rend>` is required — Verovio drops bare text in a `<pgFoot>`.
    text_element(
        writer,
        "rend",
        copyright,
        &[
            ("xml:id", FRONT_COPYRIGHT),
            ("halign", "center"),
            ("fontsize", "x-small"),
        ],
    )?;
    end(writer, "pgFoot")
}

fn write_staff_def(
    writer: &mut Writer<Vec<u8>>,
    n: usize,
    staff: &StaffPlan,
    meter: Option<(u32, u32)>,
) -> Result<(), RenderError> {
    let n_text = n.to_string();
    let mut staff_def = element("staffDef");
    staff_def.push_attribute(("n", n_text.as_str()));
    staff_def.push_attribute(("lines", "5"));
    // Polymeter is a meter on the `<staffDef>` — MEI's own way of saying a
    // staff is counted differently from the score around it.
    let (count, unit) = meter.unwrap_or((0, 0));
    let (count_text, unit_text) = (count.to_string(), unit.to_string());
    if count > 0 && unit > 0 {
        staff_def.push_attribute(("meter.count", count_text.as_str()));
        staff_def.push_attribute(("meter.unit", unit_text.as_str()));
    }
    if let Some(clef) = staff.clef() {
        let (shape, line) = clef_shape_line(clef);
        staff_def.push_attribute(("clef.shape", shape));
        staff_def.push_attribute(("clef.line", line));
    }
    writer
        .write_event(Event::Start(staff_def))
        .map_err(|error| RenderError::xml(&error))?;
    // The part's name at the left of the first system, abbreviated after it.
    // Verovio indents the first system for these on its own, which is the
    // other half of what makes a page look like an edition rather than a run
    // of staves.
    text_element(writer, "label", staff.name(), &[])?;
    text_element(writer, "labelAbbr", &abbreviate(staff.name()), &[])?;
    end(writer, "staffDef")
}

/// A part name, shortened the way an engraver shortens one when the score
/// gives no abbreviation: the first three letters and a period. A name
/// already that short is left alone — `Vla.` is help, `Va.` for `Va` is not.
fn abbreviate(name: &str) -> String {
    let head: String = name.chars().take(3).collect();
    if head.chars().count() < name.chars().count() {
        format!("{head}.")
    } else {
        name.to_string()
    }
}

/// `2s`, `3f`, `0` — MEI's spelling of a fifths count.
fn key_sig(fifths: i8) -> String {
    match fifths.cmp(&0) {
        std::cmp::Ordering::Equal => "0".to_string(),
        std::cmp::Ordering::Greater => format!("{fifths}s"),
        std::cmp::Ordering::Less => format!("{}f", fifths.saturating_abs()),
    }
}

fn key_mode(mode: Mode) -> &'static str {
    match mode {
        Mode::Major => "major",
        Mode::Minor => "minor",
    }
}

fn clef_shape_line(clef: Clef) -> (&'static str, &'static str) {
    match clef {
        Clef::Treble => ("G", "2"),
        Clef::Bass => ("F", "4"),
        Clef::Alto => ("C", "3"),
        Clef::Tenor => ("C", "4"),
    }
}

/// Which measures carry a repeat barline, and which measures a volta bracket
/// covers.
///
/// A repeat crosses the system, so it is a fact about the measure rather than
/// about a staff — which is exactly how MEI holds it.
struct Barlines<'plan> {
    starts: std::collections::HashSet<u32>,
    ends: std::collections::HashSet<u32>,
    voltas: Vec<&'plan crate::plan::VoltaMark>,
}

impl<'plan> Barlines<'plan> {
    fn of(plan: &'plan NotationPlan) -> Self {
        let mut starts = std::collections::HashSet::new();
        let mut ends = std::collections::HashSet::new();
        let mut voltas = Vec::new();
        for repeat in plan.repeats() {
            starts.insert(repeat.from);
            match repeat.endings.split_last() {
                // With endings the repeat sign goes at the end of every
                // bracket but the last, which is where the music turns back.
                Some((_, before)) => ends.extend(before.iter().map(|volta| volta.to)),
                None => {
                    ends.insert(repeat.to);
                }
            }
            voltas.extend(repeat.endings.iter());
        }
        Self { starts, ends, voltas }
    }

    fn ending_at(&self, measure: u32) -> Option<&'plan crate::plan::VoltaMark> {
        self.voltas.iter().copied().find(|volta| volta.from == measure)
    }
}

fn write_measure(
    writer: &mut Writer<Vec<u8>>,
    plan: &NotationPlan,
    index: usize,
    last: bool,
    barlines: &Barlines<'_>,
    piece_counts: &mut std::collections::HashMap<EventId, u32>,
) -> Result<(), RenderError> {
    let number = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
    // A meter change is a `<scoreDef>` between measures, which is how MEI
    // says "from here on" — the opening one is written before the first
    // measure, so only the changes after it are written here.
    let opening = plan.staves().first().and_then(|staff| staff.measures().get(index));
    let meter_change = opening.and_then(crate::plan::MeasurePlan::time_signature);
    let key_change = opening.and_then(crate::plan::MeasurePlan::key_signature);
    if index > 0 && (meter_change.is_some() || key_change.is_some()) {
        let mut score_def = element("scoreDef");
        let count_text = meter_change.map(|(count, _)| count.to_string());
        let unit_text = meter_change.map(|(_, unit)| unit.to_string());
        if let (Some(count), Some(unit)) = (count_text.as_deref(), unit_text.as_deref()) {
            score_def.push_attribute(("meter.count", count));
            score_def.push_attribute(("meter.unit", unit));
        }
        let sig_text = key_change.map(|key| key_sig(key.fifths));
        if let (Some(sig), Some(key)) = (sig_text.as_deref(), key_change) {
            score_def.push_attribute(("key.sig", sig));
            score_def.push_attribute(("key.mode", key_mode(key.mode)));
        }
        writer
            .write_event(Event::Empty(score_def))
            .map_err(|error| RenderError::xml(&error))?;
    }
    let unmeasured = opening.is_some_and(|measure| !measure.meter().is_measured());
    let n_text = index.saturating_add(1).to_string();
    let mut measure = element("measure");
    measure.push_attribute(("n", n_text.as_str()));
    if barlines.starts.contains(&number) {
        measure.push_attribute(("left", "rptstart"));
    }
    // Thin-thick at the end. Musa's pieces are finite by construction, and a
    // score that stops on a plain barline reads as a fragment. A repeat sign
    // wins where both fall: the music turns back before it stops.
    if barlines.ends.contains(&number) {
        measure.push_attribute(("right", "rptend"));
    } else if last {
        measure.push_attribute(("right", "end"));
    } else if unmeasured {
        // `metcon="false"` says this measure is not controlled by the meter,
        // and the invisible right barline is the line that is not drawn.
        // Together they are what MEI has for a cadenza.
        measure.push_attribute(("right", "invis"));
    }
    if unmeasured {
        measure.push_attribute(("metcon", "false"));
    }
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
                // A clef belongs to the staff, and MEI writes it inside a
                // layer, so it is written in the first — the same choice
                // every backend makes for the same reason.
                if lane_index == 0 { measure_plan.clefs() } else { &[] },
                piece_counts,
            )?;
        }
        end(writer, "staff")?;
    }
    // Control events are measure children in MEI, not layer children: they
    // point at the notes they belong to rather than sitting among them.
    for (staff_index, staff) in plan.staves().iter().enumerate() {
        let Some(measure_plan) = staff.measures().get(index) else {
            continue;
        };
        let staff_n = staff_index.saturating_add(1).to_string();
        for lane in measure_plan.lanes() {
            write_control_events(
                writer,
                &staff_n,
                lane,
                measure_plan.time_signature().map_or(4, |(_, unit)| unit),
            )?;
        }
    }
    write_positioned(writer, plan, index)?;
    end(writer, "measure")
}

/// The side of the staff a symbol takes, as MEI spells it.
fn place(placement: Placement) -> &'static str {
    match placement {
        Placement::Above => "above",
        Placement::Below => "below",
    }
}

/// `<slur>` and `<dynam>` for one lane, anchored by `startid`/`endid`.
///
/// A slur is written in the measure it starts in; its `endid` may point into
/// a later measure, which is what those attributes are for.
fn write_control_events(
    writer: &mut Writer<Vec<u8>>,
    staff: &str,
    lane: &VoiceLane,
    unit: u32,
) -> Result<(), RenderError> {
    // A mark standing at one place is anchored by `tstamp`, not by `startid`:
    // a breath falls between two notes and there is no note it belongs to.
    for point in lane.points() {
        let Some(name) = point.mark.def().mei else {
            continue;
        };
        // Rational arithmetic on `Ratio<i64>` is exact mathematical
        // arithmetic, not raw integer ops. MEI counts beats from one, so the
        // onset is scaled into beats and the origin added.
        #[allow(clippy::arithmetic_side_effects)]
        let beat = point.onset_in_measure.as_ratio() * num_rational::Ratio::from_integer(i64::from(unit.max(1)))
            + num_rational::Ratio::ONE;
        let stamp = timestamp(beat);
        let mut element = element(name);
        element.push_attribute(("staff", staff));
        element.push_attribute(("tstamp", stamp.as_str()));
        match &point.argument {
            Some(argument) => {
                element.push_attribute(("place", "above"));
                let words = argument.to_string();
                writer
                    .write_event(Event::Start(element))
                    .map_err(|error| RenderError::xml(&error))?;
                writer
                    .write_event(Event::Text(quick_xml::events::BytesText::new(&words)))
                    .map_err(|error| RenderError::xml(&error))?;
                end(writer, name)?;
            }
            None => writer
                .write_event(Event::Empty(element))
                .map_err(|error| RenderError::xml(&error))?,
        }
    }
    for phrase in lane.phrases() {
        let start_ref = format!("#event-{:x}", phrase.from.0);
        let end_ref = format!("#event-{:x}", phrase.to.0);
        let mut element = element("phrase");
        element.push_attribute(("staff", staff));
        element.push_attribute(("startid", start_ref.as_str()));
        element.push_attribute(("endid", end_ref.as_str()));
        element.push_attribute(("label", phrase.name.as_str()));
        writer
            .write_event(Event::Empty(element))
            .map_err(|error| RenderError::xml(&error))?;
    }
    for hairpin in lane.hairpins() {
        let start_ref = format!("#event-{:x}", hairpin.from.0);
        let end_ref = format!("#event-{:x}", hairpin.to.0);
        let mut element = element("hairpin");
        element.push_attribute(("staff", staff));
        element.push_attribute(("startid", start_ref.as_str()));
        element.push_attribute(("endid", end_ref.as_str()));
        element.push_attribute(("form", if hairpin.grows { "cres" } else { "dim" }));
        element.push_attribute(("place", place(DYNAMIC_PLACEMENT)));
        writer
            .write_event(Event::Empty(element))
            .map_err(|error| RenderError::xml(&error))?;
    }
    for slur in lane.slurs() {
        let start_ref = format!("#event-{:x}", slur.from.0);
        let end_ref = format!("#event-{:x}", slur.to.0);
        let mut element = element("slur");
        element.push_attribute(("staff", staff));
        element.push_attribute(("startid", start_ref.as_str()));
        element.push_attribute(("endid", end_ref.as_str()));
        element.push_attribute(("curvedir", place(SLUR_PLACEMENT)));
        writer
            .write_event(Event::Empty(element))
            .map_err(|error| RenderError::xml(&error))?;
    }
    // A pedal or an ottava is a control event with both ends, exactly like a
    // slur: `<pedal>` and `<octave>` are what MEI calls them, and the row says
    // which.
    for span in lane.marks() {
        let Some(name) = span.mark.def().mei else {
            continue;
        };
        let start_ref = format!("#event-{:x}", span.from.0);
        let end_ref = format!("#event-{:x}", span.to.0);
        let mut element = element(name);
        element.push_attribute(("staff", staff));
        element.push_attribute(("startid", start_ref.as_str()));
        element.push_attribute(("endid", end_ref.as_str()));
        // `<octave>` says how far and in which direction; `@dis` is the
        // interval in steps and `@dis.place` the side.
        if let Some(musa_compiler::MarkArgument::Number(shift)) = span.argument {
            element.push_attribute(("dis", if shift.abs() >= 2 { "15" } else { "8" }));
            element.push_attribute(("dis.place", if shift < 0 { "below" } else { "above" }));
        }
        writer
            .write_event(Event::Empty(element))
            .map_err(|error| RenderError::xml(&error))?;
    }
    for item in lane.items() {
        // An ornament and a fermata stand beside the note rather than inside
        // it: MEI writes them as control events anchored by `startid`, which
        // is also how it keeps a trill that runs over a tie in one piece.
        for mark in item.articulations() {
            let (Some(name), Some(Slot::Ornament | Slot::Fermata)) = (mark.def().mei, mark.slot()) else {
                continue;
            };
            let start_ref = format!("#event-{:x}", item.event().0);
            let mut element = element(name);
            element.push_attribute(("staff", staff));
            element.push_attribute(("startid", start_ref.as_str()));
            writer
                .write_event(Event::Empty(element))
                .map_err(|error| RenderError::xml(&error))?;
        }
        // A phrase is a bracket over notes and a word above them. The bracket
        // is `<phrase>`, which every MEI consumer understands; the word is a
        // `<dir>`, which every MEI consumer *prints* — a renderer that draws
        // no bracket still shows the composer what the phrase is called.
        if let Some(phrase) = item.phrase().filter(|phrase| phrase.start) {
            let start_ref = format!("#event-{:x}", item.event().0);
            let mut dir = element("dir");
            dir.push_attribute(("staff", staff));
            dir.push_attribute(("startid", start_ref.as_str()));
            dir.push_attribute(("place", "above"));
            dir.push_attribute(("type", "phrase"));
            writer
                .write_event(Event::Start(dir))
                .map_err(|error| RenderError::xml(&error))?;
            writer
                .write_event(Event::Text(quick_xml::events::BytesText::new(&phrase.name)))
                .map_err(|error| RenderError::xml(&error))?;
            end(writer, "dir")?;
        }
        // A hairpin arrives at a mark, and the mark is printed where it
        // arrives: without it the reader sees a wedge that grows to nothing.
        if let Some(hairpin) = item.hairpin().filter(|hairpin| hairpin.stop) {
            let start_ref = format!("#event-{:x}", item.event().0);
            let mut dynam = element("dynam");
            dynam.push_attribute(("staff", staff));
            dynam.push_attribute(("startid", start_ref.as_str()));
            dynam.push_attribute(("place", place(DYNAMIC_PLACEMENT)));
            writer
                .write_event(Event::Start(dynam))
                .map_err(|error| RenderError::xml(&error))?;
            writer
                .write_event(Event::Text(quick_xml::events::BytesText::new(hairpin.target.name())))
                .map_err(|error| RenderError::xml(&error))?;
            end(writer, "dynam")?;
        }
        if let Some(mark) = item.dynamic() {
            let start_ref = format!("#event-{:x}", item.event().0);
            let mut dynam = element("dynam");
            dynam.push_attribute(("staff", staff));
            dynam.push_attribute(("startid", start_ref.as_str()));
            dynam.push_attribute(("place", place(DYNAMIC_PLACEMENT)));
            writer
                .write_event(Event::Start(dynam))
                .map_err(|error| RenderError::xml(&error))?;
            writer
                .write_event(Event::Text(quick_xml::events::BytesText::new(mark.name())))
                .map_err(|error| RenderError::xml(&error))?;
            end(writer, "dynam")?;
        }
    }
    Ok(())
}

/// One `<dir>`: a text instruction at a timestamp, optionally reaching to a
/// second one.
fn write_dir(writer: &mut Writer<Vec<u8>>, stamp: &str, until: Option<&str>, text: &str) -> Result<(), RenderError> {
    let mut dir = element("dir");
    dir.push_attribute(("staff", "1"));
    dir.push_attribute(("tstamp", stamp));
    if let Some(until) = until {
        dir.push_attribute(("tstamp2", until));
    }
    dir.push_attribute(("place", "above"));
    writer
        .write_event(Event::Start(dir))
        .map_err(|error| RenderError::xml(&error))?;
    writer
        .write_event(Event::Text(quick_xml::events::BytesText::new(text)))
        .map_err(|error| RenderError::xml(&error))?;
    end(writer, "dir")
}

/// A `tstamp` as MEI counts them: beat 1 is the start of the measure, and a
/// symbol halfway through a 4/4 bar is beat 3.
fn timestamp(beats: num_rational::Ratio<i64>) -> String {
    if *beats.denom() == 1 {
        return beats.numer().to_string();
    }
    // MEI timestamps are decimal; a beat that is not a whole number is written
    // to four places, which is exact for every value a notated onset can take.
    let numerator = *beats.numer() as f64;
    let denominator = *beats.denom() as f64;
    format!("{:.4}", numerator / denominator)
}

/// `<harm>` and `<dir>` for the symbols written at a position rather than on
/// a note. Both hang off the measure with a `tstamp`, which is how MEI says
/// "here, whether or not a notehead is here".
fn write_positioned(writer: &mut Writer<Vec<u8>>, plan: &NotationPlan, index: usize) -> Result<(), RenderError> {
    let measure = u32::try_from(index.saturating_add(1)).unwrap_or(1);
    let unit = plan.staves().first().map_or(4, |staff| staff.time_signature().1);
    // The piece's markings on staff 1, and — under polytempo — each staff's
    // own over that staff. `staff` is MEI's answer to "who is this for", so
    // the two need no separate mechanism.
    let staffed = plan.staves().iter().enumerate().flat_map(|(index, staff)| {
        let n = u32::try_from(index.saturating_add(1)).unwrap_or(1);
        staff.tempos().iter().map(move |mark| (n, mark))
    });
    for (staff, tempo) in plan.tempos().iter().map(|mark| (1, mark)).chain(staffed) {
        if tempo.measure != measure {
            continue;
        }
        let staff_text = staff.to_string();
        let stamp = timestamp(tempo.beat(unit));
        let mut element = element("tempo");
        element.push_attribute(("staff", staff_text.as_str()));
        element.push_attribute(("tstamp", stamp.as_str()));
        element.push_attribute(("place", "above"));
        // `mm`/`mm.unit` say a metronome mark in the form an engraver can set
        // as a note glyph and a number, which is how one is printed; spelling
        // it as text too would be a second, worse copy of the same fact. A
        // tempo *word*, on the other hand, has nowhere else to go, so it is
        // the element's content — and a marking carrying both writes both.
        let bpm = tempo.what.metronome.map(|mark| mark.bpm.to_string());
        let note_value = tempo.what.metronome.map(|mark| mark.beat.denom().to_string());
        if let (Some(bpm), Some(note_value)) = (bpm.as_ref(), note_value.as_ref()) {
            element.push_attribute(("mm", bpm.as_str()));
            element.push_attribute(("mm.unit", note_value.as_str()));
            element.push_attribute(("midi.bpm", bpm.as_str()));
        }
        match tempo.what.text.as_ref() {
            Some(text) => {
                writer
                    .write_event(Event::Start(element))
                    .map_err(|error| RenderError::xml(&error))?;
                writer
                    .write_event(Event::Text(quick_xml::events::BytesText::new(text)))
                    .map_err(|error| RenderError::xml(&error))?;
                writer
                    .write_event(Event::End(BytesEnd::new("tempo")))
                    .map_err(|error| RenderError::xml(&error))?;
            }
            None => writer
                .write_event(Event::Empty(element))
                .map_err(|error| RenderError::xml(&error))?,
        }
    }
    for section in plan.sections().iter().filter(|mark| mark.measure == measure) {
        let stamp = timestamp(section.beat(unit));
        let mut dir = element("dir");
        dir.push_attribute(("staff", "1"));
        dir.push_attribute(("tstamp", stamp.as_str()));
        dir.push_attribute(("place", "above"));
        dir.push_attribute(("type", "section"));
        writer
            .write_event(Event::Start(dir))
            .map_err(|error| RenderError::xml(&error))?;
        writer
            .write_event(Event::Text(quick_xml::events::BytesText::new(&section.what)))
            .map_err(|error| RenderError::xml(&error))?;
        end(writer, "dir")?;
    }
    // MEI has no free-duration bracket and no open-region element, so both
    // are `<dir>`: the reach of a held note above its notehead, and the
    // instruction over the region it governs, spanning with `tstamp2` where
    // it covers more than one measure. Lossy, and stated as lossy in
    // `docs/kernel/07-backend-contract.md`.
    for hold in plan.holds().iter().filter(|mark| mark.measure == measure) {
        let stamp = timestamp(hold.beat(unit));
        write_dir(writer, &stamp, None, &format!("hold to {}", hold.what.most.as_ratio()))?;
    }
    for region in plan.open().iter().filter(|region| region.from == measure) {
        let reach = region.to.saturating_sub(region.from);
        let stamp = timestamp(num_rational::Ratio::ONE);
        let until = format!("{reach}m+1");
        write_dir(writer, &stamp, Some(until.as_str()), &region.text)?;
    }
    for chord in plan.harmony().iter().filter(|mark| mark.measure == measure) {
        let stamp = timestamp(chord.beat(unit));
        let mut harm = element("harm");
        harm.push_attribute(("staff", "1"));
        harm.push_attribute(("tstamp", stamp.as_str()));
        harm.push_attribute(("place", "above"));
        writer
            .write_event(Event::Start(harm))
            .map_err(|error| RenderError::xml(&error))?;
        writer
            .write_event(Event::Text(quick_xml::events::BytesText::new(&chord.what.text)))
            .map_err(|error| RenderError::xml(&error))?;
        end(writer, "harm")?;
    }
    Ok(())
}

fn write_layer(
    writer: &mut Writer<Vec<u8>>,
    staff: usize,
    lane: usize,
    lane_plan: &VoiceLane,
    clefs: &[crate::plan::ClefChange],
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
    let mut pending_clefs = clefs.iter();
    let mut next_clef = pending_clefs.next();
    for (index, item) in items.iter().enumerate() {
        while let Some(change) = next_clef.filter(|change| change.onset_in_measure <= item.onset_in_measure()) {
            let (shape, line) = clef_shape_line(change.clef);
            let mut clef = element("clef");
            clef.push_attribute(("shape", shape));
            clef.push_attribute(("line", line));
            writer
                .write_event(Event::Empty(clef))
                .map_err(|error| RenderError::xml(&error))?;
            next_clef = pending_clefs.next();
        }
        let beam = item.beam();
        let opens_beam = beam.is_some()
            && index
                .checked_sub(1)
                .is_none_or(|before| items.get(before).and_then(NotatedItem::beam) != beam);
        let closes_beam = beam.is_some() && items.get(index.saturating_add(1)).and_then(NotatedItem::beam) != beam;
        // A tuplet contains its beams, not the other way round: the bracket
        // is about how long the notes are, the beam about how they group.
        if let Some(tuplet) = item.tuplet().filter(|tuplet| tuplet.start) {
            let num = tuplet.num.to_string();
            let numbase = tuplet.den.to_string();
            let mut element = element("tuplet");
            element.push_attribute(("num", num.as_str()));
            element.push_attribute(("numbase", numbase.as_str()));
            writer
                .write_event(Event::Start(element))
                .map_err(|error| RenderError::xml(&error))?;
        }
        if opens_beam {
            start(writer, "beam")?;
        }
        write_item(writer, item, piece_counts)?;
        if closes_beam {
            end(writer, "beam")?;
        }
        if item.tuplet().is_some_and(|tuplet| tuplet.stop) {
            end(writer, "tuplet")?;
        }
    }
    // A change past the last note of the measure still has to be written:
    // dropping it would move the clef to somewhere it was not asked for.
    while let Some(change) = next_clef {
        let (shape, line) = clef_shape_line(change.clef);
        let mut clef = element("clef");
        clef.push_attribute(("shape", shape));
        clef.push_attribute(("line", line));
        writer
            .write_event(Event::Empty(clef))
            .map_err(|error| RenderError::xml(&error))?;
        next_clef = pending_clefs.next();
    }
    end(writer, "layer")
}

/// Whether any of an item's marks belongs inside its `<note>`.
fn has_artics(item: &NotatedItem) -> bool {
    item.articulations().iter().any(|mark| artic_name(*mark).is_some())
}

/// The `@artic` spelling for a mark, when MEI writes it *inside* the note.
///
/// Only two slots qualify: `@artic` holds articulations and technical
/// indications. An ornament and a fermata are control events, which stand
/// beside the note rather than in it, so they are written by
/// [`write_control_events`] instead.
fn artic_name(mark: Mark) -> Option<&'static str> {
    match (mark.def().mei, mark.slot()) {
        (Some(spelling), Some(Slot::Articulation | Slot::Technical)) => Some(spelling),
        _ => None,
    }
}

/// `<artic>` children for an item, when it carries any.
fn write_artics(writer: &mut Writer<Vec<u8>>, item: &NotatedItem) -> Result<(), RenderError> {
    write_mark_artics(writer, item.articulations())
}

fn write_mark_artics(writer: &mut Writer<Vec<u8>>, marks: &[Mark]) -> Result<(), RenderError> {
    for mark in marks {
        let Some(spelling) = artic_name(*mark) else {
            continue;
        };
        let mut artic = element("artic");
        artic.push_attribute(("artic", spelling));
        artic.push_attribute(("place", place(ARTICULATION_PLACEMENT)));
        writer
            .write_event(Event::Empty(artic))
            .map_err(|error| RenderError::xml(&error))?;
    }
    Ok(())
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

fn push_note_pitch(
    elem: &mut BytesStart<'_>,
    pitch: WrittenPitch,
    octave: &str,
    event: EventId,
) -> Result<(), RenderError> {
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
        3 => Some("ts"),
        -1 => Some("f"),
        -2 => Some("ff"),
        -3 => Some("tf"),
        0 => None,
        alteration => {
            return Err(RenderError::Unsupported {
                event,
                what: format!("MEI cannot faithfully encode written pitch {pitch} with alteration {alteration}"),
            });
        }
    };
    if let Some(accid) = accid {
        elem.push_attribute(("accid", accid));
    }
    Ok(())
}

/// The grace notes leaning on an item, as a `<graceGrp>` immediately before
/// it.
///
/// `attach="pre"` says the group belongs to the note that follows, which is
/// what the source says too. `grace="unknown"` on each note is deliberate:
/// MEI's other two values, `acc` and `unacc`, are *performance* answers —
/// on the beat or ahead of it — and that question belongs to the profile
/// (`musa_compiler::GracePolicy`), not to the page. Writing either one here
/// would put one reading of the piece into a file that is meant to carry the
/// piece. A consumer with a house style applies it, exactly as a performer
/// does.
///
/// `dur="8"` because MEI requires a written duration on a note and a grace has
/// none. It is a drawing instruction — an eighth-note flag — and nothing reads
/// it as time: the group is outside the measure's duration arithmetic.
fn write_graces(writer: &mut Writer<Vec<u8>>, item: &NotatedItem) -> Result<(), RenderError> {
    if item.graces().is_empty() {
        return Ok(());
    }
    let mut group = element("graceGrp");
    group.push_attribute(("attach", "pre"));
    writer
        .write_event(Event::Start(group))
        .map_err(|error| RenderError::xml(&error))?;
    for grace in item.graces() {
        let mut note = element("note");
        note.push_attribute(("dur", "8"));
        note.push_attribute(("grace", "unknown"));
        push_note_pitch(&mut note, grace.pitch, &grace.pitch.octave.to_string(), item.event())?;
        if grace.articulations.iter().any(|mark| artic_name(*mark).is_some()) {
            writer
                .write_event(Event::Start(note))
                .map_err(|error| RenderError::xml(&error))?;
            write_mark_artics(writer, &grace.articulations)?;
            end(writer, "note")?;
        } else {
            writer
                .write_event(Event::Empty(note))
                .map_err(|error| RenderError::xml(&error))?;
        }
    }
    end(writer, "graceGrp")?;
    Ok(())
}

fn write_item(
    writer: &mut Writer<Vec<u8>>,
    item: &NotatedItem,
    piece_counts: &mut std::collections::HashMap<EventId, u32>,
) -> Result<(), RenderError> {
    let id = item_id(item, piece_counts);
    let (dur, dots) = dur_attrs(item);
    write_graces(writer, item)?;
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
            push_note_pitch(&mut note, *pitch, &pitch.octave.to_string(), item.event())?;
            if let Some(tie) = tie_attr(item) {
                note.push_attribute(("tie", tie));
            }
            if has_artics(item) {
                writer
                    .write_event(Event::Start(note))
                    .map_err(|error| RenderError::xml(&error))?;
                write_artics(writer, item)?;
                end(writer, "note")?;
            } else {
                writer
                    .write_event(Event::Empty(note))
                    .map_err(|error| RenderError::xml(&error))?;
            }
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
            write_artics(writer, item)?;
            for pitch in pitches {
                let mut tone = element("note");
                push_note_pitch(&mut tone, *pitch, &pitch.octave.to_string(), item.event())?;
                writer
                    .write_event(Event::Empty(tone))
                    .map_err(|error| RenderError::xml(&error))?;
            }
            end(writer, "chord")?;
        }
    }
    Ok(())
}
