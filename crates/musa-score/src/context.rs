//! What is in force, and from where.
//!
//! Key, meter and clef were three mechanisms — two scalars and a field on a
//! part — for one question: *what is in force here*. None of them could say
//! "here", because none of them had a location. This module is the shape they
//! share: a value, a scope, and the instant it starts at, projected out of
//! the timeline that already stated all three as occurrences.
//!
//! Consumers need **boundaries**, not points. "Where does the meter change"
//! is what every exporter asks, and neither `covering` (D10) nor `prevailing`
//! (D11) answers it — they answer about an instant. So the projection is a
//! track, not a query, and the per-instant queries stay the kernel's.
//!
//! Two invariants live here rather than in the callers:
//!
//! - **A context change belongs to a place in the piece, not to material.** A
//!   body elaborated once and referenced many times (`elaborate`'s `Share`)
//!   sits at several absolute times, so a key written inside it would be in
//!   force at several places that have nothing to do with each other. Context
//!   statements inside motif and bar bodies are therefore forbidden (§2:
//!   motif definition ≠ its expansions).
//! - **A track is built over one time coordinate.** Notation folds repeats
//!   (`musa-notation`'s `Fold`); performance does not. A track read on the
//!   wrong side of that fold answers about the wrong measure. Nothing can
//!   violate either rule yet — there is one stretch per kind — and they are
//!   written down so the change that makes context positional has something
//!   to enforce rather than something to discover.

use serde::{Deserialize, Serialize};

use crate::scope::{ContextKind, Inheritance, Scope, innermost};
use crate::time::MusicalTime;

/// One value in force from one instant, in one scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Stretch<V> {
    scope: Scope,
    start: MusicalTime,
    value: V,
}

/// Everything one kind of context says about a piece.
///
/// Ascending by start within each scope, and read through the inheritance
/// rule the kind declares — so `at` answers what a *reader of this scope*
/// would see, not what some scope happened to write last.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextTrack<V> {
    kind: ContextKind,
    stretches: Vec<Stretch<V>>,
}

impl<V: Clone> ContextTrack<V> {
    /// An empty track of one kind. The kind carries the inheritance rule, so
    /// no caller ever names the rule.
    pub(crate) fn new(kind: ContextKind) -> Self {
        Self {
            kind,
            stretches: Vec::new(),
        }
    }

    /// State a value from an instant, in a scope.
    ///
    /// Called in canonical order (N2: start, then end, then payload key), so
    /// the stretches arrive sorted and stay that way.
    pub fn state(&mut self, scope: Scope, start: MusicalTime, value: V) {
        self.stretches.push(Stretch { scope, start, value });
    }

    /// What a reader in `scope` sees at `at`, or `None` when nothing has been
    /// said yet — a piece that names no key has none, and saying so is more
    /// use than inventing C major.
    pub fn at(&self, scope: Scope, at: MusicalTime) -> Option<&V> {
        match self.kind.inheritance() {
            Inheritance::Override => {
                let winner = scope
                    .chain()
                    .find(|inner| self.stretches.iter().any(|stretch| stretch.scope == *inner))?;
                self.latest(at, |stretch| stretch.scope == winner)
            }
            Inheritance::Latest => self.latest(at, |stretch| stretch.scope.contains(scope)),
        }
    }

    /// Every change a reader in `scope` sees, in time order, beginning with
    /// the value in force at the start.
    ///
    /// This is what an exporter writes barline by barline. It is empty
    /// exactly when [`Self::at`] is `None` everywhere.
    pub fn changes(&self, scope: Scope) -> impl Iterator<Item = (MusicalTime, &V)> + '_ {
        let winner = match self.kind.inheritance() {
            Inheritance::Override => scope
                .chain()
                .find(|inner| self.stretches.iter().any(|stretch| stretch.scope == *inner)),
            Inheritance::Latest => None,
        };
        self.visible(scope, winner)
            .map(|stretch| (stretch.start, &stretch.value))
    }

    /// Whether nothing ever changes anywhere — the case every exporter has a
    /// fast path for, and the reason those fast paths keep working while the
    /// general path is still unreachable from the grammar.
    pub fn is_constant(&self) -> bool {
        self.stretches.len() <= 1
    }

    /// The stretches a reader in `scope` sees, in time order; `winner` is the
    /// single scope an `Override` kind reads, and `None` means read the whole
    /// chain.
    fn visible(&self, scope: Scope, winner: Option<Scope>) -> impl Iterator<Item = &Stretch<V>> + '_ {
        self.stretches.iter().filter(move |stretch| match winner {
            Some(only) => stretch.scope == only,
            None => stretch.scope.contains(scope),
        })
    }

    /// The last stretch to begin at or before `at` among those `wanted`
    /// admits, with the innermost scope winning a tie.
    fn latest(&self, at: MusicalTime, wanted: impl Fn(&Stretch<V>) -> bool) -> Option<&V> {
        self.stretches
            .iter()
            .filter(|stretch| stretch.start <= at && wanted(stretch))
            .max_by(|left, right| {
                left.start
                    .cmp(&right.start)
                    .then_with(|| innermost(left.scope, right.scope))
            })
            .map(|stretch| &stretch.value)
    }
}

#[cfg(test)]
mod tests {
    use num_rational::Ratio;

    use super::ContextTrack;
    use crate::scope::{ContextKind, Scope};
    use crate::time::MusicalTime;

    const VIOLA: Scope = Scope::Part { part: 1 };

    fn at(whole_notes: i64) -> MusicalTime {
        MusicalTime::new(Ratio::from_integer(whole_notes))
    }

    #[test]
    fn a_key_written_for_a_part_still_follows_the_pieces_modulation() {
        // The counterexample the `Latest` rule exists for: a viola part in
        // its own key at bar 1 must not be dragged back by the piece's key,
        // but must hear the piece modulate at bar 60.
        let mut keys = ContextTrack::new(ContextKind::Key);
        keys.state(Scope::Piece, at(0), "c");
        keys.state(VIOLA, at(0), "f");
        keys.state(Scope::Piece, at(60), "a");
        assert_eq!(keys.at(VIOLA, at(0)).copied(), Some("f"));
        assert_eq!(keys.at(VIOLA, at(59)).copied(), Some("f"));
        assert_eq!(keys.at(VIOLA, at(60)).copied(), Some("a"));
        assert_eq!(keys.at(Scope::Piece, at(0)).copied(), Some("c"));
    }

    #[test]
    fn a_parts_clef_is_never_reached_by_the_piece() {
        // The counterexample the `Override` rule exists for: a piece-level
        // clef change must not move a part that stated its own.
        let mut clefs = ContextTrack::new(ContextKind::Clef);
        clefs.state(Scope::Piece, at(0), "treble");
        clefs.state(VIOLA, at(0), "alto");
        clefs.state(Scope::Piece, at(60), "bass");
        assert_eq!(clefs.at(VIOLA, at(0)).copied(), Some("alto"));
        assert_eq!(clefs.at(VIOLA, at(60)).copied(), Some("alto"));
        assert_eq!(clefs.at(Scope::Piece, at(60)).copied(), Some("bass"));
    }

    #[test]
    fn a_lane_in_its_own_meter_does_not_rejoin_the_pieces() {
        // Polymeter, stated as an inheritance rule rather than a feature: a
        // 7/8 lane does not come back to 4/4 at the piece's next barline.
        let lane = Scope::Voice { part: 0, voice: 1 };
        let mut meters = ContextTrack::new(ContextKind::Meter);
        meters.state(Scope::Piece, at(0), (4, 4));
        meters.state(lane, at(0), (7, 8));
        meters.state(Scope::Piece, at(8), (3, 4));
        assert_eq!(meters.at(lane, at(8)).copied(), Some((7, 8)));
        assert_eq!(meters.at(Scope::Piece, at(8)).copied(), Some((3, 4)));
    }

    #[test]
    fn a_scope_that_says_nothing_reads_the_scope_that_does() {
        let mut clefs = ContextTrack::new(ContextKind::Clef);
        clefs.state(Scope::Piece, at(0), "treble");
        assert_eq!(clefs.at(VIOLA, at(0)).copied(), Some("treble"));
        let mut keys = ContextTrack::new(ContextKind::Key);
        keys.state(Scope::Piece, at(0), "c");
        assert_eq!(keys.at(VIOLA, at(0)).copied(), Some("c"));
    }

    #[test]
    fn nothing_is_in_force_before_the_first_thing_said() {
        let mut keys = ContextTrack::new(ContextKind::Key);
        keys.state(Scope::Piece, at(4), "c");
        assert_eq!(keys.at(Scope::Piece, at(0)), None);
        assert_eq!(keys.at(Scope::Piece, at(4)).copied(), Some("c"));
        assert_eq!(
            ContextTrack::<&str>::new(ContextKind::Key).at(Scope::Piece, at(0)),
            None
        );
    }

    #[test]
    fn a_constant_track_reports_exactly_one_change() {
        let mut meters = ContextTrack::new(ContextKind::Meter);
        assert!(meters.is_constant(), "an empty track never changes");
        meters.state(Scope::Piece, at(0), (4, 4));
        assert!(meters.is_constant());
        assert_eq!(
            meters
                .changes(Scope::Piece)
                .map(|(at, value)| (at, *value))
                .collect::<Vec<_>>(),
            vec![(at(0), (4, 4))]
        );
        meters.state(Scope::Piece, at(8), (3, 4));
        assert!(!meters.is_constant());
        assert_eq!(
            meters
                .changes(Scope::Piece)
                .map(|(at, value)| (at, *value))
                .collect::<Vec<_>>(),
            vec![(at(0), (4, 4)), (at(8), (3, 4))]
        );
    }
}
