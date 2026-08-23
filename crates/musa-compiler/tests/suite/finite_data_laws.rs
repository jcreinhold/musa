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
        .filter(|diagnostic| diagnostic.severity == musa_score::Severity::Error)
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

/// A `data` declaration names a type and its constructors, and the only way
/// into a value of it is `match` (`docs/rules/language/02-core-calculus.md`
/// §1.1). A fold is therefore ordinary source — a recursive function over the
/// constructors — rather than a name the declaration generates, which is what
/// `shape_fold` is here. The one-field product is a `record`, projected by
/// field name.
///
/// The descending argument comes first because §2.4's measure holds the
/// arguments *before* the recursive position fixed, so a traversal that both
/// descends and carries closures has to descend in the first one.
#[test]
fn a_declaration_names_a_type_and_its_constructors() {
    let compilation = compile_data(
        "data Shape { Silence, Sounded(held: Duration<WrittenTime>), Then(first: Shape, second: Shape) } \
         record Sides { left: Nat; right: Bool; } \
         let quiet: Shape = Silence; \
         let held: Shape = Sounded(duration_of(1/4)); \
         let sequenced: Shape = Then(quiet, held); \
         fn one(held: Duration<WrittenTime>) -> Nat { 1 } \
         fn joined(first: Nat, second: Nat) -> Nat { first } \
         fn shape_fold(shape: Shape, silence: Nat, sounded: Duration<WrittenTime> -> Nat, then: Nat -> Nat -> Nat) \
             -> Nat { \
             match shape { \
                 Silence -> silence, \
                 Sounded(d) -> sounded(d), \
                 Then(a, b) -> then(shape_fold(a, silence, sounded, then), shape_fold(b, silence, sounded, then)), \
             } \
         } \
         let counted: Nat = shape_fold(sequenced, 0, one, joined); \
         let both: Sides = Sides { left = counted, right = true }; \
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
             let subject: EventTrack<WrittenTime> = music { c4/4 d4/4 }; \
             fn raised(by: Interval, inner: EventTrack<WrittenTime>) -> EventTrack<WrittenTime> { transpose(by, inner) } \
             let plan: Gesture = Higher(P8, Higher(P5, Plain)); \
             fn gesture_fold(plan: Gesture, plain: EventTrack<WrittenTime>, \
                 higher: Interval -> EventTrack<WrittenTime> -> EventTrack<WrittenTime>) \
                 -> EventTrack<WrittenTime> { \
                 match plan { \
                     Plain -> plain, \
                     Higher(by, inner) -> higher(by, gesture_fold(inner, plain, higher)), \
                 } \
             } \
             let folded: EventTrack<WrittenTime> = gesture_fold(plan, subject, raised); \
             let written: EventTrack<WrittenTime> = transpose(P8, transpose(P5, subject)); \
             score { part p { voice by_fold { use folded; } voice by_hand { use written; } } } }",
            "fold-law.musa",
        ),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    let score = compilation.snapshot().expect("a score");
    let voices: Vec<
        Vec<(
            musa_score::MusicalTime,
            musa_score::NotatedDuration,
            musa_score::ScoreEventKind,
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

/// The declaration-order check *terminates* on two declarations that reach each
/// other, which is the law: it walks a finite declaration graph, so a cycle in
/// it is an answer rather than a loop. If it did not terminate this test would
/// hang instead of failing, which is the honest shape of the claim.
///
/// The answer this compiler gives is a refusal, not a group. `order_families`
/// in `musa-compiler`'s `document` module orders each written `data` after the
/// declarations its fields name and refuses a cycle, because one core group is
/// one `RawData` and two written declarations are two. `02-core-calculus.md`
/// §1.1 admits mutually recursive families in the *core*, so building the
/// group from the cycle is available and is a language decision rather than a
/// missing line — see `docs/plan/prompts/142-surface-cutover.md`.
#[test]
fn the_group_check_terminates_on_a_mutually_recursive_group() {
    let compilation = compile_data(
        "data Statement { Say(what: Expression), Both(first: Statement, second: Statement), Done } \
         data Expression { Number(value: Nat), Grouped(inner: Statement) } \
         let program: Statement = Both(Say(Number(3)), Done);",
    );
    let reported = errors(&compilation);
    assert!(
        reported.contains("`Statement`, `Expression` name each other"),
        "{reported}"
    );
    // The help speaks of "a type declaration" since the cutover's diagnostic
    // rewrite (285bdf3): the word the author writes is `data`, and the word
    // the rule is about is the type.
    assert!(
        reported.contains("a type declaration may not depend on one that depends on it"),
        "{reported}"
    );
}

/// Strict positivity, reported at the field that broke it rather than at the
/// group: a value of a declaration may not be an argument to a function
/// stored inside it.
///
/// The message is the core's, which names the occurrence rather than the arrow
/// it stands to the left of. That is weaker than what the old checker said and
/// prompt 165 owns the wording; what this law is about is that the declaration
/// is refused at all, and at `Trap`.
#[test]
fn a_non_positive_declaration_is_rejected_at_its_field() {
    let compilation = compile_data("data Bad { Trap(escape: Bad -> Nat) }");
    let reported = errors(&compilation);
    assert!(
        reported.contains("`Bad` occurs in `Trap` where a recursive occurrence is not allowed"),
        "{reported}"
    );
}

/// Storability, which prompt 128's amendment moved off the declaration:
/// `02-core-calculus.md` §1.2 makes `Storable` a *constraint* whose instances
/// are generated, so a declaration that stores an arrow is an ordinary type
/// that simply has no instance. Positivity still refuses the declaration
/// itself, which is the test above; this one is the case positivity allows.
///
/// The refusal it earns is at the site that requires storability — a track
/// payload, a machine port, a primitive's configuration — and
/// `machine_laws::a_port_that_holds_a_function_is_refused_where_it_is_written`
/// is where that half is stated, because the port is where a program can write
/// one.
#[test]
fn a_declaration_that_stores_a_function_is_a_type_with_no_storable_instance() {
    let compilation = compile_data("data Held { Keeps(action: Nat -> Nat) } fn kept(h: Held) -> Held { h }");
    assert_eq!(errors(&compilation), "");
}

/// A declaration's arity is fixed by the declaration, so instantiating it at
/// the wrong number of arguments is an error where it is written.
///
/// A parameter is an ordinary explicit binder in the core (`Sided<A>` is
/// `(A : Type) → Type`), so both mistakes are reported by the elaborator as
/// what they are: one argument too many is an application of something that is
/// no longer a function, and one too few leaves a function standing where a
/// type is needed. Naming the count is prompt 165's, and the span is the law —
/// the written type, not the declaration.
#[test]
fn a_declaration_instantiated_at_the_wrong_arity_is_rejected() {
    let compilation =
        compile_data("data Sided<A> { Both(left: A, right: A) } fn wrong(p: Sided<Nat, Bool>) -> Nat { 0 }");
    let reported = errors(&compilation);
    assert!(
        reported.contains("this is applied to an argument, but its type is not a function type"),
        "{reported}"
    );

    let bare = compile_data("data Sided<A> { Both(left: A, right: A) } fn wrong(p: Sided) -> Nat { 0 }");
    let reported = errors(&bare);
    assert!(
        reported.contains("this stands where a type is needed, but it is not one"),
        "{reported}"
    );
}

/// `01-surface.md` §1.2's update along a path, elaborated: `region.anchor = 1`
/// replaces the field the path *ends* at and leaves everything beside it alone.
///
/// The law is here because the failure it guards against is silent — the old
/// reader took the first identifier under an update, which for `region.anchor`
/// is `region`, and replacing a whole `Region` with a `Nat` is a program that
/// says something else. Prompt 142 wired `musa-compiler` to the core's own
/// update, so the path goes through whole; what stands here is the check that
/// it does, and that the one-segment form did not change meaning with it.
#[test]
fn an_update_along_a_path_replaces_the_field_the_path_ends_at() {
    let compilation = compile_data(
        "record Region { anchor: Nat; span: Nat; } record Pending { read: Nat; region: Region; } \
         fn shift(held: Pending) -> Pending { held with { region.anchor = 1 } } \
         let start: Pending = Pending { read = 0, region = Region { anchor = 5, span = 2 } }; \
         let moved: Pending = shift(start); \
         let anchor: Nat = moved.region.anchor; \
         let span: Nat = moved.region.span;",
    );
    assert_eq!(errors(&compilation), "");

    // And the single-segment form 127dcfac built still elaborates: the repair
    // was to the reader, not to what the reader accepts.
    let flat =
        compile_data("record Region { anchor: Nat; span: Nat; } fn widen(r: Region) -> Region { r with { span = 2 } }");
    assert_eq!(
        errors(&flat),
        "",
        "a one-segment update is the form that already worked"
    );
}
