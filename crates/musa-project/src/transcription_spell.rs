//! Pitch spelling and written-end facts per completed note (prompt 205b).
//!
//! Spelling reads the key/collection at the insertion point and preserves MIDI
//! identity: a chromatic note offers its enharmonic alternative, and no absent
//! key is ever a silent C-major claim. Written ends default to the quantized
//! key-release interval prompt 204b already measured (so the 47/58 admission
//! cannot regress), and a pedal-extended sounding interval is reported as a
//! fact beside the written end, never scored as the end itself.

use crate::midi::{spell, spell_alternatives};
use crate::transcription_pairing::CompletedNote;
use musa_score::Key;

/// The written spelling of one completed note, with its alternatives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PitchSpelling {
    /// The primary spelling (`spell`'s choice), e.g. `c#4`.
    pub spelled: String,
    /// The enharmonic reading of a chromatic note, empty for a note the key
    /// already spells unambiguously.
    pub alternatives: Vec<String>,
}

/// Spell every completed note from the key, independently, so chord members
/// keep their individual spellings.
pub(crate) fn spell_pitches(notes: &[CompletedNote], key: Option<Key>) -> Vec<PitchSpelling> {
    notes
        .iter()
        .map(|note| PitchSpelling {
            spelled: spell(note.note, key),
            alternatives: spell_alternatives(note.note, key),
        })
        .collect()
}

/// The written end of one completed note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WrittenEnd {
    /// Quantized key-release length in grid ticks; the written end.
    pub duration_ticks: u32,
    /// Sound continues past the key release under a sustain pedal. The written
    /// end stays at the key release; this is a reported fact, never a lengthening.
    pub pedal_extended: bool,
    /// The sounding interval is so short it belongs to the grace tier: shown as
    /// grace, not silently shrunk into an ordinary tiny note.
    pub grace: bool,
}

/// A grace candidate is a written length below this many ticks: shorter than a
/// sixteenth on the 24-tick grid.
const GRACE_TICKS: u32 = 5;

/// Attach a written end to each note from the quantized key durations the
/// rhythm pass measured. The duration is preserved exactly (identity), so
/// spelling stage can never regress the 47/58 key-duration admission.
pub(crate) fn infer_written_ends(notes: &[CompletedNote], durations_ticks: &[u32]) -> Vec<WrittenEnd> {
    notes
        .iter()
        .zip(durations_ticks)
        .map(|(note, &duration_ticks)| WrittenEnd {
            duration_ticks,
            pedal_extended: note.pedal_extended,
            grace: duration_ticks < GRACE_TICKS,
        })
        .collect()
}

#[cfg(test)]
mod laws {
    #![allow(clippy::arithmetic_side_effects)]
    #![allow(clippy::indexing_slicing)]
    use super::*;
    use musa_score::{Accidental, Letter, Mode, PitchClass};

    /// A key with the given signature, built by tonic rather than by a shared
    /// table the code could also consult: spelling reads only `Key::fifths`.
    fn key(fifths: i8) -> Option<Key> {
        let (letter, accidental) = match fifths {
            -3 => (Letter::E, -1),
            0 => (Letter::C, 0),
            1 => (Letter::G, 0),
            2 => (Letter::D, 0),
            5 => (Letter::B, 0),
            _ => (Letter::C, 0),
        };
        Some(Key::new(
            PitchClass {
                letter,
                accidental: Accidental(accidental),
            },
            Mode::Major,
        ))
    }

    fn note(note: u8, pedal: bool) -> CompletedNote {
        CompletedNote {
            note,
            attack_velocity: 64,
            release_velocity: 0,
            onset_micros: 0,
            key_release_micros: 500_000,
            sounding_end_micros: if pedal { 1_700_000 } else { 500_000 },
            derivation: [0, 0],
            pedal_extended: pedal,
        }
    }

    #[test]
    fn every_spelling_sounds_where_the_midi_note_says() {
        for fifths in [-3_i8, 0, 2, 5] {
            let key = key(fifths);
            for pitch in 24_u8..=96 {
                let parsing = spell_pitches(&[note(pitch, false)], key);
                assert_eq!(sounds_at(&parsing[0].spelled), Some(pitch));
                for alternative in &parsing[0].alternatives {
                    assert_eq!(sounds_at(alternative), Some(pitch), "alternative must sound the same");
                }
            }
        }
    }

    #[test]
    fn a_chromatic_note_offers_the_other_enharmonic() {
        // C major: note 61 is chromatic; spell leans sharp, the alternative is flat.
        let parsing = spell_pitches(&[note(61, false)], key(0));
        assert_eq!(parsing[0].spelled, "c#4");
        assert_eq!(parsing[0].alternatives, vec!["db4".to_owned()]);
    }

    #[test]
    fn a_diatonic_note_has_no_alternative() {
        let parsing = spell_pitches(&[note(66, false)], key(1)); // G major spells F# as the key's
        assert_eq!(parsing[0].spelled, "f#4");
        assert!(parsing[0].alternatives.is_empty());
    }

    #[test]
    fn no_key_is_not_c_major() {
        // No key leans sharp for spelling, but that is a notation default, not
        // a declared C-major scope; chromatic notes still get alternatives.
        let parsing = spell_pitches(&[note(61, false)], None);
        assert_eq!(parsing[0].spelled, "c#4");
        assert_eq!(parsing[0].alternatives, vec!["db4".to_owned()]);
    }

    #[test]
    fn chord_members_keep_individual_spellings() {
        let chord = [60, 63, 67].map(|pitch| note(pitch, false));
        let parsing = spell_pitches(&chord, key(0));
        // C major leans sharp for its chromatic note, so D-sharp, not E-flat.
        assert_eq!(
            parsing.iter().map(|sp| sp.spelled.as_str()).collect::<Vec<_>>(),
            ["c4", "d#4", "g4"]
        );
    }

    #[test]
    fn written_end_is_never_the_pedal_extended_sound() {
        let notes = [note(60, true)];
        let ends = infer_written_ends(&notes, &[24]);
        assert_eq!(ends[0].duration_ticks, 24);
        assert!(ends[0].pedal_extended);
        assert!(!ends[0].grace);
    }

    #[test]
    fn a_short_generated_grace_is_flagged_not_shrunk() {
        let ends = infer_written_ends(&[note(60, false)], &[3]);
        assert_eq!(ends[0].duration_ticks, 3);
        assert!(ends[0].grace);
    }

    /// Read a written pitch back as the MIDI note it sounds at.
    fn sounds_at(pitch: &str) -> Option<u8> {
        let mut chars = pitch.chars();
        let natural = match chars.next()? {
            'c' => 0,
            'd' => 2,
            'e' => 4,
            'f' => 5,
            'g' => 7,
            'a' => 9,
            'b' => 11,
            _ => return None,
        };
        let rest: String = chars.collect();
        let (accidental, octave) = rest.split_at(rest.find(|c: char| c.is_ascii_digit())?);
        let alter = match accidental {
            "##" => 2,
            "#" => 1,
            "b" => -1,
            "bb" => -2,
            "" => 0,
            _ => return None,
        };
        let octave: i32 = octave.parse().ok()?;
        u8::try_from((octave + 1) * 12 + natural + alter).ok()
    }
}
