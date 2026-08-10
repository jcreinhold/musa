//! What a signature, a structure, and a functor promise (docs/prompts/104, and
//! `docs/language/04-templates-and-modules.md` §4).
//!
//! A structure is a name for a group of declarations, not a thing. Every law
//! here is a consequence of that: matching is by name and exact type, what a
//! signature does not list is private, applying a functor is a second
//! *checking* of one body rather than a copy of it, and the result is
//! indistinguishable from the same declarations written out by hand.
//!
//! Generativity is observed the way the rest of the compiler observes it —
//! through the names a document ends up holding. Two instances of one functor
//! are two sets of declarations under two addresses, never one shared structure,
//! because there is no structure at run time to share. The digest behind those
//! addresses is checked where it is computed, in `crate::module`'s own unit
//! tests; nothing outside the compiler can see it, and that is the point.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{Code, CompileOptions, NameKind, ScoreSnapshot, Severity, SourceDocument, compile};

const STUDY: &str = include_str!("../../../examples/module-functor-study.musa");

fn compile_named(source: &str) -> musa_compiler::Compilation {
    compile(&SourceDocument::new(source, "test.musa"), &CompileOptions::default())
}

fn errors_of(source: &str) -> Vec<(Code, String)> {
    compile_named(source)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
        .collect()
}

/// Every label text of the first error whose message contains `needle`.
fn labels_for(source: &str, needle: &str) -> Vec<String> {
    compile_named(source)
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.severity == Severity::Error && diagnostic.message.contains(needle))
        .map(|diagnostic| diagnostic.labels.iter().map(|label| label.text.clone()).collect())
        .unwrap_or_default()
}

fn snapshot_of(source: &str) -> ScoreSnapshot {
    let compilation = compile_named(source);
    let errors = errors_of(source);
    assert!(errors.is_empty(), "expected a clean compile, got {errors:?}");
    compilation.into_snapshot().expect("compiles")
}

/// Every note of a snapshot as `part/voice onset kind`: musical equality,
/// with no provenance in it.
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

/// Every name of the given kind the document ended up holding.
fn names(source: &str, kind: NameKind) -> Vec<String> {
    compile_named(source)
        .references()
        .iter()
        .filter(|reference| reference.kind == kind)
        .map(|reference| reference.name.clone())
        .collect()
}

/// A context, a functor over it, and one instance — the shape every law here
/// varies.
const MADE: &str = r#"
signature TonalContext {
    let tonic: key;
    let collection: scale;
}

structure CMajor: TonalContext {
    let tonic: key = key c major;
    let collection: scale = scale c ionian;
    let bass: pitch = c3;
}

template structure Shifted(C: TonalContext, lift: interval): TonalContext {
    let tonic: key = C.tonic;
    let collection: scale = C.collection;
    let lifted: interval = lift;
}

make Shifted(CMajor, P5) as Away;

piece "Study" {
    meter 4/4;
    key Away.tonic;
    score {
        part piano {
            voice right {
                in scale Away.collection {
                    c4/4 d4/4 e4/4 f4/4
                }
            }
        }
    }
}
"#;

/// The same music with the structure written out as ordinary declarations.
const HANDWRITTEN: &str = r#"
piece "Study" {
    meter 4/4;
    key c major;
    score {
        part piano {
            voice right {
                in scale c ionian {
                    c4/4 d4/4 e4/4 f4/4
                }
            }
        }
    }
}
"#;

#[test]
fn the_bundled_example_compiles() {
    let compilation = compile(
        &SourceDocument::new(STUDY, "examples/module-functor-study.musa"),
        &CompileOptions::default(),
    );
    let errors: Vec<_> = compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert!(compilation.into_snapshot().is_some());
}

#[test]
fn a_module_is_equivalent_to_the_declarations_it_names() {
    assert_eq!(music(&snapshot_of(MADE)), music(&snapshot_of(HANDWRITTEN)));
}

#[test]
fn matching_is_by_name_and_exact_type() {
    let missing = MADE.replace("    let collection: scale = scale c ionian;\n", "");
    let (code, message) = errors_of(&missing)
        .into_iter()
        .find(|(_, message)| message.contains("does not define"))
        .expect("a missing member is refused");
    assert_eq!(code, Code::UnknownName);
    assert_eq!(message, "`CMajor` does not define `collection`");

    let mistyped = MADE.replace(
        "    let collection: scale = scale c ionian;",
        "    let collection: nat = 4;",
    );
    let (code, message) = errors_of(&mistyped)
        .into_iter()
        .find(|(_, message)| message.contains("wrong type"))
        .expect("a mistyped member is refused");
    assert_eq!(code, Code::TypeMismatch);
    assert_eq!(message, "`CMajor` gives `collection` the wrong type");
}

#[test]
fn a_mismatch_is_labelled_at_the_signature_and_at_the_module() {
    let missing = MADE.replace("    let collection: scale = scale c ionian;\n", "");
    let labels = labels_for(&missing, "does not define");
    assert_eq!(
        labels,
        vec!["`TonalContext` is not satisfied here", "required as a scale"],
        "reading one label alone never says what to change"
    );

    let mistyped = MADE.replace(
        "    let collection: scale = scale c ionian;",
        "    let collection: nat = 4;",
    );
    let labels = labels_for(&mistyped, "wrong type");
    assert_eq!(labels, vec!["this is a nat", "`TonalContext` requires a scale"]);
}

#[test]
fn a_member_the_signature_does_not_list_is_private() {
    // `bass` is defined by `CMajor` and listed by nothing, so it exists for
    // `CMajor`'s own definitions and for no one else.
    let outside = MADE.replace(
        "piece \"Study\" {",
        "let root: pitch = CMajor.bass;\n\npiece \"Study\" {",
    );
    let (code, message) = errors_of(&outside)
        .into_iter()
        .find(|(_, message)| message.contains("private"))
        .expect("a private member is not nameable from outside");
    assert_eq!(code, Code::UnknownName);
    assert_eq!(message, "`CMajor.bass` is private");
    assert_eq!(
        labels_for(&outside, "private"),
        vec![
            "named from outside the structure that defines it",
            "`TonalContext` does not export it",
        ]
    );

    // Inside, it is an ordinary name.
    let inside = MADE.replace(
        "    let bass: pitch = c3;",
        "    let bass: pitch = c3;\n    let lowest: pitch = bass;",
    );
    assert!(errors_of(&inside).is_empty(), "{:?}", errors_of(&inside));
}

#[test]
fn a_functor_sees_its_parameters_signature_and_not_the_module_behind_it() {
    // `CMajor` defines `bass`; `TonalContext` does not list it. A functor
    // over `TonalContext` may not reach it, however the site's structure was
    // written — that is what makes two structures interchangeable.
    let peeking = MADE.replace(
        "    let lifted: interval = lift;",
        "    let lifted: interval = lift;\n    let root: pitch = C.bass;",
    );
    let (code, message) = errors_of(&peeking)
        .into_iter()
        .find(|(_, message)| message.contains("private"))
        .expect("a functor sees exactly its parameter's signature");
    assert_eq!(code, Code::UnknownName);
    assert_eq!(message, "`C.bass` is private");
}

#[test]
fn applying_a_functor_binds_rather_than_rewrites() {
    // The body is checked once per instance with `C` naming what the site
    // passed, so the same body means two different things in two instances
    // and neither is a copy of any syntax.
    let two = MADE
        .replace(
            "make Shifted(CMajor, P5) as Away;",
            "structure AMinor: TonalContext {\n    let tonic: key = key a minor;\n    let collection: scale = scale a aeolian;\n    let bass: pitch = a2;\n}\n\nmake Shifted(CMajor, P5) as Away;\nmake Shifted(AMinor, P4) as Other;",
        )
        .replace("in scale Away.collection {", "in scale Other.collection {");
    assert!(errors_of(&two).is_empty(), "{:?}", errors_of(&two));
    let held = names(&two, NameKind::Value);
    assert!(held.contains(&"Away.tonic".to_owned()), "{held:?}");
    assert!(held.contains(&"Other.tonic".to_owned()), "{held:?}");
}

#[test]
fn two_instances_with_equal_arguments_stay_two_modules() {
    // Generative, not applicative: nothing is shared, because there is no
    // structure at run time to share. Both addresses exist, separately, and
    // renaming one leaves the other alone.
    let twice = MADE.replace(
        "make Shifted(CMajor, P5) as Away;",
        "make Shifted(CMajor, P5) as Away;\nmake Shifted(CMajor, P5) as Same;",
    );
    assert!(errors_of(&twice).is_empty(), "{:?}", errors_of(&twice));
    let held = names(&twice, NameKind::Value);
    for name in ["Away.tonic", "Away.lifted", "Same.tonic", "Same.lifted"] {
        assert!(held.contains(&name.to_owned()), "{name} is missing from {held:?}");
    }
    let structures = names(&twice, NameKind::Module);
    assert!(
        structures.contains(&"Away".to_owned()) && structures.contains(&"Same".to_owned()),
        "{structures:?}"
    );
}

#[test]
fn an_instance_may_be_made_from_an_instance_written_below_it() {
    // Order is a question about dependencies, not about lines: what a site
    // passes must exist, and where it is written is the author's business.
    let forward = MADE.replace(
        "make Shifted(CMajor, P5) as Away;",
        "make Shifted(Away, P4) as Far;\nmake Shifted(CMajor, P5) as Away;",
    );
    assert!(errors_of(&forward).is_empty(), "{:?}", errors_of(&forward));
    assert!(names(&forward, NameKind::Value).contains(&"Far.tonic".to_owned()));
}

#[test]
fn instantiation_is_acyclic() {
    let cyclic = MADE.replace(
        "make Shifted(CMajor, P5) as Away;",
        "make Shifted(Back, P5) as Away;\nmake Shifted(Away, P4) as Back;",
    );
    let cycles: Vec<_> = errors_of(&cyclic)
        .into_iter()
        .filter(|(code, _)| *code == Code::DependencyCycle)
        .map(|(_, message)| message)
        .collect();
    assert_eq!(
        cycles,
        vec![
            "`Away` is made from something it makes".to_owned(),
            "`Back` is made from something it makes".to_owned(),
        ]
    );
}

#[test]
fn a_functor_takes_a_module_where_its_signature_says_so() {
    let wrong = MADE.replace("make Shifted(CMajor, P5) as Away;", "make Shifted(P5, P5) as Away;");
    let (code, message) = errors_of(&wrong)
        .into_iter()
        .find(|(_, message)| message.contains("takes a structure"))
        .expect("a value is not a structure");
    assert_eq!(code, Code::TypeMismatch);
    assert_eq!(message, "`C` takes a structure matching `TonalContext`");

    let unknown = MADE.replace(
        "make Shifted(CMajor, P5) as Away;",
        "make Shifted(Nowhere, P5) as Away;",
    );
    assert!(
        errors_of(&unknown)
            .iter()
            .any(|(code, message)| *code == Code::UnknownName && message == "no structure called `Nowhere`"),
        "{:?}",
        errors_of(&unknown)
    );
}

#[test]
fn a_functors_value_arguments_are_evaluated_where_the_site_stands() {
    // `lift` is an ordinary typed parameter, so it is checked in the scope
    // the `make` is written in — not inside the functor, which has never
    // heard of `rise`.
    let from_site = MADE.replace(
        "make Shifted(CMajor, P5) as Away;",
        "let rise: interval = P5;\n\nmake Shifted(CMajor, rise) as Away;",
    );
    assert!(errors_of(&from_site).is_empty(), "{:?}", errors_of(&from_site));
    assert_eq!(music(&snapshot_of(&from_site)), music(&snapshot_of(HANDWRITTEN)));
}

#[test]
fn a_module_is_not_a_value() {
    let as_value = MADE.replace("piece \"Study\" {", "let held: key = CMajor;\n\npiece \"Study\" {");
    assert!(
        errors_of(&as_value)
            .iter()
            .any(|(code, _)| *code == Code::UnknownName || *code == Code::TypeMismatch),
        "a structure cannot be written where a value belongs: {:?}",
        errors_of(&as_value)
    );
}

#[test]
fn a_signature_a_module_names_must_exist() {
    let unknown = MADE.replace("structure CMajor: TonalContext {", "structure CMajor: NoSuchThing {");
    assert!(
        errors_of(&unknown)
            .iter()
            .any(|(code, message)| *code == Code::UnknownName && message == "no signature called `NoSuchThing`"),
        "{:?}",
        errors_of(&unknown)
    );
}

#[test]
fn a_bundled_context_is_imported_like_any_other_library() {
    let importing = r#"
import std::context;

piece "Study" {
    meter 4/4;
    key CMajor.tonic;
    score {
        part piano {
            voice right {
                in scale CMajor.collection {
                    c4/4 d4/4 e4/4 f4/4
                }
            }
        }
    }
}
"#;
    assert!(errors_of(importing).is_empty(), "{:?}", errors_of(importing));
    assert_eq!(music(&snapshot_of(importing)), music(&snapshot_of(HANDWRITTEN)));

    // Sealing crosses the import boundary too: `home` is `CMajor`'s own.
    let peeking = importing.replace(
        "piece \"Study\" {",
        "let register: option[frame] = CMajor.home;\n\npiece \"Study\" {",
    );
    assert!(
        errors_of(&peeking)
            .iter()
            .any(|(_, message)| message == "`CMajor.home` is private"),
        "{:?}",
        errors_of(&peeking)
    );
}
