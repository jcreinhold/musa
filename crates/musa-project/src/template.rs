//! Starting points for a new project.

/// What a newly created project contains.
///
/// A new file is never blank by default: an empty document gives a beginner
/// nothing to modify and gives the score view nothing to draw. The default
/// template is a piece that already compiles and already sounds
/// (`docs/rules/desktop/05-states.md` §2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Template {
    /// One part, one voice, four notes — a piece that compiles as written.
    #[default]
    Piece,
    /// Nothing at all, for callers that will immediately set the source.
    Empty,
}

impl Template {
    /// The source text this template starts from. `title` names the piece.
    pub(crate) fn source(self, title: &str) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Piece => format!(
                "piece {title:?} {{\n    \
                     tempo quarter = 96;\n    \
                     meter 4/4;\n    \
                     key c major;\n\n    \
                     score {{\n        \
                         part piano {{\n            \
                             clef treble;\n\n            \
                             voice upper {{\n                \
                                 c4/4\n                \
                                 e4/4\n                \
                                 g4/4\n                \
                                 c5/4\n            \
                             }}\n        \
                         }}\n    \
                     }}\n\
                 }}\n"
            ),
        }
    }
}
