//! The admitted equality: the deterministic key a score fact normalizes by
//! (docs/rules/kernel/05 N3, 12).

use super::fact::{FactKind, ScoreFact};
use musa_score::scope::Scope;
use musa_score::score::Mode;
use std::fmt::Write as _;

impl musa_kernel::Canonical for ScoreFact {
    const OWNER_TYPE_ID: &'static str = "musa.compiler.ScoreFact";
    const QUOTIENT_VERSION: u32 = 1;

    /// Deterministic key for canonical ordering and the admitted score-fact
    /// equality (docs/rules/kernel/05 N3, 12): scope, kind, source span, and
    /// expansion path. Other stored compilation details are deliberately not
    /// part of this quotient.
    ///
    /// A note or rest with nothing written on it keys exactly as it did
    /// before facts were heterogeneous, so a piece of plain notes has the
    /// normal form it has always had.
    fn canonical_key(&self) -> String {
        let articulations = |marks: &[musa_score::Mark]| {
            if marks.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = marks.iter().map(|mark| mark.name()).collect();
                format!("|artic:{}", names.join(","))
            }
        };
        let held = |free: Option<&musa_score::score::FreeDuration>| {
            free.map_or_else(String::new, |free| format!("|to:{}", free.most.as_ratio()))
        };
        let kind = match &self.kind {
            FactKind::Note {
                pitch,
                duration,
                articulations: marks,
                free,
            } => format!(
                "note:{pitch}|{}{}{}",
                duration.spelling,
                articulations(marks),
                held(free.as_ref())
            ),
            FactKind::Rest {
                duration,
                articulations: marks,
                free,
            } => format!(
                "rest|{}{}{}",
                duration.spelling,
                articulations(marks),
                held(free.as_ref())
            ),
            // The argument is in the key: two `mark text` occurrences over one
            // span say different things, and N3 must be able to tell them
            // apart or the semantic hash would call them equal.
            FactKind::Mark { mark, argument } => match argument {
                Some(argument) => format!("mark:{mark}:{argument}|"),
                None => format!("mark:{mark}|"),
            },
            // The index is in the key for the reason its own doc gives: every
            // grace note of a group shares a span, so this string is the only
            // thing N2 has to order them by.
            FactKind::Grace {
                pitch,
                articulations: marks,
                index,
            } => format!("grace:{pitch}:{index}|{}", articulations(marks)),
            FactKind::Slur => "slur|".to_owned(),
            FactKind::Phrase { name } => format!("phrase:{name}|"),
            FactKind::Tuplet { num, den } => format!("tuplet:{num}/{den}|"),
            FactKind::Dynamic { mark } => format!("dynamic:{}|", mark.name()),
            FactKind::Hairpin { grows, target, shape } => {
                format!(
                    "hairpin:{}:{}:{}|",
                    if *grows { "cres" } else { "dim" },
                    target.name(),
                    shape.canonical_key()
                )
            }
            FactKind::Key { tonic, mode } => {
                let mode = match mode {
                    Mode::Major => "major",
                    Mode::Minor => "minor",
                };
                format!("key:{tonic}:{mode}|")
            }
            FactKind::Meter { numerator, denominator } => format!("meter:{numerator}/{denominator}|"),
            FactKind::Clef { clef } => format!("clef:{}|", clef.name()),
            FactKind::Tempo { metronome, text, ramp } => {
                let mark = metronome.map_or_else(String::new, |mark| format!("{}={}", mark.beat, mark.bpm));
                let ramp = ramp.as_ref().map_or_else(String::new, |ramp| {
                    format!(
                        "{}>{}>{}",
                        ramp.to.map_or_else(String::new, |bpm| bpm.to_string()),
                        ramp.over.as_ratio(),
                        ramp.shape.canonical_key()
                    )
                });
                format!("tempo:{mark}:{}:{ramp}|", text.as_deref().unwrap_or_default())
            }
            FactKind::Section { name } => format!("section:{name}|"),
            FactKind::Harmony { symbol } => format!("harmony:{}|", symbol.text()),
            FactKind::Repeat { times, range } => match range {
                Some((least, most)) => format!("repeat:{times}:{least}:{most}|"),
                None => format!("repeat:{times}|"),
            },
            FactKind::Ending { bracket, pass } => format!("ending:{bracket}:{pass}|"),
            FactKind::Mobile { fragments, order } => {
                let order: Vec<String> = order.iter().map(u32::to_string).collect();
                format!("mobile:{}:{}|", fragments.join(","), order.join(","))
            }
            FactKind::Improvise { over } => format!("improvise:{}|", over.as_deref().unwrap_or_default()),
        };
        // Written rather than `format!`ed so the scope costs no second
        // allocation: P4 walks every occurrence on every edit.
        let mut key = String::with_capacity(kind.len().saturating_add(32));
        match self.scope {
            // `*` sorts before any part number, so at one instant the context
            // a reader meets first is the context that prints first.
            Scope::Piece => key.push_str("*|*|"),
            // A part sorts with its own number and before any of its voices,
            // which is where a reader meets its clef.
            Scope::Part { part } => {
                let _ = write!(key, "{part}|*|");
            }
            Scope::Voice { part, voice } => {
                let _ = write!(key, "{part}|{voice}|");
            }
        }
        let _ = write!(
            key,
            "{}|{}|{}|{:?}",
            kind, self.origin.source_span.start, self.origin.source_span.end, self.origin.expansion_path,
        );
        key
    }
}
