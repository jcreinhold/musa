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
///
/// [`Self::as_str`] and [`Self::ALL`] are generated together, from the table
/// below the enum, for the same reason. The roster used to be written out a
/// second time by hand, and `expansion` was left out of it: the code existed,
/// raised, and had an explanation written for it, but [`Self::parse`] only
/// knows the codes the roster names, so `musa explain expansion` answered
/// "no such code". A variant the table omits now fails to compile at
/// `as_str`'s match instead of shipping unreachable.
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
    /// An adapter region that cannot be resolved, expanded, or whose answer is
    /// not one ordinary expression.
    Expansion,
    /// Elaboration left a hole nothing in the program determined.
    UnsolvedMetavariable,
    /// Two types elaboration had to make equal are not.
    ///
    /// Distinct from [`Self::TypeMismatch`], which the rank-1 checker raises
    /// about types the author wrote. This one can name a normal form nobody
    /// wrote, which is why it has its own explanation and its own policy on how
    /// much of one to print.
    ConversionMismatch,
    /// A `data` declaration whose constructor mentions the family being
    /// declared somewhere strict positivity does not allow.
    NonPositiveOccurrence,
    /// A dependent `match` that leaves a constructor of the family it splits
    /// on with no branch.
    ///
    /// Distinct from [`Self::NonExhaustiveMatch`], which the rank-1 checker
    /// raises about a finite match over a closed set of literals and shapes.
    /// This one is decided while compiling the match to a recursor, so what it
    /// names is a constructor of an inductive family.
    IncompleteMatch,
    /// A dependent `match` arm an earlier arm already covers.
    ///
    /// Distinct from [`Self::UnreachablePattern`] for the same reason
    /// [`Self::IncompleteMatch`] is distinct from
    /// [`Self::NonExhaustiveMatch`]: it is a property of the case tree, not of
    /// the rank-1 pattern list.
    UnreachableBranch,
    /// A `match` whose scrutinee's index is not a distinct variable, which is
    /// the one shape index refinement is defined for.
    ForcedIndex,
    /// A recursive call the termination rule cannot see is smaller.
    UncheckedRecursion,
    /// A name that exists and is `private` to the module that declares it.
    ///
    /// Distinct from [`Self::UnknownName`] on purpose: the whole value of the
    /// marker is in telling a reader that the thing they wrote is real and
    /// maintained somewhere else, which sends them to an interface rather than
    /// looking for a typo.
    PrivateName,
    /// An enum with a `private` case beside a public one.
    MixedVisibility,
    /// A `match` on a type whose constructors are private here.
    AbstractMatch,
    /// A `trait` named `Storable`, whose instances the elaborator generates.
    ReservedClass,
    /// A `trait` with no parameters, so nothing can be an instance of it.
    HeadlessClass,
    /// A `trait` declaring one method name twice.
    DuplicateMethod,
    /// A trait applied to the wrong number of arguments.
    ClassArity,
    /// A source `impl Storable`, behind any spelling.
    HandWrittenStorable,
    /// An `impl` whose head argument is a bare type variable.
    BlanketInstance,
    /// A second `impl` for a key another already answers.
    DuplicateInstance,
    /// An `impl` in a package that declares neither its trait nor its head.
    OrphanInstance,
    /// An instance whose context the termination measure cannot see decrease.
    UnboundedInstance,
    /// An `impl` supplying a method its trait derives.
    DerivedMethod,
    /// An `impl` supplying a method its trait does not declare.
    NoSuchMethod,
    /// An `impl` leaving a required method undefined.
    MissingMethod,
    /// A constraint no instance and no enclosing `where` answers.
    UnresolvedInstance,
    /// A constraint on a type variable no enclosing `where` supplies.
    UnconstrainedVariable,
    /// A constraint on a type no instance could ever be keyed on.
    UnkeyedConstraint,
    /// `x.m(…)` where `x`'s type is not a declared type constructor.
    MethodOnVariable,
    /// `x.m(…)` where no trait with a dictionary at `x`'s head declares `m`.
    NoMethodForType,
    /// `x.m(…)` where two traits with a dictionary at `x`'s head declare `m`.
    AmbiguousMethod,
    /// A method, field, or case whose name repeats the declaration it belongs to.
    RedundantNamePrefix,
    /// A `where` clause naming one constraint twice.
    DuplicateConstraint,
    /// A `$…` splice whose value is not of the category its position demands.
    ///
    /// Distinct from [`Self::TypeMismatch`] because both sides are `Syntax`
    /// and what differs is the index — a claim about how a tree parses, not a
    /// type the author wrote out — so the report names two categories rather
    /// than two types, and its repair is a parse rather than an annotation.
    SpliceCategory,
    /// A `$..xs` written where the grammar admits one node and not a run.
    UnspreadSequence,
    /// A name written literally in a quote that the printer's own renaming
    /// could produce, so a binder the quote introduces would capture it.
    QuotedCapture,
    /// Two `$..xs` written among one group's children, which would make where
    /// the first run ends a guess.
    AmbiguousSpread,
    /// A quote pattern written against a value that has no category.
    ///
    /// Distinct from [`Self::SpliceCategory`], which is a splice of the wrong
    /// category into a position that has one: here there is no category at
    /// all, because the value being matched is not syntax.
    PatternCategory,
    /// A name a quote pattern wrote literally, used in the arm as though the
    /// pattern had bound it.
    QuotedLiteralName,
}

/// Writes each code's spelling once, and derives the roster from the same
/// line.
///
/// The `match` that `as_str` expands to has to cover [`Code`], so the table
/// cannot be short; `ALL` is built from that table, so the roster cannot
/// disagree with it. Two lists that had to be kept in step by hand is how
/// `expansion` came to be a code nothing could look up.
macro_rules! code_table {
    ($($variant:ident => $text:literal,)+) => {
        impl Code {
            /// The code as it is written and typed.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)+
                }
            }

            /// Every code, in the order the enum declares them, for
            /// `musa explain` with no argument and for the tests that keep the
            /// explanation table honest.
            pub const ALL: [Self; [$(code_table!(@unit $variant)),+].len()] = [$(Self::$variant,)+];
        }
    };
    // One `()` per line of the table, so `ALL` is as long as the table is.
    (@unit $variant:ident) => {
        ()
    };
}

code_table! {
    Syntax => "syntax",
    UnknownName => "unknown-name",
    DuplicateName => "duplicate-name",
    UnknownWord => "unknown-word",
    NotAValue => "not-a-value",
    OutOfRange => "out-of-range",
    Misplaced => "misplaced",
    DoesNotAddUp => "does-not-add-up",
    Import => "import",
    Studio => "studio",
    Ignored => "ignored",
    TypeMismatch => "type-mismatch",
    WrongArity => "wrong-arity",
    DependencyCycle => "dependency-cycle",
    ResourceLimit => "resource-limit",
    NonExhaustiveMatch => "non-exhaustive-match",
    UnreachablePattern => "unreachable-pattern",
    UnsupportedLanguageStage => "unsupported-language-stage",
    UnusedMaterial => "unused-material",
    UnassignedPatch => "unassigned-patch",
    RedundantMarking => "redundant-marking",
    CopiedBars => "copied-bars",
    UnmetClaim => "unmet-claim",
    UnsupportedPayload => "unsupported-payload",
    Expansion => "expansion",
    UnsolvedMetavariable => "unsolved-metavariable",
    ConversionMismatch => "conversion-mismatch",
    NonPositiveOccurrence => "non-positive-occurrence",
    IncompleteMatch => "incomplete-match",
    UnreachableBranch => "unreachable-branch",
    ForcedIndex => "forced-index",
    UncheckedRecursion => "unchecked-recursion",
    PrivateName => "private-name",
    MixedVisibility => "mixed-visibility",
    AbstractMatch => "abstract-match",
    ReservedClass => "reserved-class",
    HeadlessClass => "headless-class",
    DuplicateMethod => "duplicate-method",
    ClassArity => "class-arity",
    HandWrittenStorable => "hand-written-storable",
    BlanketInstance => "blanket-instance",
    DuplicateInstance => "duplicate-instance",
    OrphanInstance => "orphan-instance",
    UnboundedInstance => "unbounded-instance",
    DerivedMethod => "derived-method",
    NoSuchMethod => "no-such-method",
    MissingMethod => "missing-method",
    UnresolvedInstance => "unresolved-instance",
    UnconstrainedVariable => "unconstrained-variable",
    UnkeyedConstraint => "unkeyed-constraint",
    MethodOnVariable => "method-on-variable",
    NoMethodForType => "no-method-for-type",
    AmbiguousMethod => "ambiguous-method",
    RedundantNamePrefix => "redundant-name-prefix",
    DuplicateConstraint => "duplicate-constraint",
    SpliceCategory => "splice-category",
    UnspreadSequence => "unspread-sequence",
    QuotedCapture => "quoted-capture",
    AmbiguousSpread => "ambiguous-spread",
    PatternCategory => "pattern-category",
    QuotedLiteralName => "quoted-literal-name",
}

impl Code {
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

/// A diagnostic about a document other than the one being compiled.
///
/// Spans in `labels` are spans in `document` and in no other file. That is
/// what keeps [`Diagnostic::remap_spans`] correct without a runtime check:
/// the source map moves the composer's own text, and a cause is not in it.
///
/// A cause carries no fixes and no causes of its own. No fixes for the reason
/// `remap_spans` already gives about generated text — an edit offered against
/// a file the composer cannot see would silently rewrite it — and no causes
/// because the only thing that produces one is reading an adapter module, and
/// a module may not import.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cause {
    /// The document, by the key the import resolved to.
    pub document: String,
    /// Its stable name, the same vocabulary as [`Diagnostic::code`].
    pub code: Code,
    /// What is wrong, in the checker's own words.
    pub message: String,
    /// Where, in `document`. Primary first, as in a diagnostic.
    pub labels: Vec<Label>,
    /// What to do about it.
    pub help: Option<String>,
    /// The rule behind it.
    pub note: Option<String>,
}

impl Cause {
    /// Restate one diagnostic as a fault in `document`.
    ///
    /// The fixes are dropped here rather than at the renderers, so no consumer
    /// has to know they were ever there.
    pub(crate) fn of(document: impl Into<String>, diagnostic: Diagnostic) -> Self {
        Self {
            document: document.into(),
            code: diagnostic.code,
            message: diagnostic.message,
            labels: diagnostic.labels,
            help: diagnostic.help,
            note: diagnostic.note,
        }
    }
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
/// `help`, `note`, `fix`, and `caused_by` methods, in that order — which is
/// also the order they are read in.
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
    /// Faults in *another* document that this one is the consequence of.
    ///
    /// Empty for nearly every diagnostic. What fills it is a compilation that
    /// had to check a second document to answer about this one — an adapter
    /// module named by an `import syntax` — where the whole of what the
    /// checker said about that document belongs to its author and none of it
    /// is a place in the composer's file.
    pub causes: Vec<Cause>,
}

impl Diagnostic {
    /// Move every place this points at back into the composer's own text.
    ///
    /// A fix is moved with the labels: an edit offered against generated text
    /// would silently rewrite a file the composer cannot see, which is worse
    /// than offering no fix at all. A fix that lands on a whole region says
    /// "replace this region", which is at least a place they can act on.
    ///
    /// [`Self::causes`] is left alone, and that is the rule rather than an
    /// omission: a cause's spans are already in the document it names, and
    /// this map describes only the composer's own text.
    pub(crate) fn remap_spans(&mut self, map: &crate::expand::SourceMap) {
        for label in &mut self.labels {
            label.span = map.span(label.span);
        }
        for fix in &mut self.fixes {
            for edit in &mut fix.edits {
                edit.span = map.span(edit.span);
            }
        }
    }

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
            causes: Vec::new(),
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

    /// Attach what a second document's own checker said, whole.
    ///
    /// Nothing is spliced into the message: the wrapper says which module and
    /// which import, and the causes say what is wrong inside it. Saying it
    /// twice is what `docs/rules/desktop/05-states.md` §5 forbids of a label,
    /// and a summarized first diagnostic is the same mistake with the other
    /// diagnostics missing as well.
    #[must_use]
    pub(crate) fn caused_by(mut self, causes: impl IntoIterator<Item = Cause>) -> Self {
        self.causes.extend(causes);
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
        // Reading the roster is sound here only because the roster and the
        // `as_str` match are one table: a variant missing from it stops the
        // crate compiling, so this loop cannot be short the way it once was.
        for code in Code::ALL {
            assert_eq!(Code::parse(code.as_str()), Some(code));
        }
        assert_eq!(Code::parse("not-a-code"), None);
    }

    #[test]
    fn no_two_codes_are_spelled_the_same() {
        // `parse` answers with the first match, so a spelling written twice
        // would make one of the two unreachable — the same failure a missing
        // roster entry caused, arriving by the one route the table still
        // leaves open.
        let mut seen = std::collections::BTreeSet::new();
        for code in Code::ALL {
            assert!(seen.insert(code.as_str()), "{code} shares a spelling with another code");
        }
    }

    #[test]
    fn expansion_is_a_code_you_can_look_up() {
        // For one stretch it was not: the enum, `as_str`, and the explanation
        // in musa-project all knew `expansion`, and only the hand-written
        // roster did not, so `musa explain expansion` said there was no such
        // code.
        assert_eq!(Code::parse("expansion"), Some(Code::Expansion));
        assert!(Code::ALL.contains(&Code::Expansion));
    }
}
