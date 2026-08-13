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

use musa_compiler::{Code, CompileOptions, ScoreEventKind, SourceDocument, compile};

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
        .filter(|diagnostic| diagnostic.severity == musa_compiler::Severity::Error)
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
             fn raised(by: Interval, line: Music) -> Music {{ \
                 compose_music(fn (inner: Music) -> Music {{ transpose(by, inner) }}, retrograde, line) \
             }} \
             let subject: Music = music {{ c4/4 e4/4 }}; \
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
    let reported = errors("let raise: Music -> Music = transpose(P8);");
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

/// §1.1 is untouched: a function value may be applied and passed, and a
/// `data` field that would store one is refused whether the function has a
/// name or not.
#[test]
fn a_lambda_may_not_be_stored_in_a_data_field() {
    let reported = errors("data Held { Keeps(action: Nat -> Nat) } let kept: Held = Keeps(fn (n: Nat) -> Nat { n });");
    assert!(
        reported
            .iter()
            .any(|(_, message)| message.contains("a stored field may not be a function")),
        "{reported:?}"
    );
}
