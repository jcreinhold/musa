//! Every call supplies every parameter, and the anonymous function is where a
//! specialization gets written down.
//!
//! `docs/rules/constitution.md` §9 refuses partial application, default
//! parameters, and named hole filling as three spellings of one ambiguity:
//! what a call with a missing argument means. `docs/rules/language/`
//! `02-core-calculus.md` §5 answers the question that refusal opens — a named
//! `fn` is declared where the declarations are, so it cannot close over an
//! argument its caller just supplied — by giving the surface the lambda the
//! core already had. These are the laws of both halves.
//!
//! # The written section
//!
//! Prompt 142a repaired §1.3 rather than reopening it. The ambiguity the rule
//! is about is *silence* — `f(x)` quietly becoming a function — and a section
//! is not silent: `f(x, _)` says which slot is left, so every slot is still
//! named at every call and an under-applied call with no `_` is the same type
//! error it was. What changed is only that the domain's operations may be
//! values, which `note 51 §6` argues is the first thing a transformational
//! theory needs.

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::{Code, ScoreEventKind};

fn compile_core(declarations: &str) -> musa_compiler::Compilation {
    let source = SourceDocument::new(
        format!(
            "piece \"Complete calls\" {{ import std::core; import std::list; {declarations} \
             score {{ part p {{ voice v {{ c4/1 }} }} }} }}"
        ),
        "complete-call-laws.musa",
    );
    compile(&source, &CompileOptions::default())
}

fn errors(declarations: &str) -> Vec<(Code, String)> {
    compile_core(declarations)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_score::Severity::Error)
        .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
        .collect()
}

/// The job partial application was carrying: specializing a higher-order call
/// with a value the caller supplied.
#[test]
fn a_lambda_specializes_a_higher_order_call() {
    let compilation = compile_core(
        "fn risen(by: Nat, from: Nat) -> Nat { from } \
         let walk: List<Nat> = map(fn (from: Nat) -> Nat { risen(2, from) }, range(4));",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

/// A lambda closes over its enclosing scope by value, which is the whole
/// reason a named `fn` could not stand in for one: `by` is the caller's
/// argument, and the declarations have no name for it.
#[test]
fn a_lambda_captures_the_parameter_of_the_function_that_writes_it() {
    let sounding = |source: &str| {
        let compilation = compile(
            &SourceDocument::new(source, "complete-call-laws.musa"),
            &CompileOptions::default(),
        );
        assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
        compilation
            .into_snapshot()
            .map(|snapshot| {
                snapshot
                    .parts()
                    .iter()
                    .flat_map(|(_, part)| part.voices())
                    .flat_map(|(_, voice)| voice.events().to_vec())
                    .map(|event| match event.kind {
                        ScoreEventKind::Note { pitch } => pitch.to_string(),
                        ScoreEventKind::Rest => "rest".to_owned(),
                        ScoreEventKind::Chord { pitches } => format!("chord:{pitches:?}"),
                    })
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default()
    };
    let raised = |interval: &str| {
        format!(
            "piece \"Capture\" {{ import std::core; \
             fn raised(by: Interval, line: EventTrack<WrittenTime>) -> EventTrack<WrittenTime> {{ \
                 compose_music(fn (inner: EventTrack<WrittenTime>) -> EventTrack<WrittenTime> {{ transpose(by, inner) }}, retrograde, line) \
             }} \
             let subject: EventTrack<WrittenTime> = music {{ c4/4 e4/4 }}; \
             score {{ part p {{ voice v {{ use raised({interval}, subject); }} }} }} }}"
        )
    };
    assert_eq!(sounding(&raised("P1")), ["e4", "c4"]);
    assert_eq!(sounding(&raised("P8")), ["e5", "c5"]);
}

/// An unannotated lambda reads its types from the position it stands in, the
/// same way any other expression does (prompt 127aa).
#[test]
fn a_lambda_may_omit_the_annotations_a_declaration_may_omit() {
    let compilation = compile_core("let walk: List<Nat> = map(fn (from) { identity_nat(from) }, range(4));");
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

/// A call that omits an argument is refused, and the diagnostic names the
/// parameter that has no value rather than reporting a type it could not
/// build.
#[test]
fn a_call_that_omits_an_argument_is_refused_by_name() {
    let reported = errors("fn lifted(what: Nat, by: Nat) -> Nat { what } let raised: Nat = lifted(1);");
    assert!(
        reported
            .iter()
            .any(|(code, message)| *code == Code::WrongArity && message.contains("`by`")),
        "{reported:?}"
    );
}

/// The same rule reaches a builtin, which is where partial application used
/// to live as a value: `BuiltinValue::bound` is gone, so an under-applied
/// builtin is a call with a missing argument like any other.
#[test]
fn an_under_applied_builtin_is_refused() {
    let reported = errors("let raise: EventTrack<WrittenTime> -> EventTrack<WrittenTime> = transpose(P8);");
    assert!(
        reported.iter().any(|(code, _)| *code == Code::WrongArity),
        "{reported:?}"
    );
}

/// A lambda is not a second way to leave an argument out: applying one to
/// too few arguments is the same error as applying anything else.
#[test]
fn an_under_applied_lambda_is_refused() {
    let reported = errors(
        "let pick = fn (left: Nat, right: Nat) -> Nat { left }; \
         let one: Nat = pick(1);",
    );
    assert!(
        reported.iter().any(|(code, _)| *code == Code::WrongArity),
        "{reported:?}"
    );
}

/// §1.1 is untouched by the complete-call rule: an anonymous function is a
/// value, so it may be applied, passed, and *stored*.
///
/// This law used to say the last of those was refused, and read `data Held {
/// Keeps(action: Nat -> Nat) }` back as "a stored field may not be a function".
/// Prompt 128's amendment moved that rule rather than deleting it: storability
/// is the `Storable A` constraint of `02-core-calculus.md` §1.2, required at
/// the five positions §1.2 lists and at no others, and a `data` field is on
/// none of them. `Held` is therefore a perfectly good type that simply has no
/// `Storable` instance — which is the whole of what a complete call has to say
/// about it, since a lambda that could not be stored would be a value with a
/// missing power rather than a value.
///
/// The refusal itself did not go missing: it fires at the *use* that needs an
/// encoding, and `inferred_core_laws::a_function_may_not_hide_where_an_encoding_is_required`
/// is where both halves of that are checked.
#[test]
fn a_lambda_is_a_value_and_may_be_stored() {
    let reported = errors(
        "data Held { Keeps(action: Nat -> Nat) } \
         let kept: Held = Keeps(fn (n: Nat) -> Nat { n });",
    );
    assert!(reported.is_empty(), "{reported:?}");
}

/// A section names the function a call was still waiting for, and the slot it
/// is waiting for is written.
///
/// The pair to [`an_under_applied_builtin_is_refused`]: the same builtin, the
/// same two parameters, and the `_` is the whole difference between a refusal
/// and a value.
#[test]
fn a_section_names_the_function_a_call_is_waiting_for() {
    let reported = errors("let raise: EventTrack<WrittenTime> -> EventTrack<WrittenTime> = transpose(P8, _);");
    assert!(reported.is_empty(), "{reported:?}");
}

/// And it is the *right* function: the section applied to the argument it
/// stood for does what the complete call would have done.
#[test]
fn a_section_applied_is_the_call_it_came_from() {
    let sounding = |source: &str| {
        let compilation = compile(
            &SourceDocument::new(source, "complete-call-laws.musa"),
            &CompileOptions::default(),
        );
        assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
        compilation
            .into_snapshot()
            .map(|snapshot| {
                snapshot
                    .parts()
                    .iter()
                    .flat_map(|(_, part)| part.voices())
                    .flat_map(|(_, voice)| voice.events().to_vec())
                    .map(|event| match event.kind {
                        ScoreEventKind::Note { pitch } => pitch.to_string(),
                        ScoreEventKind::Rest => "rest".to_owned(),
                        ScoreEventKind::Chord { pitches } => format!("chord:{pitches:?}"),
                    })
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default()
    };
    let piece = |declarations: &str, used: &str| {
        format!(
            "piece \"Sections\" {{ import std::core; \
             let subject: EventTrack<WrittenTime> = music {{ c4/4 e4/4 }}; {declarations} \
             score {{ part p {{ voice v {{ use {used}; }} }} }} }}"
        )
    };
    let whole = sounding(&piece("", "transpose(P8, subject)"));
    let sectioned = sounding(&piece(
        "let raise: EventTrack<WrittenTime> -> EventTrack<WrittenTime> = transpose(P8, _);",
        "raise(subject)",
    ));
    assert_eq!(sectioned, whole);
    assert_eq!(whole, ["c5", "e5"]);
}

/// Two `_`s read left to right, which the *types* witness: the section's first
/// parameter is the call's first slot, so an argument list that read them the
/// other way round would not even check.
#[test]
fn two_sections_read_left_to_right() {
    let reported =
        errors("let moved: Interval -> EventTrack<WrittenTime> -> EventTrack<WrittenTime> = transpose(_, _);");
    assert!(reported.is_empty(), "{reported:?}");
    let swapped =
        errors("let moved: EventTrack<WrittenTime> -> Interval -> EventTrack<WrittenTime> = transpose(_, _);");
    assert!(!swapped.is_empty(), "the other order was accepted");
}

/// A method call reads a `_` the same way an ordinary call does — one reading
/// in the lowering, not two.
#[test]
fn a_method_call_takes_a_section_too() {
    let reported = errors(
        "import std::list; \
         let total: (Nat -> Nat -> Nat) -> Nat = range(4).fold_from_start(0, _);",
    );
    assert!(reported.is_empty(), "{reported:?}");
}

/// A `_` outside an argument list is refused for what it is, rather than
/// looked up as a name and reported missing.
#[test]
fn an_underscore_that_stands_for_no_slot_is_refused_as_itself() {
    let reported = errors("let held: Nat = _;");
    assert!(
        reported
            .iter()
            .any(|(_, message)| message.contains("`_` is not a value")),
        "{reported:?}"
    );
}
