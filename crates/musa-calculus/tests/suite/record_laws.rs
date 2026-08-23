//! `01-surface.md` §1.2's records and §1.3's enums: what a field update means,
//! what a record pattern binds, and how a bare constructor is read.
//!
//! The two are one suite because after prompt 157 they are one mechanism spelled
//! two ways. A record is a `data` family with one constructor, its fields are
//! that constructor's telescope, and `r.f` is a generated accessor; an enum is a
//! `data` family with several and no accessors at all. Both are nominal, and the
//! laws below are what survives of the structural reading: η holds at the one
//! that has a single constructor, `with` is the literal it desugars to, and a
//! record pattern is the projections it stands for.

use musa_calculus::{Cx, Raw, RawArm, RawData, RawPattern, Refusal, Term};

use crate::family_laws::{apply, binder, constructor, data, family, nat_context, type0, var, vec};
use crate::programs::WRITTEN;

/// `data Pair (A : Type 0) { Pair(fst: A, snd: A) }`.
///
/// The smallest one-constructor family there is over a parameter, and the one
/// `normalization_laws.rs` and `provenance_laws.rs` state η at: both are about
/// what quotation *writes*, so they want a record with nothing else in it.
pub(crate) fn pair() -> RawData {
    data(
        vec![binder("A", type0())],
        vec![family(
            "Pair",
            vec![constructor(
                "Pair",
                vec![binder("fst", var("A")), binder("snd", var("A"))],
            )],
        )],
    )
}

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
    let cx = cx.declaring(&slur);
    // The records the laws below are stated over, declared here for the reason
    // every other type in this context is: after prompt 157 a record type is not
    // something a term can write inline.
    [
        data(
            Vec::new(),
            vec![family(
                "Reading",
                vec![constructor(
                    "Reading",
                    vec![binder("refusal", var("Nat")), binder("count", var("Nat"))],
                )],
            )],
        ),
        data(
            Vec::new(),
            vec![family(
                "Pending",
                vec![constructor(
                    "Pending",
                    vec![binder("tie", var("Tying")), binder("read", var("Reading"))],
                )],
            )],
        ),
        data(
            Vec::new(),
            vec![family(
                "Counted",
                vec![constructor(
                    "Counted",
                    vec![binder("A", type0()), binder("x", var("A"))],
                )],
            )],
        ),
        pair(),
    ]
    .iter()
    .fold(cx, |cx, declared| {
        let group = musa_calculus::declare(&cx, declared).expect("the record laws' declarations are declarations");
        cx.declaring(&group)
    })
}

/// `record Pending { tie : Tying, read : Reading }`, declared in
/// [`tying_context`].
///
/// The staff adapter's `Pending`, cut down to the shape the laws need: one field
/// at a declared type and one nested record, so a path update has somewhere to
/// go and a carried-over field has something to carry.
fn pending() -> Raw {
    var("Pending")
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

/// §1.2 after prompt 157: two records with the same fields are **two** types.
///
/// The sentence this file used to state the other way round. A record was
/// structural while it was a shape in the core; it is a `data` family now, and a
/// family is nominal for the reason §1.3 gives — the declaration is what the
/// type *is*. The question is asked where it bites: two declarations, field for
/// field identical, and a value of one handed to a function that takes the
/// other.
///
/// Nothing depended on the old reading. It existed so that trait dictionaries
/// were convertible without being declared, and prompt 146 deleted the traits.
#[test]
fn two_records_with_the_same_fields_are_two_types() {
    let cx = tying_context();
    let holding = |name: &str| {
        data(
            Vec::new(),
            vec![family(name, vec![constructor(name, vec![binder("here", var("Nat"))])])],
        )
    };
    let cx = [holding("Here"), holding("Also")].iter().fold(cx, |cx, declared| {
        let group = musa_calculus::declare(&cx, declared).expect("each is a declaration");
        cx.declaring(&group)
    });
    let also = core(&cx, "Also", &var("Also"));
    let Err(error) = musa_calculus::check(&cx, &also, &apply(var("Here.Here"), [var("Nat.Zero")])) else {
        panic!("a `Here` must not inhabit `Also`, however alike they are written");
    };
    let refusal = crate::programs::refusal("one record's value at the other's type", error);
    assert!(matches!(refusal, Refusal::Mismatch(_)), "refused, but as `{refusal}`");
}

/// §1.2's η, kept: a record is its fields, read back and put together again.
///
/// The rule prompt 157 was free to drop and chose to keep, so this is the law
/// that says the choice was made. It is stated over a *variable*, because that
/// is the only place η can be observed: a literal is already the constructor
/// applied to its fields, and the question is whether something that is not one
/// is convertible with the one it would expand to.
#[test]
fn a_record_is_its_fields_put_back_together() {
    let cx = tying_context();
    let ty = core(&cx, "Pending → Pending", &arrow(pending(), pending()));
    let expanded = apply(
        var("Pending.Pending"),
        [
            Raw::project(WRITTEN, var("p"), "tie"),
            Raw::project(WRITTEN, var("p"), "read"),
        ],
    );
    same(
        &cx,
        "a variable against the literal η expands it to",
        &ty,
        &of_pending(var("p")),
        &of_pending(expanded),
    );
}

/// The same rule where the second field's type mentions the first.
///
/// The case a field-by-field comparison gets wrong by walking a fixed telescope
/// instead of the one the earlier fields decided: `x`'s type is whatever `A`
/// stood at, so reading `x` back means knowing what was read back for `A`.
#[test]
fn eta_holds_where_a_later_field_depends_on_an_earlier_one() {
    let cx = tying_context();
    let ty = core(&cx, "Counted → Counted", &arrow(counted(), counted()));
    let of_counted = |body: Raw| Raw::annotated_lam(WRITTEN, "v", counted(), body);
    let expanded = apply(
        var("Counted.Counted"),
        [
            Raw::project(WRITTEN, var("v"), "A"),
            Raw::project(WRITTEN, var("v"), "x"),
        ],
    );
    same(
        &cx,
        "a dependent record against the literal η expands it to",
        &ty,
        &of_counted(var("v")),
        &of_counted(expanded),
    );
}

/// §1.2: a one-constructor family with a *recursive* field admits no η.
///
/// The guard on the rule above, and the reason it is a guard rather than a
/// special case: expanding `r` into `Wrap (r.next)` writes a term that expands
/// again, so η at a recursive record does not terminate. The two are still
/// convertible when they are the same term, which is what this checks — the
/// point is that the checker answers at all.
#[test]
fn a_recursive_one_constructor_family_is_left_alone() {
    let (cx, _) = nat_context();
    let group = musa_calculus::declare(
        &cx,
        &data(
            Vec::new(),
            vec![family(
                "Stream",
                vec![constructor(
                    "Stream",
                    vec![binder("head", var("Nat")), binder("tail", var("Stream"))],
                )],
            )],
        ),
    )
    .expect("Stream is a declaration");
    let cx = cx.declaring(&group);
    let ty = core(&cx, "Stream → Stream", &arrow(var("Stream"), var("Stream")));
    same(
        &cx,
        "a recursive record against itself",
        &ty,
        &Raw::annotated_lam(WRITTEN, "s", var("Stream"), var("s")),
        &Raw::annotated_lam(WRITTEN, "s", var("Stream"), var("s")),
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
                Raw::update(WRITTEN, var("v"), [(&["A"][..], var("Tying"))]),
            ),
            ty: arrow(counted(), counted()),
            expected: |refusal| matches!(refusal, Refusal::Mismatch(_)),
        },
    ]
}

/// `record Counted { A : Type 0, x : A }` — a record whose second field's type
/// mentions its first, with the dependency a parameter rather than an index:
/// §1's value dependency needs nothing more.
fn counted() -> Raw {
    var("Counted")
}

/// §1.2: a record declaring one field twice is refused at the declaration.
///
/// Not in [`refused_records`] with the rest, because it is not a program: a
/// record is a declaration now, so the mistake is made — and has to be caught —
/// where the accessors are generated. Two fields of one name would be two
/// constants of one name, and one of the fields would be unreachable.
#[test]
fn a_record_declaring_one_field_twice_is_refused() {
    let (name, cx, twice) = one_field_twice();
    let Err(error) = musa_calculus::declare(&cx, &twice) else {
        panic!("a record with two `a`s must not declare");
    };
    let refusal = crate::programs::refusal(name, error);
    assert!(
        matches!(refusal, Refusal::DuplicateField { .. }),
        "refused, but as `{refusal}`"
    );
}

/// The declaration above, so `elaboration_laws` can count the refusal it
/// reaches: §1.2's duplicate field is the one refusal in this file no *term*
/// can reach, because after prompt 157 the mistake is made where the accessors
/// are generated.
pub(crate) fn one_field_twice() -> (&'static str, Cx, RawData) {
    (
        "a record declaring one field twice",
        tying_context(),
        data(
            Vec::new(),
            vec![family(
                "Twice",
                vec![constructor(
                    "Twice",
                    vec![binder("a", var("Nat")), binder("a", var("Tying"))],
                )],
            )],
        ),
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
