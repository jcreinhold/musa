//! Structured score edits: musical intent in, text edits out (roadmap §11).
//!
//! An [`EditCommand`] names what a composer did — respell this note, write a
//! note here, lift these bars into a motif — in the same vocabulary the
//! snapshot uses, so a frontend can issue one from what it is already
//! showing. Turning that into text is this module's job, and it is the whole
//! of the "resolve provenance → authored or generated → compute edits" half
//! of roadmap §14.6; the session performs the transaction.
//!
//! The provenance step is the interesting one. An event's origin carries two
//! spans: where it *came from* (a `use sigh();` call site, for generated
//! music) and the statement that *spells* it (the note inside the motif
//! body). Editing a generated note means rewriting the second, which is why
//! doing so changes every occurrence — and why the interface must say so
//! before it happens, which is what [`EditImpact`] is for.

use musa_language::{Anchor, EditIntent, Statement};

use crate::error::ProjectError;
use crate::facts::{EventFacts, ScoreFacts};

/// What to do when the edited event was produced by an expansion
/// (roadmap §9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratedEditMode {
    /// Rewrite the motif's definition. Every occurrence of it changes, and
    /// [`EditImpact`] says how many before the edit is made.
    EditDefinition,
    /// Give this occurrence its own copy and change only that. Requires
    /// occurrence specialization (`use sigh() with { … }`), which arrives in
    /// prompt 34; until then this is refused with
    /// [`ProjectError::NotYetImplemented`] rather than quietly doing the
    /// other thing.
    Specialize,
}

/// A statement to write, in the session's vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoteSpec {
    /// A note: a written pitch and a notated duration, both as spelled.
    Note {
        /// Written pitch (`gs4`).
        pitch: String,
        /// Notated duration (`1/8`).
        duration: String,
    },
    /// A rest.
    Rest {
        /// Notated duration.
        duration: String,
    },
    /// A chord.
    Chord {
        /// Written pitches, as entered.
        pitches: Vec<String>,
        /// Notated duration.
        duration: String,
    },
}

impl NoteSpec {
    fn to_statement(&self) -> Statement {
        match *self {
            Self::Note {
                ref pitch,
                ref duration,
            } => Statement::Note {
                pitch: pitch.clone(),
                duration: duration.clone(),
            },
            Self::Rest { ref duration } => Statement::Rest {
                duration: duration.clone(),
            },
            Self::Chord {
                ref pitches,
                ref duration,
            } => Statement::Chord {
                pitches: pitches.clone(),
                duration: duration.clone(),
            },
        }
    }
}

/// Where a new statement goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InsertAt {
    /// Before the statement that produced this event.
    Before {
        /// Event id from the snapshot.
        event: String,
    },
    /// After the statement that produced this event. For a generated event
    /// that is after the whole `use`, because that is the statement the
    /// composer can see at that place in the score.
    After {
        /// Event id from the snapshot.
        event: String,
    },
    /// At the end of a voice — where entry lands when nothing is selected.
    EndOfVoice {
        /// The part's name.
        part: String,
        /// The voice's name.
        voice: String,
    },
}

/// A structured score edit (roadmap §11).
///
/// Deviation from the roadmap's sketch, deliberately: `ChangeDuration`
/// carries a [`GeneratedEditMode`] too. Renotating a generated note has
/// exactly the consequence respelling one does, and §9 forbids making that
/// choice on the composer's behalf in either case.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditCommand {
    /// Write a note, rest, or chord into a voice.
    InsertNote {
        /// Where it goes.
        at: InsertAt,
        /// What to write.
        note: NoteSpec,
    },
    /// Respell an event's pitch.
    ChangePitch {
        /// Event id from the snapshot.
        event: String,
        /// The new written pitch.
        pitch: String,
        /// What to do if the event was generated.
        mode: GeneratedEditMode,
    },
    /// Renotate an event's duration.
    ChangeDuration {
        /// Event id from the snapshot.
        event: String,
        /// The new notated duration.
        duration: String,
        /// What to do if the event was generated.
        mode: GeneratedEditMode,
    },
    /// Lift the statements behind these events into a new `motif`, leaving a
    /// `use` in their place.
    ExtractMotif {
        /// Event ids; they must be one contiguous run of one voice.
        events: Vec<String>,
        /// The motif's name.
        name: String,
    },
}

/// What an edit would change, stated the way `04-provenance.md` §4 states it.
///
/// Computed before the edit is applied, so the interface can show the choice
/// with real counts and halo the affected notes. The frontend derives none of
/// this: it is a fact about expansion, and expansion happens here.
///
/// It travels to the frontend as `musa-project`'s own serialization, like the
/// snapshot does, so there is one wire shape for it and not two.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditImpact {
    /// Whether the edited event was produced by an expansion. False means
    /// there is no choice to offer.
    pub generated: bool,
    /// The motif whose definition an `EditDefinition` would rewrite.
    pub motif: Option<String>,
    /// The occurrence the edited event belongs to, labelled as the inspector
    /// labels it (`transpose down P5 ▸ sigh()`).
    pub occurrence: Option<String>,
    /// How many occurrences would change — the *2 occurrences* of the
    /// inline choice.
    pub occurrences: u32,
    /// Every event that would change, so the interface can halo them without
    /// working out which ones they are.
    pub events: Vec<String>,
}

impl EditImpact {
    /// An edit that changes only what was pointed at.
    fn authored(event: &str) -> Self {
        Self {
            generated: false,
            motif: None,
            occurrence: None,
            occurrences: 0,
            events: vec![event.to_owned()],
        }
    }
}

/// Find an event by the id the snapshot published.
fn event<'a>(facts: &'a ScoreFacts, id: &str) -> Result<&'a EventFacts, ProjectError> {
    facts
        .events
        .iter()
        .find(|event| event.id == id)
        .ok_or_else(|| ProjectError::NoSuchEvent(id.to_owned()))
}

/// The label an occurrence goes by, for reporting.
fn occurrence_label(facts: &ScoreFacts, id: Option<&str>) -> Option<String> {
    let id = id?;
    facts
        .occurrences
        .iter()
        .find(|occurrence| occurrence.id == id)
        .map(|occurrence| occurrence.label.clone())
}

fn motif_of(facts: &ScoreFacts, id: Option<&str>) -> Option<String> {
    let id = id?;
    facts
        .occurrences
        .iter()
        .find(|occurrence| occurrence.id == id)
        .and_then(|occurrence| occurrence.motif.clone())
}

/// What changing the event at `id` would affect.
///
/// The rule is one line long and holds for nesting, repeats, and transposed
/// occurrences alike: an edit rewrites one statement, so it changes exactly
/// the events that statement spelled.
pub(crate) fn impact(facts: &ScoreFacts, id: &str) -> Result<EditImpact, ProjectError> {
    let target = event(facts, id)?;
    if !target.origin.generated {
        return Ok(EditImpact::authored(id));
    }
    let definition = target.origin.definition_span;
    let affected: Vec<&EventFacts> = facts
        .events
        .iter()
        .filter(|event| event.origin.definition_span == definition)
        .collect();
    let mut occurrences: Vec<&str> = affected
        .iter()
        .filter_map(|event| event.origin.occurrence.as_deref())
        .collect();
    occurrences.sort_unstable();
    occurrences.dedup();
    Ok(EditImpact {
        generated: true,
        motif: motif_of(facts, target.origin.occurrence.as_deref()),
        occurrence: occurrence_label(facts, target.origin.occurrence.as_deref()),
        occurrences: u32::try_from(occurrences.len()).unwrap_or(u32::MAX),
        events: affected.iter().map(|event| event.id.clone()).collect(),
    })
}

/// The impact a command would have, or a trivial one for commands that
/// change nothing that already exists.
pub(crate) fn impact_of(facts: &ScoreFacts, command: &EditCommand) -> Result<EditImpact, ProjectError> {
    match *command {
        EditCommand::ChangePitch { ref event, .. } | EditCommand::ChangeDuration { ref event, .. } => {
            impact(facts, event)
        }
        EditCommand::InsertNote { .. } => Ok(EditImpact {
            generated: false,
            motif: None,
            occurrence: None,
            occurrences: 0,
            events: Vec::new(),
        }),
        EditCommand::ExtractMotif { ref events, .. } => Ok(EditImpact {
            generated: false,
            motif: None,
            occurrence: None,
            occurrences: 0,
            events: events.clone(),
        }),
    }
}

/// The statement an edit to this event rewrites.
///
/// For authored music this is the note the composer typed; for generated
/// music it is the note inside the `motif` body, which is why the edit
/// changes every occurrence.
fn definition_offset(target: &EventFacts) -> u32 {
    target.origin.definition_span.start
}

/// The statement that stands for this event *in the score* — the `use` for
/// generated music. Insertion and extraction work here, because this is the
/// text the composer sees at that place in the piece.
fn site_offset(target: &EventFacts) -> u32 {
    target.origin.span.start
}

/// Refuse a specialization cleanly rather than silently editing the
/// definition instead (`04-provenance.md` §4).
fn check_mode(generated: bool, mode: GeneratedEditMode) -> Result<(), ProjectError> {
    match mode {
        GeneratedEditMode::EditDefinition => Ok(()),
        GeneratedEditMode::Specialize if !generated => Ok(()),
        GeneratedEditMode::Specialize => Err(ProjectError::NotYetImplemented {
            feature: "occurrence specialization (`use sigh() with { … }`)",
        }),
    }
}

/// Turn a command into the language's edit intent.
///
/// # Errors
/// [`ProjectError::NoSuchEvent`] for an id this revision does not have,
/// [`ProjectError::NotYetImplemented`] for a specialization, and
/// [`ProjectError::Uneditable`] when the command does not describe something
/// the source can express.
pub(crate) fn intent_of(facts: &ScoreFacts, command: &EditCommand) -> Result<EditIntent, ProjectError> {
    match *command {
        EditCommand::ChangePitch {
            event: ref id,
            ref pitch,
            mode,
        } => {
            let target = event(facts, id)?;
            check_mode(target.origin.generated, mode)?;
            Ok(EditIntent::SetPitch {
                at: definition_offset(target),
                pitch: pitch.clone(),
            })
        }
        EditCommand::ChangeDuration {
            event: ref id,
            ref duration,
            mode,
        } => {
            let target = event(facts, id)?;
            check_mode(target.origin.generated, mode)?;
            Ok(EditIntent::SetDuration {
                at: definition_offset(target),
                duration: duration.clone(),
            })
        }
        EditCommand::InsertNote { ref at, ref note } => {
            let anchor = match *at {
                InsertAt::Before { event: ref id } => Anchor::Before {
                    at: site_offset(event(facts, id)?),
                },
                InsertAt::After { event: ref id } => Anchor::After {
                    at: site_offset(event(facts, id)?),
                },
                InsertAt::EndOfVoice { ref part, ref voice } => Anchor::EndOfVoice {
                    part: part.clone(),
                    voice: voice.clone(),
                },
            };
            Ok(EditIntent::Insert {
                anchor,
                statement: note.to_statement(),
            })
        }
        EditCommand::ExtractMotif { ref events, ref name } => {
            if events.is_empty() {
                return Err(ProjectError::Uneditable("nothing is selected to extract".to_owned()));
            }
            let mut offsets = Vec::with_capacity(events.len());
            for id in events {
                let target = event(facts, id)?;
                if target.origin.generated {
                    // Extracting a motif out of an expansion of a motif is a
                    // refactor of the definition, not of the score, and the
                    // composer would not be looking at what changed.
                    return Err(ProjectError::Uneditable(
                        "this music is already generated; edit its motif instead".to_owned(),
                    ));
                }
                offsets.push(site_offset(target));
            }
            let first = offsets.iter().copied().min().unwrap_or_default();
            let last = offsets.iter().copied().max().unwrap_or_default();
            Ok(EditIntent::ExtractMotif {
                first,
                last,
                name: name.clone(),
            })
        }
    }
}

/// How a failed command names itself in [`ProjectError::RejectedEdit`].
pub(crate) fn describe(command: &EditCommand) -> String {
    match *command {
        EditCommand::InsertNote { .. } => "inserting a note".to_owned(),
        EditCommand::ChangePitch { ref pitch, .. } => format!("changing the pitch to {pitch}"),
        EditCommand::ChangeDuration { ref duration, .. } => format!("changing the duration to {duration}"),
        EditCommand::ExtractMotif { ref name, .. } => format!("extracting the motif {name}"),
    }
}
