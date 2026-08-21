//! `01-surface.md` §1.2's records and §1.3's enums: what a field update means,
//! what a record pattern binds, and how a bare constructor is read.
//!
//! The two are one suite because they are one mechanism seen twice. A record is
//! structural — two record types with the same fields at the same types are the
//! same type, and nothing declares them — while an enum is nominal, each `data`
//! generating its own family whose constructors live in its own namespace. Every
//! law below turns on exactly that difference, so separating them into two files
//! would put each half of a contrast in a different place.

use musa_calculus::{Cx, Raw, RawArm, RawPattern, Refusal, Term};

use crate::family_laws::{apply, binder, constructor, data, family, nat_context, var, vec};
use crate::programs::WRITTEN;

/// `Nat`, `Vec`, and two enums that both declare `Untied`.
///
/// The second `Untied` is the point rather than a curiosity: it is the collision
/// that made the old checker carry a hard-coded list of which surface names were
/// phase types, and §1.3's namespaces are what delete it. A law about namespaced
/// constructors stated over a context where every case name happened to be
/// unique would be a law about that context.
///
/// # Panics
///
/// If any declaration is refused, which would be a defect in this crate.
pub(crate) fn tying_context() -> Cx {
    let (cx, _) = nat_context();
    let vectors = musa_calculus::declare(&cx, &vec()).expect("Vec is a declaration");
    let cx = cx.declaring(&vectors);
    let tying = musa_calculus::declare(
        &cx,
        &data(
            Vec::new(),
            vec![family(
                "Tying",
                vec![
                    constructor("Untied", Vec::new()),
                    constructor("TiedOn", vec![binder("n", var("Nat"))]),
                ],
            )],
        ),
    )
    .expect("Tying is a declaration");
    let cx = cx.declaring(&tying);
    let slur = musa_calculus::declare(
        &cx,
        &data(
            Vec::new(),
            vec![family("Slur", vec![constructor("Untied", Vec::new())])],
        ),
    )
    .expect("Slur is a declaration");
    cx.declaring(&slur)
}

/// `{ tie : Tying, read : { refusal : Nat, count : Nat } }`.
///
/// The staff adapter's `Pending`, cut down to the shape the laws need: one field
/// at a declared type and one nested record, so a path update has somewhere to
/// go and a carried-over field has something to carry.
fn pending() -> Raw {
    Raw::record_type(
        WRITTEN,
        [
            ("tie", var("Tying")),
            (
                "read",
                Raw::record_type(WRITTEN, [("refusal", var("Nat")), ("count", var("Nat"))]),
            ),
        ],
    )
}

fn arrow(domain: Raw, codomain: Raw) -> Raw {
    Raw::pi(WRITTEN, "_", domain, codomain)
}

/// A written type as a core term, for a checking question.
///
/// # Panics
///
/// If it is not a type, which is a defect in the test that wrote it.
fn core(cx: &Cx, name: &str, ty: &Raw) -> Term {
    musa_calculus::infer(cx, ty)
        .unwrap_or_else(|error| panic!("{name}: {error}"))
        .0
}

/// Assert two terms at `ty` are definitionally equal.
///
/// # Panics
///
/// If they are not, or if either is not a term at `ty`.
fn same(cx: &Cx, name: &str, ty: &Term, left: &Raw, right: &Raw) {
    let left = musa_calculus::check(cx, ty, left).unwrap_or_else(|error| panic!("{name} (left): {error}"));
    let right = musa_calculus::check(cx, ty, right).unwrap_or_else(|error| panic!("{name} (right): {error}"));
    assert!(
        musa_calculus::convertible(cx, ty, &left, &right).unwrap_or_else(|error| panic!("{name}: {error}")),
        "{name}"
    );
}

/// A function of a `Pending`, written with its binder type.
fn of_pending(body: Raw) -> Raw {
    Raw::annotated_lam(WRITTEN, "p", pending(), body)
}

/// `p.read.count`, the leaf every update law reads back.
fn count(record: Raw) -> Raw {
    Raw::project(WRITTEN, Raw::project(WRITTEN, record, "read"), "count")
}

/// `p with { read.count = n }`.
fn recount(record: Raw, replacement: Raw) -> Raw {
    Raw::update(WRITTEN, record, [(&["read", "count"][..], replacement)])
}

/// §1.2: a field read back out of a literal is the value that was written.
///
/// The β-law of records, at a nested one: `{ … }.f` computes rather than
/// standing as a neutral, and it computes through a `let` that gave the literal
/// its type.
#[test]
fn a_field_read_back_is_the_one_that_was_written() {
    let cx = tying_context();
    let nat = core(&cx, "Nat", &var("Nat"));
    let written = Raw::annotated_bind(
        WRITTEN,
        "p",
        pending(),
        Raw::record(
            WRITTEN,
            [
                ("tie", var("Tying.Untied")),
                (
                    "read",
                    Raw::record(
                        WRITTEN,
                        [
                            ("refusal", var("Nat.Zero")),
                            ("count", apply(var("Nat.Succ"), [var("Nat.Zero")])),
                        ],
                    ),
                ),
            ],
        ),
        count(var("p")),
    );
    same(
        &cx,
        "a nested projection out of a literal",
        &nat,
        &written,
        &apply(var("Nat.Succ"), [var("Nat.Zero")]),
    );
}

/// §1.2: an update along a path replaces the field it names and carries every
/// other one over unchanged.
///
/// Stated as three laws about the same term rather than one, because "replaces
/// one field" and "carries the rest" are the two halves that an update can get
/// wrong independently, and the second is the half a naive implementation drops.
/// Stated over a *variable* `p` rather than a literal, so the carried-over
/// fields are neutral projections and nothing computes them away.
#[test]
fn an_update_replaces_one_field_and_carries_the_rest() {
    let cx = tying_context();
    let one = apply(var("Nat.Succ"), [var("Nat.Zero")]);
    let updated = || recount(var("p"), one.clone());
    let to_nat = core(&cx, "Pending → Nat", &arrow(pending(), var("Nat")));
    same(
        &cx,
        "the replaced field is the replacement",
        &to_nat,
        &of_pending(count(updated())),
        &of_pending(one.clone()),
    );
    same(
        &cx,
        "a sibling of the replaced field is carried over",
        &to_nat,
        &of_pending(Raw::project(
            WRITTEN,
            Raw::project(WRITTEN, updated(), "read"),
            "refusal",
        )),
        &of_pending(Raw::project(
            WRITTEN,
            Raw::project(WRITTEN, var("p"), "read"),
            "refusal",
        )),
    );
    let to_tying = core(&cx, "Pending → Tying", &arrow(pending(), var("Tying")));
    same(
        &cx,
        "a field outside the path is carried over",
        &to_tying,
        &of_pending(Raw::project(WRITTEN, updated(), "tie")),
        &of_pending(Raw::project(WRITTEN, var("p"), "tie")),
    );
}

/// §1.2: the update is the literal the author would otherwise have written.
///
/// The whole meaning of `with`, in one conversion: it is sugar, and this is what
/// it is sugar *for*. §9.1's `let` is why the subject appears once on the left
/// and three times on the right and the two are still the same term.
#[test]
fn an_update_is_the_literal_it_desugars_to() {
    let cx = tying_context();
    let ty = core(&cx, "Pending → Pending", &arrow(pending(), pending()));
    let one = apply(var("Nat.Succ"), [var("Nat.Zero")]);
    let by_hand = Raw::record(
        WRITTEN,
        [
            ("tie", Raw::project(WRITTEN, var("p"), "tie")),
            (
                "read",
                Raw::record(
                    WRITTEN,
                    [
                        (
                            "refusal",
                            Raw::project(WRITTEN, Raw::project(WRITTEN, var("p"), "read"), "refusal"),
                        ),
                        ("count", one.clone()),
                    ],
                ),
            ),
        ],
    );
    same(
        &cx,
        "an update against the literal it stands for",
        &ty,
        &of_pending(recount(var("p"), one)),
        &of_pending(by_hand),
    );
}

/// §1.2: two record types with the same fields at the same types are one type.
///
/// Structural, not nominal — the distinguishing property, and the one that
/// cannot be observed by writing the same type twice in one place. The question
/// is asked where it bites: a function's domain is one written record type and
/// its argument's annotation is another, and nothing relates them but their
/// fields.
#[test]
fn two_records_with_the_same_fields_are_one_type() {
    let cx = tying_context();
    let nat = core(&cx, "Nat", &var("Nat"));
    let applied = Raw::annotated_bind(
        WRITTEN,
        "r",
        // Written a second time, field for field. A nominal record would make
        // this a different type from the λ's domain below.
        Raw::record_type(WRITTEN, [("here", var("Nat"))]),
        Raw::record(WRITTEN, [("here", var("Nat.Zero"))]),
        apply(
            Raw::annotated_lam(
                WRITTEN,
                "q",
                Raw::record_type(WRITTEN, [("here", var("Nat"))]),
                Raw::project(WRITTEN, var("q"), "here"),
            ),
            [var("r")],
        ),
    );
    same(
        &cx,
        "a record built at one written type, used at another",
        &nat,
        &applied,
        &var("Nat.Zero"),
    );
}

/// §6.2 at a record: a record pattern binds the fields it names, and nothing
/// else.
///
/// A record has one shape, so matching it asks no coverage question: the column
/// expands into one column per named field and the same matrix is re-solved.
/// The law that says so is that the `match` and the projections are the same
/// function.
#[test]
fn a_record_pattern_is_the_projections_it_stands_for() {
    let cx = tying_context();
    let ty = core(&cx, "Pending → Nat", &arrow(pending(), var("Nat")));
    let by_pattern = of_pending(Raw::match_on(
        WRITTEN,
        [var("p")],
        vec![RawArm {
            patterns: vec![RawPattern::record(
                WRITTEN,
                [(
                    "read",
                    RawPattern::record(WRITTEN, [("count", RawPattern::bind(WRITTEN, "c"))]),
                )],
            )],
            body: var("c"),
        }],
    ));
    same(
        &cx,
        "a nested record pattern against the projections it names",
        &ty,
        &by_pattern,
        &of_pending(count(var("p"))),
    );
}

/// §1.3: a bare constructor is read in the namespace of the type expected here.
///
/// The rule that makes `Untied` writable at all, and the reason it is admitted
/// only in checking position: `Tying` names the family, and the family names the
/// namespace the word is looked up in. Both spellings must mean the same term,
/// or the short one would be a second constructor rather than a way of writing
/// the first.
#[test]
fn a_bare_constructor_is_the_one_the_expected_type_names() {
    let cx = tying_context();
    let tying = core(&cx, "Tying", &var("Tying"));
    same(
        &cx,
        "a bare case against its qualified spelling",
        &tying,
        &var("Untied"),
        &var("Tying.Untied"),
    );
    let slur = core(&cx, "Slur", &var("Slur"));
    same(
        &cx,
        "the same word, read in the other family's namespace",
        &slur,
        &var("Untied"),
        &var("Slur.Untied"),
    );
}

/// §1.3: the same case name in two families is two constructors.
///
/// The collision, stated as a refusal rather than as an inequality: the two
/// terms inhabit different types, so "are they equal" is not a question §3 can
/// be asked. What can be asked is whether one is accepted where the other
/// belongs, and it must not be.
#[test]
fn the_same_case_name_in_two_families_does_not_collide() {
    let cx = tying_context();
    let tying = core(&cx, "Tying", &var("Tying"));
    let Err(error) = musa_calculus::check(&cx, &tying, &var("Slur.Untied")) else {
        panic!("`Slur.Untied` must not inhabit `Tying`");
    };
    let refusal = crate::programs::refusal("the other family's case at this type", error);
    assert!(matches!(refusal, Refusal::Mismatch(_)), "refused, but as `{refusal}`");
}

/// A record or enum program §1.2 or §1.3 refuses, and the refusal it owes.
pub(crate) struct RefusedRecord {
    pub(crate) name: &'static str,
    pub(crate) raw: Raw,
    /// The type to check it against, written rather than elaborated: these
    /// programs are checked in [`tying_context`], and a `Term` cannot be built
    /// without it.
    pub(crate) ty: Raw,
    pub(crate) expected: fn(&Refusal) -> bool,
}

/// Every way §1.2 and §1.3 say a record or an enum is written wrong.
pub(crate) fn refused_records() -> Vec<RefusedRecord> {
    vec![
        RefusedRecord {
            name: "a record type declaring one field twice",
            raw: Raw::record_type(WRITTEN, [("a", var("Nat")), ("a", var("Tying"))]),
            // §1.1: two fixed universes, and the second is not itself a type —
            // the expectation is elaborated by being inferred, so the largest
            // writable one is the first, which this record's small fields meet.
            ty: Raw::universe(WRITTEN, musa_calculus::Sort::ZERO),
            expected: |refusal| matches!(refusal, Refusal::DuplicateField { .. }),
        },
        RefusedRecord {
            name: "an update whose paths cover one another",
            raw: of_pending(Raw::update(
                WRITTEN,
                var("p"),
                [
                    (
                        &["read"][..],
                        Raw::record(WRITTEN, [("refusal", var("Nat.Zero")), ("count", var("Nat.Zero"))]),
                    ),
                    (&["read", "count"][..], var("Nat.Zero")),
                ],
            )),
            ty: arrow(pending(), pending()),
            expected: |refusal| matches!(refusal, Refusal::OverlappingUpdate { .. }),
        },
        RefusedRecord {
            name: "a bare constructor where nothing says of which type",
            // A `let` whose value is inferred: §1.3 admits the short spelling
            // where a type is expected, and here nothing expects one.
            raw: Raw::bind(WRITTEN, "t", var("Untied"), var("t")),
            ty: var("Tying"),
            expected: |refusal| matches!(refusal, Refusal::BareConstructor { .. }),
        },
        RefusedRecord {
            name: "the other family's case in a match arm",
            raw: Raw::lam(
                WRITTEN,
                "t",
                Raw::match_on(
                    WRITTEN,
                    [var("t")],
                    vec![RawArm {
                        patterns: vec![RawPattern::constructor(WRITTEN, "Slur.Untied", [])],
                        body: var("Nat.Zero"),
                    }],
                ),
            ),
            ty: arrow(var("Tying"), var("Nat")),
            expected: |refusal| matches!(refusal, Refusal::NoSuchConstructor { .. }),
        },
        RefusedRecord {
            name: "a record pattern against a family",
            raw: Raw::lam(
                WRITTEN,
                "t",
                Raw::match_on(
                    WRITTEN,
                    [var("t")],
                    vec![RawArm {
                        patterns: vec![RawPattern::record(WRITTEN, [("tie", RawPattern::bind(WRITTEN, "x"))])],
                        body: var("Nat.Zero"),
                    }],
                ),
            ),
            ty: arrow(var("Tying"), var("Nat")),
            expected: |refusal| matches!(refusal, Refusal::NotARecord { .. }),
        },
        RefusedRecord {
            name: "a constructor pattern against a record",
            raw: of_pending(Raw::match_on(
                WRITTEN,
                [var("p")],
                vec![RawArm {
                    patterns: vec![RawPattern::constructor(WRITTEN, "Tying.Untied", [])],
                    body: var("Nat.Zero"),
                }],
            )),
            ty: arrow(pending(), var("Nat")),
            expected: |refusal| matches!(refusal, Refusal::NoSuchConstructor { .. }),
        },
        RefusedRecord {
            name: "an update that invalidates a later field's type",
            // §1.2's coherence, and it is not a rule this crate states: rebuilding
            // checks each field at the type the *new* earlier values give it, so
            // `x : A` stops fitting the moment `A` moves to another type.
            raw: Raw::annotated_lam(
                WRITTEN,
                "v",
                counted(),
                Raw::update(WRITTEN, var("v"), [(&["A"][..], Raw::record_type(WRITTEN, []))]),
            ),
            ty: arrow(counted(), counted()),
            expected: |refusal| matches!(refusal, Refusal::Mismatch(_)),
        },
    ]
}

/// `{ A : Type 0, x : A }` — a record whose second field's type mentions its
/// first, with the dependency a parameter rather than an index: §1's value
/// dependency needs nothing more.
fn counted() -> Raw {
    Raw::record_type(
        WRITTEN,
        [
            ("A", Raw::universe(WRITTEN, musa_calculus::Sort::ZERO)),
            ("x", var("A")),
        ],
    )
}

/// §1.2 and §1.3, refused where they must be.
#[test]
fn a_record_or_an_enum_is_refused_for_the_reason_it_is_wrong() {
    let cx = tying_context();
    for RefusedRecord {
        name,
        raw,
        ty,
        expected,
    } in refused_records()
    {
        let ty = core(&cx, name, &ty);
        let Err(error) = musa_calculus::check(&cx, &ty, &raw) else {
            panic!("{name}: elaboration accepted a program it must refuse");
        };
        let refusal = crate::programs::refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
    }
}
