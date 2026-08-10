//! What a declaration template promises (docs/prompts/103, and
//! `docs/language/04-templates-and-modules.md` §1–§3).
//!
//! A template is a declaration with parameters, and the whole claim of this
//! file is that making one is *binding*, not rewriting: the instance means
//! exactly what the same declaration written out by hand would mean, plus one
//! provenance step saying which instance it is. Everything else here is the
//! consequence of that — arguments come from the site's scope, the body reads
//! only its own parameters, identity comes from where the site is rather than
//! what it was given, and nothing may make itself.
//!
//! Identity is checked as a string rather than a number because that is how
//! it leaves the compiler: an editor that remembers a collapsed instance
//! across an edit is remembering this text.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{
    Code, CompileOptions, ExpansionStep, ScoreSnapshot, Severity, SourceDocument, compile, kernel_normal_form,
};

const STUDY: &str = include_str!("../../../examples/template-study.musa");

fn compile_named(source: &str, name: &str) -> musa_compiler::Compilation {
    compile(&SourceDocument::new(source, name), &CompileOptions::default())
}

fn snapshot_of(source: &str) -> ScoreSnapshot {
    let compilation = compile_named(source, "test.musa");
    let errors = errors_of(source);
    assert!(errors.is_empty(), "expected a clean compile, got {errors:?}");
    compilation.into_snapshot().expect("compiles")
}

fn errors_of(source: &str) -> Vec<(Code, String)> {
    compile_named(source, "test.musa")
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
        .collect()
}

/// Every note of a snapshot as `part/voice onset pitch duration`: musical
/// equality, with no provenance in it.
fn music(snapshot: &ScoreSnapshot) -> Vec<String> {
    let mut out = Vec::new();
    for (part_id, part) in snapshot.parts().iter() {
        for (voice_id, voice) in part.voices() {
            for event in voice.events() {
                out.push(format!(
                    "{}/{} {} {:?}",
                    part_id.0,
                    voice_id.0,
                    event.onset.as_ratio(),
                    event.kind
                ));
            }
        }
    }
    out
}

/// Every template-instance step in a snapshot, as `alias identity`, in the
/// order the events carry them and without repeats.
fn instances(snapshot: &ScoreSnapshot) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (_, part) in snapshot.parts().iter() {
        for (_, voice) in part.voices() {
            for event in voice.events() {
                for step in &event.origin.expansion_path {
                    if let ExpansionStep::TemplateInstance { alias, identity, .. } = step {
                        let entry = format!("{alias} {identity}");
                        if !out.contains(&entry) {
                            out.push(entry);
                        }
                    }
                }
            }
        }
    }
    out
}

/// The bundled example, which is the shape every law here varies.
const MADE: &str = r#"
fn theme() -> Music { music { c4/4 d4/4 } }

template voice answer(subject: Music, transform: Music -> Music) {
    use transform(subject);
}

template piece study(k: Key, mode: Scale, subject: Music) "Study" {
    meter 4/4;
    key k;
    score {
        part piano {
            voice right { in scale mode { use subject; } }
            make answer(subject, transpose(P8)) as upper;
        }
    }
}

make study(key g major, scale g mixolydian, theme()) as study_in_g;
"#;

/// The same music, written out by hand, with no template anywhere in it.
const HANDWRITTEN: &str = r#"
piece "Study" {
    meter 4/4;
    key g major;
    score {
        part piano {
            voice right { in scale g mixolydian { c4/4 d4/4 } }
            voice upper { transpose up P8 { c4/4 d4/4 } }
        }
    }
}
"#;

#[test]
fn the_example_compiles() {
    let snapshot = snapshot_of(STUDY);
    assert_eq!(snapshot.title(), "Study");
    let aliases: Vec<String> = instances(&snapshot)
        .iter()
        .filter_map(|entry| entry.split(' ').next().map(str::to_owned))
        .collect();
    assert_eq!(aliases, ["study_in_g", "upper", "higher"]);
}

/// Substitution and equivalence: an instance is the declaration the arguments
/// make of the template, and nothing else. The made piece and the handwritten
/// one are the same music, event for event.
#[test]
fn making_a_template_is_writing_the_declaration_out() {
    assert_eq!(music(&snapshot_of(MADE)), music(&snapshot_of(HANDWRITTEN)));
}

/// …and the *only* difference is the provenance step, which the handwritten
/// piece does not have and the made one does.
#[test]
fn an_instance_is_marked_as_one() {
    assert!(instances(&snapshot_of(HANDWRITTEN)).is_empty());
    let made = instances(&snapshot_of(MADE));
    assert_eq!(made.len(), 2, "the piece and the voice inside it: {made:?}");
}

/// A parameter is bound to the argument's *value*, so an argument that is a
/// function is applied inside the body.
#[test]
fn a_higher_order_argument_transforms_the_body() {
    let octave = MADE.replace("transpose(P8)", "transpose(P15)");
    let plain = MADE.replace("transpose(P8)", "transpose(P1)");
    assert_ne!(music(&snapshot_of(&octave)), music(&snapshot_of(MADE)));
    assert_ne!(music(&snapshot_of(&plain)), music(&snapshot_of(MADE)));
}

/// Context authority (prompt 63): a template's `key k;` is the piece's key,
/// and which key it is comes from the site.
#[test]
fn a_context_statement_in_a_template_is_the_instances_context() {
    let in_g = snapshot_of(MADE);
    let in_e = snapshot_of(&MADE.replace("key g major", "key e minor"));
    assert_ne!(format!("{:?}", in_g.keys()), format!("{:?}", in_e.keys()));
}

/// Two documents, one template text, two sets of arguments: the key/scale
/// study made twice. Prompt 103's Target puts this here rather than in the
/// example because one document is one piece.
#[test]
fn one_template_makes_a_family() {
    let in_g = snapshot_of(MADE);
    let in_e = snapshot_of(&MADE.replace(
        "make study(key g major, scale g mixolydian, theme())",
        "make study(key e minor, scale e aeolian, music { g4/4 a4/4 })",
    ));
    assert_ne!(music(&in_g), music(&in_e));
    assert_ne!(format!("{:?}", in_g.keys()), format!("{:?}", in_e.keys()));
    // Same site, same document name, so the same identity: what changed is
    // what the instance was given, not which instance it is (§2).
    assert_eq!(instances(&in_g), instances(&in_e));
}

/// Identity comes from the site, so an edit that does not move the site does
/// not change it.
#[test]
fn identity_survives_an_unrelated_edit() {
    let before = instances(&snapshot_of(MADE));
    let commented = format!("// a remark\n{MADE}");
    let renamed = MADE.replace("fn theme()", "fn tune()").replace("theme()", "tune()");
    assert_eq!(instances(&snapshot_of(&commented)), before);
    assert_eq!(instances(&snapshot_of(&renamed)), before);
}

/// Alpha-renaming a template's parameters changes neither the music nor the
/// identity: the names are the body's own business.
#[test]
fn renaming_a_parameter_changes_nothing() {
    let renamed = MADE
        .replace(
            "study(k: Key, mode: Scale, subject: Music)",
            "study(tonality: Key, sc: Scale, tune: Music)",
        )
        .replace("key k;", "key tonality;")
        .replace("in scale mode", "in scale sc")
        .replace("use subject;", "use tune;")
        .replace("answer(subject, transpose(P8))", "answer(tune, transpose(P8))");
    assert_eq!(music(&snapshot_of(&renamed)), music(&snapshot_of(MADE)));
    assert_eq!(instances(&snapshot_of(&renamed)), instances(&snapshot_of(MADE)));
}

/// Two sites are two declarations, however alike their arguments are.
#[test]
fn distinct_sites_have_distinct_identities() {
    let twice = MADE.replace(
        "make answer(subject, transpose(P8)) as upper;",
        "make answer(subject, transpose(P8)) as upper;\n            make answer(subject, transpose(P8)) as twin;",
    );
    let identities = instances(&snapshot_of(&twice));
    assert_eq!(identities.len(), 3);
    let digests: Vec<&str> = identities.iter().filter_map(|entry| entry.split(' ').nth(1)).collect();
    let mut unique = digests.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), digests.len(), "identities repeated: {identities:?}");
}

/// A document's name is part of the key, so the same file compiled under two
/// names is two families rather than one.
#[test]
fn identity_is_scoped_to_the_document() {
    let here = compile_named(MADE, "here.musa").into_snapshot().expect("compiles");
    let there = compile_named(MADE, "there.musa").into_snapshot().expect("compiles");
    assert_eq!(music(&here), music(&there));
    assert_ne!(instances(&here), instances(&there));
}

/// Compiling the same source twice gives the same everything.
#[test]
fn expansion_is_deterministic() {
    let first = snapshot_of(MADE);
    let second = snapshot_of(MADE);
    assert_eq!(music(&first), music(&second));
    assert_eq!(instances(&first), instances(&second));
    assert_eq!(
        kernel_normal_form(
            &SourceDocument::new(MADE, "test.musa"),
            &musa_compiler::Realization::default()
        ),
        kernel_normal_form(
            &SourceDocument::new(MADE, "test.musa"),
            &musa_compiler::Realization::default()
        ),
    );
}

/// A made piece is the document's piece all the way down, including on the
/// kernel interchange path.
#[test]
fn a_made_piece_has_a_kernel_term() {
    let text = kernel_normal_form(
        &SourceDocument::new(MADE, "test.musa"),
        &musa_compiler::Realization::default(),
    )
    .expect("a made piece elaborates");
    assert!(text.contains("note:c5"), "the made voice is in the term: {text}");
    assert!(text.contains("study_in_g"), "the instance step is in the term: {text}");
}

// --- What is refused ----------------------------------------------------

fn refuses(source: &str, code: Code) -> String {
    let errors = errors_of(source);
    let found = errors
        .iter()
        .find(|(reported, _)| *reported == code)
        .unwrap_or_else(|| panic!("expected {code:?}, got {errors:?}"));
    found.1.clone()
}

/// Termination: nothing may make itself. The dependency graph is finite and
/// acyclic, and this is the one edge that could close it.
#[test]
fn a_template_may_not_make_itself() {
    let source = r#"
template voice loop() {
    make loop() as again;
}
piece "P" { meter 4/4; score { part p { make loop() as once; } } }
"#;
    // The self-instance is refused inside the body; the outer site is fine.
    let recursive = r#"
template piece loop() "P" {
    meter 4/4;
    score { part p { make loop() as inner; } }
}
make loop() as outer;
"#;
    assert!(!errors_of(source).is_empty());
    refuses(recursive, Code::DependencyCycle);
}

#[test]
fn an_argument_of_the_wrong_type_is_refused() {
    let wrong = MADE.replace("make study(key g major,", "make study(scale g major,");
    assert!(
        !errors_of(&wrong).is_empty(),
        "a scale where a key belongs must not compile"
    );
}

#[test]
fn the_wrong_number_of_arguments_is_refused() {
    let short = MADE.replace(
        "make study(key g major, scale g mixolydian, theme())",
        "make study(key g major)",
    );
    let message = refuses(&short, Code::WrongArity);
    assert!(message.contains("takes 3 arguments"), "{message}");
}

#[test]
fn a_named_argument_is_refused() {
    let named = MADE.replace("make study(key g major,", "make study(k: key g major,");
    refuses(&named, Code::WrongArity);
}

#[test]
fn a_voice_template_may_not_stand_where_a_piece_belongs() {
    let source = r"
template voice answer() { c4/4 }
make answer() as p;
";
    let message = refuses(source, Code::Misplaced);
    assert!(message.contains("voice template"), "{message}");
}

#[test]
fn a_piece_template_may_not_stand_among_voices() {
    let source = r#"
template piece inner() "I" { meter 4/4; score { part p { voice v { c4/4 } } } }
piece "P" { meter 4/4; score { part p { make inner() as v; } } }
"#;
    let message = refuses(source, Code::Misplaced);
    assert!(message.contains("piece template"), "{message}");
}

#[test]
fn an_unknown_template_is_refused() {
    let source = "piece \"P\" { meter 4/4; score { part p { make nobody() as v; } } }";
    refuses(source, Code::UnknownName);
}

#[test]
fn two_templates_may_not_share_a_name() {
    let source = r#"
template voice a() { c4/4 }
template voice a() { d4/4 }
piece "P" { meter 4/4; score { part p { make a() as v; } } }
"#;
    refuses(source, Code::DuplicateName);
}

/// A parameterized declaration written without `template` is a mistake with
/// a name, not a silent success.
#[test]
fn parameters_without_the_word_template_are_refused() {
    let piece = "piece study(k: Key) \"S\" { meter 4/4; score { part p { voice v { c4/4 } } } }";
    let voice = "piece \"P\" { meter 4/4; score { part p { voice v(k: Key) { c4/4 } } } }";
    assert!(refuses(piece, Code::Misplaced).contains("needs `template`"));
    assert!(refuses(voice, Code::Misplaced).contains("needs `template`"));
}

/// A template body reads its own parameters and the file's lexical root —
/// never the piece the site stands in.
#[test]
fn a_template_body_cannot_see_the_site() {
    let source = r#"
template voice answer() {
    use local;
}
piece "P" {
    meter 4/4;
    let local: Music = music { c4/4 };
    score { part p { make answer() as v; } }
}
"#;
    refuses(source, Code::UnknownName);
}
