//! Where a fact sits in the score's *structure*, and how an inner scope reads
//! what an enclosing one said.
//!
//! Never where it sits in time — that is the occurrence's span, and keeping
//! the two apart is the point (docs/kernel/03). A slur moves in time without
//! changing voice; a voice is renamed without moving anything.
//!
//! This module owns the *inheritance table*: the one place that says what a
//! part inherits from its piece. It is a table rather than a rule because the
//! rules genuinely differ per kind, and inferring one from the other is the
//! mistake a uniform design makes — see [`ContextKind`].

/// One place in the score's structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Scope {
    /// The piece as a whole: key, meter, form markers, chord symbols.
    Piece,
    /// One part: its clef, and — once the grammar allows one — its own key.
    Part {
        /// The part's id, as the snapshot numbers them.
        part: u32,
    },
    /// One voice of one part, by the ids the snapshot uses.
    Voice {
        /// The part's id.
        part: u32,
        /// The voice's id within that part.
        voice: u32,
    },
}

impl Scope {
    /// The (part, voice) pair, for bucketing during projection, or `None`
    /// when the fact belongs to something wider than a voice.
    pub(crate) fn voice(self) -> Option<(u32, u32)> {
        match self {
            Self::Piece | Self::Part { .. } => None,
            Self::Voice { part, voice } => Some((part, voice)),
        }
    }

    /// This scope and every scope containing it, **innermost first**.
    ///
    /// The order is the one both inheritance rules read in, which is why it
    /// is fixed here rather than chosen per caller.
    pub fn chain(self) -> impl Iterator<Item = Self> {
        let rest = match self {
            Self::Piece => [None, None],
            Self::Part { .. } => [Some(Self::Piece), None],
            Self::Voice { part, .. } => [Some(Self::Part { part }), Some(Self::Piece)],
        };
        std::iter::once(self).chain(rest.into_iter().flatten())
    }

    /// Whether `inner` is this scope or lies inside it.
    pub fn contains(self, inner: Self) -> bool {
        inner.chain().any(|scope| scope == self)
    }

    /// How far in from the piece this scope is, for breaking a tie between
    /// two answers that begin at the same instant: the innermost wins.
    fn depth(self) -> u8 {
        match self {
            Self::Piece => 0,
            Self::Part { .. } => 1,
            Self::Voice { .. } => 2,
        }
    }
}

/// How a scope reads what an enclosing scope said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Inheritance {
    /// The innermost scope with anything to say wins outright, and the outer
    /// scopes are not consulted at all.
    Override,
    /// Every scope in the chain is flattened into one sequence and time order
    /// decides; an outer change after an inner one is heard.
    Latest,
}

/// Every kind of context musa tracks, and — the reason this enum exists —
/// how each one inherits.
///
/// A single rule gets key wrong, and the counterexample is an ordinary one: a
/// viola part written in a different key from the piece must not be dragged
/// back by the piece's key signature, but it *must* follow the piece's
/// modulation at bar 60. Clef is the opposite: a part's clef is the part's
/// business, and no piece-level clef should ever reach it. So the table is
/// data, declared once, and never a parameter a caller passes — every caller
/// would pass the same value, which is the signal that the default belongs
/// inside.
///
/// | Kind | Rule | Why |
/// | --- | --- | --- |
/// | `Clef` | `Override` | a part's clef is the part's business |
/// | `Meter` | `Override` | this *is* polymeter: a 7/8 lane does not rejoin 4/4 |
/// | `Key` | `Latest` | a part follows the piece's modulations |
/// | `Tempo` | `Override` | likewise polytempo: a lane at its own speed keeps it |
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ContextKind {
    /// The key signature.
    Key,
    /// The time signature.
    Meter,
    /// The clef a staff is read in.
    Clef,
    /// The tempo marking printed over the staff.
    Tempo,
}

impl ContextKind {
    /// The table.
    pub(crate) fn inheritance(self) -> Inheritance {
        match self {
            Self::Key => Inheritance::Latest,
            Self::Meter | Self::Clef | Self::Tempo => Inheritance::Override,
        }
    }
}

/// Order two answers that begin at the same instant: the innermost scope
/// wins, which is what makes `Latest` agree with `Override` whenever a part
/// states its own value at the same place the piece states one.
pub(crate) fn innermost(left: Scope, right: Scope) -> std::cmp::Ordering {
    left.depth().cmp(&right.depth())
}

#[cfg(test)]
mod tests {
    use super::Scope;

    #[test]
    fn a_chain_runs_from_the_voice_out_to_the_piece() {
        let voice = Scope::Voice { part: 1, voice: 2 };
        assert_eq!(
            voice.chain().collect::<Vec<_>>(),
            vec![voice, Scope::Part { part: 1 }, Scope::Piece]
        );
        assert_eq!(Scope::Piece.chain().collect::<Vec<_>>(), vec![Scope::Piece]);
    }

    #[test]
    fn containment_follows_the_chain_and_nothing_else() {
        let voice = Scope::Voice { part: 1, voice: 2 };
        assert!(Scope::Piece.contains(voice));
        assert!(Scope::Part { part: 1 }.contains(voice));
        assert!(voice.contains(voice));
        assert!(!Scope::Part { part: 0 }.contains(voice));
        assert!(!voice.contains(Scope::Part { part: 1 }));
    }
}
