//! Session-level diagnostics.
//!
//! A deliberate restatement of the compiler's diagnostic type rather than a
//! re-export: `musa-project` is the boundary at which compiler types stop
//! (roadmap §15.7), and a frontend that pattern-matched on
//! `musa_compiler::Severity` would be coupled to the compiler forever.
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
    /// The primary label's span, repeated.
    ///
    /// Not redundant in practice: the editor's lint decorations want one
    /// range per diagnostic and should not have to know which label is
    /// primary, and every existing caller reads this field.
    pub span: Option<Span>,
}

impl Diagnostic {
    /// Restate a compiler diagnostic against the source it was produced from.
    pub(crate) fn from_compiler(diagnostic: &musa_compiler::Diagnostic, lines: &Lines<'_>) -> Self {
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
                musa_compiler::Severity::Error => Severity::Error,
                musa_compiler::Severity::Warning => Severity::Warning,
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

fn span_of(span: musa_compiler::SourceSpan) -> Span {
    Span {
        start: span.start,
        end: span.end,
    }
}

/// Every diagnostic code, in the order they are listed by `musa explain`.
#[must_use]
pub fn codes() -> Vec<&'static str> {
    musa_compiler::Code::ALL.iter().map(|code| code.as_str()).collect()
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
    let known = musa_compiler::Code::parse(code)?;
    Some(match known {
        musa_compiler::Code::Syntax => {
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
        musa_compiler::Code::UnknownName => {
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
        musa_compiler::Code::DuplicateName => {
            "Two declarations share one name, and musa will not pick between them.\n\n\
             Shadowing is not an error you find later — it is a piece that sounds \
             different depending on which declaration a reader happens to notice. The \
             second label points at the first declaration."
        }
        musa_compiler::Code::UnknownWord => {
            "A word is in the right place but is not one musa knows: a clef, an \
             articulation, a dynamic, a chord quality. The help line lists the words \
             that fit here.\n\n\
             Broken:\n    \
             clef trebble;\n\n\
             Fixed:\n    \
             clef treble;"
        }
        musa_compiler::Code::NotAValue => {
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
        musa_compiler::Code::OutOfRange => {
            "A number is outside the range its parameter allows. The label names the \
             range. Clamping silently would make a piece sound different from what it \
             says, so musa stops instead."
        }
        musa_compiler::Code::Misplaced => {
            "The statement is valid musa but cannot be here.\n\n\
             Where a statement may appear is part of what it means: `clef` belongs to a \
             part because a clef is a property of a staff, and `tempo` belongs to a \
             piece because a tempo is a property of the music, not of one voice within \
             it."
        }
        musa_compiler::Code::DoesNotAddUp => {
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
        musa_compiler::Code::Import => {
            "A `use` cannot be followed.\n\n\
             Paths are relative to the file that writes them, are joined without \
             consulting the filesystem, and must name a `library`, not a `piece`. An \
             import loop is an error rather than a resolution order, because the order \
             would be an accident of which file you opened.\n\n\
             Broken:\n    \
             use \"patches.musa\";      // a piece\n\n\
             Fixed:\n    \
             use \"../library/patches.musa\";   // a library"
        }
        musa_compiler::Code::Studio => {
            "The studio graph cannot be built: an input nothing feeds, a cycle, a send \
             to a bus that does not exist.\n\n\
             The graph is checked before anything is rendered, because the audio \
             callback cannot report a problem — it can only produce silence or a \
             click."
        }
        musa_compiler::Code::Ignored => {
            "Something was skipped and the piece still plays without it. The label \
             points at what was skipped; the message says what was lost."
        }
        musa_compiler::Code::TypeMismatch => {
            "An elaboration expression has a different type from the value its context requires. Musa does not insert hidden coercions: written pitch, exact ratios, durations, and natural numbers remain distinct values. The primary label names the expression and the message states both types."
        }
        musa_compiler::Code::WrongArity => {
            "A function call does not supply its declared parameters exactly once. Positional arguments fill parameters from left to right; named arguments use the parameter's written name; only a parameter with a default may be omitted."
        }
        musa_compiler::Code::DependencyCycle => {
            "Elaboration definitions are total and non-recursive, but these definitions depend on each other in a cycle. The diagnostic prints that cycle. Pass the changing value as an argument or use one of Musa's finite structural folds instead of recursion."
        }
        musa_compiler::Code::ResourceLimit => {
            "The expression is finite and type-correct, but its deterministic work, value size, specialization count, or estimated music output exceeds Musa's compilation budget. The diagnostic names the operation, metric, attempted amount, and limit; no partial value or score is published."
        }
        musa_compiler::Code::NonExhaustiveMatch => {
            "A match must say what happens for every value of its scrutinee type. Cover both option or list constructors, both booleans, or finish literal cases with a binding or `_` fallback."
        }
        musa_compiler::Code::UnreachablePattern => {
            "An earlier match arm already covers every value this pattern could select. Remove the arm or move a more specific pattern before the catch-all arm."
        }
        musa_compiler::Code::UnsupportedLanguageStage => {
            "The source uses valid Musa syntax whose semantic compiler stage is not \
             installed yet. This is deliberately different from `syntax`: the editor, \
             formatter, and parser already understand the construct, but compilation \
             cannot assign it meaning without silently guessing.\n\n\
             Finish or upgrade to the prompt named by the diagnostic's help text; do \
             not rewrite the expression as notation merely to make the message vanish."
        }
        musa_compiler::Code::UnusedMaterial => {
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
        musa_compiler::Code::UnassignedPatch => {
            "A `patch` is declared in the studio and no `assign` connects it to a \
             part (style guide §1). Wiring with nothing at the end of it costs DSP \
             to build and makes silence: assign a part to it, or delete it."
        }
        musa_compiler::Code::RedundantMarking => {
            "A tempo, meter, or key marking states the value already in force \
             (style guide §2: a marking changes something).\n\n\
             A marking is a *change*, written where it happens; one that changes \
             nothing reads as an event and is none — the player marks their part \
             for a statement that was already true. If the reassurance is wanted, \
             it belongs in a comment. The offered fix deletes the marking."
        }
        musa_compiler::Code::CopiedBars => {
            "One bar is written out identically three or more times in one voice \
             (style guide §4: say it once).\n\n\
             Two is an accident of phrasing; three is a motif that has not been \
             named yet. The cost is the edit: change one copy and the others are \
             now wrong in a way nothing flags, because each still spells fine. A \
             `motif` makes the repetition a fact the compiler can check."
        }
    })
}

#[cfg(test)]
mod tests {
    use super::explain;

    #[test]
    fn every_code_has_an_explanation() {
        for code in musa_compiler::Code::ALL {
            let text = explain(code.as_str()).unwrap_or_default();
            assert!(!text.is_empty(), "{code} has no explanation");
        }
    }

    #[test]
    fn an_unknown_code_has_none() {
        assert_eq!(explain("not-a-code"), None);
    }
}
