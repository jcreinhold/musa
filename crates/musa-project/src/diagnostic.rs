//! Session-level diagnostics.
//!
//! A deliberate restatement of the compiler's diagnostic type rather than a
//! re-export: `musa-project` is the boundary at which compiler types stop
//! (roadmap §15.7), and a frontend that pattern-matched on
//! `musa_score::Severity` would be coupled to the compiler forever.
//!
//! The restatement adds one thing the compiler cannot supply: the line and
//! column of every label. The compiler measures the source in bytes because
//! that is what a parser has; a reader counts lines because that is what an
//! editor shows. Converting between the two needs the source text, which the
//! session has and the compiler was handed only borrowed (see
//! [`position`](crate::position)).

use crate::position::{Lines, Position};

/// How serious a diagnostic is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// The source does not compile.
    Error,
    /// Something was skipped or is suspicious; compilation continued.
    Warning,
}

/// A byte range in the current source text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub struct Span {
    /// First byte.
    pub start: u32,
    /// One past the last byte.
    pub end: u32,
}

/// One place a diagnostic points at, and what is true about it.
///
/// The primary label is first and is where the caret goes. Secondary labels
/// are the places that explain it — the first declaration of a duplicated
/// name, the meter a bar overflows — and are what let the message stay one
/// line.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    /// Where, in bytes on this side of the wire and UTF-16 code units on the
    /// other (see [`utf16`](crate::utf16)).
    pub span: Span,
    /// Where, as a person would say it. Never derived by the frontend.
    pub at: Position,
    /// What is wrong *here*, in a few words. Never the message again.
    pub text: String,
    /// Whether this is the place to jump to.
    pub primary: bool,
}

/// One place a *cause* points at, in the cause's own document.
///
/// Deliberately not a [`Label`]: that type documents its `span` as a byte
/// range in the current source text, and a byte range into a file the frontend
/// does not hold is a footgun with no use. This carries the two ends as
/// [`Position`]s and no span at all, so there is nothing to misuse.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CauseLabel {
    /// Where it begins, in the cause's document.
    ///
    /// `None` only when this compilation was not handed that document's text,
    /// which the session's own compilations never are. `at` and `to` are known
    /// together or not at all: both come from the same index over the same
    /// text.
    pub at: Option<Position>,
    /// Where it ends. See [`Self::at`].
    pub to: Option<Position>,
    /// What is wrong *here*, in the checker's few words.
    pub text: String,
    /// Whether this is the place the cause is chiefly about.
    pub primary: bool,
}

/// A diagnostic about a document other than the one being edited.
///
/// What produces one is a compilation that had to check a second file to
/// answer about this one — an adapter module named by an `import syntax`. The
/// composer cannot edit that file, so a cause is not a navigation target and
/// carries no fix; it says where in words (`docs/rules/desktop/05-states.md`
/// §5).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cause {
    /// The document, by the key its import resolved to. A path for a file, a
    /// `std::`-style URI for a bundled module.
    pub document: String,
    /// Its stable name, the same vocabulary as [`Diagnostic::code`].
    pub code: String,
    /// What is wrong, in the compiler's own words.
    pub message: String,
    /// Where, in `document`. Primary first.
    pub labels: Vec<CauseLabel>,
    /// What to do about it — advice for that document's author.
    pub help: Option<String>,
    /// The rule behind it.
    pub note: Option<String>,
}

/// An edit that resolves a diagnostic, offered only when it is certain.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fix {
    /// What applying it does, phrased as the action: ``add `;` ``.
    pub title: String,
    /// The replacements, in document order.
    pub edits: Vec<FixEdit>,
}

/// One replacement inside a [`Fix`].
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FixEdit {
    /// The bytes to replace. An empty range inserts.
    pub span: Span,
    /// What to put there. Empty deletes.
    pub replacement: String,
}

/// A problem with the source, positioned where the interface can show it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    /// How serious it is.
    pub severity: Severity,
    /// Its stable name, kebab-case: the argument to `musa explain`.
    pub code: String,
    /// What is wrong, in the interface's voice.
    pub message: String,
    /// Where, primary label first. Empty when the problem is the whole file.
    pub labels: Vec<Label>,
    /// What to do about it.
    pub help: Option<String>,
    /// The rule behind it.
    pub note: Option<String>,
    /// Edits that resolve it without guesswork.
    pub fixes: Vec<Fix>,
    /// Faults in another document that this one is the consequence of.
    ///
    /// Empty for nearly every diagnostic. Listed under it rather than folded
    /// into the message, because a set of diagnostics about a different file
    /// is not one sentence about this one.
    pub causes: Vec<Cause>,
    /// The primary label's span, repeated.
    ///
    /// Not redundant in practice: the editor's lint decorations want one
    /// range per diagnostic and should not have to know which label is
    /// primary, and every existing caller reads this field.
    pub span: Option<Span>,
}

impl Diagnostic {
    /// Restate a compiler diagnostic against the source it was produced from.
    ///
    /// `imports` is the same closed world the compilation was handed, and it
    /// is here for one reason: a cause's labels are places in *another*
    /// document, and turning a byte offset into a line and column needs that
    /// document's text. The frontend still derives nothing
    /// (`docs/rules/desktop/03-interaction.md` §7) — the positions arrive
    /// computed, in whichever file they belong to.
    pub(crate) fn from_compiler(
        diagnostic: &musa_score::Diagnostic,
        lines: &Lines<'_>,
        imports: &musa_compiler::ImportSources,
    ) -> Self {
        let labels: Vec<Label> = diagnostic
            .labels
            .iter()
            .map(|label| Label {
                span: span_of(label.span),
                at: lines.at(label.span.start),
                text: label.text.clone(),
                primary: label.primary,
            })
            .collect();
        Self {
            severity: match diagnostic.severity {
                musa_score::Severity::Error => Severity::Error,
                musa_score::Severity::Warning => Severity::Warning,
            },
            code: diagnostic.code.to_string(),
            message: diagnostic.message.clone(),
            span: diagnostic.primary_span().map(span_of),
            labels,
            help: diagnostic.help.clone(),
            note: diagnostic.note.clone(),
            fixes: diagnostic
                .fixes
                .iter()
                .map(|fix| Fix {
                    title: fix.title.clone(),
                    edits: fix
                        .edits
                        .iter()
                        .map(|edit| FixEdit {
                            span: span_of(edit.span),
                            replacement: edit.replacement.clone(),
                        })
                        .collect(),
                })
                .collect(),
            causes: diagnostic
                .causes
                .iter()
                .map(|cause| Cause::from_compiler(cause, imports))
                .collect(),
        }
    }

    /// The primary label, when the diagnostic has one.
    pub fn primary(&self) -> Option<&Label> {
        self.labels
            .iter()
            .find(|label| label.primary)
            .or_else(|| self.labels.first())
    }

    /// The single fix, when there is exactly one.
    ///
    /// An interface offering a control for "the fix" needs to know there is
    /// only one; offering the first of several is how an editor applies the
    /// wrong one.
    pub fn only_fix(&self) -> Option<&Fix> {
        match self.fixes.as_slice() {
            [fix] => Some(fix),
            _ => None,
        }
    }
}

impl Cause {
    /// Restate one cause against the document it is about.
    ///
    /// A document this compilation was not handed keeps its label texts and
    /// loses their positions, rather than being dropped: the composer still
    /// has to be told what the checker said, and a cause with no coordinates
    /// says less than a full one and much more than nothing.
    fn from_compiler(cause: &musa_score::Cause, imports: &musa_compiler::ImportSources) -> Self {
        let text = imports.get(&cause.document);
        let lines = text.map(Lines::new);
        Self {
            document: cause.document.clone(),
            code: cause.code.to_string(),
            message: cause.message.clone(),
            labels: cause
                .labels
                .iter()
                .map(|label| CauseLabel {
                    at: lines.as_ref().map(|lines| lines.at(label.span.start)),
                    to: lines.as_ref().map(|lines| lines.at(label.span.end)),
                    text: label.text.clone(),
                    primary: label.primary,
                })
                .collect(),
            help: cause.help.clone(),
            note: cause.note.clone(),
        }
    }
}

fn span_of(span: musa_score::SourceSpan) -> Span {
    Span {
        start: span.start,
        end: span.end,
    }
}

/// Every diagnostic code, in the order they are listed by `musa explain`.
#[must_use]
pub fn codes() -> Vec<&'static str> {
    musa_score::Code::ALL.iter().map(|code| code.as_str()).collect()
}

/// The long form of a diagnostic code: the rule, an example that breaks it,
/// and the same example fixed.
///
/// Only codes whose rule is non-obvious have one. `syntax` does not need a
/// paragraph explaining that the file is not shaped like musa; `does-not-add-
/// up` does, because the rule it enforces is a decision the language made and
/// not a fact about text.
///
/// Returns `None` for an unknown code, which is how the CLI tells "no
/// explanation written" from "no such code".
#[must_use]
pub fn explain(code: &str) -> Option<&'static str> {
    let known = musa_score::Code::parse(code)?;
    Some(match known {
        musa_score::Code::Syntax => {
            "The file is not shaped like musa.\n\n\
             Missing punctuation is reported where it belongs rather than where the \
             parser noticed: a `;` left off the end of one statement is flagged at the \
             end of that statement, not at the word on the next line. Where the repair \
             is one character in one place, the report carries it as a fix.\n\n\
             A note, a rest and a chord end themselves; every other statement ends \
             with a `;` or a `}`. An event is a word — `c4/4`, `rest/8`, `[c3 g3]/2` \
             — and nothing else in a voice begins the way one does, so there is \
             nothing for a `;` after it to separate."
        }
        musa_score::Code::UnknownName => {
            "A name used here is not declared anywhere the piece can see it.\n\n\
             Names are flat, so this means the name is spelled differently where it \
             was declared, or lives in a library this piece does not `use`. A motif \
             declared *after* the one that calls it is a different problem, reported \
             as `misplaced`.\n\n\
             Broken:\n    \
             motif sigh(root: pitch) { root/8 root/8 }\n    \
             voice right { use sigh(c5); use sihg(e5); }\n\n\
             Fixed:\n    \
             voice right { use sigh(c5); use sigh(e5); }"
        }
        musa_score::Code::DuplicateName => {
            "Two declarations share one name, and musa will not pick between them.\n\n\
             Shadowing is not an error you find later — it is a piece that sounds \
             different depending on which declaration a reader happens to notice. The \
             second label points at the first declaration."
        }
        musa_score::Code::UnknownWord => {
            "A word is in the right place but is not one musa knows: a clef, an \
             articulation, a dynamic, a chord quality. The help line lists the words \
             that fit here.\n\n\
             Broken:\n    \
             clef trebble;\n\n\
             Fixed:\n    \
             clef treble;"
        }
        musa_score::Code::NotAValue => {
            "Something in a value position will not read as one.\n\n\
             The four value shapes are a pitch (`c4`, `bb3`, `g#5`), a duration \
             (`1/4`, `3/8`), a meter (`4/4`), and a key (`d major`). A number that \
             takes a unit must carry it — `250 hz`, not `250` — because a unit is part \
             of the syntax and musa never guesses one.\n\n\
             Broken:\n    \
             cutoff = 250;\n\n\
             Fixed:\n    \
             cutoff = 250 hz;"
        }
        musa_score::Code::OutOfRange => {
            "A number is outside the range its parameter allows. The label names the \
             range. Clamping silently would make a piece sound different from what it \
             says, so musa stops instead."
        }
        musa_score::Code::Misplaced => {
            "The statement is valid musa but cannot be here.\n\n\
             Where a statement may appear is part of what it means: `clef` belongs to a \
             part because a clef is a property of a staff, and `tempo` belongs to a \
             piece because a tempo is a property of the music, not of one voice within \
             it."
        }
        musa_score::Code::DoesNotAddUp => {
            "The written durations do not sum to what the surrounding structure \
             requires: a voice that is not a whole number of measures, or a bar that \
             overflows its meter.\n\n\
             musa checks this because the alternative is an engraver silently inventing \
             a barline, and a piece that looks right on your screen and wrong on \
             someone else's.\n\n\
             Broken, in 4/4 — five quarters in a four-quarter measure:\n    \
             voice right { c5/4 d5/4 e5/4 f5/4 g5/4 }\n\n\
             Fixed:\n    \
             voice right { c5/4 d5/4 e5/4 f5/4 rest/2. }"
        }
        musa_score::Code::Import => {
            "A `use` cannot be followed.\n\n\
             Paths are relative to the file that writes them, are joined without \
             consulting the filesystem, and must name a `library`, not a `piece`. An \
             import loop is an error rather than a resolution order, because the order \
             would be an accident of which file you opened.\n\n\
             Broken:\n    \
             import \"patches.musa\";      // a piece\n\n\
             Fixed:\n    \
             import \"../library/patches.musa\";   // a library"
        }
        musa_score::Code::Studio => {
            "The studio graph cannot be built: an input nothing feeds, a cycle, a send \
             to a bus that does not exist.\n\n\
             The graph is checked before anything is rendered, because the audio \
             callback cannot report a problem — it can only produce silence or a \
             click."
        }
        musa_score::Code::Ignored => {
            "Something was skipped and the piece still plays without it. The label \
             points at what was skipped; the message says what was lost."
        }
        musa_score::Code::TypeMismatch => {
            "An elaboration expression has a different type from the value its context requires. Musa does not insert hidden coercions: written pitch, exact ratios, durations, and natural numbers remain distinct values. The primary label names the expression and the message states both types."
        }
        musa_score::Code::WrongArity => {
            "A function call does not supply its declared parameters exactly once. Positional arguments fill parameters from left to right; named arguments use the parameter's written name; only a parameter with a default may be omitted."
        }
        musa_score::Code::DependencyCycle => {
            "Elaboration definitions are total and non-recursive, but these definitions depend on each other in a cycle. The diagnostic prints that cycle. Pass the changing value as an argument or use one of Musa's finite structural folds instead of recursion."
        }
        musa_score::Code::ResourceLimit => {
            "The expression is finite and type-correct, but its deterministic work, value size, specialization count, or estimated music output exceeds Musa's compilation budget. The diagnostic names the operation, metric, attempted amount, and limit; no partial value or score is published."
        }
        musa_score::Code::NonExhaustiveMatch => {
            "A match must say what happens for every value of its scrutinee type. Cover both option or list constructors, both booleans, or finish literal cases with a binding or `_` fallback."
        }
        musa_score::Code::UnreachablePattern => {
            "An earlier match arm already covers every value this pattern could select. Remove the arm or move a more specific pattern before the catch-all arm."
        }
        musa_score::Code::UnsupportedLanguageStage => {
            "The source uses valid Musa syntax whose semantic compiler stage is not \
             installed yet. This is deliberately different from `syntax`: the editor, \
             formatter, and parser already understand the construct, but compilation \
             cannot assign it meaning without silently guessing.\n\n\
             Do not rewrite the expression as notation merely to make the \
             message vanish."
        }
        musa_score::Code::UnusedMaterial => {
            "A `motif` or `fragment` is declared and never used (style \
             guide §1: a name is a promise). A named `bar` is different — it \
             plays where it stands, so its name is an address, not a promise \
             of reuse.\n\n\
             The reader spends attention on the promise that the name will be spoken \
             again, and hunts the score for a use that does not exist. Use it, or \
             delete it — the offered fix deletes the declaration.\n\n\
             Broken:\n    \
             motif answer() { g4/4 a4/4 }\n    \
             voice right { c4/4 }\n\n\
             Fixed:\n    \
             voice right { use answer(); c4/4 }"
        }
        musa_score::Code::UnassignedPatch => {
            "A `patch` is declared in the studio and no `assign` connects it to a \
             part (style guide §1). Wiring with nothing at the end of it costs DSP \
             to build and makes silence: assign a part to it, or delete it."
        }
        musa_score::Code::RedundantMarking => {
            "A tempo, meter, or key marking states the value already in force \
             (style guide §2: a marking changes something).\n\n\
             A marking is a *change*, written where it happens; one that changes \
             nothing reads as an event and is none — the player marks their part \
             for a statement that was already true. If the reassurance is wanted, \
             it belongs in a comment. The offered fix deletes the marking."
        }
        musa_score::Code::CopiedBars => {
            "One bar is written out identically three or more times in one voice \
             (style guide §4: say it once).\n\n\
             Two is an accident of phrasing; three is a motif that has not been \
             named yet. The cost is the edit: change one copy and the others are \
             now wrong in a way nothing flags, because each still spells fine. A \
             `motif` makes the repetition a fact the compiler can check."
        }
        musa_score::Code::UnmetClaim => {
            "An `assert` claims something about the passage inside it, and the \
             passage does not do it.\n\n\
             This never happens unasked. Musa has no opinion about whether a \
             piece stays in its key, spells its chords completely, or keeps \
             four voices in their ranges — until a composer writes the claim \
             down, at which point checking it is the whole point of having \
             written it. The diagnostic names the smallest counterexample: the \
             one note that leaves the collection, the member that never \
             sounds, the voice that goes out of range. Nothing is repaired, \
             because a claim that the compiler could satisfy by editing the \
             music would not be a claim about the music.\n\n\
             The claims are `fills_meter()`, `pitches_in(scale)`, \
             `realizes(chord, policy)`, `voices(count)`, and \
             `within_ranges(ranges)`. A measure claim that fails reports as \
             `does-not-add-up` instead, because a bar too long is that same \
             mistake and has said so since bars existed."
        }
        musa_score::Code::UnsupportedPayload => {
            "An event track interchange file is well formed, and its occurrences \
             carry a payload type this build has no reader for.\n\n\
             The file is not wrong. `% musa-events-3` fixes the grammar of \
             terms — `let`, `follow`, `together`, `shift`, `scale`, \
             `restrict` — and leaves what an occurrence *is* to the producer, \
             which is what lets one format carry a score, a sketch, and \
             whatever a later tool invents. This build knows `ScoreFact`.\n\n\
             What still works is everything that does not need the payload: \
             the file opens, `musa format` lays it out canonically, and its \
             term structure is checked for free names and empty compositions. \
             What does not work is meaning — no score, no render, no export — \
             because a track whose occurrences cannot be decoded is a shape \
             without content. Nothing is silently converted: reading the \
             payloads as something else would be inventing music the file does \
             not contain."
        }
        musa_score::Code::Expansion => {
            "A region was not expanded, and the fault is in the region or in \
             what reads it.\n\n\
             `syntax <name> { … }` hands its interior to the adapter a \
             `import syntax … as <name>;` header named. Three things go wrong \
             at that handoff and this code covers all of them: no header \
             names that adapter, or the header comes after the region that \
             uses it, and the phase has nothing to call; the adapter refuses \
             the region, because the interior is not what it reads; or the \
             adapter answers with something a region may not stand for.\n\n\
             What a region may become is one ordinary expression. It may not \
             become an import, a module, a type declaration, a value \
             declaration, or another region — that is what keeps the set of \
             declarations in a file known before any expansion runs, and it \
             is why an adapter cannot grow the language sideways. The report \
             names the region, not a position inside text nobody wrote.\n\n\
             A limit crossed during expansion reports as `resource-limit` \
             instead. An adapter is total, so a run that stops is a run that \
             was given too much to read, not a run that would not have \
             stopped."
        }
        musa_score::Code::UnsolvedMetavariable => {
            "Elaboration left a hole, and nothing in the program says what \
             belongs in it.\n\n\
             Some things a program does not have to write down: the type of a \
             binder a later use pins down, an implicit argument the explicit \
             ones determine. Elaboration stands a *metavariable* in each such \
             place and lets the rest of the declaration decide it. This report \
             means the rest of the declaration did not.\n\n\
             It is never guessed at. Musa will not pick the smallest type that \
             fits, will not fall back to a default, and will not generalize the \
             hole into a type variable — each of those decides what a program \
             means on evidence the author did not give, and none of them can be \
             taken back by a later reader. The report says which kind of hole it \
             is (the type of a binder, an implicit argument, an annotation the \
             author omitted), where the term that opened it was written, and — \
             when one constraint was still waiting on another — which term that \
             constraint came from.\n\n\
             The fix is to write the missing thing: annotate the binder, or \
             pass the argument in braces at the use site."
        }
        musa_score::Code::ConversionMismatch => {
            "Two types elaboration had to make equal are not equal, after \
             computing both as far as deciding the question required.\n\n\
             This is not the same question as `type-mismatch`. Types here can \
             *compute* — one may be a function applied to a value, or a \
             definition that unfolds — so `equal` means equal after that \
             computation, and either side may be a form the author never \
             wrote.\n\n\
             How much of one gets printed is a stated policy, because a \
             conversion error that prints two forty-line normal forms is a \
             failed diagnostic even when it is a correct one. The report gives \
             **the smallest pair of subterms that actually disagreed, and the \
             path from the two whole types down to that pair** — \"the result \
             type: expected …, found …\" rather than both function types in \
             full. Each side is unfolded exactly as far as comparing it \
             required and no further, which is what the comparison itself did, \
             and each carries the source node it came from so the message can \
             point at text even when neither side is text."
        }
        musa_score::Code::NonPositiveOccurrence => {
            "A `data` declaration mentions the family it is declaring in a \
             place that would make the family unsound.\n\n\
             A constructor may take arguments of the family being declared — \
             that is what makes it recursive — but only *positively*: to the \
             right of every arrow it passes through. An occurrence to the left \
             of an arrow, as in `Bad (f : Bad -> Nat)`, lets a value of the \
             type consume itself, and a type that can do that can be given a \
             looping inhabitant with no recursion written anywhere. Musa is \
             total, so admitting one would make every proof in the language \
             worth nothing.\n\n\
             The check is conservative by construction: a declaration it \
             cannot see through is refused rather than admitted, because the \
             cost of being wrong is not a bad error message but an unsound \
             language. It runs on the whole mutually recursive group, so an \
             occurrence that is positive in its own constructor and negative \
             through a sibling is still caught.\n\n\
             The report names the occurrence and the constructor it sits in. \
             \"Not strictly positive\" without a location is the least \
             actionable thing a type checker can say."
        }
        musa_score::Code::IncompleteMatch => {
            "A `match` leaves a constructor of the family it splits on with no \
             branch.\n\n\
             The report names the missing constructors. A match compiles to \
             the family's recursor, which needs one method per constructor, so \
             this is not a policy that could have gone the other way: there is \
             no term to build until every branch exists. Nothing is filled in \
             with a failure case, because a total language has no failure case \
             to fill it with.\n\n\
             A nested pattern reports the constructor it is missing at the \
             position it is missing it, rather than reporting the outer \
             family, since the outer split already succeeded.\n\n\
             `non-exhaustive-match` is the same mistake in a finite match over \
             literals and shapes; this one is about an inductive family."
        }
        musa_score::Code::UnreachableBranch => {
            "A `match` arm can never be selected, because an earlier arm \
             already covers everything it would.\n\n\
             This is reported rather than quietly dropped. An arm that cannot \
             run is usually a mistaken mental model — a catch-all written \
             above the specific case it was meant to fall through to, or two \
             arms whose patterns the author believed were different. Telling \
             the author is worth more than the arm.\n\n\
             It is a property of the pattern matrix alone. Under the index \
             rule musa implements, no branch is unreachable *because of* an \
             index: a constructor's chosen index always meets a variable, so \
             there is no impossible branch to discharge. If an arm is \
             unreachable, an earlier arm covers it, and the report names \
             which one."
        }
        musa_score::Code::UncheckedRecursion => {
            "A recursive call the termination rule cannot see is smaller.\n\n\
             Musa is total, and a `rec` definition is admitted by rewriting \
             each recursive call into the induction hypothesis its branch was \
             handed — so the check and the compilation are one step, and there \
             is no fixed point left over to take on trust. A call the rewrite \
             cannot make is a call there is no hypothesis for.\n\n\
             The rule reads the recursive position off the definition's own \
             top-level `match`: the argument that `match` scrutinizes is the \
             one the recursion is on, and a call passes a pattern binder from \
             that column in that position. The remaining arguments are \
             unconstrained — the hypothesis already answers for that field at \
             the indices it has, so conversion decides the rest.\n\n\
             What this refuses: passing the argument the definition was \
             given, rather than a piece of it; passing something no `match` \
             made smaller; scrutinizing two of the definition's arguments at \
             the top level, so nothing says which one the recursion is on; and \
             using the definition as a value, where there is no call to \
             rewrite.\n\n\
             There is no escape hatch. No `partial`, no assume-it-terminates \
             flag: musa runs adapters at compile time and re-typesets per \
             keystroke, so a definition that might not stop is a compiler and \
             editor that might not stop."
        }
        musa_score::Code::UntypedRecursion => {
            "A definition that names itself, and wrote no type.\n\n\
             Musa infers a definition's type from its value when the value is \
             enough to say. A body that names the definition it is defining is \
             not: there is nothing to infer from until the answer is already \
             known.\n\n\
             It is also what the termination rule needs. A recursive \
             definition is admitted by rewriting each call into the induction \
             hypothesis its branch was handed, and the hypothesis's type is \
             the definition's own — so with no written type there is no \
             hypothesis to hand out.\n\n\
             Write the signature. Every other recursive definition in the \
             language has one for the same reason."
        }
        musa_score::Code::PrivateName => {
            "A name that exists, and is `private` to the module that declares \
             it.\n\n\
             This is deliberately not `unknown-name`. The two are different \
             sentences: \"no such thing\" sends a reader looking for a typo, \
             while this one tells them the name is real and that some other \
             module maintains it. The report names that module, which is where \
             the interface is.\n\n\
             `private` before a declaration hides it. `private` before an \
             enum's cases hides the constructors and leaves the *type* public, \
             which is the shape a package uses to maintain an invariant: a \
             chord whose symbol has to agree with its tones is built through \
             the function that keeps them in step, and there is no raw \
             constructor to route around it. A smart constructor beside a \
             reachable raw one is decoration.\n\n\
             What is hidden is hidden from outside only. Inside the declaring \
             module a private name is ordinary — a sibling definition writes \
             it bare, with no ceremony — so this never fires on the code that \
             maintains the invariant, only on the code that would break it.\n\n\
             The generated recursor is hidden with the cases, because \
             eliminating a family is the case analysis the marker exists to \
             prevent. Hiding the pattern spelling and leaving `Chord.elim` \
             reachable would hide nothing at all."
        }
        musa_score::Code::MixedVisibility => {
            "An enum marks some of its cases `private` and leaves others \
             public.\n\n\
             Refused at the declaration rather than at the first client who \
             trips over it, because a partly private type has no coverage rule \
             anyone would want to explain. Outside the module, the arms an \
             author is allowed to write can never exhaust the type, so every \
             such `match` would need a catch-all for cases the author cannot \
             see — and a catch-all standing in for something invisible is a \
             worse thing to explain than a refusal.\n\n\
             The fix is to decide what the type is. If its invariant is \
             package-maintained, mark every case `private` and export the \
             functions that build and read one. If it is an ordinary data \
             type, mark none of them.\n\n\
             Re-opening this would need a program with a genuinely public case \
             beside a private one, and a stated answer for what its `match` \
             coverage means."
        }
        musa_score::Code::AbstractMatch => {
            "A `match` would take apart a type whose constructors are private \
             here.\n\n\
             Refused where it is written rather than silently made \
             inexhaustive. The report names the type and the module that \
             maintains it; a client eliminates through whatever that module \
             exports — `members`, `analyze`, `write` — which is the package's \
             own interface and the entire point of hiding the \
             constructors.\n\n\
             A `match` that only binds is not this: it never splits, so it \
             never takes the type apart, and it is admitted. What is refused \
             is naming a constructor of the type in a pattern.\n\n\
             Inside the declaring module the same `match` is an ordinary \
             program. If the elimination genuinely belongs to the client, the \
             repair is a function the package exports, not a marker \
             removed."
        }
        musa_score::Code::ReservedClass => {
            "A `trait` was declared with the name `Storable`.\n\nStorability is a structural fact about a type, not a claim anyone may assert: `Storable` holds exactly when a type contains no function at any depth and has a versioned exact encoding, and the elaborator generates the instance or the type simply does not have one. The *word* is reserved and not only the instances, because a second trait spelled `Storable` would shadow the generated ones with hand-written ones, and the event track payload boundary would have a hole in it.\n\nA signature may require the constraint; nothing may supply it."
        }
        musa_score::Code::HeadlessClass => {
            "A `trait` was declared with no parameters.\n\nInstance lookup is keyed on the trait and the head constructor of its first argument, so a trait with no parameters has nothing to key on. What such a declaration describes is a record of global values, and `record` is the word for that.\n\nAdd the parameter the methods are about: `trait Eq<A>` rather than `trait Eq`."
        }
        musa_score::Code::ConstrainedField => {
            "A `trait` gave a required method a `where` clause.\n\n`10-traits.md` §1 splits a trait's methods in two, and the split decides who may carry a constraint. A *required* method is a field of the dictionary, filled by the `impl` that writes the instance — and a constraint in a field's type would have to be discharged by somebody who never wrote it: not the impl, which supplies a value and not a type, and not the use site, where resolving it would hide a second lookup inside a projection.\n\nA *derived* method is a function, and a function can take a dictionary. So it may carry a `where` clause, and that is the half of the split the constraint belongs to.\n\nTwo repairs: give the method a body, which makes it derived, or move the constraint onto the trait itself where it applies to every method at once."
        }
        musa_score::Code::DuplicateMethod => {
            "A `trait` declares one method name twice.\n\nThe dictionary a trait elaborates to is a record, and a record has one field per name. Two methods spelled alike would leave every use site with a choice nothing in the language could settle.\n\nThe report names both declarations. Rename one, or delete it if the second was meant to replace the first."
        }
        musa_score::Code::ClassArity => {
            "A trait was written with the wrong number of arguments.\n\nA trait's parameters are fixed by its declaration, and each argument is checked at the type the corresponding parameter was declared with — so a missing one cannot be inferred from the others and an extra one has nowhere to go.\n\nThe report says how many the trait takes and how many were written."
        }
        musa_score::Code::NotANumeralFamily => {
            "A number was written at a type that cannot be counted.\n\nA written number is stored as a count rather than as that many applications of a successor constructor, which is what keeps `repeat 384` one node instead of 384. That representation is only sound for a *counting* type: one with no parameters, no indices, and exactly two cases — one with no fields, and one taking a single value of the same type. `Nat` is the one the language writes.\n\nThe report says which of those conditions the named type fails. This is a mistake in whatever read the source, not in the source: the reader is what chose the type the number was written at."
        }
        musa_score::Code::IndexArity => {
            "A type was written with an index it does not carry, or without the one it does.\n\nAn index is the number in `Pc(12)` — written in parentheses, where a type *parameter* is written in angle brackets — and a type carries one only when its declaration says so: `data Pc(n: Nat)`. The two spellings are different because the questions are: a parameter says what a type is a type of and survives into the compiled program, while an index says how many and is erased once it has decided what typechecks.\n\nSo `Nat(12)` is an error rather than another way of writing `Nat`, and a `Pc` written bare is a type still missing its number. Both repairs are read off the declaration: write the index it declares, or take off the one it does not."
        }
        musa_score::Code::NotAnIndexSort => {
            "A declaration's index was given a type no index can be drawn from.\n\nAn index is compared by arithmetic rather than by the ordinary rules — `Bar(p + q)` and `Bar(q + p)` are one type — so the values it ranges over have to be readable as numbers. Three kinds are: a whole number, an exact fraction, and a finite set of literals.\n\nThe report names the binder. If the declaration wants to range over something else, what it wants is a type *parameter*, written in angle brackets."
        }
        musa_score::Code::HandWrittenStorable => {
            "An `impl Storable` was written.\n\n`Storable` is the one trait whose instances the elaborator generates and no program supplies. This is the deliberate exception to the rule that instances are declarations, and it is an exception in the safe direction: the set of instances is smaller than an author could write, never larger.\n\nIf a type should be storable and is not, the repair is in the type — a field holding a function, at any depth, is what makes it unstorable — and never in an instance asserting otherwise."
        }
        musa_score::Code::BlanketInstance => {
            "An `impl` was written whose head argument is a bare type variable.\n\nSuch an instance matches every type, so it is not a key: every lookup that found nothing would fall back to it, and the single table read that makes resolution predictable would become a search with a default.\n\nWrite the instance at each head it is actually for. If that is genuinely every type, what the code wants is an ordinary polymorphic function, not an instance."
        }
        musa_score::Code::DuplicateInstance => {
            "Two `impl` declarations answer for the same trait and head type.\n\nAt most one instance per (trait, head) pair is what makes a use site mean the same thing everywhere it appears — including inside a generic function compiled once and used at many types. With two, the same expression could elaborate to two different dictionaries depending on what happened to be in scope.\n\nThe report names both declarations. Delete one, or narrow one to a head the other does not cover."
        }
        musa_score::Code::OrphanInstance => {
            "An `impl` was written in a package that declares neither its trait nor its head type.\n\nCoherence has to hold across packages that never see each other, and the only way to check that locally is to require every instance to live with one of the two things it mentions. Otherwise two unrelated packages could each add an instance for the same pair, and a program that depended on both would be rejected for a conflict neither author could have known about.\n\nMove the instance into the package that declares the trait or the one that declares the type. Where neither is yours, the usual repair is a wrapper type in your own package."
        }
        musa_score::Code::ConstrainedData => {
            "An `enum` or `record` was declared with a `where` clause.\n\nA constraint on a type would be a dictionary every construction of it had to supply, and finding that dictionary is the synthesis this language refuses: a reader could not name the type without answering a question the declaration did not let it see.\n\nState the constraint on the functions that use the type instead, where it is an ordinary parameter, or take the dictionary as an argument explicitly."
        }
        musa_score::Code::SuperClass => {
            "A trait was declared with a `where` clause — a supertrait.\n\nA super-constraint is a dictionary obligation the elaborator would have to synthesize at every use of the trait, and synthesis is the recursive search this language refuses on purpose. The trait's dictionary is a record of exactly its methods, so a use site never has to ask where the rest came from.\n\nState the need where it is used instead: the method or function takes the second dictionary as an argument or states it in its own `where` clause, which is an ordinary parameter and not a synthesis obligation."
        }
        musa_score::Code::ConstrainedInstance => {
            "An `impl` was declared with a `where` clause.\n\nResolution is three steps and no recursion: a local dictionary, then one table lookup on the concrete head, then an error. An instance whose own constraints had to be synthesized from other instances is the recursion — `impl<A> Eq<List<A>> where Eq<A>` is the canonical shape.\n\nWrite the composition explicitly instead: an ordinary function from the dictionaries to the dictionary, `fn list_eq<A>(eq: Eq<A>) -> Eq<List<A>>`, or a macro that generates the concrete instances. Composition the author can read is the extensibility mechanism; synthesis the author cannot is what was removed."
        }
        musa_score::Code::DerivedMethod => {
            "An `impl` defines a method its trait derives.\n\nA derived method is written once, in the trait, in terms of the required ones. An instance that could replace it would make two dictionaries for the same instance distinguishable, which is what coherence exists to prevent — and it is why specialization is refused as a mechanism rather than as a rule.\n\nDelete the definition. If the derived version is wrong for this type, the method belongs in the required set."
        }
        musa_score::Code::NoSuchMethod => {
            "An `impl` defines a method its trait does not declare.\n\nUsually a spelling: the report lists the methods the trait does have.\n\nAn instance cannot add a method of its own, because a use site reaches a method through the trait and would have no way to know this instance has one. A function on the concrete type is the way to add behaviour to one type."
        }
        musa_score::Code::MissingMethod => {
            "An `impl` leaves a required method undefined.\n\nEvery required method is a field of the dictionary, and a record with a missing field is not a value of its type. There is no default to fall back to: a trait that wants one declares the method derived, with a body, and then no instance defines it at all.\n\nThe report names the method."
        }
        musa_score::Code::UnresolvedInstance => {
            "Nothing implements this trait for this type.\n\nResolution is one table read, so this is the whole answer: no instance is declared for the trait and the head type named, and no enclosing `where` supplies one. There is no search, no default, and no second attempt.\n\nTwo repairs, and the report gives both halves of the key so the choice is clear: write the instance, or — if the code is generic and this type is a parameter — add the constraint to the enclosing signature."
        }
        musa_score::Code::UnkeyedConstraint => {
            "A trait was needed for a type that cannot have instances.\n\nAn instance is filed under the name at the head of its first argument, so a type with no name at its head — a function type, a universe, a record type written out — is one no `impl` could ever answer. This is not a missing library: nobody can write the instance.\n\nFor `Storable` this is the rule rather than an accident. `02-core-calculus.md` §1.2 says a function is never storable and neither is anything holding one, so a signature that asks to store a function is asking for something the language does not have. Store the data the function was built from, or the name of the process, and rebuild it.\n\nFor any other trait, the repair is to name the type: declare it, and implement the trait for the name."
        }
        musa_score::Code::RedundantNamePrefix => {
            "A method, field, or case repeats the name of the declaration it belongs to.\n\nEvery one of these names is read after the thing it belongs to — a method after its receiver or its trait, a field after its record, a case after its type — so the prefix arrives second and says what the page already said. `d.duration_of()` says duration twice; `d.of()` says it once.\n\nThis is a lint rather than an error: the program means what it says. What it catches is a habit from a language with no receivers, where `duration_of(r)` had nothing to be read after and the prefix carried the type information a receiver now carries. Moving such a name across unchanged writes the old shape in the new spelling.\n\nRename it to the part that is not the prefix. Where that leaves a name too thin to read alone, what the code wants is an inherent *function* rather than a method — `Duration::of(r)` reads because the path supplies what the receiver would have.\n\nA trait instance is never checked: those method names belong to the trait, and an impl that renamed one would not be implementing it."
        }
        musa_score::Code::DuplicateConstraint => {
            "A `where` clause names one constraint twice.\n\nOne constraint is one dictionary however many times it is asked for, so the second copy changes nothing at the call site and nothing in the elaborated program. What it costs is the next edit: change one copy and a reader has two clauses to reconcile that were never two facts.\n\nDelete the repeat."
        }
        musa_score::Code::QualifiedPath => {
            "A `::` path names more than one item inside a type's namespace.\n\nA path is read left to right and capitalization decides where the module prefix ends: lowercase segments are modules, the first capitalized segment names a type or a trait, and exactly one segment follows it. `std::tonal::TokenKind::PitchLiteral` has one reading — three modules, a type, and its case — and nothing after `PitchLiteral` could be read as anything, because a type's namespace holds items and not further namespaces.\n\nThis is decided by looking at the written path and nothing else, which is why it arrives before any name is resolved: no declaration anywhere would make a second segment readable.\n\nThe report points at the extra segment. If the item it names belongs to a *type* rather than to the one before it, write that type's own path."
        }
        musa_score::Code::MethodOnVariable => {
            "Method syntax was written on a receiver whose type is not a declared type.\n\nA method call is resolved by the receiver's type and nothing else: the head of that type says which instances are in scope, and those instances say which traits have a method by that name. A type variable has no head an instance can be filed under, and neither does a function type or a record type written out — so there is nothing to look in.\n\nNote what the repair is *not*. Adding `where Add<A>` supplies the dictionary and still leaves `x.add(y)` unresolvable, because finding `add` from a bare `A` would mean scanning every trait in scope, and then adding a trait to a package would change what existing code already meant.\n\nThe repair is to name the trait: `Add.add(x, y)` resolves wherever the constraint does, and it is the form that says out loud which trait was meant."
        }
        musa_score::Code::NoMethodForType => {
            "No trait with an instance for this type declares a method by this name.\n\nThe candidates for `x.m(…)` are the traits that declare an `m` *and* have a dictionary for the head of `x`'s type. This report means that set is empty, and it names both halves so the missing one is visible: either no trait spells the method that way, or the trait that does has no instance for this type.\n\nThree repairs, depending on which half is missing: correct the spelling, write the instance, or call an ordinary function on the concrete type — a method is not the only way to add behaviour to one type, only the shortest."
        }
        musa_score::Code::AmbiguousMethod => {
            "Two traits in scope declare a method by this name for this type.\n\nCoherence keeps one instance per trait and head type; it says nothing about two *different* traits declaring a method spelled alike, which is what a program importing two libraries has. Nothing in the language ranks one candidate over the other, and inventing an order here would make the call's meaning depend on which imports happened to be visible.\n\nThe report names both traits. Write the one that was meant: `Ord.less(x, y)` rather than `x.less(y)`. Qualification always resolves, which is what makes the strictness of method syntax affordable."
        }
        musa_score::Code::SpliceCategory => {
            "A splice's value is not of the category the position it stands in demands.\n\n`Syntax<Cat>` is indexed by how a tree parses: `Syntax<Expr>` says the real parser read this tree as an expression, and `Syntax<TokenTree>` claims nothing at all. A quote body is read by that same parser, so every position inside it already knows which category it needs — and a `TokenTree` handed to an expression position would let a quote build syntax the grammar does not admit.\n\nThe forgetting direction is free: an expression is a token tree, and a `Syntax<TokenTree>` position takes either. It is only the other direction that needs evidence.\n\nThe repair is to get the claim: `as_expression(t)` runs the parser and answers `Some` exactly when the tree is an expression, so match on it and splice the certified value."
        }
        musa_score::Code::UnspreadSequence => {
            "A `$..xs` was written where one node stands rather than a run of them.\n\nThe two directions fail here for two reasons, and the report says which. A quote that *builds* spreads a list into a position whose own grammar supplies the separator — an argument list, a list literal, a product. That is the whole reason it is a separate form: the commas come from the position, so a list of any length is right, empty included. Everywhere else there is no separator to supply.\n\nA quote that *matches* needs no separator, only siblings: a spread binds the run it stands among, so it may stand among any group's children. What it cannot be is the whole body, because a pattern's body is one node and one node has no siblings.\n\nWrite `$x` for the one node the position holds, or move the spread among the children it was meant to run over."
        }
        musa_score::Code::QuotedCapture => {
            "A quote writes a name that its own hygiene could have produced.\n\nA binder a quote introduces is renamed on the way out — the printer appends `_g` and a number, so `item` becomes `item_g0` — and that is what keeps a name written in a quote and a name spliced into it from ever being the same name. A quote that writes `item_g0` itself defeats that: the two would print alike, and the binder would capture a name it never bound.\n\nThis is refused where the quote is written rather than where it expands, because the author who can rename it is the one reading this.\n\nRename it. The suffix is the compiler's, and no name in an adapter needs to end that way."
        }
        musa_score::Code::AmbiguousSpread => {
            "Two `$..xs` were written among one group's children.\n\nA spread binds however many nodes are left over once the templates on either side of it have matched. With one spread that leftover is determined — what is before it matches from the front, what is after it matches from the back, and the rest is the run. With two, the boundary between them is whatever the compiler picks, and every choice matches.\n\nSearching for a split that makes the arm's body succeed is the one thing this language keeps refusing: it would make a pattern's meaning depend on the arm it opens, and a small edit to the body could silently change which nodes were bound.\n\nBind the whole run with one `$..xs` and take it apart in the body, where the rule you had in mind is written down and can be read."
        }
        musa_score::Code::PatternCategory => {
            "A quote pattern was written against a value that is not syntax.\n\n`quote { … }` in a pattern position matches a `Syntax<Cat>` and binds pieces of it. It is read at the scrutinee's own category — that is what decides whether `$a` binds a `Syntax<Expr>` or a `Syntax<TokenTree>` — so a value with no category leaves the pattern with nothing to claim about what it binds.\n\nA type variable is this report too, and for the same reason: nothing downstream would settle the category, because the pattern is what would have to.\n\nMatch a syntax value, or use the patterns the type you have actually offers. Every other type in the language has its own."
        }
        musa_score::Code::QuotedLiteralName => {
            "A pattern quote wrote a name literally, and the arm used it as though it had been bound.\n\nA quote pattern binds only its splices: `quote { f($a) }` binds `a`, and `f` is a word that has to be there for the pattern to match. Everything not written with a `$` is a literal, which is what keeps a mistyped token from silently becoming a wildcard that matches everything.\n\nSo the name in the arm resolves to nothing, and this report says which one and why rather than leaving it as an undeclared name.\n\nWrite `$name` in the pattern if binding was meant. If the literal was meant, the arm needs a different way to say what it wants — the name in the pattern refers to nothing the arm can reach."
        }
        musa_score::Code::UnconstrainedVariable => {
            "A trait was needed for a type variable that nothing constrains.\n\nA type variable can never acquire a global instance: it stands for a type the caller chooses, and the instance would have to be chosen with it. So unlike an unresolved instance, there is exactly one repair, and it is on the signature this code is inside rather than in a library somewhere.\n\nAdd the constraint — `where Eq<A>` — and the dictionary becomes an argument the caller supplies. That is also what makes the function's behaviour a consequence of its own signature, which is why the constraint is written rather than inferred."
        }
        musa_score::Code::DuplicateExtern => {
            "One name was registered twice as a base type or a builtin.\n\nThe compiler hands the core a table of the types and operations it owns, and every entry in it has to mean one thing: two entries under one spelling would give every use site whichever was inserted last, silently and without either author knowing which they got.\n\nThis is a report about the compiler's own table rather than about anything in the file being compiled, so nothing in the source can repair it. It is here because a diagnostic nobody can read is worse than one that names its own author."
        }
        musa_score::Code::HigherOrderDelta => {
            "A compiler-owned operation was declared to take or return a function.\n\nThe δ-builtins are the operations the compiler computes for itself — arithmetic on durations, comparison on pitches — and they are first-order on purpose: every argument type and the result type is a base type or a finite structure over base types, with no arrow anywhere. That restriction is what lets the language reason about them at all, because a builtin that took a function would be a control operator whose behaviour is a fact about the compiler rather than about the language.\n\nAn operation that genuinely needs a function argument is an ordinary definition in the standard library, where it is written in Musa and checked like everything else."
        }
        musa_score::Code::UnknownBase => {
            "A compiler-owned operation names a base type nothing registered.\n\nA base type has no cases and no eliminator, and that inertness is true of it *because it is registered as one*. A signature over a type nobody registered claims an inertness nobody declared, so the guarantee the operation is resting on does not exist yet.\n\nRegister the base type alongside the operations over it. The table is one table for exactly this reason: the type and the operations that compute on it are one decision."
        }
        musa_score::Code::BaseNotMatchable => {
            "A pattern tried to take apart a value that has no structure.\n\nBase types — text, numbers, durations, pitches — are the language's atoms: they have no cases to match and no fields to open, so a constructor or record pattern at one is asking for parts that do not exist. The only pattern that fits is a name, which binds the whole value.\n\nTo branch on *which* value it is, compare it: `if k == \"PitchLiteral\"`, or a `match` on the result of that comparison. Equality on a base type is decidable and is what the language offers in place of a case analysis it cannot give."
        }
        musa_score::Code::TargetOutsideSignature => {
            "A compiler-owned traversal says it walks an argument it does not take.\n\nA traversal — `recurse_syntax` and the operations derived from it — reduces once the value it is walking has arrived, and the registration says which argument that is. An index past the end of the signature names no argument at all, so the operation could never take a step: every call would stay blocked, and a program built on it would fail somewhere else entirely, as a type that would not converge.\n\nThis is a report about the compiler's own table rather than about the file being compiled. It is caught at registration so that the failure is one sentence here instead of an unexplainable conversion error later."
        }
        musa_score::Code::TargetNotABase => {
            "A compiler-owned traversal walks a type that already knows how to be walked.\n\nThese traversals exist for the types that have no cases — syntax trees, whose node representation the reader owns — because a type with no constructors has no recursor, so there is nothing for a library `match` to take apart. A type declared with `data` is the other kind: its recursor is generated from the declaration, and the language reduces it by that rule.\n\nGiving such a type a second, compiler-owned rule would mean two ways to reduce one term, and which one you got would depend on how the call was written. Write the traversal in Musa, over the cases the declaration already gives."
        }
        musa_score::Code::NotFiniteData => {
            "A compiler-owned operation names a type it could not be handed or answer.\n\nThese operations compute on *data*: a base type — text, a number, a duration, a pitch — or a declared type like `Option`, `List`, or `Result` holding more of the same, however deeply nested. That is the whole of what one can read and the whole of what it can build, so a signature mentioning a record type, a universe, or a bare type variable describes an operation whose rule could not be written down.\n\nThis is a report about the compiler's own table rather than about the file being compiled. If the operation genuinely needs one of those shapes, it is an ordinary definition in the standard library, written in Musa and checked like everything else."
        }
        musa_score::Code::OperationRefused => {
            "An operation of the language refused the arguments it was given.\n\nThese are the operations that build and reshape notated music — sounding a fact, playing a chord, stretching or shifting a track — and each one holds to a law about what it can mean: a stretch factor is greater than zero, music does not start before the start, a fact fits inside the length it was given. Arguments that break the law describe music that has no reading, so the operation says so rather than answering something close.\n\nThe message is the operation's own sentence and the span is the call that raised it. This is a report about the file being compiled: the numbers reaching that call are what to change."
        }
    })
}

#[cfg(test)]
mod tests {
    use super::explain;

    #[test]
    fn every_code_has_an_explanation() {
        for code in musa_score::Code::ALL {
            let text = explain(code.as_str()).unwrap_or_default();
            assert!(!text.is_empty(), "{code} has no explanation");
        }
    }

    #[test]
    fn an_unknown_code_has_none() {
        assert_eq!(explain("not-a-code"), None);
    }

    #[test]
    fn a_listed_code_is_a_code_explain_answers_about() {
        // `codes()` is what `musa explain` prints with no argument, and
        // `explain` is what it prints with one. A code in the listing that
        // `explain` returns `None` for reads as "no such code" to the person
        // who just read it off the listing.
        for code in super::codes() {
            assert!(explain(code).is_some(), "{code} is listed but has no explanation");
        }
        assert!(super::codes().contains(&"expansion"));
    }
}
