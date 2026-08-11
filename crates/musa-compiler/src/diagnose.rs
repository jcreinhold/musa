//! What a diagnostic is made of.
//!
//! A musa diagnostic is a small document, not a sentence: a stable name, a
//! claim, the place, the other places that explain the place, one line of
//! advice, and — where there is no guesswork — the edit that resolves it.
//!
//! The shape is the discipline. A single string forces every message to carry
//! its whole explanation in a clause, and gives it nowhere to put the *second*
//! span that would make it obvious; that is why the messages this replaced all
//! read like a parser talking to itself. With somewhere for each part to go,
//! the prose gets shorter.
//!
//! Rendering lives at the edges — `miette` in the CLI, type in the app. This
//! module has no opinion about colour.

use crate::origin::SourceSpan;

/// A diagnostic's stable name.
///
/// Kebab-case rather than a number, because `unknown-motif` is both a lookup
/// key and an explanation, and it survives being read aloud. Codes are part of
/// the interface once shipped: `musa explain <code>` takes one, and a composer
/// may have written one down.
///
/// Deliberately *not* `#[non_exhaustive]`: `musa_project::explain` matches on
/// every variant, so adding a code without writing its explanation fails to
/// compile. That is the cheapest possible guard against a code that ships with
/// nothing behind it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Code {
    /// The source is not shaped like musa.
    Syntax,
    /// A name that nothing declares.
    UnknownName,
    /// Two declarations of one name.
    DuplicateName,
    /// A word musa knows the category of but not the value: a clef, an
    /// articulation, a dynamic, a chord quality.
    UnknownWord,
    /// A pitch, duration, meter, or key that will not parse as one.
    NotAValue,
    /// A number outside the range its parameter allows, or carrying the wrong
    /// unit.
    OutOfRange,
    /// A statement that cannot be where it is.
    Misplaced,
    /// The music does not add up: a voice that is not a whole number of
    /// measures, a bar that overflows its meter.
    DoesNotAddUp,
    /// A `use` that cannot be read, resolved, or is not a library.
    Import,
    /// A studio graph that cannot be built: an unwired input, a cycle, a send
    /// to nothing.
    Studio,
    /// Something was skipped, and the piece is still playable without it.
    Ignored,
    /// An elaboration expression has a different type than its context requires.
    TypeMismatch,
    /// A function call supplies too many, too few, repeated, or unknown arguments.
    WrongArity,
    /// Named elaboration definitions form a recursive dependency cycle.
    DependencyCycle,
    /// A finite elaboration exceeds a deterministic compilation resource limit.
    ResourceLimit,
    /// A finite match leaves an inhabitant of its scrutinee type uncovered.
    NonExhaustiveMatch,
    /// A match arm can never be selected because an earlier arm covers it.
    UnreachablePattern,
    /// Valid language syntax belonging to a compiler stage that is not
    /// implemented yet, rather than being mistaken for a parse error.
    UnsupportedLanguageStage,
    /// A `motif` or `fragment` that nothing uses (style guide §1). A named
    /// `bar` is not checked: it plays where it stands, so its name is an
    /// address, not a promise of reuse.
    UnusedMaterial,
    /// A `patch` no `assign` connects to a part (style guide §1).
    UnassignedPatch,
    /// A tempo, meter, or key marking that states the value already in
    /// force (style guide §2).
    RedundantMarking,
    /// A bar written out identically three or more times in one voice, where
    /// a `motif` would say it once (style guide §4).
    CopiedBars,
    /// An `assert` whose claim the music does not meet. Never raised on its
    /// own: something has to have been claimed, in writing, for the compiler
    /// to have anything to disprove.
    UnmetClaim,
    /// A well-formed kernel document whose payload type this build has no
    /// implementation for. The file is right; the reader is short.
    UnsupportedPayload,
}

impl Code {
    /// The code as it is written and typed.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Syntax => "syntax",
            Self::UnknownName => "unknown-name",
            Self::DuplicateName => "duplicate-name",
            Self::UnknownWord => "unknown-word",
            Self::NotAValue => "not-a-value",
            Self::OutOfRange => "out-of-range",
            Self::Misplaced => "misplaced",
            Self::DoesNotAddUp => "does-not-add-up",
            Self::Import => "import",
            Self::Studio => "studio",
            Self::Ignored => "ignored",
            Self::TypeMismatch => "type-mismatch",
            Self::WrongArity => "wrong-arity",
            Self::DependencyCycle => "dependency-cycle",
            Self::ResourceLimit => "resource-limit",
            Self::NonExhaustiveMatch => "non-exhaustive-match",
            Self::UnreachablePattern => "unreachable-pattern",
            Self::UnsupportedLanguageStage => "unsupported-language-stage",
            Self::UnusedMaterial => "unused-material",
            Self::UnassignedPatch => "unassigned-patch",
            Self::RedundantMarking => "redundant-marking",
            Self::CopiedBars => "copied-bars",
            Self::UnmetClaim => "unmet-claim",
            Self::UnsupportedPayload => "unsupported-payload",
        }
    }

    /// Every code, for `musa explain` with no argument and for the tests that
    /// keep the explanation table honest.
    pub const ALL: [Self; 24] = [
        Self::Syntax,
        Self::UnknownName,
        Self::DuplicateName,
        Self::UnknownWord,
        Self::NotAValue,
        Self::OutOfRange,
        Self::Misplaced,
        Self::DoesNotAddUp,
        Self::Import,
        Self::Studio,
        Self::Ignored,
        Self::TypeMismatch,
        Self::WrongArity,
        Self::DependencyCycle,
        Self::ResourceLimit,
        Self::NonExhaustiveMatch,
        Self::UnreachablePattern,
        Self::UnsupportedLanguageStage,
        Self::UnusedMaterial,
        Self::UnassignedPatch,
        Self::RedundantMarking,
        Self::CopiedBars,
        Self::UnmetClaim,
        Self::UnsupportedPayload,
    ];

    /// Parse a code back from its written form.
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|code| code.as_str() == text)
    }
}

impl std::fmt::Display for Code {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.as_str())
    }
}

/// One place, and what is true about it.
///
/// The primary label is where the reader should look first and the only one
/// an interface has to be able to show. Secondary labels are the *other*
/// places — the first declaration of a duplicated name, the meter a bar
/// overflows — and a diagnostic that has one is usually a diagnostic that
/// needed one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    /// Where.
    pub span: SourceSpan,
    /// What is wrong *here*, in a few words and lower case.
    ///
    /// Never the message again: the message says what is wrong with the
    /// piece, the label says what is wrong at this character. Repeating one in
    /// the other wastes the reader's second glance.
    pub text: String,
    /// Whether this is the place to jump to.
    pub primary: bool,
}

/// An edit that resolves a diagnostic, offered only when it is certain.
///
/// "Add the missing `;`" is certain. "Did you mean `sigh`?" is not, unless one
/// candidate stands alone — a wrong fix that applies in one keystroke is worse
/// than no fix, so an uncertain suggestion goes in `help` and stays prose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fix {
    /// What applying it does, phrased as the action: `add ';'`.
    pub title: String,
    /// Where, and what to put there. Byte ranges in the document that
    /// produced the diagnostic.
    pub edits: Vec<FixEdit>,
}

/// One replacement inside a [`Fix`].
///
/// Deliberately not `musa_language::TextEdit`: the compiler does not depend on
/// the CST for this, and a diagnostic crossing to the app must not drag Rowan's
/// vocabulary with it (roadmap §10.6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixEdit {
    /// The bytes to replace. An empty range inserts.
    pub span: SourceSpan,
    /// What to put there. Empty deletes.
    pub replacement: String,
}

/// Diagnostic severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// Compilation failed for this construct; the snapshot may be absent.
    Error,
    /// Something was skipped or is suspicious; compilation continues.
    Warning,
}

/// A semantic diagnostic.
///
/// Built with the `error`/`warning` constructors and the chained `at`, `also`,
/// `help`, `note`, and `fix` methods, in that order — which is also the order
/// they are read in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// How bad it is.
    pub severity: Severity,
    /// Its stable name.
    pub code: Code,
    /// What is wrong. One line, lower case, no trailing period, no apology.
    pub message: String,
    /// Where. The primary label, when there is one, is first.
    pub labels: Vec<Label>,
    /// What to do about it.
    pub help: Option<String>,
    /// The rule behind it, where knowing the rule prevents the next one.
    pub note: Option<String>,
    /// Edits that resolve it without guesswork.
    pub fixes: Vec<Fix>,
}

impl Diagnostic {
    /// A diagnostic that stops compilation of its construct.
    pub(crate) fn error(code: Code, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, message)
    }

    /// A diagnostic that does not.
    pub(crate) fn warning(code: Code, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, code, message)
    }

    fn new(severity: Severity, code: Code, message: impl Into<String>) -> Self {
        Self {
            severity,
            code,
            message: message.into(),
            labels: Vec::new(),
            help: None,
            note: None,
            fixes: Vec::new(),
        }
    }

    /// Set the place to look first.
    #[must_use]
    pub(crate) fn at(mut self, span: SourceSpan, text: impl Into<String>) -> Self {
        self.labels.insert(
            0,
            Label {
                span,
                text: text.into(),
                primary: true,
            },
        );
        self
    }

    /// Set the place to look first, when the caller has one.
    ///
    /// The studio's diagnostics all carry `Option<SourceSpan>` because a
    /// graph is checked after it is built and some of what is checked has no
    /// single node behind it.
    #[must_use]
    pub(crate) fn maybe_at(self, span: Option<SourceSpan>, text: impl Into<String>) -> Self {
        match span {
            Some(span) => self.at(span, text),
            None => self,
        }
    }

    /// Add another place that explains the first.
    #[must_use]
    pub(crate) fn also(mut self, span: SourceSpan, text: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            text: text.into(),
            primary: false,
        });
        self
    }

    /// Add another place, when there is one to add.
    ///
    /// The `Option` is here because most callers with a second span got it
    /// from a lookup that may have missed, and `if let` around a builder chain
    /// reads worse than the thing it guards.
    #[must_use]
    pub(crate) fn maybe_also(self, span: Option<SourceSpan>, text: impl Into<String>) -> Self {
        match span {
            Some(span) => self.also(span, text),
            None => self,
        }
    }

    /// Say what to do about it.
    #[must_use]
    pub(crate) fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Say what to do about it, when there is something to say.
    ///
    /// The `Option` is here for the same reason as [`Self::maybe_also`]: the
    /// advice usually comes from a near-miss search that may have found
    /// nothing, and a diagnostic with no suggestion is better than one that
    /// suggests the wrong word.
    #[must_use]
    pub(crate) fn maybe_help(self, help: Option<impl Into<String>>) -> Self {
        match help {
            Some(help) => self.help(help),
            None => self,
        }
    }

    /// State the rule.
    #[must_use]
    pub(crate) fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Offer an edit. Only when it is certain — see [`Fix`].
    #[must_use]
    pub(crate) fn fix(mut self, title: impl Into<String>, span: SourceSpan, replacement: impl Into<String>) -> Self {
        self.fixes.push(Fix {
            title: title.into(),
            edits: vec![FixEdit {
                span,
                replacement: replacement.into(),
            }],
        });
        self
    }

    /// Where an interface should put the caret: the primary label, or the
    /// first label of any kind, or nowhere.
    pub fn primary_span(&self) -> Option<SourceSpan> {
        self.labels
            .iter()
            .find(|label| label.primary)
            .or_else(|| self.labels.first())
            .map(|label| label.span)
    }
}

/// The nearest name to `written` among `candidates`, when one stands out.
///
/// Damerau-Levenshtein, because the mistakes people make at a keyboard are
/// substitutions and transpositions in roughly equal measure and plain
/// Levenshtein charges two for a swap. Two guards keep it from inventing
/// advice: the distance may not exceed a third of the typed name's length, so
/// short names are held to a stricter standard than long ones, and a tie
/// between two candidates suggests neither, because a coin flip printed as
/// help is worse than silence.
pub(crate) fn nearest<'a>(written: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let budget = (written.chars().count() / 3).clamp(1, 3);
    let mut best: Option<(usize, &str)> = None;
    let mut tied = false;
    for candidate in candidates {
        let distance = edit_distance(written, candidate);
        if distance > budget {
            continue;
        }
        match best {
            Some((shortest, _)) if distance > shortest => {}
            Some((shortest, _)) if distance == shortest => tied = true,
            _ => {
                best = Some((distance, candidate));
                tied = false;
            }
        }
    }
    if tied { None } else { best.map(|(_, name)| name) }
}

/// Damerau-Levenshtein distance over characters, restricted to adjacent
/// transpositions.
///
/// Three rolling rows rather than a full table: the restricted variant looks
/// back exactly two rows, so that is all there is to keep.
fn edit_distance(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    if left.is_empty() {
        return right.len();
    }
    if right.is_empty() {
        return left.len();
    }
    let width = right.len().saturating_add(1);
    let mut two_above: Vec<usize> = vec![0; width];
    let mut above: Vec<usize> = (0..width).collect();
    let mut current: Vec<usize> = vec![0; width];
    // Reading off the end is impossible given the loop bounds; the fallback
    // is a value large enough never to win a `min` rather than a panic.
    let read = |row: &[usize], column: usize| row.get(column).copied().unwrap_or(usize::MAX / 2);
    for row in 1..=left.len() {
        let here = left.get(row.saturating_sub(1)).copied();
        if let Some(cell) = current.first_mut() {
            *cell = row;
        }
        for column in 1..=right.len() {
            let there = right.get(column.saturating_sub(1)).copied();
            let cost = usize::from(here != there);
            let mut best = read(&current, column.saturating_sub(1))
                .saturating_add(1)
                .min(read(&above, column).saturating_add(1))
                .min(read(&above, column.saturating_sub(1)).saturating_add(cost));
            // The transposition case: `sigh` for `sihg` is one mistake, and
            // charging two for it is what makes plain Levenshtein miss the
            // suggestion a reader would call obvious.
            if row > 1
                && column > 1
                && here == right.get(column.saturating_sub(2)).copied()
                && left.get(row.saturating_sub(2)).copied() == there
            {
                best = best.min(read(&two_above, column.saturating_sub(2)).saturating_add(1));
            }
            if let Some(cell) = current.get_mut(column) {
                *cell = best;
            }
        }
        // `above` becomes the row just finished, and the row it replaces
        // becomes the scratch space for the next one.
        std::mem::swap(&mut two_above, &mut above);
        std::mem::swap(&mut above, &mut current);
    }
    read(&above, right.len())
}

#[cfg(test)]
mod tests {
    use super::{Code, edit_distance, nearest};

    #[test]
    fn a_transposition_costs_one() {
        assert_eq!(edit_distance("sigh", "sihg"), 1);
        assert_eq!(edit_distance("sigh", "sigh"), 0);
        assert_eq!(edit_distance("", "abc"), 3);
    }

    #[test]
    fn suggests_the_obvious_neighbour() {
        assert_eq!(nearest("sigh", ["sight", "chorus"]), Some("sight"));
        assert_eq!(nearest("theeme", ["theme", "chorus"]), Some("theme"));
    }

    #[test]
    fn says_nothing_when_two_are_equally_close() {
        // `bar` is one edit from both. Printing either would be a coin flip
        // with a confident voice.
        assert_eq!(nearest("bar", ["car", "far"]), None);
    }

    #[test]
    fn says_nothing_when_nothing_is_close() {
        assert_eq!(nearest("trombone", ["c5", "lead"]), None);
        // A short name gets a budget of one, so this is not a neighbour.
        assert_eq!(nearest("am", ["e7"]), None);
    }

    #[test]
    fn every_code_round_trips_through_its_text() {
        for code in Code::ALL {
            assert_eq!(Code::parse(code.as_str()), Some(code));
        }
        assert_eq!(Code::parse("not-a-code"), None);
    }
}
