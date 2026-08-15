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
             import \"patches.musa\";      // a piece\n\n\
             Fixed:\n    \
             import \"../library/patches.musa\";   // a library"
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
             Do not rewrite the expression as notation merely to make the \
             message vanish."
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
        musa_compiler::Code::UnmetClaim => {
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
        musa_compiler::Code::UnsupportedPayload => {
            "A kernel interchange file is well formed, and its occurrences \
             carry a payload type this build has no reader for.\n\n\
             The file is not wrong. `% musa-kernel-2` fixes the grammar of \
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
        musa_compiler::Code::Expansion => {
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
        musa_compiler::Code::UnsolvedMetavariable => {
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
        musa_compiler::Code::ConversionMismatch => {
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
        musa_compiler::Code::NonPositiveOccurrence => {
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
        musa_compiler::Code::IncompleteMatch => {
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
        musa_compiler::Code::UnreachableBranch => {
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
        musa_compiler::Code::ForcedIndex => {
            "A `match` scrutinee's index is not a distinct variable, and index \
             refinement is defined only for that shape.\n\n\
             Splitting refines indices by *generalizing* them into the \
             recursor's motive: `xs : Vec A n` at a variable `n` becomes a \
             motive quantified over `n`, and each constructor's own index then \
             refines it in that constructor's branch. The motive is the \
             refinement, and it needs no equality proof, no injectivity \
             lemma, and no deletion rule.\n\n\
             At `Vec A (succ n)` there is no variable to generalize, so the \
             refinement has nothing to work with. Making the remaining branch \
             usable would need the deletion rule, which requires K — an axiom \
             musa has not adopted, and which a paper trial found no musa \
             program needs. Refusing here is that decision checked at the \
             rule rather than at each of its uses.\n\n\
             The report names the index it was stuck on. The fix is to \
             scrutinize at a variable and let the branch supply the shape: \
             match on the vector, not on a vector already known to be \
             non-empty. The same index written twice — `Vec A n n` — is \
             refused for the same reason: the second is no longer distinct."
        }
        musa_compiler::Code::UncheckedRecursion => {
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
        musa_compiler::Code::PrivateName => {
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
        musa_compiler::Code::MixedVisibility => {
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
        musa_compiler::Code::AbstractMatch => {
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
        musa_compiler::Code::ReservedClass => {
            "A `trait` was declared with the name `Storable`.\n\nStorability is a structural fact about a type, not a claim anyone may assert: `Storable` holds exactly when a type contains no function at any depth and has a versioned exact encoding, and the elaborator generates the instance or the type simply does not have one. The *word* is reserved and not only the instances, because a second trait spelled `Storable` would shadow the generated ones with hand-written ones, and the kernel payload boundary would have a hole in it.\n\nA signature may require the constraint; nothing may supply it."
        }
        musa_compiler::Code::HeadlessClass => {
            "A `trait` was declared with no parameters.\n\nInstance lookup is keyed on the trait and the head constructor of its first argument, so a trait with no parameters has nothing to key on. What such a declaration describes is a record of global values, and `record` is the word for that.\n\nAdd the parameter the methods are about: `trait Eq<A>` rather than `trait Eq`."
        }
        musa_compiler::Code::DuplicateMethod => {
            "A `trait` declares one method name twice.\n\nThe dictionary a trait elaborates to is a record, and a record has one field per name. Two methods spelled alike would leave every use site with a choice nothing in the language could settle.\n\nThe report names both declarations. Rename one, or delete it if the second was meant to replace the first."
        }
        musa_compiler::Code::ClassArity => {
            "A trait was written with the wrong number of arguments.\n\nA trait's parameters are fixed by its declaration, and each argument is checked at the type the corresponding parameter was declared with — so a missing one cannot be inferred from the others and an extra one has nowhere to go.\n\nThe report says how many the trait takes and how many were written."
        }
        musa_compiler::Code::HandWrittenStorable => {
            "An `impl Storable` was written.\n\n`Storable` is the one trait whose instances the elaborator generates and no program supplies. This is the deliberate exception to the rule that instances are declarations, and it is an exception in the safe direction: the set of instances is smaller than an author could write, never larger.\n\nIf a type should be storable and is not, the repair is in the type — a field holding a function, at any depth, is what makes it unstorable — and never in an instance asserting otherwise."
        }
        musa_compiler::Code::BlanketInstance => {
            "An `impl` was written whose head argument is a bare type variable.\n\nSuch an instance matches every type, so it is not a key: every lookup that found nothing would fall back to it, and the single table read that makes resolution predictable would become a search with a default.\n\nWrite the instance at each head it is actually for. If that is genuinely every type, what the code wants is an ordinary polymorphic function, not an instance."
        }
        musa_compiler::Code::DuplicateInstance => {
            "Two `impl` declarations answer for the same trait and head type.\n\nAt most one instance per (trait, head) pair is what makes a use site mean the same thing everywhere it appears — including inside a generic function compiled once and used at many types. With two, the same expression could elaborate to two different dictionaries depending on what happened to be in scope.\n\nThe report names both declarations. Delete one, or narrow one to a head the other does not cover."
        }
        musa_compiler::Code::OrphanInstance => {
            "An `impl` was written in a package that declares neither its trait nor its head type.\n\nCoherence has to hold across packages that never see each other, and the only way to check that locally is to require every instance to live with one of the two things it mentions. Otherwise two unrelated packages could each add an instance for the same pair, and a program that depended on both would be rejected for a conflict neither author could have known about.\n\nMove the instance into the package that declares the trait or the one that declares the type. Where neither is yours, the usual repair is a wrapper type in your own package."
        }
        musa_compiler::Code::UnboundedInstance => {
            "An instance was declared whose `where` clause need not terminate.\n\nResolving a constraint may need the instance's own constraints resolved first, so an instance whose context is not smaller than its head can recurse forever. Two things are checked, both at the declaration: each constraint's first argument must be strictly smaller than the head, and no type variable may occur in it more often than it occurs in the head. The second is what refuses `impl<A> C<F<A>> where D<G<A, A>>`, whose argument is smaller and still duplicates its variable.\n\nThis is checked where the instance is written and never where it is used, because the author reading a use-site failure is not the author who can fix it."
        }
        musa_compiler::Code::DerivedMethod => {
            "An `impl` defines a method its trait derives.\n\nA derived method is written once, in the trait, in terms of the required ones. An instance that could replace it would make two dictionaries for the same instance distinguishable, which is what coherence exists to prevent — and it is why specialization is refused as a mechanism rather than as a rule.\n\nDelete the definition. If the derived version is wrong for this type, the method belongs in the required set."
        }
        musa_compiler::Code::NoSuchMethod => {
            "An `impl` defines a method its trait does not declare.\n\nUsually a spelling: the report lists the methods the trait does have.\n\nAn instance cannot add a method of its own, because a use site reaches a method through the trait and would have no way to know this instance has one. A function on the concrete type is the way to add behaviour to one type."
        }
        musa_compiler::Code::MissingMethod => {
            "An `impl` leaves a required method undefined.\n\nEvery required method is a field of the dictionary, and a record with a missing field is not a value of its type. There is no default to fall back to: a trait that wants one declares the method derived, with a body, and then no instance defines it at all.\n\nThe report names the method."
        }
        musa_compiler::Code::UnresolvedInstance => {
            "Nothing implements this trait for this type.\n\nResolution is one table read, so this is the whole answer: no instance is declared for the trait and the head type named, and no enclosing `where` supplies one. There is no search, no default, and no second attempt.\n\nTwo repairs, and the report gives both halves of the key so the choice is clear: write the instance, or — if the code is generic and this type is a parameter — add the constraint to the enclosing signature."
        }
        musa_compiler::Code::UnkeyedConstraint => {
            "A trait was needed for a type that cannot have instances.\n\nAn instance is filed under the name at the head of its first argument, so a type with no name at its head — a function type, a universe, a record type written out — is one no `impl` could ever answer. This is not a missing library: nobody can write the instance.\n\nFor `Storable` this is the rule rather than an accident. `02-core-calculus.md` §1.2 says a function is never storable and neither is anything holding one, so a signature that asks to store a function is asking for something the language does not have. Store the data the function was built from, or the name of the process, and rebuild it.\n\nFor any other trait, the repair is to name the type: declare it, and implement the trait for the name."
        }
        musa_compiler::Code::UnconstrainedVariable => {
            "A trait was needed for a type variable that nothing constrains.\n\nA type variable can never acquire a global instance: it stands for a type the caller chooses, and the instance would have to be chosen with it. So unlike an unresolved instance, there is exactly one repair, and it is on the signature this code is inside rather than in a library somewhere.\n\nAdd the constraint — `where Eq<A>` — and the dictionary becomes an argument the caller supplies. That is also what makes the function's behaviour a consequence of its own signature, which is why the constraint is written rather than inferred."
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
