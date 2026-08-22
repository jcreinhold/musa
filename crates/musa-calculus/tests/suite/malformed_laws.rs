//! Every [`Malformed`] the kernel can answer with, reached by something.
//!
//! `Refusal` has had a coverage gate since prompt 134:
//! `each_refusal_is_reached_by_the_program_it_is_about` matches every variant,
//! lists every tag, and reaches each one from a program, so a refusal nobody
//! can reach fails the suite rather than shipping as a message nobody has read.
//! [`Malformed`] had none. This is the same gate for the other enum.
//!
//! It cannot be the *same* gate, because the two enums are reached by different
//! things, which is the whole of what separates them (see [`CoreError`]'s doc).
//! A refusal is reached by a program an author could write. A malformation is
//! reached by a term, a registry, or a context **built by hand** — nobody wrote
//! it, and that is what the variant says. So the cases below hand the kernel
//! terms no elaborator would produce.
//!
//! Seven variants are reached by nothing here and say why in
//! [`UNREACHED`](self::UNREACHED). An entry there is an argument, not an
//! exemption: it has to name what would have to go wrong for the variant to
//! fire, and why no test can arrange that from outside the crate. Three of
//! them — the metavariable ones prompt 153 added — *are* reached, by unit tests
//! inside `kernel/unify.rs`, and the argument each makes here is why they have
//! to be reached from in there rather than from out here.

use std::any::Any;
use std::sync::Arc;

use musa_calculus::{
    Base, Budget, Builtin, Checked, CoreError, Cx, Datum, ElabError, Family, Index, Literal, Malformed, Payload, Raw,
    Registry, Role, Sort, Term,
};

/// Where every term this file builds says it was written.
const HERE: musa_calculus::Origin = musa_calculus::Origin::node(900);

/// `{}` as a type, and as its one inhabitant.
fn unit_type() -> Term {
    Term::record_type(HERE, [])
}

fn unit() -> Term {
    Term::record(HERE, [])
}

/// The malformation `outcome` carries, or a panic naming what arrived instead.
///
/// # Panics
///
/// When the outcome is not a malformation — an accepted term and an exhausted
/// budget are both the gate failing to reach what it claims to reach.
fn malformed<T>(name: &str, outcome: Result<T, CoreError>) -> Malformed {
    match outcome {
        Ok(_) => panic!("{name}: the kernel accepted a term nobody should have built"),
        Err(CoreError::Malformed(fault)) => fault,
        Err(other) => panic!("{name}: reached `{other}` rather than a malformation"),
    }
}

/// Which malformation this is, as a tag the gate can compare.
///
/// Exhaustive on purpose: a new variant is a compile error here, and then a
/// missing entry in [`ALL_MALFORMED`], and then a missing case.
fn kind(fault: &Malformed) -> &'static str {
    match fault {
        Malformed::UnboundVariable(_) => "unbound-variable",
        Malformed::UndeclaredName(_) => "undeclared-name",
        Malformed::NotAFunction => "not-a-function",
        Malformed::NotARecord => "not-a-record",
        Malformed::NoSuchField(_) => "no-such-field",
        Malformed::NotAType => "not-a-type",
        Malformed::EscapedVariable => "escaped-variable",
        Malformed::UnsolvedMeta(_) => "unsolved-meta",
        Malformed::Mistyped { .. } => "mistyped",
        Malformed::Uninferable => "uninferable",
        Malformed::AlreadySolved(_) => "already-solved",
        Malformed::BuiltinStuck(_) => "builtin-stuck",
        Malformed::MisfitAnswer(_) => "misfit-answer",
        Malformed::NotALiteral(_) => "not-a-literal",
        Malformed::UnregisteredCarrier(_) => "unregistered-carrier",
        Malformed::LevelArity(_) => "level-arity",
        Malformed::Cyclic(_) => "cyclic-meta",
        Malformed::MetaTelescope(_) => "meta-telescope",
        Malformed::EscapedSolution(_) => "escaped-solution",
        Malformed::UnreachableAlternative => "unreachable-alternative",
    }
}

/// Every malformation this crate can answer with.
const ALL_MALFORMED: [&str; 20] = [
    "unbound-variable",
    "undeclared-name",
    "not-a-function",
    "not-a-record",
    "no-such-field",
    "not-a-type",
    "escaped-variable",
    "unsolved-meta",
    "mistyped",
    "uninferable",
    "already-solved",
    "builtin-stuck",
    "misfit-answer",
    "not-a-literal",
    "unregistered-carrier",
    "level-arity",
    "cyclic-meta",
    "meta-telescope",
    "escaped-solution",
    "unreachable-alternative",
];

/// The ones nothing here reaches, each with the argument for why.
///
/// Read this as the gate's honest remainder rather than as a list of things to
/// get to later. Six of the eight *cannot* be reached from outside this crate
/// by construction, and saying so is the finding; one is not this crate's to
/// raise at all, and one has nothing that builds it yet.
const UNREACHED: [(&str, &str); 8] = [
    (
        "unreachable-alternative",
        "an `Impossible` node reaching emission. Nothing builds one: with no indices \
         (§1.1, and prompt 155's Stop) there is nothing for unification to refute, so \
         `CaseTree::Impossible` has no producer until prompt 156 adds index unification. \
         The variant is here because the node is, and the node is here because §6.2's \
         third case is part of what a case tree *is*.",
    ),
    (
        "escaped-variable",
        "quotation reaching a level its own depth does not name means levels and indices \
         were confused inside the kernel. No term expresses that — a `Term` holds indices \
         and the depth is quotation's own — so arranging it needs an edit to `quote.rs`, \
         which is the defect the variant reports.",
    ),
    (
        "unsolved-meta",
        "no public constructor builds a `Term` holding a metavariable, which is \
         `Checked`'s design rather than an oversight: the type is what stops one crossing, \
         so a test that could build one would be evidence the boundary leaks.",
    ),
    (
        "already-solved",
        "solutions are write-once, and the second write is the conversion checker \
         assigning a meta it had already assigned. Reaching it means editing `convert.rs` \
         to drop the check that makes it unreachable.",
    ),
    (
        "cyclic-meta",
        "the occurs check answers about two values held by the unifier, and reaching it needs \
         a metavariable — which `unsolved-meta` above says no public constructor builds. \
         `kernel::unify`'s own test module reaches it, being the one caller that can make one.",
    ),
    (
        "meta-telescope",
        "a metavariable whose type does not have the telescope its arity claims, or an \
         occurrence standing where that scope does not reach. Both need a metavariable built \
         wrong, which is not a thing a `Term` from out here can hold; `kernel::unify`'s test \
         module builds both.",
    ),
    (
        "escaped-solution",
        "the re-checker's second scope check, over a solution the unifier would never have \
         written. Arranging one means writing a solution directly, and `Meta::solve` is not \
         public — `kernel::unify`'s test module holds the negative control prompt 153 owes.",
    ),
    (
        "not-a-literal",
        "this crate never raises it. It is the word a *host* uses when a closed normal \
         form at a base type it registered does not hold its datum — `musa-compiler` \
         raises it in `registry.rs` and `document.rs` — and §5's canonicity is the \
         promise being broken. A gate here would be testing a caller.",
    ),
];

/// The coverage gate: every malformation is reached, or argued for.
///
/// The two halves have to agree exactly. A variant that is both reached and
/// listed as unreachable is a stale argument; a variant that is neither is a
/// message nobody has read.
#[test]
fn every_malformation_is_reached_by_something_or_argued_for() {
    let mut reached: std::collections::BTreeSet<&'static str> = std::collections::BTreeSet::new();
    for (name, fault) in reachable() {
        reached.insert(kind(&fault));
        assert!(
            !UNREACHED.iter().any(|(tag, _)| *tag == kind(&fault)),
            "{name}: `{}` is reached and also argued unreachable",
            kind(&fault)
        );
    }
    for (tag, argument) in UNREACHED {
        assert!(
            ALL_MALFORMED.contains(&tag),
            "`{tag}` is argued for and is not a malformation"
        );
        assert!(argument.len() > 80, "`{tag}`'s argument is too short to be one");
        reached.insert(tag);
    }
    assert_eq!(
        reached,
        ALL_MALFORMED.iter().copied().collect(),
        "every malformation needs something that reaches it, or an argument for why nothing can"
    );
}

/// One malformation each, from a term, a registry, or a context built by hand.
///
/// Returned rather than asserted in place so the gate above can compare the set
/// against [`ALL_MALFORMED`]; each case also asserts its own shape here, so a
/// case that reached the *wrong* malformation fails where it was written rather
/// than as a set difference.
fn reachable() -> Vec<(&'static str, Malformed)> {
    let cx = Cx::with_budget(Budget::LANGUAGE);
    let mut found = Vec::new();

    // An index with no binder under it. The smallest malformed term there is.
    let fault = malformed(
        "a variable in the empty context",
        musa_calculus::normalize(&cx, &unit_type(), &Term::var(HERE, Index(0))),
    );
    assert!(matches!(fault, Malformed::UnboundVariable(Index(0))), "{fault}");
    found.push(("a variable in the empty context", fault));

    // A name the elaborator resolved to nothing, which the elaborator would
    // have refused where it was written.
    let fault = malformed(
        "a name no declaration answers to",
        musa_calculus::normalize(&cx, &unit_type(), &Term::named(HERE, "nowhere", Role::Defined)),
    );
    assert!(matches!(fault, Malformed::UndeclaredName(_)), "{fault}");
    found.push(("a name no declaration answers to", fault));

    // `{} {}` — a record literal applied to one.
    let fault = malformed(
        "applying a record literal",
        musa_calculus::normalize(&cx, &unit_type(), &Term::app(HERE, unit(), unit())),
    );
    assert!(matches!(fault, Malformed::NotAFunction), "{fault}");
    found.push(("applying a record literal", fault));

    // `(Type 0).f` — a universe projected at a field.
    let fault = malformed(
        "projecting a universe",
        musa_calculus::normalize(
            &cx,
            &unit_type(),
            &Term::project(HERE, Term::universe(HERE, Sort::ZERO), "f"),
        ),
    );
    assert!(matches!(fault, Malformed::NotARecord), "{fault}");
    found.push(("projecting a universe", fault));

    // `{ a = {} }.b` — a field the literal does not have.
    let fault = malformed(
        "projecting a field the literal does not have",
        musa_calculus::normalize(
            &cx,
            &unit_type(),
            &Term::project(HERE, Term::record(HERE, [("a", unit())]), "b"),
        ),
    );
    assert!(matches!(fault, Malformed::NoSuchField(_)), "{fault}");
    found.push(("projecting a field the literal does not have", fault));

    // `let u : {} = {} in (x : u) → {}` — a value standing as a domain. The
    // `let` is what gives the domain a type at all: a bare `{}` is an
    // introduction form and would be refused one step earlier.
    let fault = rechecking(
        "a value standing in domain position",
        &cx,
        &Term::universe(HERE, Sort::ZERO),
        &Term::bind(
            HERE,
            "u",
            unit_type(),
            unit(),
            Term::pi(HERE, "x", Term::var(HERE, Index(0)), unit_type()),
        ),
    );
    assert!(matches!(fault, Malformed::NotAType), "{fault}");
    found.push(("a value standing in domain position", fault));

    // `Type 0 : Type 0` — off by exactly one universe, the smallest wrong
    // answer §1.1 admits.
    let fault = rechecking(
        "a universe one level too low",
        &cx,
        &Term::universe(HERE, Sort::ZERO),
        &Term::universe(HERE, Sort::ZERO),
    );
    assert!(matches!(fault, Malformed::Mistyped { .. }), "{fault}");
    found.push(("a universe one level too low", fault));

    // `(λx. x).f` — a λ in inference position. §2 makes a λ a checking form
    // because it carries no domain, so the term around this one gave it no
    // type and there is none to derive.
    let fault = rechecking(
        "a lambda projected at a field",
        &cx,
        &unit_type(),
        &Term::project(HERE, Term::lam(HERE, "x", Term::var(HERE, Index(0))), "f"),
    );
    assert!(matches!(fault, Malformed::Uninferable), "{fault}");
    found.push(("a lambda projected at a field", fault));

    found.extend(crate::sort_laws::level_faults());
    found.extend(host_faults());
    found
}

/// Re-check `term` at `ty` and take the malformation it owes.
///
/// # Panics
///
/// Naming the case, when the kernel accepts the term or answers otherwise.
fn rechecking(name: &str, cx: &Cx, ty: &Term, term: &Term) -> Malformed {
    let checked = Checked::try_from(term.clone())
        .unwrap_or_else(|fault| panic!("{name}: the term holds no metavariable, and: {fault}"));
    malformed(name, musa_calculus::recheck(cx, ty, &checked))
}

/// The malformation an *elaborated* outcome carries.
///
/// [`Malformed::UnregisteredCarrier`] is raised while a program is being
/// checked, so it arrives inside an [`ElabError`] rather than a [`CoreError`].
/// Which enum carries it does not change which enum it *is*: the sentence is
/// addressed to whoever maintains this compiler, and the host's own rule named
/// the thing that is missing.
///
/// # Panics
///
/// When the outcome is not a malformation.
fn refused<T>(name: &str, outcome: Result<T, ElabError>) -> Malformed {
    match outcome {
        Ok(_) => panic!("{name}: elaboration accepted a program the registry cannot serve"),
        Err(ElabError::Malformed(fault)) => fault,
        Err(other) => panic!("{name}: reached `{other}` rather than a malformation"),
    }
}

// ---- a host whose table and its own signatures disagree ---------------------
//
// Four malformations are reachable only through a *registration*, and each is
// the same defect at a different moment: the host's rule and the host's
// signature say different things, and the program that met them was well typed.
// So this section registers a small host that is wrong on purpose. It is not
// Musa's, for `index_laws.rs`'s reason — `Count` is an index domain and `Row` is
// a type that carries one, and neither is a musical word.

/// A whole number, as a host would carry one.
#[derive(Debug)]
struct Count(i128);

impl Payload for Count {
    fn same(&self, other: &dyn Payload) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|it| it.0 == self.0)
    }

    fn shown(&self) -> String {
        self.0.to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `Count : Type 0`, the host domain every fixture below is written over.
fn count() -> Base {
    Base::new("Count", Term::universe(HERE, Sort::ZERO))
}

fn count_type() -> Term {
    count().term(HERE)
}

fn count_lit(value: i128) -> Literal {
    Literal::new(count_type(), Arc::new(Count(value)))
}

fn as_count(literal: &Literal) -> Option<i128> {
    literal.payload().as_any().downcast_ref::<Count>().map(|it| it.0)
}

fn arrow(domain: Term, codomain: Term) -> Term {
    Term::pi(HERE, "_", domain, codomain)
}

/// `Row : Count → Type 0`, naming a carrier nobody registered.
///
/// The [`musa_calculus::Accepts`] rule is the defect: the host says a `Row` at
/// one index may stand where a `Row` at another was wanted, and names
/// `count_coerce` to carry it across — and the registry below does not hold a
/// builtin by that name.
fn row() -> Base {
    Base::new("Row", arrow(count_type(), Term::universe(HERE, Sort::ZERO)))
        .accepting(|_wanted, _held| Some("count_coerce"))
}

/// `count_add : Count → Count → Count`, and `checked_add` answers nothing at the
/// top of the range.
///
/// §5.8's D2 promises a closed value of the declared result type for every tuple
/// of closed values of the declared argument types. This is that promise broken,
/// and no source edit can fix it.
fn count_add() -> Builtin {
    Builtin::new(
        "count_add",
        arrow(count_type(), arrow(count_type(), count_type())),
        Family::Delta,
        |arguments| match arguments {
            [Datum::Lit(left), Datum::Lit(right)] => {
                Some(Datum::Lit(count_lit(as_count(left)?.checked_add(as_count(right)?)?)).into())
            }
            _ => None,
        },
    )
}

/// `count_tally : Count → Count`, whose rule answers a *count at a family*.
///
/// The signature says the answer is a `Count`, which is a base type; the rule
/// answers data standing at a counting family called `Nat`, which this host
/// never declared. Realization reads the type rather than the name it was
/// handed, finds no family there, and says so.
fn count_tally() -> Builtin {
    Builtin::new(
        "count_tally",
        arrow(count_type(), count_type()),
        Family::Delta,
        |arguments| match arguments {
            [Datum::Lit(_)] => Some(
                Datum::Count {
                    family: "Nat".into(),
                    count: 1,
                }
                .into(),
            ),
            _ => None,
        },
    )
}

/// `row_of : (n : Count) → Row n` — a constructor, so a `Row` at a known index
/// can be written at all.
fn row_of() -> Builtin {
    Builtin::constructor(
        "row_of",
        Term::pi(
            HERE,
            "n",
            count_type(),
            Term::app(HERE, row().term(HERE), Term::var(HERE, Index(0))),
        ),
        Family::Machine,
    )
}

/// The host, registered.
///
/// # Panics
///
/// If the registry refuses its own signatures. It does not: every fault below is
/// a disagreement between a signature and a *rule*, and D1 reads signatures.
fn host() -> Cx {
    let registry = Registry::new(vec![count(), row()], vec![count_add(), count_tally(), row_of()])
        .expect("the signatures are admissible; it is the rules that are wrong");
    Cx::with_budget(Budget::LANGUAGE).with_externs(Arc::new(registry))
}

/// `f a b …`, as a raw term.
fn calls(function: &str, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    arguments
        .into_iter()
        .fold(Raw::var(HERE, function), |applied, argument| {
            Raw::app(HERE, applied, argument)
        })
}

/// `Row n`, as a type — a base type applied to a literal.
fn row_at(index: i128) -> Term {
    Term::app(HERE, row().term(HERE), count_lit(index).term(HERE))
}

/// The three a *registration* reaches, which no hand-built term alone can.
///
/// Gathered here rather than beside the cases above because they need a registry
/// rather than a term, and because they are one finding rather than three: a host
/// that writes down two descriptions of itself can make them disagree, and the
/// kernel's word for each disagreement is a [`Malformed`].
///
/// # Panics
///
/// Naming the case, when a program the registry should choke on is accepted.
fn host_faults() -> Vec<(&'static str, Malformed)> {
    let cx = host();
    let mut found = Vec::new();

    // A rule that answers nothing at closed literals of the types it declares it
    // accepts: D2 broken, discovered mid-evaluation with the registration long
    // out of sight.
    let program = calls(
        "count_add",
        [Raw::lit(HERE, count_lit(i128::MAX)), Raw::lit(HERE, count_lit(1))],
    );
    let term = musa_calculus::check(&cx, &count_type(), &program).expect("the application is well typed");
    let fault = malformed(
        "a rule with nothing to say",
        musa_calculus::normalize(&cx, &count_type(), &term),
    );
    assert!(matches!(fault, Malformed::BuiltinStuck(_)), "{fault}");
    found.push(("a rule with nothing to say", fault));

    // A rule that answers data the type it answers at cannot hold.
    let program = calls("count_tally", [Raw::lit(HERE, count_lit(3))]);
    let term = musa_calculus::check(&cx, &count_type(), &program).expect("the application is well typed");
    let fault = malformed(
        "a rule answering at the wrong type",
        musa_calculus::normalize(&cx, &count_type(), &term),
    );
    assert!(matches!(fault, Malformed::MisfitAnswer(_)), "{fault}");
    found.push(("a rule answering at the wrong type", fault));

    // A base type that says one index accepts a value at another, and names
    // nothing that can carry it across.
    let program = calls("row_of", [Raw::lit(HERE, count_lit(1))]);
    let fault = refused(
        "a carrier the registry does not hold",
        musa_calculus::check(&cx, &row_at(2), &program),
    );
    assert!(matches!(fault, Malformed::UnregisteredCarrier(_)), "{fault}");
    found.push(("a carrier the registry does not hold", fault));

    found
}
