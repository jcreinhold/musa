//! One checked piece, as a finite schedule a host reads by position.
//!
//! [`crate::hosted`] is the other half of the boundary: an instrument, driven
//! by MIDI a host decides. This is the piece itself, projected as MIDI for a
//! host to place on its own timeline — Apple's MIDI Processor, Logic's MIDI FX
//! slot. `06-daw-boundary.md` §3 gives the host the transport, so what a
//! processor owns is not a cursor but a *finite, immutable, randomly
//! accessible* schedule: the host says where it is, and the schedule says what
//! sounds there.
//!
//! That distinction is the whole design. A forward-only sequencer has to be
//! replayed from the beginning after a seek, and a host seeks constantly —
//! cycling, scrubbing, bouncing a range. Every query here is a binary search
//! into an immutable array, so seeking to bar 200 costs what seeking to bar 1
//! costs, and looping is not a special case of anything.
//!
//! # What is decided where
//!
//! Nothing musical is decided here. [`musa_notation::midi_schedule`] is the
//! one path that turns a performance into MIDI, and it is the same path the
//! Standard MIDI File writer and the live `CoreMIDI` projection consume. This
//! module re-indexes that schedule for random access and converts its exact
//! rational instants into the one float a host callback can compare against —
//! once, on the control side, which is where `AGENTS.md` allows a float.

use std::path::PathBuf;

use musa_notation::{MidiMode, MidiOptions, MidiPart, midi_schedule};
use num_rational::Ratio;

use crate::error::ProjectError;

/// Which timeline a schedule's positions are counted on.
///
/// Never inferred. `06-daw-boundary.md` §6 forbids a silent flattening, and a
/// component that guessed between these two would be silently choosing
/// whether the host's tempo changes what the piece is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostedTimeline {
    /// The piece's own exact physical schedule. Positions are seconds from
    /// the start of the performance, and the host's tempo map is not
    /// consulted: the piece plays at the speed its source says it does.
    Piece,
    /// The host's musical timeline. Positions are quarter notes from the
    /// start, and the host's tempo decides when each one sounds — which is
    /// what makes the piece follow a tempo change the host makes, and what
    /// makes a polytempo piece impossible to place.
    Host,
}

impl HostedTimeline {
    /// The word a saved document writes, and reads back.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Piece => "piece",
            Self::Host => "host",
        }
    }

    /// The timeline that word names, or `None` for a word this version does
    /// not know — which is a refusal, never a default.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "piece" => Some(Self::Piece),
            "host" => Some(Self::Host),
            _ => None,
        }
    }

    /// What one unit of this timeline is called, for a report that has to say
    /// what a number means.
    #[must_use]
    pub const fn unit(self) -> &'static str {
        match self {
            Self::Piece => "seconds",
            Self::Host => "quarter notes",
        }
    }
}

/// What a host asked to be scheduled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedScheduleRequest {
    /// A `.musa` piece, or the project directory holding it.
    pub project: PathBuf,
    /// Which piece of that project. `None` takes the project's first.
    pub piece: Option<String>,
    /// The written reading or the played one.
    pub mode: MidiMode,
    /// Which timeline the positions are counted on.
    pub timeline: HostedTimeline,
}

/// What a restored document must name for a schedule to be the same one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedScheduleIdentity {
    /// The compiled music's semantic identity.
    pub music: String,
    /// The verified asset closure's identity, hex-encoded.
    pub assets: String,
    /// The piece within the project.
    pub piece: String,
}

/// One channel-voice message, at the position it sounds.
///
/// The status byte already carries its channel, exactly as the shared
/// schedule decided it: a consumer that re-derived the channel from the part
/// index would be making a second channel decision.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HostedScheduleEvent {
    /// Where it sounds, on [`HostedSchedule::timeline`]'s units.
    pub position: f64,
    /// Which part, indexed into [`HostedSchedule::parts`].
    pub part: u32,
    /// The status byte with its channel in the low nibble.
    pub status: u8,
    /// The one or two data bytes the status takes.
    pub data: [u8; 2],
}

/// One note that is already sounding at some position.
///
/// What a host needs after a seek or a loop wrap: the notes whose attack is
/// behind the play head and whose release is still ahead of it. Without this
/// a processor that seeks into the middle of a held chord emits the releases
/// and nothing else, and the chord is silent for the rest of the piece.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HostedActiveNote {
    /// Where the attack was.
    pub start: f64,
    /// Where the release is.
    pub end: f64,
    /// Which part, indexed into [`HostedSchedule::parts`].
    pub part: u32,
    /// The note-on status byte, channel included.
    pub status: u8,
    /// The key number and the velocity it was struck with.
    pub note: u8,
    pub velocity: u8,
}

/// Every MIDI fact about one piece, indexed for random access by position.
///
/// Immutable once built. Every query is a binary search and a bounded read,
/// so a render callback may make them: none of them allocates, takes a lock,
/// or depends on where the last query was.
#[derive(Clone, Debug)]
pub struct HostedSchedule {
    identity: HostedScheduleIdentity,
    timeline: HostedTimeline,
    mode: MidiMode,
    parts: Vec<MidiPart>,
    events: Vec<HostedScheduleEvent>,
    losses: Vec<String>,
    /// Sounding notes, sorted by where they start.
    spans: Vec<HostedActiveNote>,
    /// `prefix_max_end[i]` is the latest end among `spans[0..=i]`, so it is
    /// non-decreasing and can be binary-searched. It is what turns "which
    /// notes have finished by here" from a scan into a lookup.
    prefix_max_end: Vec<f64>,
    extent: f64,
}

impl HostedSchedule {
    /// What this schedule was made from.
    #[must_use]
    pub const fn identity(&self) -> &HostedScheduleIdentity {
        &self.identity
    }

    /// Which timeline [`HostedScheduleEvent::position`] is counted on.
    #[must_use]
    pub const fn timeline(&self) -> HostedTimeline {
        self.timeline
    }

    /// The written reading or the played one.
    #[must_use]
    pub const fn mode(&self) -> MidiMode {
        self.mode
    }

    /// Every sounding part, in track order, with the channel it was given.
    #[must_use]
    pub fn parts(&self) -> &[MidiPart] {
        &self.parts
    }

    /// Every message, in the order it sounds.
    #[must_use]
    pub fn events(&self) -> &[HostedScheduleEvent] {
        &self.events
    }

    /// What this projection could not carry, each already a sentence.
    #[must_use]
    pub fn losses(&self) -> &[String] {
        &self.losses
    }

    /// Where the last message sounds.
    #[must_use]
    pub const fn extent(&self) -> f64 {
        self.extent
    }

    /// The first message at or after `position`.
    ///
    /// A host's render interval is half-open: a message exactly at the start
    /// of the block belongs to this block, and one exactly at the end belongs
    /// to the next. `lower_bound(start)..lower_bound(end)` is that interval,
    /// and it is why the same frame is never emitted twice at a block seam.
    #[must_use]
    pub fn lower_bound(&self, position: f64) -> usize {
        self.events.partition_point(|event| event.position < position)
    }

    /// The notes already sounding at `position`, written into `into`.
    ///
    /// Returns how many notes qualified, which may be more than `into` could
    /// hold — a caller that gets a number larger than its buffer has been
    /// told it was truncated rather than left to believe a chord was thinner
    /// than it is.
    ///
    /// A note starting exactly at `position` is *not* here: it is in
    /// [`Self::lower_bound`]'s own range, and counting it twice would double
    /// the attack.
    ///
    /// Real-time safe. Two binary searches bracket the scan, and what is
    /// scanned is the notes that began after the earliest one still sounding
    /// — never the piece.
    pub fn active(&self, position: f64, into: &mut [HostedActiveNote]) -> usize {
        let low = self.prefix_max_end.partition_point(|&end| end <= position);
        let high = self.spans.partition_point(|span| span.start < position);
        let mut found = 0usize;
        let Some(window) = self.spans.get(low..high) else {
            return 0;
        };
        for span in window {
            if span.end > position {
                if let Some(slot) = into.get_mut(found) {
                    *slot = *span;
                }
                found = found.saturating_add(1);
            }
        }
        found
    }

    /// The first span this schedule would have to look at for `position`.
    ///
    /// Published so that a measurement can state the difference the index
    /// makes: seeking into the middle of a piece starts here, not at zero.
    #[must_use]
    pub fn active_scan_start(&self, position: f64) -> usize {
        self.prefix_max_end.partition_point(|&end| end <= position)
    }
}

/// Compile one piece and index its MIDI for a host to read by position.
///
/// # Errors
/// [`ProjectError::NoValidScore`] if the source does not compile or its assets
/// are not verified, [`ProjectError::Performance`] if the gestures do not
/// lower or the chosen timeline cannot carry the piece, and
/// [`ProjectError::Notation`] if the MIDI schedule cannot be decided.
pub fn open_hosted_schedule(request: &HostedScheduleRequest) -> Result<HostedSchedule, ProjectError> {
    let mut project = crate::Project::open(&request.project)?;
    if let Some(piece) = request.piece.as_deref() {
        project.show(piece)?;
    }
    let piece = project.showing().to_owned();
    let session = project.current();
    let valid = session.hosted_artifacts().ok_or(ProjectError::NoValidScore)?;
    let performance =
        musa_compiler::lower_gestures(&valid.score).map_err(|error| ProjectError::Performance(error.to_string()))?;
    let options = MidiOptions {
        mode: request.mode,
        ..MidiOptions::default()
    };
    let schedule = midi_schedule(&performance, &options).map_err(|error| ProjectError::Notation(error.to_string()))?;

    // §6: a loss is refused or recorded. This one is a refusal, because a
    // polytempo piece has no single quarter-note grid to place on a host's,
    // and placing it on one anyway would silently pick a tempo.
    let polytempo = schedule.losses().iter().any(|loss| loss.kind() == "polytempo");
    if polytempo && request.timeline == HostedTimeline::Host {
        return Err(ProjectError::Performance(
            "this piece is polytempo: its parts play at their own speeds, so there is no one \
             quarter-note grid to lay on the host's. Play it on the piece's own timeline, where \
             every message keeps the second it sounds at."
                .to_owned(),
        ));
    }

    let ticks_per_quarter = f64::from(schedule.ticks_per_quarter());
    let place = |seconds: Ratio<i64>, tick: u64| -> f64 {
        match request.timeline {
            HostedTimeline::Piece => ratio(seconds),
            HostedTimeline::Host => tick as f64 / ticks_per_quarter,
        }
    };
    let events: Vec<HostedScheduleEvent> = schedule
        .messages()
        .iter()
        .map(|message| HostedScheduleEvent {
            position: place(message.seconds, message.tick),
            part: u32::try_from(message.part).unwrap_or(u32::MAX),
            status: message.status,
            data: message.data,
        })
        .collect();
    let extent = events.last().map_or(0.0, |event| event.position);
    let (spans, prefix_max_end) = index_spans(&events, extent);

    let mut losses: Vec<String> = schedule
        .losses()
        .iter()
        .map(|loss| format!("{}: {}", loss.kind(), loss.message()))
        .collect();
    // What a MIDI Processor cannot say, said once rather than discovered.
    losses.push(
        "conductor: a MIDI effect emits channel messages, so the piece's tempo, meter, and key \
         are the host's to set and are not in this projection"
            .to_owned(),
    );
    if request.timeline == HostedTimeline::Host {
        losses.push(
            "tempo: on the host's timeline the host's tempo decides when each quarter sounds, so \
             the piece's own tempo map is a proportion here rather than a speed"
                .to_owned(),
        );
    }

    Ok(HostedSchedule {
        identity: HostedScheduleIdentity {
            music: valid.identity.to_string(),
            assets: crate::hosted::hex(&session.hosted_assets().identity()),
            piece,
        },
        timeline: request.timeline,
        mode: request.mode,
        parts: schedule.parts().to_vec(),
        events,
        losses,
        spans,
        prefix_max_end,
        extent,
    })
}

/// An exact rational instant as the one float a host callback can compare.
fn ratio(value: Ratio<i64>) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "the exact value stays in `musa-notation`; this is the host edge, where a \
                  sample time is a double and nothing downstream is exact anyway"
    )]
    {
        *value.numer() as f64 / *value.denom() as f64
    }
}

/// Pair every attack with its release, and build the index a seek reads.
fn index_spans(events: &[HostedScheduleEvent], extent: f64) -> (Vec<HostedActiveNote>, Vec<f64>) {
    let mut pending: Vec<(u8, u8, HostedActiveNote)> = Vec::new();
    let mut spans: Vec<HostedActiveNote> = Vec::new();
    for event in events {
        let kind = event.status & 0xF0;
        let channel = event.status & 0x0F;
        let note = event.data[0];
        if kind == 0x90 && event.data[1] > 0 {
            pending.push((
                channel,
                note,
                HostedActiveNote {
                    start: event.position,
                    end: extent,
                    part: event.part,
                    status: event.status,
                    note,
                    velocity: event.data[1],
                },
            ));
        } else if kind == 0x80 || kind == 0x90 {
            // The most recent unreleased attack of this key on this channel,
            // which is how a keyboard's own re-strike pairs and how the
            // shared schedule emits them.
            if let Some(index) = pending
                .iter()
                .rposition(|(pending_channel, pending_note, _)| *pending_channel == channel && *pending_note == note)
            {
                let (_, _, mut span) = pending.remove(index);
                span.end = event.position;
                spans.push(span);
            }
        }
    }
    // An attack with no release ends where the piece does. It is a schedule
    // fault rather than a host one, and truncating it here is what keeps a
    // seek past the end from reporting a note that never stops.
    spans.extend(pending.into_iter().map(|(_, _, span)| span));
    spans.sort_by(|left, right| left.start.total_cmp(&right.start));
    let mut prefix_max_end = Vec::with_capacity(spans.len());
    let mut highest = f64::NEG_INFINITY;
    for span in &spans {
        highest = highest.max(span.end);
        prefix_max_end.push(highest);
    }
    (spans, prefix_max_end)
}
