//! `01-surface.md` §1.3: what a `private` declaration hides, from whom, and what
//! it says when it refuses.
//!
//! The fixture is note 43 §5.1's `Chord` written into the core. That note's
//! finding was that `Chord::NamedChord(ChordSymbol::GSeven, [Spelling::C])` — a
//! chord whose symbol contradicts its tones — is constructible by anyone, so the
//! smart constructor maintaining the invariant is decoration. The laws here are
//! the repair: the package builds one through `build`, and a client that reaches
//! for the constructor is told the name exists and is maintained elsewhere.
//!
//! `musa-core` mints no module identity — the caller does (`origin.rs`'s bargain)
//! — so these tests assign the numbers a compiler will assign at prompt 142. Two
//! numbers is the whole apparatus: [`INSIDE`] is the package, [`OUTSIDE`] is a
//! client.

use std::sync::Arc;

use musa_core::{Cx, Group, ModuleId, Raw, RawArm, RawData, RawPattern, Refusal, Term, check, declare, infer};

use crate::family_laws::{apply, binder, constructor, data, family, hidden_case, hidden_family, var};
use crate::programs::{WRITTEN, refusal};

/// The module that declares the fixture.
const INSIDE: ModuleId = ModuleId::new(1);

/// A module that merely imports it.
const OUTSIDE: ModuleId = ModuleId::new(2);

/// `data Symbol { GSeven, CMajor }` and `data Chord { private Named(Symbol) }`.
///
/// Two families rather than one because the point of §1.3 is a *public type* with
/// package-maintained cases: `Symbol` is what a client may still write, and
/// `Chord` is what it may only receive.
fn chord() -> RawData {
    data(
        Vec::new(),
        vec![
            family(
                "Symbol",
                vec![constructor("GSeven", Vec::new()), constructor("CMajor", Vec::new())],
            ),
            family(
                "Chord",
                vec![hidden_case(constructor("Named", vec![binder("symbol", var("Symbol"))]))],
            ),
        ],
    )
}

/// The package's own context: inside [`INSIDE`], with the fixture declared.
fn package() -> (Cx, Arc<Group>) {
    let cx = Cx::new().in_module(INSIDE);
    let group = declare(&cx, &chord()).expect("the chord fixture is a declaration");
    let cx = cx.declaring(&group);
    (cx, group)
}

/// A client's context: the same group, in scope, seen from [`OUTSIDE`].
///
/// The *same* group and not a second declaration, which is the whole mechanism:
/// visibility is a question about where a name is read, and the thing being read
/// is one declaration that remembers where it was written.
fn client(group: &Arc<Group>) -> Cx {
    Cx::new().in_module(OUTSIDE).declaring(group)
}

/// `Chord.Named Symbol.GSeven`, the construction the invariant depends on.
fn named() -> Raw {
    apply(var("Chord.Named"), [var("Symbol.GSeven")])
}

/// `match c { Named(s) => s }`, the elimination §1.3 keeps to the package.
fn symbol_of(subject: Raw) -> Raw {
    Raw::match_on(
        WRITTEN,
        [subject],
        vec![RawArm {
            patterns: vec![RawPattern::constructor(
                WRITTEN,
                "Chord.Named",
                [RawPattern::bind(WRITTEN, "s")],
            )],
            body: var("s"),
        }],
    )
}

/// §1.3: inside its own module a private case is an ordinary name, with no
/// ceremony at all — that is what makes a smart constructor writable.
#[test]
fn a_private_case_is_an_ordinary_name_inside_its_module() {
    let (cx, _) = package();
    let (chord, _) = infer(&cx, &var("Chord")).expect("the type is a name");
    check(&cx, &chord, &named()).expect("the package builds its own chord");
    let (symbol, _) = infer(&cx, &var("Symbol")).expect("the symbol type is a name");
    let build = Raw::pi(WRITTEN, "s", var("Symbol"), var("Chord"));
    let (build, _) = infer(&cx, &build).expect("the smart constructor has a type");
    check(
        &cx,
        &build,
        &Raw::lam(WRITTEN, "s", apply(var("Chord.Named"), [var("s")])),
    )
    .expect("`build` is what the package exports instead of the constructor");
    check(&cx, &symbol, &var("Symbol.GSeven")).expect("a public case of a public type is public");
}

/// §1.3: the type stays public. Hiding the cases is not hiding the type — a
/// client writes `Chord` in a signature and receives one from `build`.
#[test]
fn a_type_whose_cases_are_private_is_still_nameable() {
    let (_, group) = package();
    let cx = client(&group);
    infer(&cx, &var("Chord")).expect("the type is public");
    let signature = Raw::pi(WRITTEN, "c", var("Chord"), var("Chord"));
    infer(&cx, &signature).expect("a client may write it in a signature");
}

/// §1.3: the constructor is not. The refusal names the module rather than
/// falling through to "no such name", because the two send a reader to different
/// places.
#[test]
fn a_private_case_is_refused_outside_its_module_by_name() {
    let (_, group) = package();
    let cx = client(&group);
    let Err(error) = infer(&cx, &var("Chord.Named")) else {
        panic!("a client must not reach the constructor its package maintains");
    };
    let refusal = refusal("a client naming a private constructor", error);
    let Refusal::Private { name, module, .. } = &refusal else {
        panic!("expected a private-name refusal, got `{refusal}`");
    };
    assert_eq!(&**name, "Chord.Named");
    assert_eq!(*module, INSIDE, "the refusal says which module maintains it");
    assert_eq!(
        refusal.to_string(),
        "`Chord.Named` is private to the module that declares it"
    );
}

/// §1.3: and neither is the recursor. Hiding the pattern spelling while leaving
/// `Chord.elim` in scope would hide nothing — eliminating a family *is* the case
/// analysis the marker exists to prevent.
#[test]
fn the_recursor_is_hidden_with_the_cases() {
    let (_, group) = package();
    let cx = client(&group);
    let Err(error) = infer(&cx, &var("Chord.elim")) else {
        panic!("a recursor is case analysis by another spelling");
    };
    let refusal = refusal("a client naming the recursor", error);
    assert!(
        matches!(refusal, Refusal::Private { .. }),
        "expected a private-name refusal, got `{refusal}`"
    );
}

/// §1.3: a `match` outside the module is refused where it is written, naming the
/// type, rather than silently becoming inexhaustive.
#[test]
fn an_abstract_type_is_not_taken_apart_outside_its_module() {
    let (_, group) = package();
    let cx = client(&group);
    let goal = Raw::pi(WRITTEN, "c", var("Chord"), var("Symbol"));
    let (goal, _) = infer(&cx, &goal).expect("the client may write the signature it cannot implement");
    let Err(error) = check(&cx, &goal, &Raw::lam(WRITTEN, "c", symbol_of(var("c")))) else {
        panic!("a client must not take apart a type its package maintains");
    };
    let refusal = refusal("a client matching on an abstract type", error);
    let Refusal::AbstractMatch { family, module, .. } = &refusal else {
        panic!("expected an abstract-match refusal, got `{refusal}`");
    };
    assert_eq!(&**family, "Chord");
    assert_eq!(*module, INSIDE);
    assert_eq!(
        refusal.to_string(),
        "`Chord`'s cases are private to the module that declares it, so this cannot take one apart"
    );
}

/// The same `match`, written by the package, is an ordinary program. Without
/// this the previous law would be satisfied by refusing every `match`.
#[test]
fn the_package_matches_on_its_own_type() {
    let (cx, _) = package();
    let goal = Raw::pi(WRITTEN, "c", var("Chord"), var("Symbol"));
    let (goal, _) = infer(&cx, &goal).expect("the signature is a type");
    check(&cx, &goal, &Raw::lam(WRITTEN, "c", symbol_of(var("c"))))
        .expect("the package eliminates the family it declares");
}

/// §1.3: `private` on the declaration itself hides the type too, and then
/// nothing about it is reachable — not the type, not its cases.
#[test]
fn a_private_family_hides_the_type_as_well_as_its_cases() {
    let declaration = data(
        Vec::new(),
        vec![hidden_family(family("Ledger", vec![constructor("Empty", Vec::new())]))],
    );
    let cx = Cx::new().in_module(INSIDE);
    let group = declare(&cx, &declaration).expect("a private family is a declaration");
    let inside = cx.declaring(&group);
    infer(&inside, &var("Ledger")).expect("its own module names it");
    let outside = client(&group);
    for name in ["Ledger", "Ledger.Empty"] {
        let Err(error) = infer(&outside, &var(name)) else {
            panic!("`{name}` must not be reachable outside the module that declares it");
        };
        let refusal = refusal(name, error);
        assert!(
            matches!(refusal, Refusal::Private { .. }),
            "`{name}`: expected a private-name refusal, got `{refusal}`"
        );
    }
}

/// A context that names no module is inside every module.
///
/// The clause that keeps this rule invisible to every caller that has no
/// packages — including every other suite in this crate, and every caller of
/// `musa-core` until prompt 142 supplies real numbers.
#[test]
fn a_context_with_no_module_sees_everything() {
    let (_, group) = package();
    let cx = Cx::new().declaring(&group);
    let (chord, _) = infer(&cx, &var("Chord")).expect("the type is a name");
    check(&cx, &chord, &named()).expect("a caller with no packages builds one");
    let goal = Raw::pi(WRITTEN, "c", var("Chord"), var("Symbol"));
    let (goal, _) = infer(&cx, &goal).expect("the signature is a type");
    check(&cx, &goal, &Raw::lam(WRITTEN, "c", symbol_of(var("c")))).expect("and takes one apart");
}

/// A declaration written outside any module hides from nobody.
///
/// The other half of the same clause, and the one that keeps `private` from
/// meaning "private to nowhere, therefore private from everywhere".
#[test]
fn a_declaration_written_in_no_module_hides_from_nobody() {
    let cx = Cx::new();
    let group = declare(&cx, &chord()).expect("the fixture declares without a module");
    let cx = client(&group);
    let (chord, _) = infer(&cx, &var("Chord")).expect("the type is a name");
    check(&cx, &chord, &named()).expect("nothing was hidden, because nowhere is where it was written");
}

/// §1.3's re-opening condition made checkable: marking a declaration private
/// changes no program that did not name it.
#[test]
fn marking_a_case_private_changes_no_program_that_did_not_name_it() {
    let public = {
        let cx = Cx::new().in_module(INSIDE);
        let group = declare(&cx, &open_chord()).expect("the open fixture is a declaration");
        cx.declaring(&group)
    };
    let (_, group) = package();
    let hidden = client(&group);
    let program = apply(var("Symbol.CMajor"), []);
    let (open_ty, open) = infer(&public, &program).expect("a public case elaborates");
    let (hidden_ty, closed) = infer(&hidden, &program).expect("and so does the same one beside a hidden neighbour");
    assert_eq!(open, closed, "the marker changed a term that never named it");
    assert_eq!(open_ty, hidden_ty, "nor may it change the type");
}

/// The same fixture with nothing hidden, for the law above to compare against.
fn open_chord() -> RawData {
    data(
        Vec::new(),
        vec![
            family(
                "Symbol",
                vec![constructor("GSeven", Vec::new()), constructor("CMajor", Vec::new())],
            ),
            family(
                "Chord",
                vec![constructor("Named", vec![binder("symbol", var("Symbol"))])],
            ),
        ],
    )
}

/// A program reaching each visibility refusal, for `elaboration_laws.rs`'s
/// coverage gate.
///
/// Its own corpus rather than a row in the shared one, because both of these are
/// questions about *where* a term is elaborated and the shared corpus has one
/// context per block. The type is a term already, since the client can name the
/// type even when it cannot name its cases.
pub(crate) struct RefusedOutside {
    pub(crate) name: &'static str,
    pub(crate) raw: Raw,
    pub(crate) ty: Term,
    pub(crate) expected: fn(&Refusal) -> bool,
}

/// The client context those programs are refused in, and the programs.
pub(crate) fn refused_outside() -> (Cx, Vec<RefusedOutside>) {
    let (_, group) = package();
    let cx = client(&group);
    let chord = infer(&cx, &var("Chord")).expect("the type is public").0;
    let symbol = infer(&cx, &var("Symbol")).expect("the symbol type is public").0;
    let programs = vec![
        RefusedOutside {
            name: "a client naming a private constructor",
            raw: named(),
            ty: chord.clone(),
            expected: |refusal: &Refusal| matches!(*refusal, Refusal::Private { .. }),
        },
        RefusedOutside {
            name: "a client matching on an abstract type",
            raw: Raw::lam(WRITTEN, "c", symbol_of(var("c"))),
            ty: Term::pi(WRITTEN, "c", chord, symbol),
            expected: |refusal: &Refusal| matches!(*refusal, Refusal::AbstractMatch { .. }),
        },
    ];
    (cx, programs)
}
