//! Deriving a plan from a score snapshot: front matter, staves, and the
//! marks that sit above them.
//!
//! One concern of the `plan` module; see its docs for what a plan is.

use musa_compiler::{BarLines, Clef, Key, Meter, Metronome, MusicalTime, Scope, ScoreSnapshot};

use super::collect::Marks;
use super::fold::Fold;
use super::marks::{KeySignature, OpenMark, OpenShape, PositionedMark, TempoText};
use super::score::{FrontMatter, NotationOptions, NotationPlan};
use super::staff::plan_staff;

/// Plan notation for a compiled score.
///
/// # Errors
/// Returns [`NotationError`] when a duration cannot be spelled with standard
/// values and ties inside one measure.
pub fn plan_notation(score: &ScoreSnapshot, _options: &NotationOptions) -> Result<NotationPlan, crate::NotationError> {
    // Notation's own barlines. `Fold` prints a repeat once and plays it
    // twice, so a written measure is not a sounding measure; the snapshot's
    // own `bars()` is the unfolded one, and mixing them would number the page
    // by what it sounds like.
    let fold = Fold::of(score);
    // The piece's grid: what the piece-wide marks below are positioned
    // against. Each staff draws its own, which under polymeter is a different
    // grid with different measure numbers in it.
    let bars = page_bars(score, &fold, Scope::Piece);
    let key = score.key_at(Scope::Piece, MusicalTime::ZERO).map(key_signature);
    // Every key the piece states, on the page's own clock. A modulation
    // inside a folded repeat prints once, at the measure the page numbers it.
    let keys: Vec<(MusicalTime, KeySignature)> = score
        .keys()
        .changes(Scope::Piece)
        .filter_map(|(at, key)| Some((fold.at(at)?, key_signature(*key))))
        .collect();
    let marks = Marks::collect(score);
    // Whether any part states a tempo of its own. Compared as the markings
    // themselves rather than as printed marks: two lists positioned against
    // two different barline grids differ in their measure numbers whether or
    // not they say the same thing.
    let shared: Vec<_> = score.tempos().changes(Scope::Piece).collect();
    let polytempo = score.parts().iter().any(|(id, _)| {
        score
            .tempos()
            .changes(Scope::Part { part: id.0 })
            .ne(shared.iter().copied())
    });
    let mut staves = Vec::new();
    for (_, part) in score.parts().iter() {
        let scope = Scope::Part { part: part.id().0 };
        let clefs: Vec<(MusicalTime, Clef)> = score
            .clefs()
            .changes(scope)
            .filter_map(|(at, clef)| Some((fold.at(at)?, *clef)))
            .collect();
        let staff_bars = page_bars(score, &fold, scope);
        // Exactly one of the two tempo lists carries the marking: the piece's
        // when every part plays at the same speed, each staff's when they do
        // not. A backend prints both and needs no rule of its own, and a
        // piece that is not polytempo plans exactly the document it planned
        // before.
        let tempos = if polytempo {
            tempo_texts(score, &fold, scope, &staff_bars)
        } else {
            Vec::new()
        };
        staves.push(plan_staff(
            score,
            part,
            &staff_bars,
            key,
            &keys,
            &clefs,
            &marks,
            &fold,
            tempos,
        )?);
    }
    // Every tempo marking the piece states, on the page's own clock — the
    // header's included. There is no separate "starting tempo": the header
    // states a fact at zero like any other, so nothing here has to
    // reconstruct one. Under polytempo there is no such marking to print: the
    // piece's tempo is a reading no staff plays, and printing it over a staff
    // that plays something else would be a page that lies.
    let tempos = if polytempo {
        Vec::new()
    } else {
        tempo_texts(score, &fold, Scope::Piece, &bars)
    };
    let sections = score
        .annotations()
        .sections()
        .iter()
        .filter_map(|section| Some(positioned(&bars, fold.at(section.at)?, section.name.clone())))
        .collect();
    let harmony = score
        .annotations()
        .harmony()
        .iter()
        .filter_map(|chord| Some(positioned(&bars, fold.at(chord.at)?, chord.symbol.clone())))
        .collect();
    let repeats = fold.marks(score, &bars);
    let open = score
        .annotations()
        .open()
        .iter()
        .filter_map(|region| {
            let from = positioned(&bars, fold.at(region.start)?, ()).measure;
            // A region that ends on a barline ends in the measure *before*
            // it: the last measure it covers is the last one with music of
            // its own in it, which is where a reader expects the bracket to
            // close.
            let closes = positioned(&bars, fold.at(region.end)?, ());
            let to = if closes.onset_in_measure == musa_compiler::MusicalDuration::ZERO {
                closes.measure.saturating_sub(1)
            } else {
                closes.measure
            };
            Some(OpenMark {
                from,
                to: to.max(from),
                text: open_text(&region.kind),
                kind: match region.kind {
                    musa_compiler::OpenKind::Mobile { .. } => OpenShape::Mobile,
                    musa_compiler::OpenKind::Improvise { .. } => OpenShape::Improvise,
                },
            })
        })
        .collect();
    // A ranged repeat's instruction, printed at the opening barline. The
    // repeat sign already says where the passes are; what no format can say
    // is how many the composer left open, so that is what the text carries.
    let mut open: Vec<OpenMark> = open;
    open.extend(repeats.iter().filter_map(|repeat| {
        let (least, most) = repeat.range?;
        Some(OpenMark {
            from: repeat.from,
            to: repeat.from,
            text: format!("{least}\u{2013}{most}\u{d7}"),
            kind: OpenShape::Passes,
        })
    }));
    open.sort_by_key(|mark| (mark.from, mark.to));
    let holds = staves
        .iter()
        .flat_map(|staff| staff.measures())
        .flat_map(|measure| {
            measure.lanes().iter().flat_map(move |lane| {
                lane.items().iter().filter_map(move |item| {
                    Some(PositionedMark {
                        measure: measure.number(),
                        onset_in_measure: item.onset_in_measure(),
                        what: *item.free()?,
                    })
                })
            })
        })
        .collect();
    Ok(NotationPlan {
        front: FrontMatter {
            title: score.title().to_string(),
            subtitle: score.front_matter().subtitle.clone(),
            composer: score.front_matter().composer.clone(),
            arranger: score.front_matter().arranger.clone(),
            copyright: score.front_matter().copyright.clone(),
            performance: score.performance(),
        },
        staves,
        tempos,
        sections,
        harmony,
        repeats,
        open,
        holds,
    })
}

/// What a reader is told, in the words a printed part uses.
fn open_text(kind: &musa_compiler::OpenKind) -> String {
    match kind {
        musa_compiler::OpenKind::Mobile { fragments, order } => {
            let played: Vec<&str> = order
                .iter()
                .filter_map(|index| fragments.get(*index as usize))
                .map(String::as_str)
                .collect();
            format!("any order — this reading: {}", played.join(", "))
        }
        musa_compiler::OpenKind::Improvise { over } => match over {
            Some(changes) => format!("improvise over {changes}"),
            None => "improvise".to_owned(),
        },
    }
}

/// The tempo markings a scope states, positioned on the page's own clock.
///
/// One function for the piece's markings and a part's, because printing them
/// is the same job either way — which is what makes polytempo a scope
/// argument here too rather than a second code path.
fn tempo_texts(score: &ScoreSnapshot, fold: &Fold, scope: Scope, bars: &BarLines) -> Vec<PositionedMark<TempoText>> {
    score
        .tempos()
        .changes(scope)
        .flat_map(|(at, marking)| {
            let mut printed = vec![(
                at,
                TempoText {
                    metronome: marking.metronome,
                    text: marking.text.clone(),
                },
            )];
            // A gradual change prints as its word where it starts and as the
            // speed it reached where it ends. No format has a continuous
            // tempo — none of the four can draw a *rit.* as a function — and
            // a mark at each end is what an engraver writes for the same
            // reason. The shape stays in the score for the performance to
            // integrate; the page says what a reader needs.
            if let (Some(ramp), Some(mark)) = (marking.ramp.as_ref(), marking.metronome)
                && let Some(bpm) = ramp.to
            {
                printed.push((
                    at + ramp.over,
                    TempoText {
                        metronome: Some(Metronome { beat: mark.beat, bpm }),
                        text: None,
                    },
                ));
            }
            printed
        })
        .filter_map(|(at, text)| Some(positioned(bars, fold.at(at)?, text)))
        .collect()
}

/// Where the barlines fall **on the page**.
///
/// The snapshot's own [`ScoreSnapshot::bars`] is built over performed time; a
/// repeat prints once and plays twice, so a meter change after a repeat sits
/// at a different measure on the page than in the performance. Folding the
/// changes before folding them into barlines is the whole difference, and it
/// is why the two coordinates are two values rather than one with a flag.
///
/// A change the page does not print — one inside a stretch a repeat swallows
/// — is dropped, as is one that does not land on a page barline. Both are
/// conditions the compiler refuses; rendering has to stay total.
fn page_bars(score: &ScoreSnapshot, fold: &Fold, scope: Scope) -> BarLines {
    let mut changes = score.meters().changes(scope);
    let opening = changes.next().map_or_else(Meter::default, |(_, meter)| *meter);
    let mut bars = BarLines::uniform(opening);
    for (at, meter) in changes {
        if let Some(printed) = fold.at(at) {
            let _ = bars.change(printed, *meter);
        }
    }
    bars
}

/// Where a positioned symbol falls, in the coordinates the backends use.
pub(super) fn positioned<T>(bars: &BarLines, at: MusicalTime, what: T) -> PositionedMark<T> {
    let position = bars.at(at);
    PositionedMark {
        measure: position.measure,
        onset_in_measure: position.into,
        what,
    }
}

pub(super) fn key_signature(key: Key) -> KeySignature {
    KeySignature {
        fifths: key.fifths(),
        mode: key.mode(),
    }
}
