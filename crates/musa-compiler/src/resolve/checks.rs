#![allow(clippy::arithmetic_side_effects)]
use num_rational::Ratio;

use crate::diagnose::{Code, Diagnostic};
use crate::score::ScoreSnapshot;

use super::Resolver;

/// A groove needs a meter to swing against.
///
/// A groove displaces the beat inside a cell named by the meter's denominator
/// (`groove.rs`), and unmeasured music has no denominator to name one — so a
/// swung cadenza is not a thing with a wrong answer, it is a question with
/// none. Refused rather than straightened silently, which would be the
/// composer's feel dropped without a word.
pub(crate) fn check_groove_has_a_meter(resolver: &mut Resolver, snapshot: &ScoreSnapshot) {
    let rules = std::mem::take(&mut resolver.groove_rules);
    if rules.is_empty() {
        return;
    }
    for (id, part) in snapshot.parts().iter() {
        let bars = snapshot.bars(crate::Scope::Part { part: id.0 });
        let Some(profile) = snapshot.profiles().for_part(part.name()) else {
            continue;
        };
        let Some((_, span)) = rules.iter().find(|(name, _)| name == profile.name()) else {
            continue;
        };
        let unmeasured = part
            .voices()
            .flat_map(|(_, voice)| voice.events())
            .any(|event| !bars.meter_at(event.onset).is_measured());
        if !unmeasured {
            continue;
        }
        resolver.report(
            Diagnostic::error(Code::Misplaced, "this groove has no beat to lay itself over")
                .at(*span, "the part playing this reaches unmeasured music")
                .help("end the unmeasured stretch before this part plays, or give the part a straight profile")
                .note("a groove displaces the beat inside a cell the meter names, and `meter none` names none"),
        );
    }
}

pub(crate) fn check_measure_sanity(resolver: &mut Resolver, snapshot: &ScoreSnapshot) {
    for (id, part) in snapshot.parts().iter() {
        // Whose barlines: a part in 7/8 stops short of *its* barline, and
        // measuring it against the piece's 4/4 would complain about music
        // that is right (polymeter).
        let bars = snapshot.bars(crate::Scope::Part { part: id.0 });
        for (voice_id, voice) in part.voices() {
            // A voice that holds a note as long as it likes is not measured
            // after that note: how far it reaches is the performance's
            // answer, so a barline arithmetic complaint about it would be a
            // complaint about the freedom rather than about a mistake.
            if voice.events().iter().any(|event| event.free.is_some()) {
                continue;
            }
            let span = voice.span();
            let end = crate::MusicalTime::ZERO + span;
            // Unmeasured music stops where it stops. "Part-way through a
            // measure" is a complaint about barlines, and there are none.
            if !bars.meter_at(end).is_measured() {
                continue;
            }
            let stops = bars.at(end);
            if span.as_ratio() != Ratio::ZERO && stops.into != crate::MusicalDuration::ZERO {
                let name = part.voice_name(voice_id).unwrap_or("?");
                // No span: this is a fact about a whole voice, and pointing
                // at its first note would send the reader somewhere the
                // mistake probably is not.
                // How far past the last barline the voice stops, said in the
                // units the composer writes durations in.
                let measure = bars.measure_at(crate::MusicalTime::ZERO + span);
                let finished = stops.measure;
                let short = (measure.end - (crate::MusicalTime::ZERO + span)).as_ratio();
                resolver.report(
                    Diagnostic::warning(
                        Code::DoesNotAddUp,
                        format!(
                            "voice `{name}` in part `{}` stops part-way through measure {}",
                            part.name(),
                            finished.max(1),
                        ),
                    )
                    .help(format!(
                        "it is {short} short — add a rest, or check the durations above"
                    ))
                    .note("a short last measure puts every part after it out of step"),
                );
            }
        }
    }
}
