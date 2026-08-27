//! Completed-note pairing and adaptive onset groups (prompt 205).
//!
//! Raw capture keeps note-ons, note-offs, and controller transitions as edge
//! facts; this module turns them into the exact intervals a transcription
//! search reads. A completed note separates three times that MIDI slurs
//! together: the **onset** (note-on), the **physical key interval** (note-off),
//! and the **pedal-extended sounding interval** (sustain pedal release). It
//! keeps both velocities and the raw event ids it derived from, and it never
//! invents an interval for an unpaired edge; those are declared losses.
//!
//! The onset-group clustering below reuses the one measured decision prompt 203
//! fixed: a group window of one twelfth of the local beat, clamped to
//! 18–70 ms, as a candidate cost rather than a hard split.

// Every slice read here is index by an id derived from `0..len` or from a
// non-empty group's own member list; none of those can leave the slice.
#![allow(clippy::indexing_slicing)]

use std::collections::{BTreeMap, VecDeque};

use musa_playback::MidiMessageKind;

use crate::midi::CapturedMidiEvent;

/// One pedal or controller span retained beside the notes, never folded into a
/// written end. Sustain and sostenuto are distinct controller records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ControllerSpan {
    /// MIDI channel, 0–15.
    pub channel: u8,
    /// Controller number (64 sustain, 66 sostenuto, ...).
    pub controller: u8,
    /// The value at press (the high run).
    pub value: u8,
    /// Press time, project micros.
    pub press_micros: u64,
    /// Release time, project micros; the take end when still held.
    pub release_micros: u64,
}

/// A completed note: the exact intervals and evidence a search reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CompletedNote {
    /// MIDI note number, 0–127.
    pub note: u8,
    pub attack_velocity: u8,
    pub release_velocity: u8,
    /// Note-on time, project micros.
    pub onset_micros: u64,
    /// Note-off time (the physical key release), project micros.
    pub key_release_micros: u64,
    /// Pedal-extended sound end, else the key release, project micros.
    pub sounding_end_micros: u64,
    /// The note-on and note-off raw event ids, in that order.
    pub derivation: [u64; 2],
    /// Whether sound extended past the key release under a sustain pedal.
    pub pedal_extended: bool,
}

/// The pairing result over one take: notes, controller spans, and losses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CompletedTake {
    pub notes: Vec<CompletedNote>,
    pub controllers: Vec<ControllerSpan>,
    /// Raw ids of note-ons that never found a note-off.
    pub unpaired_note_ons: Vec<u64>,
    /// Raw ids of note-offs that never found a note-on.
    pub unpaired_note_offs: Vec<u64>,
    /// Notes whose sound was still pedal-held when the capture ended.
    pub held_at_end: usize,
}

/// One note-on waiting for its note-off, in per-(channel, pitch) arrival order.
#[derive(Clone, Copy)]
struct PendingOn {
    onset_event: u64,
    onset_micros: u64,
    attack_velocity: u8,
}

/// A note whose key released while sustain is down, awaiting pedal release.
#[derive(Clone, Copy)]
struct Extended {
    note: u8,
    attack_velocity: u8,
    release_velocity: u8,
    onset_micros: u64,
    key_release_micros: u64,
    derivation: [u64; 2],
}

/// Turn one take's raw events into completed notes. Note-ons pair to note-offs
/// per channel and pitch in arrival order, so a retrigger of a held key is a
/// distinct note rather than a merge. Sustain (CC64) extends the sounding end
/// of a released note to the pedal release; it never rewrites the key release.
pub(crate) fn complete(events: &[CapturedMidiEvent]) -> CompletedTake {
    let mut pending: BTreeMap<(u8, u8), VecDeque<PendingOn>> = BTreeMap::new();
    let mut sustain: [bool; 16] = [false; 16];
    let mut open_pedal: BTreeMap<(u8, u8), (u64, u8)> = BTreeMap::new();
    let mut extended: BTreeMap<u8, Vec<Extended>> = BTreeMap::new();
    let mut result = CompletedTake::default();
    let mut last_micros = 0_u64;

    for event in events {
        last_micros = event.project_micros;
        let raw = event.raw;
        match raw.kind {
            MidiMessageKind::NoteOn => {
                pending
                    .entry((raw.channel, raw.data))
                    .or_default()
                    .push_back(PendingOn {
                        onset_event: event.id,
                        onset_micros: event.project_micros,
                        attack_velocity: u8::try_from(raw.value).unwrap_or(0),
                    });
            }
            MidiMessageKind::NoteOff => {
                let key = (raw.channel, raw.data);
                let Some(press) = pending.get_mut(&key).and_then(VecDeque::pop_front) else {
                    result.unpaired_note_offs.push(event.id);
                    continue;
                };
                let release_velocity = u8::try_from(raw.value).unwrap_or(0);
                if sustain_is_down(&sustain, raw.channel) {
                    extended.entry(raw.channel).or_default().push(Extended {
                        note: raw.data,
                        attack_velocity: press.attack_velocity,
                        release_velocity,
                        onset_micros: press.onset_micros,
                        key_release_micros: event.project_micros,
                        derivation: [press.onset_event, event.id],
                    });
                } else {
                    result.notes.push(CompletedNote {
                        note: raw.data,
                        attack_velocity: press.attack_velocity,
                        release_velocity,
                        onset_micros: press.onset_micros,
                        key_release_micros: event.project_micros,
                        sounding_end_micros: event.project_micros,
                        derivation: [press.onset_event, event.id],
                        pedal_extended: false,
                    });
                }
            }
            MidiMessageKind::ControlChange if raw.data == 64 || raw.data == 66 => {
                let channel = raw.channel;
                let pressed = raw.value >= 64;
                if pressed {
                    if let std::collections::btree_map::Entry::Vacant(slot) = open_pedal.entry((channel, raw.data)) {
                        slot.insert((event.project_micros, u8::try_from(raw.value).unwrap_or(0)));
                    }
                    if raw.data == 64 {
                        set_sustain(&mut sustain, channel, true);
                    }
                } else {
                    if raw.data == 64 {
                        set_sustain(&mut sustain, channel, false);
                        release_extended(&mut result, &mut extended, channel, event.project_micros);
                    }
                    if let Some((press_micros, value)) = open_pedal.remove(&(channel, raw.data)) {
                        result.controllers.push(ControllerSpan {
                            channel,
                            controller: raw.data,
                            value,
                            press_micros,
                            release_micros: event.project_micros,
                        });
                    }
                }
            }
            MidiMessageKind::ControlChange
            | MidiMessageKind::PitchBend
            | MidiMessageKind::ChannelPressure
            | MidiMessageKind::KeyPressure
            | MidiMessageKind::ProgramChange => {}
        }
    }

    // Key-press evidence without a release is a loss, not an interval.
    for (_, queue) in pending {
        for press in queue {
            result.unpaired_note_ons.push(press.onset_event);
        }
    }
    // Pedal still down at capture end: the held notes sound to the last observed
    // time and are reported as held, never given a fabricated key release. Open
    // pedal spans close at the capture end so the controller list names them.
    for ((channel, controller), (press_micros, value)) in open_pedal {
        result.controllers.push(ControllerSpan {
            channel,
            controller,
            value,
            press_micros,
            release_micros: last_micros,
        });
    }
    result
        .controllers
        .sort_by_key(|span| (span.press_micros, span.controller));
    for (_, held) in extended {
        for note in held {
            result.held_at_end = result.held_at_end.saturating_add(1);
            result.notes.push(CompletedNote {
                note: note.note,
                attack_velocity: note.attack_velocity,
                release_velocity: note.release_velocity,
                onset_micros: note.onset_micros,
                key_release_micros: note.key_release_micros,
                sounding_end_micros: last_micros,
                derivation: note.derivation,
                pedal_extended: true,
            });
        }
    }

    result.notes.sort_by_key(|note| (note.onset_micros, note.note));
    result
}

/// Read the sustain state for a channel, treating an out-of-range channel as
/// released rather than panicking on a device value.
fn sustain_is_down(sustain: &[bool; 16], channel: u8) -> bool {
    sustain.get(usize::from(channel)) == Some(&true)
}

/// Write the sustain state for a channel, ignoring an out-of-range channel.
fn set_sustain(sustain: &mut [bool; 16], channel: u8, down: bool) {
    if let Some(slot) = sustain.get_mut(usize::from(channel)) {
        *slot = down;
    }
}

/// Move the pedal-held notes of one channel to the completed list at pedal-off.
fn release_extended(
    result: &mut CompletedTake,
    extended: &mut BTreeMap<u8, Vec<Extended>>,
    channel: u8,
    release_micros: u64,
) {
    if let Some(held) = extended.remove(&channel) {
        for note in held {
            result.notes.push(CompletedNote {
                note: note.note,
                attack_velocity: note.attack_velocity,
                release_velocity: note.release_velocity,
                onset_micros: note.onset_micros,
                key_release_micros: note.key_release_micros,
                sounding_end_micros: release_micros,
                derivation: note.derivation,
                pedal_extended: true,
            });
        }
    }
}

/// The adaptive onset-group window: one twelfth of the local beat, clamped to
/// 18–70 ms. This is the measured starting proposal from prompt 203, not a
/// fixed split.
pub(crate) fn adaptive_window_micros(quarter_micros: u64) -> u64 {
    (quarter_micros / 12).clamp(18_000, 70_000)
}

/// One onset group: the completed-note indices clustered by the adaptive
/// window, plus enough shape facts to enumerate block/rolled/arpeggio
/// alternatives without any source text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OnsetGroup {
    /// Indices into the completed-note list, onset-ascending.
    pub notes: Vec<usize>,
    pub onset_micros: u64,
    /// Largest onset minus smallest onset within the group.
    pub spread_micros: u64,
    /// Whether pitch is strictly monotonic in onset order (an "ordered" spread).
    pub ordered: bool,
}

/// The shapes one group can still describe. `Line` is not a group shape: a fast
/// scale lands in separate single-note groups, so its notes offer no chord
/// reading at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupShape {
    Block,
    Rolled,
    Arpeggio,
}

/// Cluster completed notes into onset groups with the adaptive window.
pub(crate) fn group_notes(notes: &[CompletedNote], quarter_micros: u64) -> Vec<OnsetGroup> {
    let window = adaptive_window_micros(quarter_micros);
    let mut order: Vec<usize> = (0..notes.len()).collect();
    order.sort_by_key(|&index| (notes[index].onset_micros, notes[index].note, index));
    let mut groups: Vec<OnsetGroup> = Vec::new();
    for index in order {
        let onset = notes[index].onset_micros;
        if let Some(group) = groups.last_mut()
            && onset.saturating_sub(group.onset_micros) <= window
        {
            group.notes.push(index);
            group.spread_micros = onset.saturating_sub(notes[group.notes[0]].onset_micros);
            group.ordered = ordered(&group.notes, notes);
        } else {
            groups.push(OnsetGroup {
                notes: vec![index],
                onset_micros: onset,
                spread_micros: 0,
                ordered: true,
            });
        }
    }
    groups
}

/// Whether the group's pitches are strictly monotonic in onset order.
fn ordered(indices: &[usize], notes: &[CompletedNote]) -> bool {
    if indices.len() < 2 {
        return true;
    }
    let pitches = indices.iter().map(|&index| notes[index].note).collect::<Vec<_>>();
    pitches.windows(2).all(|pair| pair[0] < pair[1]) || pitches.windows(2).all(|pair| pair[0] > pair[1])
}

/// The chord-shape readings a multi-note group still offers. A compact
/// simultaneous spread may be a block or a rolled chord; an ordered spread may
/// be read as rolled or arpeggiated; a disordered spread is only arpeggio-ish.
/// A single note offers none of these.
pub(crate) fn shape_alternatives(group: &OnsetGroup) -> Vec<GroupShape> {
    if group.notes.len() < 2 {
        return Vec::new();
    }
    if group.spread_micros == 0 {
        vec![GroupShape::Block, GroupShape::Rolled]
    } else if group.ordered {
        vec![GroupShape::Rolled, GroupShape::Arpeggio]
    } else {
        vec![GroupShape::Arpeggio]
    }
}

#[cfg(test)]
mod laws {
    use super::*;
    use musa_playback::{MidiInputEvent, MidiMessageKind};

    fn raw(kind: MidiMessageKind, channel: u8, data: u8, value: i16, micros: u64) -> MidiInputEvent {
        MidiInputEvent {
            device_micros: micros.saturating_add(10_000),
            callback_micros: micros,
            cable: 0,
            channel,
            kind,
            data,
            value,
        }
    }

    fn event(id: u64, raw: MidiInputEvent) -> CapturedMidiEvent {
        CapturedMidiEvent {
            id,
            raw,
            project_micros: raw.callback_micros,
            calibration: musa_playback::MidiClockCalibrator::default().calibration(),
            voice: None,
            pairing: crate::midi::MidiPairingFact::NotANote,
        }
    }

    #[test]
    fn retrigger_keeps_two_distinct_notes() {
        let events = [
            event(0, raw(MidiMessageKind::NoteOn, 0, 60, 90, 0)),
            event(1, raw(MidiMessageKind::NoteOn, 0, 60, 91, 1_000)),
            event(2, raw(MidiMessageKind::NoteOff, 0, 60, 10, 2_000)),
            event(3, raw(MidiMessageKind::NoteOff, 0, 60, 11, 3_000)),
        ];
        let done = complete(&events);
        assert_eq!(done.notes.len(), 2);
        assert_eq!(done.notes[0].derivation, [0, 2]);
        assert_eq!(done.notes[1].derivation, [1, 3]);
        assert_eq!(done.notes[0].attack_velocity, 90);
        assert_eq!(done.notes[1].attack_velocity, 91);
    }

    #[test]
    fn sustain_extends_sounding_but_never_rewrites_the_key_release() {
        let events = [
            event(0, raw(MidiMessageKind::NoteOn, 0, 60, 80, 0)),
            event(1, raw(MidiMessageKind::ControlChange, 0, 64, 127, 1_000)),
            event(2, raw(MidiMessageKind::NoteOff, 0, 60, 10, 2_000)),
            event(3, raw(MidiMessageKind::ControlChange, 0, 64, 0, 5_000)),
        ];
        let done = complete(&events);
        assert_eq!(done.notes.len(), 1);
        let note = done.notes[0];
        assert_eq!(note.key_release_micros, 2_000);
        assert_eq!(note.sounding_end_micros, 5_000);
        assert!(note.pedal_extended);
        assert_eq!(
            done.controllers,
            vec![ControllerSpan {
                channel: 0,
                controller: 64,
                value: 127,
                press_micros: 1_000,
                release_micros: 5_000,
            }]
        );
    }

    #[test]
    fn unpaired_edges_are_losses_not_intervals() {
        let events = [
            event(0, raw(MidiMessageKind::NoteOn, 0, 60, 80, 0)),
            event(1, raw(MidiMessageKind::NoteOff, 0, 62, 10, 1_000)),
        ];
        let done = complete(&events);
        assert!(done.notes.is_empty());
        assert_eq!(done.unpaired_note_ons, vec![0]);
        assert_eq!(done.unpaired_note_offs, vec![1]);
    }

    #[test]
    fn the_window_clamps_and_depends_on_the_beat() {
        assert_eq!(adaptive_window_micros(500_000), 41_666);
        assert_eq!(adaptive_window_micros(100_000), 18_000);
        assert_eq!(adaptive_window_micros(1_000_000), 70_000);
    }

    #[test]
    fn shape_alternatives_distinguish_block_rolled_and_arpeggio() {
        let simultaneous = OnsetGroup {
            notes: vec![0, 1],
            onset_micros: 0,
            spread_micros: 0,
            ordered: false,
        };
        assert_eq!(
            shape_alternatives(&simultaneous),
            vec![GroupShape::Block, GroupShape::Rolled]
        );
        let ordered_spread = OnsetGroup {
            notes: vec![0, 1],
            onset_micros: 0,
            spread_micros: 20_000,
            ordered: true,
        };
        assert_eq!(
            shape_alternatives(&ordered_spread),
            vec![GroupShape::Rolled, GroupShape::Arpeggio]
        );
        let disordered_spread = OnsetGroup {
            notes: vec![0, 1],
            onset_micros: 0,
            spread_micros: 20_000,
            ordered: false,
        };
        assert_eq!(shape_alternatives(&disordered_spread), vec![GroupShape::Arpeggio]);
        let single = OnsetGroup {
            notes: vec![0],
            onset_micros: 0,
            spread_micros: 0,
            ordered: true,
        };
        assert!(shape_alternatives(&single).is_empty());
    }

    #[test]
    fn a_forty_ms_spread_is_one_group_at_120_bpm_and_not_at_much_faster() {
        let notes = [
            CompletedNote {
                note: 60,
                attack_velocity: 80,
                release_velocity: 0,
                onset_micros: 0,
                key_release_micros: 400_000,
                sounding_end_micros: 400_000,
                derivation: [0, 0],
                pedal_extended: false,
            },
            CompletedNote {
                note: 64,
                attack_velocity: 80,
                release_velocity: 0,
                onset_micros: 40_000,
                key_release_micros: 440_000,
                sounding_end_micros: 440_000,
                derivation: [1, 1],
                pedal_extended: false,
            },
        ];
        // 120 BPM: quarter 500 000 µs -> window 41 666 µs; 40 ms is inside it.
        assert_eq!(group_notes(&notes, 500_000).len(), 1);
        // 240 BPM: quarter 250 000 µs -> window 20 833 µs; 40 ms exceeds it.
        assert_eq!(group_notes(&notes, 250_000).len(), 2);
    }
}
