//! MIDI note entry: keyboard presses as written pitches (roadmap §12.5,
//! §14.5).
//!
//! MIDI is an edge format, and this is the edge. A note number is a sounding
//! pitch class and nothing more; the score wants a *written* pitch, and which
//! spelling is right depends on the key the piece is in (§2's first rule:
//! written pitch is not a MIDI number). So the translation lives here, on the
//! side of the boundary that can see the score — never in the frontend, which
//! `03-interaction.md` §7 forbids from computing musical facts.
//!
//! The chord rule is the same kind of decision. Two keys pressed together are
//! one chord, and "together" is a musical judgement about a human's hands, so
//! [`EntryBuffer`] makes it here rather than leaving the interface to guess
//! from a stream of events.

// Every arithmetic expression below is over small integers with known
// bounds: a note number is 0–127, a natural is 0–11, an alteration is −2–2,
// and a signature is −7–7 fifths, all held in `i32`. No sum, difference, or
// product of those can leave `i32`, and the only division is by the constant
// twelve, so there is nothing here for a checked operation to report.
#![allow(clippy::arithmetic_side_effects)]

use std::time::{Duration, Instant};

use musa_compiler::{Key, Letter};

/// How close two presses must be to be one chord.
///
/// Long enough that a hand landing on a triad is never split — the spread of
/// a deliberate chord is a few milliseconds — and short enough that a fast
/// scale is never joined: at 40 ms a run would have to pass 1 500 notes per
/// minute before two of its notes collided.
const CHORD_WINDOW: Duration = Duration::from_millis(40);

/// Notes the keyboard played, grouped as one statement to write.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiEntry {
    /// The written pitches, spelled in the piece's key, lowest first. One
    /// pitch is a note; more than one is a chord.
    pub pitches: Vec<String>,
}

/// Presses waiting to be grouped into chords.
///
/// Kept separate from the clock so the grouping rule can be tested by handing
/// it times rather than by sleeping.
#[derive(Debug, Default)]
pub(crate) struct EntryBuffer {
    /// Presses in arrival order.
    pending: Vec<(u8, Instant)>,
}

impl EntryBuffer {
    /// Record a key going down.
    pub(crate) fn press(&mut self, note: u8, at: Instant) {
        self.pending.push((note, at));
    }

    /// The groups whose window has closed by `now`, oldest first.
    ///
    /// A group is the oldest waiting press plus every press within
    /// [`CHORD_WINDOW`] of it. Nothing is released early: a chord half-played
    /// when the poll happens stays whole and arrives on the next one.
    pub(crate) fn ready(&mut self, now: Instant) -> Vec<Vec<u8>> {
        let mut groups = Vec::new();
        while let Some(&(_, first)) = self.pending.first() {
            let deadline = first.checked_add(CHORD_WINDOW).unwrap_or(first);
            if now < deadline {
                break;
            }
            let taken = self.pending.iter().take_while(|(_, at)| *at <= deadline).count();
            let mut notes: Vec<u8> = self.pending.drain(..taken).map(|(note, _)| note).collect();
            // A held key that retriggers is one note in the chord, not two.
            notes.sort_unstable();
            notes.dedup();
            groups.push(notes);
        }
        groups
    }
}

/// The natural pitch class of each letter, in the language's letter order.
const NATURALS: [(Letter, i32); 7] = [
    (Letter::C, 0),
    (Letter::D, 2),
    (Letter::E, 4),
    (Letter::F, 5),
    (Letter::G, 7),
    (Letter::A, 9),
    (Letter::B, 11),
];

/// The order accidentals are added to a signature: sharps F C G D A E B,
/// flats the same list read backwards.
const SHARP_ORDER: [Letter; 7] = [
    Letter::F,
    Letter::C,
    Letter::G,
    Letter::D,
    Letter::A,
    Letter::E,
    Letter::B,
];

/// Spell a MIDI note number the way the piece would write it.
///
/// The rule, in one sentence: **a note the key already spells is written the
/// key's way, and a note outside the key is written as an alteration of its
/// neighbour — raised in a sharp key, lowered in a flat one.**
///
/// So in G major note 66 is `f#4`, because the key spells F sharp; in F major
/// note 61 is `db4` and in D major it is `c#4`, because the two keys lean
/// opposite ways; and in B major note 65 is `e#4`, because the letter below
/// it is the one the key has. A piece with no `key` declaration is treated as
/// having none in the signature, which leans sharp — the same default every
/// notation program uses.
///
/// This is a heuristic and is meant to be: no rule can know whether a
/// composer meant an augmented fourth or a diminished fifth, and respelling
/// one note is a keystroke away. What it must never do is surprise — hence
/// the table test beside it.
pub(crate) fn spell(note: u8, key: Option<Key>) -> String {
    let fifths = key.map_or(0, Key::fifths);
    let pitch_class = i32::from(note % 12);
    let alterations = key_alterations(fifths);

    // The key's own seven spellings, by the pitch class each lands on.
    let diatonic = |wanted: i32| -> Option<(Letter, i32)> {
        NATURALS.iter().enumerate().find_map(|(index, &(letter, natural))| {
            let alter = alterations.get(index).copied().unwrap_or(0);
            ((natural + alter).rem_euclid(12) == wanted).then_some((letter, alter))
        })
    };

    let (letter, alter) = diatonic(pitch_class)
        .or_else(|| {
            // Outside the key: alter the neighbour the signature leans towards.
            if fifths >= 0 {
                diatonic((pitch_class - 1).rem_euclid(12)).map(|(letter, alter)| (letter, alter + 1))
            } else {
                diatonic((pitch_class + 1).rem_euclid(12)).map(|(letter, alter)| (letter, alter - 1))
            }
        })
        // Unreachable for any real signature: seven letters a fifth apart
        // cover every pitch class either directly or one step away.
        .unwrap_or((Letter::C, 0));

    let natural = NATURALS
        .iter()
        .find_map(|&(candidate, natural)| (candidate == letter).then_some(natural))
        .unwrap_or(0);
    // The written octave is the one that makes the spelling sound at `note`,
    // which is not always the note number's own: `b#3` sounds where `c4` does.
    let octave = (i32::from(note) - natural - alter).div_euclid(12) - 1;
    format!("{}{}{}", letter_name(letter), accidental(alter), octave.max(0))
}

/// The alteration each letter carries in a signature of `fifths`, indexed
/// like [`NATURALS`].
fn key_alterations(fifths: i8) -> [i32; 7] {
    let mut alterations = [0; 7];
    let count = usize::from(fifths.unsigned_abs()).min(SHARP_ORDER.len());
    let (altered, alter): (Vec<Letter>, i32) = if fifths >= 0 {
        (SHARP_ORDER.iter().copied().take(count).collect(), 1)
    } else {
        (SHARP_ORDER.iter().rev().copied().take(count).collect(), -1)
    };
    for letter in altered {
        if let Some(slot) = NATURALS
            .iter()
            .position(|&(candidate, _)| candidate == letter)
            .and_then(|index| alterations.get_mut(index))
        {
            *slot = alter;
        }
    }
    alterations
}

fn letter_name(letter: Letter) -> char {
    match letter {
        Letter::C => 'c',
        Letter::D => 'd',
        Letter::E => 'e',
        Letter::F => 'f',
        Letter::G => 'g',
        Letter::A => 'a',
        Letter::B => 'b',
    }
}

/// The language's accidental suffix (`#` sharp, `b` flat, doubled once).
fn accidental(alter: i32) -> &'static str {
    match alter {
        2 => "##",
        1 => "#",
        -1 => "b",
        -2 => "bb",
        _ => "",
    }
}

#[cfg(test)]
mod midi_laws {
    use super::{EntryBuffer, MidiEntry, spell};
    use musa_compiler::{Accidental, Key, Letter, Mode, PitchClass};
    use std::time::{Duration, Instant};

    fn key(letter: Letter, accidental: i8, mode: Mode) -> Option<Key> {
        Some(Key::new(
            PitchClass {
                letter,
                accidental: Accidental(accidental),
            },
            mode,
        ))
    }

    /// The spelling table the doc comment promises. Every row is a note a
    /// composer would play and the pitch they would expect to see.
    #[test]
    fn a_note_is_spelled_the_way_its_key_spells_it() {
        let c_major = key(Letter::C, 0, Mode::Major);
        let g_major = key(Letter::G, 0, Mode::Major);
        let f_major = key(Letter::F, 0, Mode::Major);
        let d_major = key(Letter::D, 0, Mode::Major);
        let e_flat = key(Letter::E, -1, Mode::Major);
        let b_major = key(Letter::B, 0, Mode::Major);
        let a_minor = key(Letter::A, 0, Mode::Minor);

        let rows: [(u8, Option<Key>, &str); 14] = [
            // Middle C is middle C in every key.
            (60, c_major, "c4"),
            (60, f_major, "c4"),
            (60, None, "c4"),
            // The key's own accidentals come back as the key writes them.
            (66, g_major, "f#4"),
            (70, f_major, "bb4"),
            (63, e_flat, "eb4"),
            // Outside the key, the signature decides which way to lean.
            (61, c_major, "c#4"),
            (61, d_major, "c#4"),
            (61, f_major, "db4"),
            (63, f_major, "eb4"),
            // A sharp key raises the letter below, even across a semitone
            // where the natural would have been easier to read.
            (65, b_major, "e#4"),
            // A minor key is its relative major's signature.
            (60, a_minor, "c4"),
            (61, a_minor, "c#4"),
            // Octaves follow the note number, an octave to twelve semitones.
            (48, c_major, "c3"),
        ];
        for (note, key, expected) in rows {
            assert_eq!(spell(note, key), expected, "note {note}");
        }
    }

    /// A spelling has to *sound* where the note number says, including when
    /// the letter and the number disagree about the octave (`b#3` is 60).
    #[test]
    fn every_spelling_sounds_at_the_note_it_came_from() {
        for fifths in [-6i8, -3, 0, 2, 5] {
            let key = tonic_for(fifths);
            for note in 24u8..=96 {
                let spelled = spell(note, key);
                assert_eq!(sounds_at(&spelled), Some(note), "{spelled} for note {note}");
            }
        }
    }

    /// A key whose signature has `fifths` accidentals, built by walking the
    /// circle rather than by a table the test shares with the code.
    fn tonic_for(fifths: i8) -> Option<Key> {
        const MAJORS: [(Letter, i8); 13] = [
            (Letter::G, -1),
            (Letter::D, -1),
            (Letter::A, -1),
            (Letter::E, -1),
            (Letter::B, -1),
            (Letter::F, 0),
            (Letter::C, 0),
            (Letter::G, 0),
            (Letter::D, 0),
            (Letter::A, 0),
            (Letter::E, 0),
            (Letter::B, 0),
            (Letter::F, 1),
        ];
        let index = usize::try_from(fifths + 6).ok()?;
        let &(letter, accidental) = MAJORS.get(index)?;
        key(letter, accidental, Mode::Major)
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

    /// Keys pressed together are one chord; keys pressed apart are two notes.
    #[test]
    fn presses_inside_the_window_are_one_chord() {
        let start = Instant::now();
        let mut buffer = EntryBuffer::default();
        buffer.press(60, start);
        buffer.press(64, start + Duration::from_millis(5));
        buffer.press(67, start + Duration::from_millis(9));
        buffer.press(72, start + Duration::from_millis(400));

        assert_eq!(
            buffer.ready(start + Duration::from_millis(100)),
            vec![vec![60, 64, 67]],
            "the triad is one group and the later note is not in it"
        );
        assert_eq!(buffer.ready(start + Duration::from_millis(500)), vec![vec![72]]);
        assert!(buffer.ready(start + Duration::from_secs(1)).is_empty());
    }

    /// A chord still being played when the poll happens is not cut in half:
    /// it waits for its window to close.
    #[test]
    fn a_chord_is_never_released_early() {
        let start = Instant::now();
        let mut buffer = EntryBuffer::default();
        buffer.press(60, start);
        assert!(buffer.ready(start + Duration::from_millis(10)).is_empty());
        buffer.press(64, start + Duration::from_millis(20));
        assert_eq!(buffer.ready(start + Duration::from_millis(60)), vec![vec![60, 64]]);
    }

    /// A key held down retriggers on some keyboards; the chord has one of it.
    #[test]
    fn a_retriggered_key_is_one_note() {
        let start = Instant::now();
        let mut buffer = EntryBuffer::default();
        buffer.press(60, start);
        buffer.press(60, start + Duration::from_millis(3));
        assert_eq!(buffer.ready(start + Duration::from_millis(60)), vec![vec![60]]);
    }

    /// The wire shape a chord takes, so the frontend has one thing to read.
    #[test]
    fn an_entry_serializes_as_its_pitches() -> Result<(), serde_json::Error> {
        let entry = MidiEntry {
            pitches: vec!["c4".to_owned(), "e4".to_owned()],
        };
        assert_eq!(serde_json::to_string(&entry)?, r#"{"pitches":["c4","e4"]}"#);
        Ok(())
    }
}
