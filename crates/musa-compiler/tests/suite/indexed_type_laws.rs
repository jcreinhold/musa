//! An indexed type, written in `.musa` source and checked.
//!
//! `docs/rules/language/02-core-calculus.md` §1.5 admits an index argument on a
//! type — `Pc(12)`, `Bar(3/4)` — drawn from a decidable arithmetic domain and
//! erased before evaluation. Prompt 142d built the stratum inside
//! `musa-calculus` and prompt 142f made it *writable*: a declaration says which
//! index it carries and at what sort, and a use site is checked against that.
//!
//! Every law here is a source document, because what 142f added is a spelling.
//! The calculus suite (`musa-calculus::index_laws`) owns the same questions
//! asked of raw terms, and the two are deliberately different: that one can
//! build a registry with an indexed base type, and this one can only write what
//! a composer can write.

#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

/// `data Pc(n: Nat)` and whatever else a law needs, inside a library.
///
/// A library rather than a piece because none of these laws is about music: a
/// piece would carry a score whose only role is to be syntactically present.
fn checked(declarations: &str) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(format!("library {{ {declarations} }}"), "indexed-type.musa"),
        &CompileOptions::default(),
    )
}

/// The declaration every law here is written against.
const PC: &str = "data Pc(n: Nat) { Zero, Next(fewer: Pc), }";

/// Every error, as `code: message` lines and the labels beneath them.
///
/// The code is part of what a refusal law asserts: `Pc(3/4)` and `Nat(12)` are
/// both refused, and a suite that only checked *that* would pass with the two
/// swapped.
fn errors(compilation: &musa_compiler::Compilation) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for diagnostic in compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_score::Severity::Error)
    {
        let _ = writeln!(out, "{}: {}", diagnostic.code.as_str(), diagnostic.message);
        for label in &diagnostic.labels {
            let _ = writeln!(out, "{}", label.text);
        }
        if let Some(help) = &diagnostic.help {
            let _ = writeln!(out, "{help}");
        }
    }
    out
}

/// A type declared to carry an index, written with it, is accepted.
///
/// The law the other five are the boundary of, and the one that did not hold
/// before 142f: `data Pc(n: Nat)` could not be written at all, so the whole
/// stratum was reachable only from a host's registry.
///
/// The recursive field is written *bare*, and that is not an oversight. §1.5's
/// index is a wrapper — a value of `Pc(12)` is a value of `Pc` — which is what
/// makes erasure structural, so a constructor never carries the index and a
/// recursive occurrence has none to write.
#[test]
fn a_type_declared_with_an_index_is_written_with_one() {
    let compilation = checked(&format!("{PC} fn f(x: Pc(12)) -> Nat {{ 0 }}"));
    assert_eq!(errors(&compilation), "", "an indexed type, written and checked");
}

/// An index on a type that declares none is refused, naming the type.
///
/// The refusal that makes `Nat(12)` an error rather than a second spelling of
/// `Nat`, and the one erasure rests on: a form nothing declares is a form
/// nothing can be erased from. It names the *type* because the type is what
/// could have declared an index and did not — the expression in the parentheses
/// is fine, and is not what is wrong.
#[test]
fn an_index_on_a_type_that_declares_none_is_refused() {
    let compilation = checked(&format!("{PC} fn h(x: Nat(1/2)) -> Nat {{ 0 }}"));
    let said = errors(&compilation);
    assert!(said.contains("index-arity"), "{said}");
    assert!(said.contains("`Nat` takes no index"), "{said}");
}

/// And a type that declares one is not a type without it.
///
/// The other half of the arity check. `Pc` alone is a type still waiting for
/// the number it carries, and admitting it would make the index optional —
/// which is the same as not having one, since two values at two indices would
/// then meet at the bare type. The message names the binder the declaration
/// wrote, so the author is told what to supply.
#[test]
fn a_type_that_declares_an_index_is_not_written_without_one() {
    let compilation = checked(&format!("{PC} fn k(x: Pc) -> Nat {{ 0 }}"));
    let said = errors(&compilation);
    assert!(said.contains("index-arity"), "{said}");
    assert!(said.contains("`Pc` carries an index"), "{said}");
    assert!(said.contains("write the `n`"), "{said}");
}

/// An index is *checked* at the sort its head declares, never inferred.
///
/// The bidirectional half of 142f, and the reason it matters. Inferring answers
/// "an exact fraction" and has nothing to compare that against, so a type
/// declared over whole numbers would silently carry a rational and two
/// signatures that disagree about what `Pc` counts would both be accepted.
#[test]
fn an_index_at_another_sort_than_the_head_declared_is_refused() {
    let compilation = checked(&format!("{PC} fn g(x: Pc(3/4)) -> Nat {{ 0 }}"));
    let said = errors(&compilation);
    assert!(said.contains("expected `Nat`, found `Ratio`"), "{said}");
}

/// A declaration's index binder must stand at a sort an index can be drawn
/// from.
///
/// §1.5 names three — a whole number, an exact fraction, and a finite set of
/// literals — and what they share is that the solver can read their values as
/// numbers. A binder at anything else declares an index no comparison could
/// decide, so it is refused where it is written rather than at the first use
/// that meets it.
#[test]
fn a_declaration_may_not_index_on_a_type_that_is_not_a_sort() {
    let compilation = checked("data Bad(n: Text) { Only, }");
    let said = errors(&compilation);
    assert!(said.contains("not-an-index-sort"), "{said}");
    assert!(said.contains("`n` is not a sort"), "{said}");
}

/// Two indices that disagree are two types, from source.
///
/// §1.5's invariance, which is the whole point of the stratum: `Pc(4)` is not a
/// `Pc(5)` and there is no coercion between them. Written as a call rather than
/// as an annotation because a call is where a composer meets it.
#[test]
fn a_value_at_one_index_does_not_stand_where_another_was_asked_for() {
    let compilation = checked(&format!(
        "{PC} fn same(a: Pc(4), b: Pc(4)) -> Nat {{ 0 }} fn wrong(a: Pc(4), b: Pc(5)) -> Nat {{ same(a, b) }}"
    ));
    let said = errors(&compilation);
    assert!(said.contains("expected `Pc(4)`, found `Pc(5)`"), "{said}");
}

/// An index variable bound by a signature is determined by the written
/// argument.
///
/// §2.1's first-order matching, which is §1.5's claim that an index variable
/// needs no binder form of its own: `n` is an ordinary parameter, so `echo(12,
/// p)` fixes it at the call and `echo(11, p)` is refused against the argument
/// the call actually wrote.
#[test]
fn an_index_variable_is_fixed_by_the_call_that_writes_it() {
    let good = checked(&format!(
        "{PC} fn echo(n: Nat, p: Pc(n)) -> Pc(n) {{ p }} fn use_it(p: Pc(12)) -> Pc(12) {{ echo(12, p) }}"
    ));
    assert_eq!(errors(&good), "", "the index the call wrote is the index it gets");
    let bad = checked(&format!(
        "{PC} fn echo(n: Nat, p: Pc(n)) -> Pc(n) {{ p }} fn bad(p: Pc(12)) -> Pc(12) {{ echo(11, p) }}"
    ));
    let said = errors(&bad);
    assert!(said.contains("expected `Pc(11)`, found `Pc(12)`"), "{said}");
}
