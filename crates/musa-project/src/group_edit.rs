//! One musical command over an explicit selection (prompt 206).
//!
//! [`crate::edit`] turns one event into one text edit. This turns a *set* of
//! events into one transaction, and the difference is not arithmetic: a
//! selection reaches several notes through provenance, several of them may be
//! spelled by the same statement, and rewriting that statement once per event
//! would apply the transformation as many times as the composer selected
//! occurrences of it. Resolving the selection down to the minimal set of
//! unique source replacements is therefore the whole job, and it happens
//! before anything is written.
//!
//! Five intents, and they are deliberately five rather than one
//! (Open Music Theory `009`–`012`, `016`, `100`):
//!
//! - setting every written value to an eighth is not fitting a phrase into
//!   the span it already occupied;
//! - scaling a passage is not setting it — scaling survives a tuplet, because
//!   multiplying every written value inside a group multiplies what the group
//!   sounds, and setting does not;
//! - moving along the staff is not transposing by an interval, because a step
//!   means whatever the collection in force says it means and an interval
//!   means one fixed thing everywhere.
//!
//! What this module never does is compute the new spelling from what the
//! composer is *looking at*. Every transformation is applied to what the
//! source *writes* at the definition the event resolves to, read back through
//! [`musa_syntax::read_statements`]. That is what makes a relative
//! transformation of generated music correct: `up P5 { c4 }` moved one step
//! becomes `up P5 { d4 }`, and the note on the page moves one step because
//! translation commutes with translation.

use std::collections::BTreeMap;

use musa_score::{Interval, WrittenPitch};
use musa_syntax::{EditIntent, WrittenKind, WrittenStatement};
use num_rational::Ratio;

use crate::command::{Revision, TextEdit};
use crate::diagnostic::{Diagnostic, Span};
use crate::edit::GeneratedEditMode;
use crate::error::ProjectError;
use crate::facts::{EventFacts, EventKind, Fraction, ScoreFacts};

/// What one group command does to every event it applies to.
///
/// Five distinct musical acts, never collapsed into "change notes": each names
/// what a musician would say they were doing, and two of them that look alike
/// on one note diverge on a passage.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GroupIntent {
    /// Write every applicable event as this exact notated duration.
    ///
    /// Absolute: after it, every selected note, chord, and rest is written as
    /// `duration`, whatever it was written as before. It says nothing about
    /// the span the passage occupied, and it is refused where the written
    /// value is not what is counted — inside a `tuplet` or a `stretch`.
    SetEachDuration {
        /// The notated duration, exactly, as the language writes one (`1/8`).
        duration: String,
    },
    /// Multiply every applicable written duration by an exact positive ratio.
    ///
    /// Relative, and therefore a different command: it keeps the rhythm of
    /// the passage and changes its rate. Because it is a factor, it composes
    /// with a `tuplet` or `stretch` around it rather than lying about one.
    ScaleDurations {
        /// The factor, exactly, as a positive rational (`2`, `1/2`, `3/2`).
        ratio: String,
    },
    /// Move every written pitch along the staff, carrying its accidental.
    MoveDiatonically {
        /// Signed diatonic steps; positive is up the staff.
        steps: i32,
    },
    /// Move every written accidental along its ladder, carrying its letter.
    ShiftAccidentals {
        /// Signed alterations; positive raises.
        steps: i32,
    },
    /// Transpose every written pitch by one spelled interval.
    TransposeBy {
        /// The interval as the source spells one: `P5`, `up P5`, `down m3`.
        interval: String,
    },
}

impl GroupIntent {
    /// Whether this intent is about rhythm rather than pitch. Rests are
    /// applicable to the first kind and inapplicable to the second, which is
    /// the whole of what "mixed selections are honest" means here.
    const fn rhythmic(&self) -> bool {
        matches!(*self, Self::SetEachDuration { .. } | Self::ScaleDurations { .. })
    }

    /// How the interface reports it, in the same words it offered it.
    fn describe(&self) -> String {
        match *self {
            Self::SetEachDuration { ref duration } => format!("write every selected value as {duration}"),
            Self::ScaleDurations { ref ratio } => format!("scale the selected durations by {ratio}"),
            Self::MoveDiatonically { steps } => format!("move the selection {} on the staff", steps_phrase(steps)),
            Self::ShiftAccidentals { steps } => format!("shift the selected accidentals {}", ladder_phrase(steps)),
            Self::TransposeBy { ref interval } => format!("transpose the selection by {interval}"),
        }
    }
}

fn steps_phrase(steps: i32) -> String {
    let count = steps.unsigned_abs();
    let noun = if count == 1 { "step" } else { "steps" };
    format!("{count} {noun} {}", if steps < 0 { "down" } else { "up" })
}

fn ladder_phrase(steps: i32) -> String {
    let count = steps.unsigned_abs();
    let noun = if count == 1 { "degree" } else { "degrees" };
    format!("{count} {noun} {}", if steps < 0 { "down" } else { "up" })
}

/// One group command: which events, what to do to them, and what to do about
/// the ones that were generated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupEdit {
    /// Event ids from the snapshot. May span voices and parts, in any order
    /// and with repeats: ordering and de-duplication are this crate's job,
    /// because the frontend collects identities by geometry and must not
    /// reason about score order (`03-interaction.md` §7).
    pub events: Vec<String>,
    /// What to do to them.
    pub intent: GroupIntent,
    /// What to do where they were produced by an expansion (roadmap §9).
    pub mode: GeneratedEditMode,
}

/// One statement the transformation rewrites, and everything that changes
/// because it does.
///
/// The unit of a group edit is the *definition*, not the event: this is what
/// `04-provenance.md` §4's counts are counts of, grouped so that a composer
/// who selected four notes of one motif is told they are rewriting one
/// statement and changing every occurrence of it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupDefinition {
    /// The statement being rewritten.
    pub definition: Span,
    /// The motif it belongs to, when it is inside one.
    pub motif: Option<String>,
    /// The occurrences that would change, labelled as the inspector labels
    /// them. Empty for authored music.
    pub occurrences: Vec<String>,
    /// The selected events that resolved to this statement.
    pub selected: Vec<String>,
    /// Every event this statement spelled, selected or not — the notes that
    /// change because the definition did.
    pub events: Vec<String>,
    /// What the statement writes now, and what it would write instead.
    pub before: String,
    /// The replacement, in the same spelling.
    pub after: String,
}

/// How one bar's written content changes, in whole notes.
///
/// Reported because a rhythmic transformation moves the bar lines and a
/// composer deciding whether to accept one wants to see that before the page
/// redraws. Measured from two compiles rather than from arithmetic on the
/// selection: what a bar holds is the compiler's answer, and a second one
/// computed here could disagree with the page.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupBarEffect {
    /// The part.
    pub part: String,
    /// The voice within it.
    pub voice: String,
    /// The 1-based bar number.
    pub bar: u32,
    /// Written content before, in whole notes.
    pub before: Fraction,
    /// Written content after.
    pub after: Fraction,
}

/// The immutable preview of one group transformation.
///
/// Produced by [`crate::ProjectSession::plan_group_edit`] and consumed, once,
/// by applying it at the same revision. Everything in it was computed by the
/// code that would apply it — the edits are the edits, the source is the
/// source, and the diagnostics are the compiler's own answer about that
/// source — so an interface showing this is showing the transaction rather
/// than a second guess at it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupEditPlan {
    pub(crate) id: u64,
    pub(crate) revision: Revision,
    pub(crate) summary: String,
    pub(crate) changed: Vec<String>,
    pub(crate) unchanged: Vec<String>,
    pub(crate) definitions: Vec<GroupDefinition>,
    pub(crate) edits: Vec<TextEdit>,
    pub(crate) source: String,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) bars: Vec<GroupBarEffect>,
    pub(crate) specializable: bool,
}

impl GroupEditPlan {
    /// The identity applying this plan consumes. Minted per plan, never
    /// reused, and meaningless against another document.
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// The revision whose byte ranges and events this plan describes.
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    /// What it does, in the words the interface offered it in.
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// Every event that would change, in score order — including the ones the
    /// composer did not select but that the same statement spelled.
    pub fn changed(&self) -> &[String] {
        &self.changed
    }

    /// The selected events this intent does not apply to, in score order: the
    /// rests under a pitch command. Stated rather than dropped, because the
    /// count is what makes a mixed selection honest.
    pub fn unchanged(&self) -> &[String] {
        &self.unchanged
    }

    /// The statements being rewritten, grouped by definition.
    pub fn definitions(&self) -> &[GroupDefinition] {
        &self.definitions
    }

    /// The exact replacements, in source order and proved not to overlap.
    pub fn edits(&self) -> &[TextEdit] {
        &self.edits
    }

    /// The complete source this transaction would commit.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// What the compiler says about that source. A group edit that would
    /// overfill a bar is previewed with the bar's complaint attached, not
    /// applied and explained afterwards.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Which bars hold something different afterwards.
    pub fn bars(&self) -> &[GroupBarEffect] {
        &self.bars
    }

    /// Whether the same selection could instead be written onto its call as
    /// overrides, leaving the motif alone (roadmap §9).
    pub const fn specializable(&self) -> bool {
        self.specializable
    }
}

/// One selected event, resolved through provenance to the statement that
/// spells it and the written value that statement carries.
struct Resolved<'a> {
    event: &'a EventFacts,
    definition: Span,
    written: WrittenStatement,
}

/// Order the selection the way the score does, and say each event once.
///
/// The frontend collects identities by geometry — a rectangle over two staves
/// hands them back in whatever order the engraving laid them out — so score
/// order is established here, against the events the core listed, and an id
/// named twice is one event.
fn ordered<'a>(facts: &'a ScoreFacts, events: &[String]) -> Result<Vec<&'a EventFacts>, ProjectError> {
    let mut wanted: Vec<&str> = events.iter().map(String::as_str).collect();
    wanted.sort_unstable();
    wanted.dedup();
    for id in &wanted {
        if !facts.events.iter().any(|event| event.id == *id) {
            return Err(ProjectError::NoSuchEvent((*id).to_owned()));
        }
    }
    Ok(facts
        .events
        .iter()
        .filter(|event| wanted.binary_search(&event.id.as_str()).is_ok())
        .collect())
}

fn span_of(event: &EventFacts) -> Span {
    event.origin.definition_span
}

/// Whether this intent can touch this kind of event at all.
const fn applies_to(intent: &GroupIntent, kind: EventKind) -> bool {
    match kind {
        EventKind::Rest => intent.rhythmic(),
        EventKind::Note | EventKind::Chord => true,
    }
}

/// A written duration as an exact rational, or nothing when the source names
/// it rather than writing it.
pub(crate) fn exact(written: &str) -> Option<Ratio<i64>> {
    match written.split_once('/') {
        Some((numerator, denominator)) => {
            let numerator: i64 = numerator.trim().parse().ok()?;
            let denominator: i64 = denominator.trim().parse().ok()?;
            (denominator != 0).then(|| Ratio::new(numerator, denominator))
        }
        None => Some(Ratio::from_integer(written.trim().parse().ok()?)),
    }
}

/// The interval a source phrase spells, with its direction.
fn interval(written: &str) -> Option<Interval> {
    let text = written.trim();
    match text.split_once(char::is_whitespace) {
        Some(("up", rest)) => Interval::parse(rest.trim(), false),
        Some(("down", rest)) => Interval::parse(rest.trim(), true),
        Some(_) => None,
        None => Interval::parse(text, false),
    }
}

fn uneditable(reason: impl Into<String>) -> ProjectError {
    ProjectError::Uneditable(reason.into())
}

/// The new written duration for one statement, or the refusal that names the
/// smallest next action.
fn rewritten_duration(intent: &GroupIntent, written: &WrittenStatement) -> Result<String, ProjectError> {
    let Some(before) = written.duration.as_deref() else {
        return Err(uneditable("this statement writes no duration to change"));
    };
    if !written.chain.is_empty() {
        return Err(uneditable(
            "this note is tied to its neighbour, and a chain is one sounding note written as several; \
             untie it before renotating one link",
        ));
    }
    match *intent {
        GroupIntent::SetEachDuration { ref duration } => {
            if written.duration_scaled {
                return Err(uneditable(
                    "this note is inside a tuplet or a stretch, so what is written is not what is counted; \
                     scale the passage instead, or change the group's ratio",
                ));
            }
            let value = exact(duration).ok_or_else(|| uneditable(format!("`{duration}` is not a notated duration")))?;
            if *value.numer() <= 0 {
                return Err(uneditable("a notated duration is longer than nothing"));
            }
            Ok(format!("{}/{}", value.numer(), value.denom()))
        }
        GroupIntent::ScaleDurations { ref ratio } => {
            let factor = exact(ratio).ok_or_else(|| uneditable(format!("`{ratio}` is not an exact ratio")))?;
            if *factor.numer() <= 0 {
                return Err(uneditable("a passage is scaled by a positive factor"));
            }
            let value = exact(before).ok_or_else(|| {
                uneditable(format!(
                    "this duration is written as `{before}`, which is a name rather than a value"
                ))
            })?;
            // Exact and checked: a factor big enough to overflow the
            // rational is a refusal, never a wrapped duration.
            let numerator = value.numer().checked_mul(*factor.numer());
            let denominator = value.denom().checked_mul(*factor.denom());
            let (Some(numerator), Some(denominator)) = (numerator, denominator) else {
                return Err(uneditable("that factor is too large to write as a notated duration"));
            };
            let scaled = Ratio::new(numerator, denominator);
            Ok(format!("{}/{}", scaled.numer(), scaled.denom()))
        }
        GroupIntent::MoveDiatonically { .. }
        | GroupIntent::ShiftAccidentals { .. }
        | GroupIntent::TransposeBy { .. } => Err(uneditable("that is not a rhythmic command")),
    }
}

/// The new written pitches for one statement, or the refusal.
fn rewritten_pitches(intent: &GroupIntent, written: &WrittenStatement) -> Result<Vec<String>, ProjectError> {
    if written.pitch_mirrored {
        return Err(uneditable(
            "this note is inside an `invert` block, which mirrors it, so moving what is written up \
             moves what is read down; edit the block, or select the authored notes",
        ));
    }
    if written.pitches.is_empty() {
        return Err(uneditable("this statement writes no pitch to change"));
    }
    written
        .pitches
        .iter()
        .map(|spelling| moved_pitch(intent, spelling))
        .collect()
}

/// One written pitch under a pitch intent, or the refusal that names why it
/// could not move.
///
/// Lifted out of [`rewritten_pitches`] because Review transforms the same way
/// (prompt 207): a proposal note carries a spelling and no source span, and
/// the arithmetic that turns `g#4` into `a#4` must be the one the source path
/// already uses, or the two surfaces would disagree about what a step is.
pub(crate) fn moved_pitch(intent: &GroupIntent, spelling: &str) -> Result<String, ProjectError> {
    let pitch = WrittenPitch::parse(spelling).ok_or_else(|| {
        uneditable(format!(
            "this pitch is written as `{spelling}`, which is a name rather than a note"
        ))
    })?;
    let moved = match *intent {
        GroupIntent::MoveDiatonically { steps } => pitch.step(i64::from(steps)),
        GroupIntent::ShiftAccidentals { steps } => pitch.alter(steps),
        GroupIntent::TransposeBy { interval: ref written } => {
            let action =
                interval(written).ok_or_else(|| uneditable(format!("`{written}` is not a written interval")))?;
            pitch.transpose(action)
        }
        GroupIntent::SetEachDuration { .. } | GroupIntent::ScaleDurations { .. } => {
            return Err(uneditable("that is not a pitch command"));
        }
    };
    moved
        .map(|result| result.to_string())
        .ok_or_else(|| uneditable(format!("`{spelling}` cannot be moved that far")))
}

/// One replacement, keyed by the statement it rewrites.
struct Replacement {
    intent: EditIntent,
    before: String,
    after: String,
}

/// What one statement would be rewritten to.
fn replacement(intent: &GroupIntent, written: &WrittenStatement) -> Result<Replacement, ProjectError> {
    if intent.rhythmic() {
        let after = rewritten_duration(intent, written)?;
        return Ok(Replacement {
            intent: EditIntent::SetDuration {
                at: written.at,
                duration: after.clone(),
            },
            before: written.duration.clone().unwrap_or_default(),
            after,
        });
    }
    let pitches = rewritten_pitches(intent, written)?;
    Ok(Replacement {
        before: written.pitches.join(" "),
        after: pitches.join(" "),
        intent: EditIntent::SetPitches {
            at: written.at,
            pitches,
        },
    })
}

/// Resolve the selection down to the statements that spell it.
///
/// Two selected events that came from one motif note resolve to one statement,
/// and the transformation is applied to that statement once: applying it per
/// event is how a group edit moves a note by four steps because the composer
/// selected four occurrences of it.
fn resolve<'a>(
    facts: &'a ScoreFacts,
    source: &str,
    edit: &GroupEdit,
) -> Result<(Vec<Resolved<'a>>, Vec<String>), ProjectError> {
    let selected = ordered(facts, &edit.events)?;
    if selected.is_empty() {
        return Err(uneditable("nothing is selected"));
    }
    let (applicable, inapplicable): (Vec<&EventFacts>, Vec<&EventFacts>) = selected
        .into_iter()
        .partition(|event| applies_to(&edit.intent, event.kind));
    if applicable.is_empty() {
        return Err(uneditable(
            "nothing in the selection has a pitch to move; a rest is silence, not a low note",
        ));
    }
    let offsets: Vec<u32> = applicable.iter().map(|event| span_of(event).start).collect();
    let written = musa_syntax::read_statements(source, &offsets);
    let mut resolved = Vec::with_capacity(applicable.len());
    for (event, written) in applicable.into_iter().zip(written) {
        let written = written.ok_or_else(|| {
            uneditable(format!(
                "the source of `{}` is not a note, rest, or chord statement",
                event.id
            ))
        })?;
        if written.kind == WrittenKind::Other {
            return Err(uneditable(
                "this music is written as a chord stack, whose members are named by a chord type rather \
                 than spelled; change the stack's root or type instead",
            ));
        }
        resolved.push(Resolved {
            event,
            definition: span_of(event),
            written,
        });
    }
    Ok((
        resolved,
        inapplicable.into_iter().map(|event| event.id.clone()).collect(),
    ))
}

/// The occurrences and events one definition span accounts for, across the
/// whole score rather than across the selection.
fn consequences(facts: &ScoreFacts, definition: Span) -> (Vec<String>, Vec<String>, Option<String>) {
    let spelled: Vec<&EventFacts> = facts
        .events
        .iter()
        .filter(|event| event.origin.definition_span == definition)
        .collect();
    let mut occurrences: Vec<String> = spelled
        .iter()
        .filter_map(|event| event.origin.occurrence.as_deref())
        .filter_map(|id| {
            facts
                .occurrences
                .iter()
                .find(|occurrence| occurrence.id == id)
                .map(|occurrence| occurrence.label.clone())
        })
        .collect();
    occurrences.sort_unstable();
    occurrences.dedup();
    let motif = spelled
        .first()
        .and_then(|event| event.origin.occurrence.as_deref())
        .and_then(|id| facts.occurrences.iter().find(|occurrence| occurrence.id == id))
        .and_then(|occurrence| occurrence.motif.clone());
    (
        occurrences,
        spelled.iter().map(|event| event.id.clone()).collect(),
        motif,
    )
}

/// The definitions and the edit intents one group command resolves to.
///
/// Returned together because they are one answer: the intents are what gets
/// written, the definitions are what the composer is told, and they are
/// derived from the same de-duplicated map so the two cannot disagree.
pub(crate) struct Resolution {
    pub(crate) intents: Vec<EditIntent>,
    pub(crate) definitions: Vec<GroupDefinition>,
    pub(crate) unchanged: Vec<String>,
    pub(crate) changed: Vec<String>,
    pub(crate) specializable: bool,
    pub(crate) summary: String,
}

pub(crate) fn resolution(facts: &ScoreFacts, source: &str, edit: &GroupEdit) -> Result<Resolution, ProjectError> {
    let (resolved, unchanged) = resolve(facts, source, edit)?;
    if edit.mode == GeneratedEditMode::Specialize {
        return specialization(facts, &resolved, edit, unchanged);
    }

    // The de-duplication the whole operation turns on: one entry per unique
    // statement, keyed by where it is written.
    let mut by_definition: BTreeMap<Span, (Replacement, Vec<String>)> = BTreeMap::new();
    for item in &resolved {
        let computed = replacement(&edit.intent, &item.written)?;
        match by_definition.get_mut(&item.definition) {
            // Two events of one statement must ask for the same text. They do,
            // because the transformation reads the statement rather than the
            // events; checking it is what makes that a proved property rather
            // than an assumption.
            Some((existing, selected)) => {
                if existing.after != computed.after {
                    return Err(uneditable(format!(
                        "two selected notes come from the same statement and ask it to become \
                         `{}` and `{}`; they cannot both be written",
                        existing.after, computed.after
                    )));
                }
                selected.push(item.event.id.clone());
            }
            None => {
                by_definition.insert(item.definition, (computed, vec![item.event.id.clone()]));
            }
        }
    }

    let mut intents = Vec::with_capacity(by_definition.len());
    let mut definitions = Vec::with_capacity(by_definition.len());
    let mut changed: Vec<String> = Vec::new();
    for (definition, (computed, selected)) in by_definition {
        let (occurrences, events, motif) = consequences(facts, definition);
        changed.extend(events.iter().cloned());
        intents.push(computed.intent);
        definitions.push(GroupDefinition {
            definition,
            motif,
            occurrences,
            selected,
            events,
            before: computed.before,
            after: computed.after,
        });
    }
    changed.sort_unstable();
    changed.dedup();
    let changed = in_score_order(facts, &changed);

    Ok(Resolution {
        specializable: !edit.intent.rhythmic() && specializable(facts, &resolved).is_ok(),
        summary: edit.intent.describe(),
        intents,
        definitions,
        unchanged,
        changed,
    })
}

/// Put a set of event ids back into the order the core listed them in.
fn in_score_order(facts: &ScoreFacts, ids: &[String]) -> Vec<String> {
    facts
        .events
        .iter()
        .filter(|event| ids.binary_search(&event.id).is_ok())
        .map(|event| event.id.clone())
        .collect()
}

/// Whether this whole selection could be written onto its calls as overrides.
///
/// Every condition is one the single-note path already imposes (roadmap §9),
/// asked of the selection as a whole rather than of one note: an override
/// respells, so it is pitch only; it belongs to a call, so the call must run
/// once; and it names a note by its position, so a chord has no place to put
/// one. A selection that fails any of them is not offered the choice, because
/// a batch that specialized what it could and rewrote the definition for the
/// rest would be exactly the partial application §4 forbids.
fn specializable(facts: &ScoreFacts, resolved: &[Resolved<'_>]) -> Result<Vec<(u32, u32)>, ProjectError> {
    let mut sites = Vec::with_capacity(resolved.len());
    for item in resolved {
        if !item.event.origin.generated {
            return Err(uneditable(
                "some of this selection is music you wrote, which has no call to write an override onto",
            ));
        }
        if item.event.kind == EventKind::Chord {
            return Err(uneditable(
                "an override respells one note, and a chord is one statement of several; edit the motif",
            ));
        }
        let id = item
            .event
            .origin
            .occurrence
            .as_deref()
            .ok_or_else(|| uneditable("this note has no occurrence to specialize"))?;
        let occurrence = facts
            .occurrences
            .iter()
            .find(|occurrence| occurrence.id == id)
            .ok_or_else(|| uneditable("this note has no occurrence to specialize"))?;
        if facts
            .occurrences
            .iter()
            .filter(|other| other.use_site == occurrence.use_site)
            .count()
            > 1
        {
            return Err(uneditable(
                "one of these calls runs more than once, so an override would change every run; \
                 edit the motif or unroll the repeat",
            ));
        }
        let position = item
            .event
            .origin
            .note_index
            .ok_or_else(|| uneditable("this note has no position in its occurrence"))?;
        sites.push((occurrence.use_site.start, position));
    }
    Ok(sites)
}

/// Resolve a group command as overrides on the calls rather than as a rewrite
/// of the definitions.
///
/// The pitch written into an override is the *definition's*, transformed — not
/// the one the page shows. `respelled` replaces the note inside the material
/// and whatever transposition the call sits under applies afterwards, so
/// writing the sounding spelling there would transpose it twice.
fn specialization(
    facts: &ScoreFacts,
    resolved: &[Resolved<'_>],
    edit: &GroupEdit,
    unchanged: Vec<String>,
) -> Result<Resolution, ProjectError> {
    if edit.intent.rhythmic() {
        return Err(uneditable(
            "an override respells a note; to renotate one, edit the motif",
        ));
    }
    let sites = specializable(facts, resolved)?;
    // One call, one `with` clause, however many of its notes were selected:
    // several insertions into one clause would collide, and which of them won
    // would depend on the order the selection arrived in.
    let mut by_call: BTreeMap<u32, Vec<(u32, String)>> = BTreeMap::new();
    let mut definitions = Vec::with_capacity(resolved.len());
    let mut changed = Vec::with_capacity(resolved.len());
    for (item, (use_site, position)) in resolved.iter().zip(sites) {
        let after = rewritten_pitches(&edit.intent, &item.written)?
            .first()
            .cloned()
            .ok_or_else(|| uneditable("this note has no written pitch to respell"))?;
        by_call.entry(use_site).or_default().push((position, after.clone()));
        changed.push(item.event.id.clone());
        definitions.push(GroupDefinition {
            definition: item.definition,
            motif: None,
            occurrences: Vec::new(),
            selected: vec![item.event.id.clone()],
            events: vec![item.event.id.clone()],
            before: item.written.pitches.join(" "),
            after,
        });
    }
    let intents = by_call
        .into_iter()
        .map(|(at, mut overrides)| {
            overrides.sort_unstable_by_key(|(position, _)| *position);
            EditIntent::SpecializeAll { at, overrides }
        })
        .collect();
    Ok(Resolution {
        intents,
        definitions,
        unchanged,
        changed,
        specializable: true,
        summary: edit.intent.describe(),
    })
}

/// Two exact durations added, or nothing when 64 bits will not hold the
/// answer. `num-rational`'s own `+` panics on overflow, and a bar total is
/// not worth a panic.
fn sum(left: Ratio<i64>, right: Ratio<i64>) -> Option<Ratio<i64>> {
    let denominator = left.denom().checked_mul(*right.denom())?;
    let numerator = left
        .numer()
        .checked_mul(*right.denom())?
        .checked_add(right.numer().checked_mul(*left.denom())?)?;
    Some(Ratio::new(numerator, denominator))
}

/// Which bars hold something different between two compiles of one piece.
pub(crate) fn bar_effects(before: &ScoreFacts, after: &ScoreFacts) -> Vec<GroupBarEffect> {
    fn totals(facts: &ScoreFacts) -> BTreeMap<(String, String, u32), Ratio<i64>> {
        let mut totals = BTreeMap::new();
        for event in &facts.events {
            let key = (event.part.clone(), event.voice.clone(), event.bar);
            let value = Ratio::new(event.duration.numerator, event.duration.denominator.max(1));
            let running = totals.entry(key).or_insert_with(Ratio::default);
            // Kept rather than wrapped: a bar whose written content exceeded a
            // 64-bit rational is not a number this reports wrongly.
            *running = sum(*running, value).unwrap_or(*running);
        }
        totals
    }
    let (before, after) = (totals(before), totals(after));
    let mut keys: Vec<&(String, String, u32)> = before.keys().chain(after.keys()).collect();
    keys.sort_unstable();
    keys.dedup();
    keys.into_iter()
        .filter_map(|key| {
            let start = before.get(key).copied().unwrap_or_default();
            let end = after.get(key).copied().unwrap_or_default();
            (start != end).then(|| GroupBarEffect {
                part: key.0.clone(),
                voice: key.1.clone(),
                bar: key.2,
                before: Fraction {
                    numerator: *start.numer(),
                    denominator: *start.denom(),
                },
                after: Fraction {
                    numerator: *end.numer(),
                    denominator: *end.denom(),
                },
            })
        })
        .collect()
}
