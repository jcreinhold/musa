#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

fn compile_data(declarations: &str) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(
            format!("piece \"Finite data\" {{ {declarations} score {{ part p {{ voice v {{ c4/1 }} }} }} }}"),
            "finite-data.musa",
        ),
        &CompileOptions::default(),
    )
}

/// A library, for the declarations a piece cannot hold: signatures and the
/// structures that seal a declaration behind one.
fn compile_library(declarations: &str) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(format!("library {{ {declarations} }}"), "finite-data-library.musa"),
        &CompileOptions::default(),
    )
}

/// Every error message, joined — what a rejection test actually asserts is
/// *which* rejection happened, not that something went wrong. Labels and notes
/// are part of it: the field a declaration was rejected at is named in a label,
/// and the rule behind the rejection is the note.
fn errors(compilation: &musa_compiler::Compilation) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for diagnostic in compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_compiler::Severity::Error)
    {
        let _ = writeln!(out, "{}", diagnostic.message);
        for label in &diagnostic.labels {
            let _ = writeln!(out, "{}", label.text);
        }
        if let Some(note) = &diagnostic.note {
            let _ = writeln!(out, "{note}");
        }
        if let Some(help) = &diagnostic.help {
            let _ = writeln!(out, "{help}");
        }
    }
    out
}

/// A `data` declaration names a type, its constructors, and one fold — and
/// nothing else is generated for it (`docs/rules/language/02-core-calculus.md`
/// §1). The one-constructor case is a record, projected by field name.
#[test]
fn a_declaration_names_a_type_its_constructors_and_one_fold() {
    let compilation = compile_data(
        "data Shape { Silence, Sounded(held: Duration<WrittenTime>), Then(first: Shape, second: Shape) } \
         data Pair<A, B> { Both(left: A, right: B) } \
         let quiet: Shape = Silence; \
         let held: Shape = Sounded(held: 1/4); \
         let sequenced: Shape = Then(quiet, held); \
         fn one(held: Duration<WrittenTime>) -> Nat { 1 } \
         fn joined(first: Nat, second: Nat) -> Nat { first } \
         let counted: Nat = shape_fold(0, one, joined, sequenced); \
         let both: Pair<Nat, Bool> = Both(left: counted, right: true); \
         let projected: Nat = both.left; \
         fn named(shape: Shape) -> Nat { match shape { Silence -> 0, Sounded(d) -> 1, Then(a, b) -> 2 } } \
         let which: Nat = named(sequenced);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

/// The fold law. A fold is *the* total traversal, so folding a plan with
/// cases that build music has to *be* the music that plan denotes: here, one
/// voice built by `gesture_fold` and one written out by hand, note for note
/// the same.
///
/// Compared event by event rather than by the compilation's identity, because
/// identity carries provenance and these two notes honestly came from
/// different places in the file. What the law is about is the music.
#[test]
fn folding_with_the_music_cases_is_the_music_the_plan_denotes() {
    let compilation = compile(
        &SourceDocument::new(
            "piece \"Folded\" { \
             data Gesture { Plain, Higher(by: Interval, inner: Gesture) } \
             let subject: Music = music { c4/4 d4/4 }; \
             fn raised(by: Interval, inner: Music) -> Music { transpose(by, inner) } \
             let plan: Gesture = Higher(by: P8, inner: Higher(by: P5, inner: Plain)); \
             let folded: Music = gesture_fold(subject, raised, plan); \
             let written: Music = transpose(P8, transpose(P5, subject)); \
             score { part p { voice by_fold { use folded; } voice by_hand { use written; } } } }",
            "fold-law.musa",
        ),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    let score = compilation.snapshot().expect("a score");
    let voices: Vec<
        Vec<(
            musa_compiler::MusicalTime,
            musa_compiler::NotatedDuration,
            musa_compiler::ScoreEventKind,
        )>,
    > = score
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .map(|(_, voice)| {
            voice
                .events()
                .iter()
                .map(|event| (event.onset, event.notated_duration.clone(), event.kind.clone()))
                .collect()
        })
        .collect();
    let (by_fold, by_hand) = (
        voices.first().expect("the folded voice"),
        voices.get(1).expect("the written voice"),
    );
    assert_eq!(by_fold.len(), 2, "{by_fold:?}");
    assert_eq!(
        by_fold, by_hand,
        "the fold and the traversal it stands for are different music"
    );
}

/// The group check's termination law. Two declarations that reach each other
/// are one group, checked once; the check walks a *finite* declaration graph,
/// so a cycle in it is an ordinary group rather than a loop. If it did not
/// terminate this test would hang instead of failing, which is the honest
/// shape of the claim.
#[test]
fn the_group_check_terminates_on_a_mutually_recursive_group() {
    let compilation = compile_data(
        "data Statement { Say(what: Expression), Both(first: Statement, second: Statement), Done } \
         data Expression { Number(value: Nat), Grouped(inner: Statement) } \
         let program: Statement = Both(Say(Number(3)), Done); \
         fn say(what: Nat) -> Nat { what } \
         fn both(first: Nat, second: Nat) -> Nat { first } \
         fn number(value: Nat) -> Nat { value } \
         fn grouped(inner: Nat) -> Nat { inner } \
         let counted: Nat = statement_fold(say, both, 0, number, grouped, program);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

/// Strict positivity, reported at the field that broke it rather than at the
/// group: a value of a declaration may not be an argument to a function
/// stored inside it.
#[test]
fn a_non_positive_declaration_is_rejected_at_its_field() {
    let compilation = compile_data("data Bad { Trap(escape: Bad -> Nat) }");
    let reported = errors(&compilation);
    assert!(
        reported.contains("`Bad` stands to the left of an arrow in its own declaration"),
        "{reported}"
    );
    assert!(reported.contains("strictly positive"), "{reported}");
}

/// Storability, which is the weaker of the two and so the one a *positive*
/// arrow trips: an arrow is never storable data, and neither is anything
/// holding one.
#[test]
fn a_stored_function_field_is_rejected_at_its_field() {
    let compilation = compile_data("data Held { Keeps(action: Nat -> Nat) }");
    let reported = errors(&compilation);
    assert!(reported.contains("a stored field may not be a function"), "{reported}");
    assert!(reported.contains("`action` stores a function"), "{reported}");
}

/// Sealing. A signature's `data Hidden;` names the type and withholds its
/// constructors, so `Only` is the structure's own however visible the type is.
#[test]
fn a_private_constructor_may_not_be_named_outside_its_structure() {
    let compilation = compile_library(
        "signature Owner { data Hidden; let made: Hidden; } \
         structure Keep: Owner { data Hidden { Only(count: Nat) } let made: Hidden = Only(1); } \
         let outside: Nat = 1; \
         let taken = Only(2);",
    );
    let reported = errors(&compilation);
    assert!(reported.contains("`Only` is private"), "{reported}");
    assert!(
        reported.contains("a structure's constructors are its own"),
        "{reported}"
    );
}

/// A signature member the structure never declares is the same failure read
/// from the other side: the type is promised and not provided.
#[test]
fn a_structure_that_declares_no_such_type_does_not_match_its_signature() {
    let compilation = compile_library("signature Owner { data Hidden; } structure Keep: Owner { let count: Nat = 1; }");
    let reported = errors(&compilation);
    assert!(reported.contains("`Keep` does not declare `Hidden`"), "{reported}");
}

/// A declaration's arity is fixed by the declaration, so instantiating it at
/// the wrong number of arguments is an error where it is written.
#[test]
fn a_declaration_instantiated_at_the_wrong_arity_is_rejected() {
    let compilation =
        compile_data("data Pair<A> { Both(left: A, right: A) } fn wrong(p: Pair<Nat, Bool>) -> Nat { 0 }");
    let reported = errors(&compilation);
    assert!(reported.contains("`Pair` takes 1 type arguments, not 2"), "{reported}");

    let bare = compile_data("data Pair<A> { Both(left: A, right: A) } fn wrong(p: Pair) -> Nat { 0 }");
    let reported = errors(&bare);
    assert!(reported.contains("`Pair` takes 1 type arguments"), "{reported}");
}

/// `01-surface.md` §1.2's update along a path parses, and this compiler says
/// so rather than quietly replacing the wrong field.
///
/// The nested form elaborates in `musa-core`, and `musa-compiler` is wired to
/// it in prompt 142. Until then the path is *syntax this stage does not
/// implement*, which is a distinct thing from a mistake the author made — the
/// reason `Code::UnsupportedLanguageStage` exists. The law is here because the
/// failure it guards against is silent: the old reader took the first
/// identifier under an update, which for `region.anchor` is `region`.
#[test]
fn an_update_along_a_path_is_refused_by_name_rather_than_read_as_its_first_segment() {
    let compilation = compile_data(
        "data Region { At(anchor: Nat, span: Nat) } data Pending { Held(read: Nat, region: Region) } \
         fn shift(held: Pending) -> Pending { held with { region.anchor = 1 } }",
    );
    let reported = errors(&compilation);
    assert!(
        reported.contains("an update along a path is not elaborated yet"),
        "{reported}"
    );

    // And the single-segment form 127dcfac built still elaborates: the repair
    // was to the reader, not to what the reader accepts.
    let flat = compile_data(
        "data Region { At(anchor: Nat, span: Nat) } fn widen(r: Region) -> Region { r with { span = 2 } }",
    );
    assert_eq!(
        errors(&flat),
        "",
        "a one-segment update is the form that already worked"
    );
}
