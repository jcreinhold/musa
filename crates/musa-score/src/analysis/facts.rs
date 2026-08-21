//! The `facts` kind.
//!
//! **Abstract domain.** The finite set of *pointed statements* of a score: a
//! sounding written pitch with its exact span, a silence with its exact span,
//! a chord symbol at an instant, and a key or meter in force from an instant —
//! each paired with where in the score it can be seen.
//!
//! **Abstraction map.** α restricts the snapshot to the requested lanes and
//! window and reads every surviving score event, harmony annotation, and key
//! and meter stretch into exactly one element of that set. A chord event
//! becomes one `Sounding` per tone, because a chord at this layer is
//! simultaneous notes and nothing more.
//!
//! **Soundness.** α is *exact on what it reports*: for every finding there is
//! a statement of the score with precisely those coordinates, and every
//! statement of the score inside the scope and window has a finding.
//! Therefore every finding is a fact — the abstraction separates every pair of
//! concrete scores differing on anything it reports, so no two readings survive
//! it and there is nothing to be a candidate about.
//!
//! What a finding licenses is exactly "the score states this here". What it
//! does *not* license is any claim about material the request excluded: γ of a
//! report is every score agreeing with it inside the scope and window, and
//! that set is not a singleton. A reader concluding "the piece is in C major"
//! from one `KeyInForce` finding over one bar has read something the map does
//! not say.
use super::{AnalysisFinding, AnalysisScope, Evidence, Lane, NoteRef, Observation, inside};
use crate::score::{ScoreEventKind, ScoreSnapshot};
use crate::time::MusicalTime;

pub(super) fn observe(
    snapshot: &ScoreSnapshot,
    lanes: &[Lane],
    scope: &AnalysisScope,
    window: Option<(MusicalTime, MusicalTime)>,
) -> Vec<AnalysisFinding> {
    let mut found = Vec::new();
    for lane in lanes {
        let Some(voice) = snapshot.parts().get(lane.part).and_then(|part| part.voice(lane.voice)) else {
            continue;
        };
        for event in voice.events() {
            if !inside(window, event.onset) {
                continue;
            }
            let evidence = Evidence::Event(NoteRef {
                part: lane.part,
                voice: lane.voice,
                id: event.id,
                span: event.origin.definition_span,
            });
            let duration = event.notated_duration.value;
            match event.kind {
                ScoreEventKind::Note { pitch } => found.push(AnalysisFinding::stated(
                    "sounding-pitch",
                    Observation::Sounding {
                        pitch,
                        onset: event.onset,
                        duration,
                    },
                    evidence,
                )),
                // A chord is simultaneous notes at this layer, so it is that
                // many findings and not one with a list inside: the next
                // analysis asks about pitches, not about noteheads.
                ScoreEventKind::Chord { ref pitches } => {
                    found.extend(pitches.iter().map(|pitch| {
                        AnalysisFinding::stated(
                            "sounding-pitch",
                            Observation::Sounding {
                                pitch: *pitch,
                                onset: event.onset,
                                duration,
                            },
                            evidence.clone(),
                        )
                    }));
                }
                ScoreEventKind::Rest => found.push(AnalysisFinding::stated(
                    "silence",
                    Observation::Silence {
                        onset: event.onset,
                        duration,
                    },
                    evidence,
                )),
            }
        }
        for (from, key) in snapshot.keys().changes(lane.scope()) {
            if inside(window, from) {
                found.push(AnalysisFinding::stated(
                    "key-in-force",
                    Observation::KeyInForce { key: *key, from },
                    Evidence::InForce {
                        scope: lane.scope(),
                        from,
                    },
                ));
            }
        }
        for (from, meter) in snapshot.meters().changes(lane.scope()) {
            if inside(window, from) {
                found.push(AnalysisFinding::stated(
                    "meter-in-force",
                    Observation::MeterInForce { meter: *meter, from },
                    Evidence::InForce {
                        scope: lane.scope(),
                        from,
                    },
                ));
            }
        }
    }
    // A chord symbol belongs to the piece, not to a staff: it says what the
    // whole texture is doing there. So it is read when the request is about
    // the piece, and left out when the request narrowed to one part or voice —
    // reporting it there would attribute a piece-wide statement to music that
    // did not make it.
    if matches!(*scope, AnalysisScope::Score) {
        found.extend(
            snapshot
                .annotations()
                .harmony()
                .iter()
                .filter(|mark| inside(window, mark.at))
                .map(|mark| {
                    AnalysisFinding::stated(
                        "written-harmony",
                        Observation::Written {
                            symbol: mark.symbol.clone(),
                            at: mark.at,
                        },
                        Evidence::Annotation {
                            span: mark.origin.source_span,
                        },
                    )
                }),
        );
    }
    found
}
