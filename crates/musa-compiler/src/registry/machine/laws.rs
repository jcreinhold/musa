//! What has to be true of the eight machine forms.
//!
//! Unit tests inside the crate for [`crate::registry::laws`]'s reason, and
//! written in *raw* syntax rather than in assembled terms, which is the one
//! thing that makes this file different from [`super::super::track::laws`].
//! Every form here is polymorphic and every one of its type arguments is
//! implicit, so a hand-built [`musa_core::Term`] would have to supply four
//! metavariable solutions the elaborator exists to find. Writing the program the
//! way a source file would and handing it to [`musa_core::check`] is what makes
//! "`identity` is a machine at whatever ports the position wants" a claim about
//! the registration rather than about the test's arithmetic.
//!
//! There is no reduction half. §2 gives these forms no reductions, so the whole
//! of what a law can ask is that the typing rules hold, that building one the
//! same way twice gives the same value and differently gives a different one,
//! and that nothing turns into anything else when normalized.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a law that cannot fail loudly is not a law"
)]

use musa_core::{Cx, Level, Raw, Shape, Term};

use super::{SPELLINGS, UNREGISTERED};
use crate::core::{BUILTIN_OWNERSHIP, Family};
use crate::registry::{HERE, owned};

// ---- writing a program the way source would ----

fn name(spelling: &str) -> Raw {
    Raw::var(HERE, spelling)
}

fn applied(head: &str, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    arguments
        .into_iter()
        .fold(name(head), |function, argument| Raw::app(HERE, function, argument))
}

/// `Machine step input output`.
fn machine(input: Raw, output: Raw) -> Raw {
    applied("Machine", [step(), input, output])
}

/// `Primitive step input output`.
fn unit_type(input: Raw, output: Raw) -> Raw {
    applied("Primitive", [step(), input, output])
}

/// `Pair first second`.
fn pair(first: Raw, second: Raw) -> Raw {
    applied("Pair", [first, second])
}

/// `domain → codomain`.
fn arrow(domain: Raw, codomain: Raw) -> Raw {
    Raw::pi(HERE, "argument", domain, codomain)
}

/// The step tag every law below wires at.
///
/// Any type at all does: §2's `K` is a tag whose only job is to keep two
/// machines whose steps mean different things from being connected, and the core
/// gives it no other meaning. `Text` is a registered base type and is therefore
/// the shortest thing to write.
fn step() -> Raw {
    name("Text")
}

/// The sample ports. Four of them, because `beside` has four.
fn a() -> Raw {
    name("Nat")
}

fn b() -> Raw {
    name("Bool")
}

fn c() -> Raw {
    name("Ratio")
}

fn d() -> Raw {
    name("Pitch")
}

// ---- asking the core ----

/// The compiler's own context, with every declaration and every registration in
/// it.
fn context() -> Cx {
    owned().expect("the compiler's own context builds")
}

/// `raw`, elaborated as a type.
fn ty(cx: &Cx, raw: &Raw) -> Term {
    musa_core::check(cx, &Term::universe(HERE, Level::ZERO), raw)
        .unwrap_or_else(|why| panic!("a law's own type does not elaborate: {why}"))
}

/// `program`, checked at `at` and re-checked independently of the elaborator
/// that produced it.
fn checked(cx: &Cx, at: &Term, program: &Raw, what: &str) -> Term {
    let term = musa_core::check(cx, at, program)
        .unwrap_or_else(|why| panic!("`{what}` is not well typed at the type §2 gives it: {why}"));
    musa_core::well_typed(cx, at, &term)
        .unwrap_or_else(|why| panic!("`{what}` elaborates to something its own type refuses: {why}"));
    term
}

/// A spine's head and how many arguments it carries.
fn spine(term: &Term) -> (Term, usize) {
    let mut head = term.clone();
    let mut taken = 0usize;
    while let Shape::App { ref function, .. } = *head.clone().shape() {
        taken = taken.saturating_add(1);
        head = function.clone();
    }
    (head, taken)
}

// ---- the laws ----

/// The eight forms are registered under the names `03-machine-calculus.md` §2
/// gives them, and the ninth row is registered nowhere.
///
/// Both halves matter. The first is the claim `rules::UNREGISTERED` no longer
/// makes — it now counts one machine row instead of nine, and a count is only
/// honest if something checks the other side. The second is what keeps
/// `primitive`'s absence a decision: a spelling that quietly appeared in the
/// registry would be the build-local lookup answered by a signature that cannot
/// know it.
#[test]
fn the_eight_machine_rows_are_registered_and_the_ninth_is_not() {
    let cx = context();
    let registered = crate::registry::builtins(&cx).expect("both tables translate");
    for spelling in SPELLINGS {
        let builtin = registered
            .iter()
            .find(|builtin| &**builtin.name() == spelling)
            .unwrap_or_else(|| panic!("`{spelling}` is registered"));
        assert_eq!(
            builtin.family(),
            musa_core::Family::Machine,
            "`{spelling}` is §5.8's fourth family"
        );
        assert!(
            BUILTIN_OWNERSHIP
                .iter()
                .any(|entry| entry.spelling == spelling && matches!(entry.family, Family::Machine(_))),
            "`{spelling}` is the machine row the old table already had"
        );
        // Reaching it through `cx` rather than through the vector above is the
        // second half: `builtins` translating a row and `owned` installing it are
        // two things, and only the second is what a program sees.
        crate::registry::laws::resolves(&cx, spelling);
    }
    for spelling in UNREGISTERED {
        assert!(
            !registered.iter().any(|builtin| &**builtin.name() == spelling),
            "`{spelling}` is typed by a registry rather than by a signature, so it is not registered"
        );
        assert!(
            musa_core::infer(&cx, &name(spelling)).is_err(),
            "and so it is not nameable either"
        );
    }
}

/// Each form checks at the type `03-machine-calculus.md` §2 gives it, on a
/// sample of ports.
///
/// §2's rules are written with premises — `Γ ⊢ m : Machine<K,A,B>` — so the
/// programs below bind their machine arguments rather than supplying built ones.
/// That is the rule stated as a program: a λ whose binder is annotated at the
/// premise's type and whose body is the form applied to it, checked at the
/// conclusion. A registration one port out would fail to check here rather than
/// waiting for the first piece that wired two of them together.
#[test]
fn each_form_checks_at_the_type_the_calculus_gives_it() {
    let cx = context();
    let programs: [(&str, Raw, Raw); 8] = [
        // machine(p) : Machine<K,A,B>, given p : Primitive<K,A,B>
        (
            SPELLINGS[0],
            arrow(unit_type(a(), b()), machine(a(), b())),
            Raw::annotated_lam(
                HERE,
                "p",
                unit_type(a(), b()),
                Raw::app(HERE, name(SPELLINGS[0]), name("p")),
            ),
        ),
        // identity : Machine<K,A,A>
        (SPELLINGS[1], machine(a(), a()), name(SPELLINGS[1])),
        // connect(m,n) : Machine<K,A,D>
        (
            SPELLINGS[2],
            arrow(machine(a(), b()), arrow(machine(b(), c()), machine(a(), c()))),
            Raw::annotated_lam(
                HERE,
                "m",
                machine(a(), b()),
                Raw::annotated_lam(
                    HERE,
                    "n",
                    machine(b(), c()),
                    applied(SPELLINGS[2], [name("m"), name("n")]),
                ),
            ),
        ),
        // beside(m,n) : Machine<K,(A,D),(B,E)>
        (
            SPELLINGS[3],
            arrow(
                machine(a(), b()),
                arrow(machine(c(), d()), machine(pair(a(), c()), pair(b(), d()))),
            ),
            Raw::annotated_lam(
                HERE,
                "m",
                machine(a(), b()),
                Raw::annotated_lam(
                    HERE,
                    "n",
                    machine(c(), d()),
                    applied(SPELLINGS[3], [name("m"), name("n")]),
                ),
            ),
        ),
        // feedback(initial,m) : Machine<K,A,B>
        (
            SPELLINGS[4],
            arrow(c(), arrow(machine(pair(a(), c()), pair(b(), c())), machine(a(), b()))),
            Raw::annotated_lam(
                HERE,
                "initial",
                c(),
                Raw::annotated_lam(
                    HERE,
                    "m",
                    machine(pair(a(), c()), pair(b(), c())),
                    applied(SPELLINGS[4], [name("initial"), name("m")]),
                ),
            ),
        ),
        // copy : Machine<K,A,(A,A)>
        (SPELLINGS[5], machine(a(), pair(a(), a())), name(SPELLINGS[5])),
        // drop : Machine<K,A,Unit>
        (SPELLINGS[6], machine(a(), name("Unit")), name(SPELLINGS[6])),
        // swap : Machine<K,(A,B),(B,A)>
        (
            SPELLINGS[7],
            machine(pair(a(), b()), pair(b(), a())),
            name(SPELLINGS[7]),
        ),
    ];
    for (spelling, at, program) in programs {
        let at = ty(&cx, &at);
        checked(&cx, &at, &program, spelling);
    }
}

/// Two machines built the same way are the same machine, and two built
/// differently are not.
///
/// This is the whole of §2's equality, and it comes free: a form has no
/// reduction, so its saturated application is a neutral spine and conversion on
/// a spine is structural. What the law adds is that nothing *else* in the core
/// collapses them — `connect(identity, identity)` and `identity` have the same
/// type and are not the same machine, which they would be if a rule had been
/// registered that quietly turned one into the other.
#[test]
fn two_machines_are_the_same_exactly_when_they_were_built_the_same_way() {
    let cx = context();
    let at = ty(&cx, &machine(a(), a()));
    let chained = applied(SPELLINGS[2], [name(SPELLINGS[1]), name(SPELLINGS[1])]);
    let left = checked(&cx, &at, &chained, "connect(identity, identity)");
    let right = checked(&cx, &at, &chained, "connect(identity, identity)");
    let bare = checked(&cx, &at, &name(SPELLINGS[1]), SPELLINGS[1]);
    assert!(
        musa_core::convertible(&cx, &at, &left, &right).expect("conversion decides"),
        "two machines built the same way are one machine"
    );
    assert!(
        !musa_core::convertible(&cx, &at, &left, &bare).expect("conversion decides"),
        "a chain of two identities is not the identity: §2 gives it no reduction that would make it one"
    );
}

/// A form applied to fewer arguments than it takes is a well-typed value, and it
/// is a function rather than a machine.
///
/// §2's grammar admits `connect(m,n)` and not `connect(m)`, and the core agrees
/// without a rule saying so: a Π applied once is a Π, so `connect(identity)` is
/// a value of function type and can stand nowhere a `Machine` is wanted. The law
/// is worth stating because arity is the one thing a constructor's registration
/// does *not* check — [`musa_core::Builtin::arity`] is read off the signature and
/// nothing fires at it — so the ordinary typing rule is carrying all of it.
#[test]
fn a_form_short_of_its_arguments_is_a_function_and_not_a_machine() {
    let cx = context();
    let partial = arrow(machine(a(), b()), machine(a(), b()));
    let at = ty(&cx, &partial);
    checked(
        &cx,
        &at,
        &Raw::app(HERE, name(SPELLINGS[2]), name(SPELLINGS[1])),
        "connect(identity)",
    );
    let refused = musa_core::check(
        &cx,
        &ty(&cx, &machine(a(), b())),
        &Raw::app(HERE, name(SPELLINGS[2]), name(SPELLINGS[1])),
    );
    assert!(
        refused.is_err(),
        "a partly applied `connect` is not a machine, and the position that wants one says so"
    );
}

/// `connect` at ports that do not meet is refused by the core.
///
/// The old checker unified `m`'s output with `n`'s input in a hand-written arm.
/// Here the two share one implicit binder, so the refusal is the ordinary
/// conversion failure the elaborator reports for every other mismatched
/// argument — which is the point: there is no machine-shaped check to forget to
/// run, and a tenth form added tomorrow would be wired by the same rule.
#[test]
fn connect_at_mismatched_ports_is_refused_by_the_core() {
    let cx = context();
    let at = ty(&cx, &machine(a(), c()));
    let mismatched = applied(
        SPELLINGS[2],
        [
            Raw::annot(HERE, name(SPELLINGS[1]), machine(a(), a())),
            Raw::annot(HERE, name(SPELLINGS[1]), machine(b(), c())),
        ],
    );
    let refused = musa_core::check(&cx, &at, &mismatched);
    assert!(
        refused.is_err(),
        "`connect` may not chain a machine that answers a `Nat` into one that reads a `Bool`"
    );
}

/// A saturated form normalizes to itself.
///
/// The reduction half of `Reduction::None`, checked where it can actually fail:
/// [`musa_core::normalize`] runs the evaluator over the whole spine, and a form
/// registered with a δ-rule or a rewrite would either fire, get stuck, or
/// realize an answer at the wrong shape. What comes back is the same spine —
/// same head, same number of arguments — which is what "its application is its
/// value" means operationally.
#[test]
fn a_saturated_form_normalizes_to_itself() {
    let cx = context();
    let at = ty(&cx, &machine(a(), a()));
    let chained = applied(SPELLINGS[2], [name(SPELLINGS[1]), name(SPELLINGS[1])]);
    let term = checked(&cx, &at, &chained, "connect(identity, identity)");
    let (head, taken) = spine(&term);
    let normal = musa_core::normalize(&cx, &at, &term).expect("a machine normalizes");
    let (settled, left) = spine(&normal);
    let Shape::Builtin(ref before) = *head.shape() else {
        panic!("`connect(identity, identity)` is not headed by a builtin: {head:?}");
    };
    let Shape::Builtin(ref after) = *settled.shape() else {
        panic!("`connect(identity, identity)` normalized to something else: {settled:?}");
    };
    assert_eq!(before, after, "the head is still `connect`");
    assert_eq!(taken, left, "and it still carries every argument it was given");
    assert!(
        musa_core::convertible(&cx, &at, &term, &normal).expect("conversion decides"),
        "a machine and its normal form are the same machine"
    );
}
