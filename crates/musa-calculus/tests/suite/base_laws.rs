//! Base types, literals, and the builtin registry: §5.8's conservative
//! extension, stated as laws over a worked registry.
//!
//! **The registry here is not Musa's.** `docs/rules/language/02-core-calculus.md`
//! §5.8's corollary is that a later musical domain needs no core amendment —
//! only a base type, arrow-free signatures, and a D1–D4 discharge — and a suite
//! that exercised the mechanism through `Pitch` and `Duration` would be evidence
//! for the opposite claim, because it would have taught this crate what those
//! are. So the base types below are `Int` and `Text`, which Musa does not have
//! under those names, and the δ-builtins over them are arithmetic and
//! concatenation. If the mechanism works for these it works for a pitch, and
//! that is the whole point of the boundary.
//!
//! **Which half of D1–D4 is checked here.** Registration checks the half a
//! signature makes visible: one name one meaning, no arrow in a δ signature, and
//! every base type a δ signature mentions registered inert. D2's totality, D3's
//! purity, and D4's bound are properties of the host's own functions over the
//! host's own domains; prompt 127ca's law suite samples them where the table
//! lives. What this file adds on top of registration is the *reduction*
//! behaviour the core owes: a literal is inert, two literals are convertible iff
//! the host says their payloads agree, δ fires exactly where ι does, a builtin
//! short of its arguments is neutral rather than an error, and every δ step is
//! charged.
//!
//! **`Option` is declared here rather than registered.** D1 admits "a base type
//! **or a finite constructor over base types**", and the second half needs a
//! family with real constructors to be about anything — so the context below
//! declares one, in the test, the way prompt 141 proved `Option`, `List`, and
//! `Result` are declarations rather than core knowledge. Nothing in `src/` learns
//! that `Option` exists; what it learns is that a family *has* constructors.

use std::any::Any;
use std::sync::Arc;

use musa_calculus::{
    Answer, Base, Budget, Builtin, CoreError, Cx, Datum, ElabError, Extern, Family, Group, Index, Literal, Origin,
    Payload, Raw, RawArm, RawData, RawPattern, Refusal, Registry, Sort, Term, check, convertible, infer, normalize,
};

use crate::family_laws::{binder, constructor, data, family, type0, var};
use crate::programs::refusal;

/// Where every type in this suite says it was written.
const TYPES: Origin = Origin::node(700);

/// Where every term in this suite says it was written.
const TERMS: Origin = Origin::node(701);

// ---- the host's payloads ---------------------------------------------------

/// An integer, as a host would carry one.
#[derive(Debug)]
struct Int(i64);

impl Payload for Int {
    fn same(&self, other: &dyn Payload) -> bool {
        // The downcast is the host's, not the core's: §5.8 makes the payload
        // opaque precisely so that this line lives here.
        other.as_any().downcast_ref::<Self>().is_some_and(|it| it.0 == self.0)
    }

    fn shown(&self) -> String {
        self.0.to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A string, as a host would carry one.
#[derive(Debug)]
struct Text(String);

impl Payload for Text {
    fn same(&self, other: &dyn Payload) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|it| it.0 == self.0)
    }

    fn shown(&self) -> String {
        format!("{:?}", self.0)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A labelled tree, as a host would carry one.
///
/// This is what §5.8's *structural eliminators* exist for, in miniature. The
/// core cannot see the shape: `Tree` is a base type, so it has no constructors,
/// so it has no recursor and no library traversal could be written over it —
/// which is exactly the position `musa-compiler`'s `Syntax` is in, and the
/// reason `recurse_syntax` cannot become library code in prompt 142.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Node {
    label: String,
    kids: Vec<Self>,
}

impl Payload for Node {
    fn same(&self, other: &dyn Payload) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|it| it == self)
    }

    fn shown(&self) -> String {
        if self.kids.is_empty() {
            return self.label.clone();
        }
        let kids: Vec<String> = self.kids.iter().map(|kid| kid.shown()).collect();
        format!("{}({})", self.label, kids.join(", "))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---- the worked registry ---------------------------------------------------

/// `Int : Type 0`.
fn int() -> Base {
    Base::new("Int", Term::universe(TYPES, Sort::ZERO))
}

/// `Text : Type 0`.
fn text() -> Base {
    Base::new("Text", Term::universe(TYPES, Sort::ZERO))
}

/// `n : Int`.
fn int_lit(value: i64) -> Literal {
    Literal::new(int().term(TYPES), Arc::new(Int(value)))
}

/// `s : Text`.
fn text_lit(value: &str) -> Literal {
    Literal::new(text().term(TYPES), Arc::new(Text(value.to_owned())))
}

/// `A → B`, over closed types, which is all a δ signature may be.
fn arrow(domain: Term, codomain: Term) -> Term {
    Term::pi(TYPES, "_", domain, codomain)
}

/// What a δ-rule reads out of an argument, or `None` when it is not one of ours.
fn as_int(literal: &Literal) -> Option<i64> {
    literal.payload().as_any().downcast_ref::<Int>().map(|it| it.0)
}

fn as_text(literal: &Literal) -> Option<&str> {
    literal
        .payload()
        .as_any()
        .downcast_ref::<Text>()
        .map(|it| it.0.as_str())
}

/// `int_add : Int → Int → Int`.
fn int_add() -> Builtin {
    Builtin::new(
        "int_add",
        arrow(int().term(TYPES), arrow(int().term(TYPES), int().term(TYPES))),
        Family::Delta,
        |arguments| match arguments {
            [Datum::Lit(left), Datum::Lit(right)] => {
                Some(Datum::Lit(int_lit(as_int(left)?.checked_add(as_int(right)?)?)).into())
            }
            _ => None,
        },
    )
}

/// `text_append : Text → Text → Text`.
fn text_append() -> Builtin {
    Builtin::new(
        "text_append",
        arrow(text().term(TYPES), arrow(text().term(TYPES), text().term(TYPES))),
        Family::Delta,
        |arguments| match arguments {
            [Datum::Lit(left), Datum::Lit(right)] => {
                Some(Datum::Lit(text_lit(&format!("{}{}", as_text(left)?, as_text(right)?))).into())
            }
            _ => None,
        },
    )
}

/// `int_div : Int → Int → Int`, the one rule here that refuses.
///
/// Division by zero is the smallest honest instance of `02-core-calculus.md`
/// §4's first outcome. The arguments are closed literals of the declared types,
/// so answering nothing would be D2 broken and a defect in this table; the
/// program is nonetheless the thing that is wrong. The rule says the sentence
/// and the core says where.
fn int_div() -> Builtin {
    Builtin::new(
        "int_div",
        arrow(int().term(TYPES), arrow(int().term(TYPES), int().term(TYPES))),
        Family::Delta,
        |arguments| match arguments {
            [Datum::Lit(left), Datum::Lit(right)] => {
                let divisor = as_int(right)?;
                if divisor == 0 {
                    return Some(Answer::Refused("an integer is not divided by zero".to_owned()));
                }
                Some(Datum::Lit(int_lit(as_int(left)?.checked_div(divisor)?)).into())
            }
            _ => None,
        },
    )
}

/// `int_show : Int → Text`, so that a δ-rule crossing base types is exercised.
fn int_show() -> Builtin {
    Builtin::new(
        "int_show",
        arrow(int().term(TYPES), text().term(TYPES)),
        Family::Delta,
        |arguments| match arguments {
            [Datum::Lit(only)] => Some(Datum::Lit(text_lit(&as_int(only)?.to_string())).into()),
            _ => None,
        },
    )
}

// ---- the declared family, and the δ-rules over it ---------------------------

/// `data Option (A : Type 0) where None : Option A; Some : (value : A) → Option A`.
///
/// Declared rather than registered, and that is the whole point: a δ-rule that
/// answers `Some 3` is answering a *constructor application*, which prompt 141
/// established is what a value of a declared family is. `Literal` cannot be one,
/// which is why [`Datum`] exists.
fn options() -> RawData {
    data(
        vec![binder("A", type0())],
        vec![family(
            "Option",
            vec![
                constructor("None", Vec::new()),
                constructor("Some", vec![binder("value", var("A"))]),
            ],
        )],
    )
}

/// `data List (A : Type 0) { Empty, Cons(first : A, rest : List A) }`.
///
/// Declared for the traversal below and for nothing else. A structural
/// eliminator whose group branch takes a `List A` is the shape `recurse_syntax`
/// has, and the only shape that needs [`Builtin::structural_with`] — so the law
/// about a registered vocabulary needs a family whose constructors a rewrite
/// would have to name and could not.
fn lists() -> RawData {
    data(
        vec![binder("A", type0())],
        vec![family(
            "List",
            vec![
                constructor("Empty", Vec::new()),
                constructor(
                    "Cons",
                    vec![binder("first", var("A")), binder("rest", calls("List", [var("A")]))],
                ),
            ],
        )],
    )
}

/// The qualified spelling a rule writes, and the one a diagnostic prints.
fn case(name: &str, fields: impl IntoIterator<Item = Datum>) -> Datum {
    Datum::Case {
        constructor: Arc::from(name),
        fields: fields.into_iter().collect(),
    }
}

/// `int_halve : Int → Option Int`, which answers a value it has to *build*.
///
/// Both constructors, on purpose: `Some` carries a field and `None` carries
/// none, and neither says what `A` is. The parameter comes from the result type
/// at realization, which is why an answer is written as a name and its fields
/// and nothing else.
fn int_halve(option_int: &Term) -> Builtin {
    Builtin::new(
        "int_halve",
        arrow(int().term(TYPES), option_int.clone()),
        Family::Delta,
        |arguments| match arguments {
            [Datum::Lit(only)] => {
                let value = as_int(only)?;
                if value % 2 == 0 {
                    return Some(case("Option.Some", [Datum::Lit(int_lit(value / 2))]).into());
                }
                Some(case("Option.None", []).into())
            }
            _ => None,
        },
    )
}

/// `option_or : Option Int → Int → Int`, which reads one.
///
/// It also hands an argument back unchanged, which is the case that decided
/// [`Datum`] is one type rather than two: the fallback is *already* a datum, so
/// answering it is a clone rather than a transcription into a second vocabulary.
fn option_or(option_int: &Term) -> Builtin {
    Builtin::new(
        "option_or",
        arrow(option_int.clone(), arrow(int().term(TYPES), int().term(TYPES))),
        Family::Delta,
        |arguments| match arguments {
            [Datum::Case { constructor, fields }, fallback] => {
                if **constructor == *"Option.Some" {
                    return fields.first().cloned().map(Answer::from);
                }
                Some(fallback.clone().into())
            }
            _ => None,
        },
    )
}

/// `option_flatten : Option (Option Int) → Option Int`, nested both ways.
///
/// The reading side has to descend into a constructor's field to see another
/// constructor, and the answering side has to realize the inner one at the
/// *inner* type — `Option Int`, read off the outer type's parameter rather than
/// off anything the rule said. A mechanism that resolved constructor names from
/// a flat table would have nothing to say about which `Option` this is.
fn option_flatten(option_int: &Term, option_option_int: &Term) -> Builtin {
    Builtin::new(
        "option_flatten",
        arrow(option_option_int.clone(), option_int.clone()),
        Family::Delta,
        |arguments| match arguments {
            [Datum::Case { constructor, fields }] => {
                if **constructor == *"Option.Some" {
                    return fields.first().cloned().map(Answer::from);
                }
                Some(case("Option.None", []).into())
            }
            _ => None,
        },
    )
}

/// `Tree : Type 0`.
fn tree() -> Base {
    Base::new("Tree", Term::universe(TYPES, Sort::ZERO))
}

/// `t : Tree`.
fn tree_lit(node: Node) -> Literal {
    Literal::new(tree().term(TYPES), Arc::new(node))
}

fn leaf(label: &str) -> Node {
    Node {
        label: label.to_owned(),
        kids: Vec::new(),
    }
}

fn branch(label: &str, kids: Vec<Node>) -> Node {
    Node {
        label: label.to_owned(),
        kids,
    }
}

/// `f a b …`, as a term.
fn applied(function: Term, arguments: impl IntoIterator<Item = Term>) -> Term {
    arguments
        .into_iter()
        .fold(function, |applied, argument| Term::app(TERMS, applied, argument))
}

/// `tree_fold : (Text → Int) → (Text → Int → Int) → Tree → Int`.
///
/// The worked structural eliminator, and everything about it is what a δ-builtin
/// cannot be: two of its arguments are functions, so no arrangement of
/// [`Rule`](musa_calculus::Rule) could ever see them as literals, and the answer it
/// gives is an *application* of one of them rather than a value it computed
/// itself.
///
/// It fires on argument 2, the tree, and its rewrite names the other two by
/// position — `Index(2)` is the first argument and `Index(0)` is the last,
/// because the arguments are read as an environment and an environment counts
/// inwards. Nothing about them is evaluated: the core already has their values
/// and hands them back where the rewrite put them.
fn tree_fold() -> Builtin {
    Builtin::structural(
        "tree_fold",
        arrow(
            arrow(text().term(TYPES), int().term(TYPES)),
            arrow(
                arrow(text().term(TYPES), arrow(int().term(TYPES), int().term(TYPES))),
                arrow(tree().term(TYPES), int().term(TYPES)),
            ),
        ),
        2,
        |builtin, literal| {
            let node = literal.payload().as_any().downcast_ref::<Node>()?;
            let label = text_lit(&node.label).term(TERMS);
            if node.kids.is_empty() {
                // `leaf label` — the algebra's first branch, applied to what
                // the leaf holds.
                return Some(Term::app(TERMS, Term::var(TERMS, Index(2)), label));
            }
            // `branch label (int_add (fold k₁) (int_add (fold k₂) … 0))`. The
            // recursive calls are terms the core has not looked at yet, and so
            // is the δ-redex that sums them: a rewrite says what to do next and
            // the evaluator does it.
            let total = node.kids.iter().rev().fold(int_lit(0).term(TERMS), |rest, kid| {
                applied(int_add().term(TERMS), [folded(builtin, kid), rest])
            });
            Some(applied(Term::var(TERMS, Index(1)), [label, total]))
        },
    )
}

/// `tree_fold leaf branch kid`, with the algebra named where the rewrite stands.
///
/// The builtin names itself through the handle it was given, because a `fn`
/// pointer cannot capture one and a traversal that could not recurse would not
/// be a traversal.
fn folded(builtin: &Builtin, kid: &Node) -> Term {
    applied(
        builtin.term(TERMS),
        [
            Term::var(TERMS, Index(2)),
            Term::var(TERMS, Index(1)),
            tree_lit(kid.clone()).term(TERMS),
        ],
    )
}

/// `tree_spin : Tree → Int`, a traversal that never descends.
///
/// Registered on purpose. §5.8 puts D2 and D4 at the table because a signature
/// cannot show them, and the same is true of "every recursive call stands at a
/// smaller literal": this rewrite reapplies itself to the tree it was handed, so
/// nothing about it converges. What the core owes is that the judgment *ends* —
/// §4's meter charges the step before the rewrite runs, so this is refused
/// rather than run forever.
fn tree_spin() -> Builtin {
    Builtin::structural(
        "tree_spin",
        arrow(tree().term(TYPES), int().term(TYPES)),
        0,
        |builtin, literal| Some(Term::app(TERMS, builtin.term(TERMS), literal.term(TERMS))),
    )
}

/// `tree_depth : (A : Type 0) → (Text → A) → (Text → List A → A) → Tree → A`.
///
/// The second worked traversal, and the one [`Builtin::structural_with`] exists
/// for. Two things separate it from [`tree_fold`]:
///
/// - **It is parameterized, and the parameter is an argument.** `A` is the
///   outermost binder, so the rewrite names it by [`Index`] exactly as it names
///   a branch — which is what makes `List A` writable at all, and what
///   `recurse_syntax` will need for its own `C` and `A`.
/// - **Its group branch takes a `List A`**, so firing at a node with children
///   means *building a list*. `List.Cons` is a constructor of a declared family;
///   a [`Rewrite`](musa_calculus::Rewrite) is handed a literal and a builtin and can
///   name neither. The vocabulary is where the host puts the two terms it
///   resolved from the context that declared them.
///
/// The signature is the first dependent one in this file, and the indices in it
/// count outwards through the arrows as well as the named binders: inside the
/// branch's second arrow, `A` is three binders up.
fn tree_depth(list: &Term, vocabulary: Vec<Term>) -> Builtin {
    let text_ty = || text().term(TYPES);
    Builtin::structural_with(
        "tree_depth",
        Term::pi(
            TYPES,
            "A",
            Term::universe(TYPES, Sort::ZERO),
            Term::pi(
                TYPES,
                "leaf",
                arrow(text_ty(), Term::var(TYPES, Index(1))),
                Term::pi(
                    TYPES,
                    "branch",
                    arrow(
                        text_ty(),
                        arrow(
                            Term::app(TYPES, list.clone(), Term::var(TYPES, Index(2))),
                            Term::var(TYPES, Index(3)),
                        ),
                    ),
                    Term::pi(TYPES, "subject", tree().term(TYPES), Term::var(TYPES, Index(3))),
                ),
            ),
        ),
        Family::Eliminator,
        3,
        vocabulary,
        |builtin, literal| {
            let node = literal.payload().as_any().downcast_ref::<Node>()?;
            let label = text_lit(&node.label).term(TERMS);
            if node.kids.is_empty() {
                return Some(Term::app(TERMS, Term::var(TERMS, Index(2)), label));
            }
            // `A`, named where the arguments are: the list this builds is a
            // `List A` and not a list of whatever the first child happened to
            // answer, which is the difference an empty group would show.
            let member = Term::var(TERMS, Index(3));
            let (empty, cons) = (builtin.vocabulary().first()?, builtin.vocabulary().get(1)?);
            let kids = node
                .kids
                .iter()
                .rev()
                .fold(Term::app(TERMS, empty.clone(), member.clone()), |rest, kid| {
                    applied(cons.clone(), [member.clone(), descended(builtin, kid), rest])
                });
            Some(applied(Term::var(TERMS, Index(1)), [label, kids]))
        },
    )
}

/// `tree_depth A leaf branch kid`, with the parameter and the algebra named
/// where the rewrite stands.
fn descended(builtin: &Builtin, kid: &Node) -> Term {
    applied(
        builtin.term(TERMS),
        [
            Term::var(TERMS, Index(3)),
            Term::var(TERMS, Index(2)),
            Term::var(TERMS, Index(1)),
            tree_lit(kid.clone()).term(TERMS),
        ],
    )
}

/// The registry every accepting law below is stated under, over the `Option`
/// terms the declaring context supplied.
///
/// # Panics
///
/// If the registry refuses its own worked example, which would be a defect in
/// this crate rather than a property of any test.
fn registry(declared: &Declared) -> Arc<Registry> {
    Arc::new(
        Registry::new(
            vec![int(), text(), tree()],
            vec![
                int_add(),
                int_div(),
                text_append(),
                int_show(),
                tree_fold(),
                tree_spin(),
                tree_depth(&declared.list, declared.vocabulary.clone()),
                int_halve(&declared.option_int),
                option_or(&declared.option_int),
                option_flatten(&declared.option_int, &declared.option_option_int),
            ],
        )
        .expect("the worked registry registers"),
    )
}

/// What a declaring context supplied, before the registry that mentions it.
///
/// One struct rather than five arguments because they are one fact: these are
/// the terms only a context that has already declared `Option` and `List` can
/// produce, and every one of them is read out at the same moment for the same
/// reason.
struct Declared {
    option_int: Term,
    option_option_int: Term,
    list: Term,
    /// `List.Empty` and `List.Cons`, in the order [`tree_depth`]'s rewrite reads
    /// them.
    vocabulary: Vec<Term>,
}

/// `Option` declared, then the registry over it, at `budget`.
///
/// The order is forced and is worth naming: a δ signature mentioning
/// `Option Int` needs the family *constant*, which only a context that has
/// already declared it can supply. So the family is declared first, its constant
/// is read out, the two `Option` types are assembled from it and the registered
/// `Int`, and the registry is built last. A host doing this for real does the
/// same thing in the same order.
///
/// The preparation runs at the language budget whatever `budget` is, because it
/// is the fixture rather than the law: the narrow-budget tests below are about
/// what a *program* costs, and a declaration nobody could afford to make would
/// say nothing about that.
///
/// # Panics
///
/// If the declaration is refused, which would be a defect in this crate.
fn host_at(budget: Budget) -> Cx {
    let (groups, declared) = declarations();
    groups
        .iter()
        .fold(Cx::with_budget(budget), |cx, group| cx.declaring(group))
        .with_externs(registry(&declared))
}

/// The declaration groups the fixture rests on, and the terms only a context
/// already carrying them can supply.
///
/// Separate from [`host_at`] because two things want it: building the host, and
/// stating what the registry was built *from*. A law about the vocabulary has to
/// reach the registered builtin, and a context does not hand its externs back
/// out — rightly, since nothing but a test would ask.
///
/// The declaring pass runs at the language budget whatever the caller's is, for
/// the reason [`host_at`] gives.
///
/// # Panics
///
/// If a declaration is refused, which would be a defect in this crate.
fn declarations() -> (Vec<Arc<Group>>, Declared) {
    let roomy = Cx::with_budget(Budget::LANGUAGE);
    let options = musa_calculus::declare(&roomy, &options()).expect("`Option` is a declaration");
    let declaring = roomy.declaring(&options);
    let lists = musa_calculus::declare(&declaring, &lists()).expect("`List` is a declaration");
    let declaring = declaring.declaring(&lists);
    let option = named(&declaring, "Option");
    let option_int = Term::app(TYPES, option.clone(), int().term(TYPES));
    let declared = Declared {
        option_option_int: Term::app(TYPES, option, option_int.clone()),
        option_int,
        list: named(&declaring, "List"),
        vocabulary: vec![named(&declaring, "List.Empty"), named(&declaring, "List.Cons")],
    };
    (vec![options, lists], declared)
}

/// The constant `name` denotes, read out of the context that declared it.
///
/// This is the one path a vocabulary term may come from, and the reason is the
/// invariant [`Builtin::structural_with`] states: the term has to be closed, and
/// a declared constant is closed by construction.
///
/// # Panics
///
/// If `name` is not declared, which would be a defect in this file.
fn named(cx: &Cx, name: &str) -> Term {
    infer(cx, &Raw::var(TYPES, name)).expect("the name is declared here").0
}

/// A context carrying it, at the language budget.
fn host() -> Cx {
    host_at(Budget::LANGUAGE)
}

/// `Option Int` as a type, read where both halves of it are in scope.
///
/// # Panics
///
/// If it is not a type, which would be a defect in this file.
fn option_int(cx: &Cx) -> Term {
    infer(cx, &calls("Option", [Raw::var(TYPES, "Int")]))
        .expect("`Option Int` is a type")
        .0
}

/// `Option.Some Int n` and `Option.None Int`, as raw terms.
///
/// The type parameter is written because a constructor takes it: `Option.Some`
/// has arity two, and that is exactly the fact realization has to supply from
/// the result type when a δ-rule answers one.
fn some(value: Raw) -> Raw {
    calls("Option.Some", [Raw::var(TYPES, "Int"), value])
}

fn none() -> Raw {
    calls("Option.None", [Raw::var(TYPES, "Int")])
}

/// `f a b …`, as a raw term.
fn calls(function: &str, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    arguments
        .into_iter()
        .fold(Raw::var(TERMS, function), |applied, argument| {
            Raw::app(TERMS, applied, argument)
        })
}

// ---- what the registry names -----------------------------------------------

/// §5.8: a registered base type is a name a program may write, and it stands in
/// type position like any other.
#[test]
fn a_registered_base_type_is_a_type() {
    let cx = host();
    let (_term, ty) = infer(&cx, &Raw::var(TERMS, "Int")).expect("`Int` resolves");
    assert_eq!(ty, Term::universe(TYPES, Sort::ZERO), "`Int : Type 0`");
}

/// A context with no registry names none of it, which is what leaves every other
/// suite in this crate unchanged.
#[test]
fn a_context_without_a_registry_names_no_base_type() {
    let error = infer(&Cx::new(), &Raw::var(TERMS, "Int")).expect_err("`Int` is nobody's name here");
    assert!(
        matches!(refusal("no registry", error), Refusal::UnknownName { .. }),
        "an unregistered base type is an unknown name, not a special case"
    );
}

/// A declaration shadows the registry rather than the other way round.
///
/// The registry is consulted last on purpose: a host that could silently
/// redefine a name in a program it never read would make what a program means
/// depend on which compiler ran it.
#[test]
fn a_binder_shadows_a_registered_name() {
    let cx = host();
    let program = Raw::annotated_lam(TERMS, "Int", Raw::record_type(TERMS, []), Raw::var(TERMS, "Int"));
    let (_, ty) = infer(&cx, &program).expect("the binder resolves");
    assert_eq!(
        ty,
        Term::pi(TYPES, "Int", Term::record_type(TERMS, []), Term::record_type(TERMS, [])),
        "the binder won, so the answer is `{{}} → {{}}` rather than anything about `Int`"
    );
}

/// A literal infers the base type it was built at.
#[test]
fn a_literal_infers_its_base_type() {
    let cx = host();
    let (_term, ty) = infer(&cx, &Raw::lit(TERMS, int_lit(3))).expect("`3` infers");
    assert_eq!(ty, int().term(TYPES), "`3 : Int`");
}

// ---- conversion ------------------------------------------------------------

/// §5.8: "two closed values of it are convertible iff they are the same
/// constant."
#[test]
fn two_literals_are_convertible_exactly_when_the_host_says_so() {
    let cx = host();
    let questions = [
        ("the same integer", int_lit(3), int_lit(3), true),
        ("two integers", int_lit(3), int_lit(4), false),
        ("the same text", text_lit("c"), text_lit("c"), true),
        ("two texts", text_lit("c"), text_lit("d"), false),
    ];
    for (name, left, right, equal) in questions {
        let ty = left.ty().clone();
        assert_eq!(
            convertible(&cx, &ty, &left.term(TERMS), &right.term(TERMS)),
            Ok(equal),
            "{name}"
        );
    }
}

/// Two literals of *different* base types are not convertible, and the question
/// is settled before either payload is asked.
///
/// It has to be: [`Payload::same`] is documented as being called only on
/// payloads of one base type, so a core that asked first would be relying on
/// every host to defend itself against a comparison the core promised not to
/// make.
#[test]
fn literals_of_different_base_types_are_not_convertible() {
    let cx = host();
    assert_eq!(
        convertible(
            &cx,
            &int().term(TYPES),
            &int_lit(3).term(TERMS),
            &text_lit("3").term(TERMS)
        ),
        Ok(false),
        "an `Int` and a `Text` are not the same value however they print"
    );
}

// ---- inertness -------------------------------------------------------------

/// D1: no reduction rule inspects a closed value of a base type.
///
/// Stated as: a literal is already its own normal form, and so is a base type.
/// There is nothing else it *could* be — the point of inertness is that no rule
/// applies — so the law is that normalization is the identity on both.
#[test]
fn a_literal_and_a_base_type_are_their_own_normal_forms() {
    let cx = host();
    let base = int().term(TYPES);
    assert_eq!(
        normalize_at(&cx, &Term::universe(TYPES, Sort::ZERO), &base),
        base,
        "a base type normalizes to itself"
    );
    let literal = int_lit(3).term(TERMS);
    assert_eq!(normalize_at(&cx, &base, &literal), literal, "and so does a literal");
}

/// D1's other half: the only pattern that may stand at a base-typed column is a
/// catch-all, so a pattern that takes one apart is refused by name.
#[test]
fn a_destructuring_pattern_at_a_base_type_is_refused() {
    for RefusedProgram {
        name,
        raw,
        ty,
        expected,
    } in refused_programs()
    {
        let cx = host();
        let (ty, _) = infer(&cx, &ty).unwrap_or_else(|error| panic!("{name}: {error}"));
        let Err(error) = check(&cx, &ty, &raw) else {
            panic!("{name}: elaboration accepted a program it must refuse");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
    }
}

/// A catch-all over a base-typed subject is accepted, and needs no coverage
/// rule to be complete.
///
/// The companion to the law above, and the reason coverage says nothing here: a
/// column of catch-alls is never tested, so there is no set of cases to be
/// exhaustive over and no infinite constructor list to enumerate in a
/// diagnostic.
#[test]
fn a_catch_all_over_a_base_typed_subject_is_complete() {
    let cx = host();
    let program = Raw::match_on(
        TERMS,
        [Raw::lit(TERMS, int_lit(3))],
        vec![RawArm {
            patterns: vec![RawPattern::bind(TERMS, "n")],
            body: Raw::var(TERMS, "n"),
        }],
    );
    let term = check(&cx, &int().term(TYPES), &program).expect("a catch-all covers a base type");
    assert_eq!(
        normalize_at(&cx, &int().term(TYPES), &term),
        int_lit(3).term(TERMS),
        "and it answers the subject it bound"
    );
}

// ---- δ ---------------------------------------------------------------------

/// D2 and D3, over a finite sample: a δ-builtin applied to literals reduces to
/// exactly what the host's own function answers.
///
/// The sample is finite and the law is not, which is the honest position: §5.8
/// makes agreement a condition on *registration*, so what a suite can check is
/// that the core runs the registered rule rather than some rule of its own.
#[test]
fn a_builtin_at_literals_answers_what_the_host_function_answers() {
    let cx = host();
    let questions: Vec<(&str, Raw, Term, Term)> = vec![
        (
            "int_add 2 3",
            calls("int_add", [Raw::lit(TERMS, int_lit(2)), Raw::lit(TERMS, int_lit(3))]),
            int().term(TYPES),
            int_lit(5).term(TERMS),
        ),
        (
            "int_add 0 0",
            calls("int_add", [Raw::lit(TERMS, int_lit(0)), Raw::lit(TERMS, int_lit(0))]),
            int().term(TYPES),
            int_lit(0).term(TERMS),
        ),
        (
            "text_append",
            calls(
                "text_append",
                [Raw::lit(TERMS, text_lit("mez")), Raw::lit(TERMS, text_lit("zo"))],
            ),
            text().term(TYPES),
            text_lit("mezzo").term(TERMS),
        ),
        (
            "int_show, which crosses base types",
            calls("int_show", [Raw::lit(TERMS, int_lit(12))]),
            text().term(TYPES),
            text_lit("12").term(TERMS),
        ),
        (
            // Nested, so that a δ-rule's argument is itself a δ-redex: the
            // inner one has to have fired before the outer one can see a
            // literal, which is what "δ fires where ι fires" buys.
            "int_show (int_add 5 7)",
            calls(
                "int_show",
                [calls(
                    "int_add",
                    [Raw::lit(TERMS, int_lit(5)), Raw::lit(TERMS, int_lit(7))],
                )],
            ),
            text().term(TYPES),
            text_lit("12").term(TERMS),
        ),
    ];
    for (name, raw, ty, expected) in questions {
        let term = check(&cx, &ty, &raw).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(normalize_at(&cx, &ty, &term), expected, "{name}");
    }
}

/// A builtin that has not been given literals is *stuck*, not wrong.
///
/// Two ways to be stuck, and both must answer a neutral rather than an error:
/// too few arguments, and an argument that is a variable. A core that refused
/// either would make a perfectly good open term — the body of any function over
/// a base type — unwritable.
#[test]
fn a_builtin_short_of_literals_is_neutral() {
    let cx = host();
    let int_ty = int().term(TYPES);
    let partial = calls("int_add", [Raw::lit(TERMS, int_lit(2))]);
    let _term = check(&cx, &arrow(int_ty.clone(), int_ty.clone()), &partial).expect("a partial application checks");

    // λn. int_add n 1 — saturated, and stuck on the binder.
    let open = Raw::annotated_lam(
        TERMS,
        "n",
        Raw::var(TERMS, "Int"),
        calls("int_add", [Raw::var(TERMS, "n"), Raw::lit(TERMS, int_lit(1))]),
    );
    let term = check(&cx, &arrow(int_ty.clone(), int_ty.clone()), &open).expect("an open body checks");
    let normal = normalize_at(&cx, &arrow(int_ty.clone(), int_ty), &term);
    assert_eq!(
        normal, term,
        "stuck on a variable, so the normal form is the term itself"
    );
}

/// D4: a δ step is charged before it is taken, so a budget that cannot afford
/// the reduction ends the judgment rather than performing it.
///
/// Stated against a term whose *only* work is δ: the same term is accepted at
/// the language budget and exhausts at a budget narrow enough, which is the only
/// way to see a charge that would otherwise be invisible.
#[test]
fn a_builtin_reduction_is_charged() {
    let cx = host();
    let int_ty = int().term(TYPES);
    let chain = (0..64).fold(Raw::lit(TERMS, int_lit(0)), |sum, _| {
        calls("int_add", [sum, Raw::lit(TERMS, int_lit(1))])
    });
    let term = check(&cx, &int_ty, &chain).expect("the chain checks at the language budget");
    assert_eq!(
        normalize_at(&cx, &int_ty, &term),
        int_lit(64).term(TERMS),
        "and it reduces to 64"
    );

    let narrow = host_at(Budget::LANGUAGE.scaled(200_000));
    let outcome = check(&narrow, &int_ty, &chain);
    assert!(
        matches!(outcome, Err(ElabError::Exhausted(_))),
        "a budget of one step cannot afford 64 δ reductions, and says so rather than answering"
    );
}

/// §4's first outcome, at the one place that used to collapse it into the third:
/// a rule that refuses is refusing the *program*, at the application the author
/// wrote.
///
/// Both halves matter. `ElabError::Refused` rather than `Malformed` is what
/// makes the sentence reach a composer instead of a bug report, and the origin
/// is what makes it land on a span — a rule sees data and never a term, so it
/// has no place to name and the core has to supply one.
#[test]
fn a_rule_that_refuses_refuses_the_program() {
    let cx = host();
    let int_ty = int().term(TYPES);
    let program = calls("int_div", [Raw::lit(TERMS, int_lit(6)), Raw::lit(TERMS, int_lit(0))]);
    // Well typed, and accepted as such: `int_div 6 0 : Int` is a true judgment
    // and checking never asks what it means. The rule looks at the arguments
    // when it *fires*, which is reduction.
    let term = check(&cx, &int_ty, &program).expect("dividing by zero is a well-typed application");
    let error = normalize(&cx, &int_ty, &term).expect_err("and reducing it is refused");
    let Refusal::BuiltinRefused { message, at } = refusal("int_div at zero", error.into()) else {
        panic!("a rule's refusal is a refusal");
    };
    assert_eq!(
        &*message, "an integer is not divided by zero",
        "the rule's own sentence"
    );
    assert_eq!(at, TERMS, "at the application, which is what the author wrote");
}

/// And the neighbour it is deliberately not: a rule that answers *nothing* at
/// closed data of its declared types is D2 broken, which no source edit can fix
/// and which therefore stays a defect in this compiler.
///
/// `int_add` overflowing is that case exactly — `checked_add` answers `None`, so
/// the table promises a result the rule cannot produce.
#[test]
fn a_rule_that_answers_nothing_is_still_a_compiler_defect() {
    let cx = host();
    let int_ty = int().term(TYPES);
    let program = calls(
        "int_add",
        [Raw::lit(TERMS, int_lit(i64::MAX)), Raw::lit(TERMS, int_lit(1))],
    );
    let term = check(&cx, &int_ty, &program).expect("the application is well typed");
    assert!(
        matches!(
            normalize(&cx, &int_ty, &term),
            Err(CoreError::Malformed(musa_calculus::Malformed::BuiltinStuck(_)))
        ),
        "a rule with nothing to say at arguments it declares it accepts is the table's mistake"
    );
}

// ---- δ over finite data ----------------------------------------------------
//
// D1's *or*. Everything above this line is a rule over base types alone, and
// every one of those laws still holds unchanged — which is the first thing this
// section claims: the firing condition widened from "every argument a literal"
// to "every argument canonical data", and the two coincide exactly where no
// declared family is involved.

/// A δ-rule reads a constructor application, its fields and all.
///
/// The subject here is not a literal and could never be one: `Option.Some Int 9`
/// is a constant applied to a type and a value, which is what prompt 141 proved
/// a value of a declared family is. A rule handed only literals would have been
/// blocked on every call.
///
/// Both cases, because the empty one is where a fields-only representation would
/// have quietly worked and told us nothing: `None` carries no field and still has
/// to be distinguishable from `Some`.
#[test]
fn a_builtin_reads_a_constructed_argument() {
    let cx = host();
    let int_ty = int().term(TYPES);
    let questions = [
        ("Some 9, so the field", some(Raw::lit(TERMS, int_lit(9))), 9),
        ("None, so the fallback", none(), 0),
    ];
    for (name, subject, expected) in questions {
        let program = calls("option_or", [subject, Raw::lit(TERMS, int_lit(0))]);
        let term = check(&cx, &int_ty, &program).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(
            normalize_at(&cx, &int_ty, &term),
            int_lit(expected).term(TERMS),
            "{name}"
        );
    }
}

/// A δ-rule answers a constructor application, and what it answers is a real
/// value of the family.
///
/// The re-check on the *normal form* is the load-bearing assertion rather than
/// the comparison above it. A rule writes `Option.Some` and one field; a value of
/// `Option Int` is `Option.Some Int 3`, with the parameter first because ι reads
/// it by position. Realization supplies that parameter from the builtin's own
/// result type, and a value missing it would compare equal to nothing and
/// re-check as nothing.
#[test]
fn a_builtin_answers_a_constructed_value() {
    let cx = host();
    let ty = option_int(&cx);
    let questions = [
        ("an even input", 6, some(Raw::lit(TERMS, int_lit(3)))),
        ("an odd one, so the empty case", 7, none()),
    ];
    for (name, input, written) in questions {
        let program = calls("int_halve", [Raw::lit(TERMS, int_lit(input))]);
        let term = check(&cx, &ty, &program).unwrap_or_else(|error| panic!("{name}: {error}"));
        let normal = normalize_at(&cx, &ty, &term);
        let expected = check(&cx, &ty, &written).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(normal, normalize_at(&cx, &ty, &expected), "{name}");
    }
}

/// Data nests, in both directions and at the right type.
///
/// Reading has to descend through a constructor's field to find another
/// constructor; answering has to realize the inner one at `Option Int` — a type
/// read off the outer type's own parameter, not off anything the rule said. A
/// mechanism that resolved constructor names against one flat table per builtin
/// would have had nothing to say about *which* `Option` this is.
#[test]
fn constructed_data_nests_in_both_directions() {
    let cx = host();
    let ty = option_int(&cx);
    let nested = |inner: Raw| calls("Option.Some", [calls("Option", [Raw::var(TYPES, "Int")]), inner]);
    let questions = [
        (
            "Some (Some 4) flattens to Some 4",
            nested(some(Raw::lit(TERMS, int_lit(4)))),
            some(Raw::lit(TERMS, int_lit(4))),
        ),
        ("Some None flattens to None", nested(none()), none()),
    ];
    for (name, subject, written) in questions {
        let program = calls("option_flatten", [subject]);
        let term = check(&cx, &ty, &program).unwrap_or_else(|error| panic!("{name}: {error}"));
        let normal = normalize_at(&cx, &ty, &term);
        let expected = check(&cx, &ty, &written).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(normal, normalize_at(&cx, &ty, &expected), "{name}");
    }
}

/// A constructor whose field is not data leaves the spine blocked.
///
/// The companion that makes the laws above non-vacuous. `Option.Some Int n` is a
/// perfectly good constructor application, and it is not canonical data, because
/// `n` is a variable — so the rule waits, exactly as it waits for a literal. A
/// core that took the outer constructor as enough would hand the rule a `Datum`
/// it had invented a field for, and the body of every function over an `Option`
/// would reduce to a lie.
#[test]
fn a_constructor_over_an_open_field_leaves_the_spine_blocked() {
    let cx = host();
    let int_ty = int().term(TYPES);
    let ty = arrow(int_ty.clone(), int_ty);
    let program = Raw::annotated_lam(
        TERMS,
        "n",
        Raw::var(TERMS, "Int"),
        calls("option_or", [some(Raw::var(TERMS, "n")), Raw::lit(TERMS, int_lit(0))]),
    );
    let term = check(&cx, &ty, &program).expect("an open field checks");
    assert_eq!(
        normalize_at(&cx, &ty, &term),
        term,
        "stuck on the binder inside the constructor, so the normal form is the term itself"
    );
}

// ---- reading data back out of a term ---------------------------------------
//
// Everything above this line reads data on the way *in*, when δ fires. These
// read it on the way out, through [`musa_calculus::canonical`], which is the same
// D1 question asked of a normal form. The corpus is the same `Option` and the
// same `Int`, deliberately: if the two readings agreed only on terms written
// for them separately they would not be one answer to one question.

/// A base literal is data, and is its own.
#[test]
fn a_literal_reads_back_as_itself() {
    assert_eq!(
        musa_calculus::canonical(&int_lit(3).term(TERMS)),
        Some(Datum::Lit(int_lit(3)))
    );
    assert_eq!(
        musa_calculus::canonical(&text_lit("c").term(TERMS)),
        Some(Datum::Lit(text_lit("c")))
    );
}

/// A saturated constructor reads back, with its parameters left out.
///
/// The parameter is the whole assertion. A value of `Option Int` is
/// `Option.Some Int 9` — the type first, because ι reads it by position — and a
/// reading that handed back two fields would be describing the term rather than
/// the data. `None` is beside it because it is where a fields-only
/// representation would have quietly worked: it carries nothing and still has to
/// be told apart from `Some`.
#[test]
fn a_saturated_constructor_reads_back_without_its_parameters() {
    let cx = host();
    let ty = option_int(&cx);
    let questions = [
        (
            "Some 9",
            some(Raw::lit(TERMS, int_lit(9))),
            case("Option.Some", [Datum::Lit(int_lit(9))]),
        ),
        ("None", none(), case("Option.None", [])),
    ];
    for (name, written, expected) in questions {
        let term = check(&cx, &ty, &written).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(
            musa_calculus::canonical(&normalize_at(&cx, &ty, &term)),
            Some(expected),
            "{name}"
        );
    }
}

/// Data nests, and the reading descends through a field rather than stopping at
/// the outer constructor.
#[test]
fn nested_data_reads_back_nested() {
    let cx = host();
    let inner = calls("Option", [Raw::var(TYPES, "Int")]);
    let ty = infer(&cx, &calls("Option", [inner.clone()]))
        .expect("`Option (Option Int)` is a type")
        .0;
    let written = calls("Option.Some", [inner, some(Raw::lit(TERMS, int_lit(4)))]);
    let term = check(&cx, &ty, &written).expect("the nested value checks");
    assert_eq!(
        musa_calculus::canonical(&normalize_at(&cx, &ty, &term)),
        Some(case("Option.Some", [case("Option.Some", [Datum::Lit(int_lit(4))])]))
    );
}

/// The reading a term gets is the reading a δ-rule gets.
///
/// The law that makes the two halves one answer. `option_or` is handed a
/// [`Datum`] by the evaluator when it fires, and it answers the field it found;
/// [`musa_calculus::canonical`] is handed the same subject as a term. If the field
/// the rule acted on and the field the reading reports could differ, a host
/// would have to know which door it came through — and §5.8's whole point is
/// that it does not.
#[test]
fn a_term_and_the_rule_that_fires_on_it_see_the_same_data() {
    let cx = host();
    let int_ty = int().term(TYPES);
    let ty = option_int(&cx);
    let subject = some(Raw::lit(TERMS, int_lit(9)));

    let held = check(&cx, &ty, &subject).expect("the subject checks");
    let read = musa_calculus::canonical(&normalize_at(&cx, &ty, &held)).expect("the term reads back as data");

    let program = calls("option_or", [subject, Raw::lit(TERMS, int_lit(0))]);
    let fired = check(&cx, &int_ty, &program).expect("the application checks");
    let answered = normalize_at(&cx, &int_ty, &fired);

    let Datum::Case { ref fields, .. } = read else {
        panic!("the reading found a constructor: {read:?}");
    };
    assert_eq!(
        fields.first(),
        Some(&Datum::Lit(int_lit(9))),
        "the reading found the field"
    );
    assert_eq!(answered, int_lit(9).term(TERMS), "and the rule answered the same one");
}

/// What is not data, which is most terms.
///
/// One law rather than eight, because the answer is one word and listing them
/// apart would say eight times that `None` means `None`. What matters is that
/// each of these is a *well-typed* term the core will happily hand a caller —
/// a partial constructor, a λ, a record, a universe — so the reading refuses
/// them on their shape rather than on their having been rejected earlier.
#[test]
fn what_is_not_canonical_data_reads_back_as_nothing() {
    let cx = host();
    let int_ty = int().term(TYPES);
    let questions = [
        (
            "a constructor one field short",
            calls("Option.Some", [Raw::var(TYPES, "Int")]),
            arrow(int_ty.clone(), option_int(&cx)),
        ),
        (
            "the family itself",
            calls("Option", [Raw::var(TYPES, "Int")]),
            Term::universe(TYPES, Sort::ZERO),
        ),
        (
            "a λ",
            Raw::annotated_lam(TERMS, "n", Raw::var(TERMS, "Int"), Raw::var(TERMS, "n")),
            arrow(int_ty.clone(), int_ty),
        ),
        (
            "a record",
            Raw::record(TERMS, [("held", Raw::lit(TERMS, int_lit(1)))]),
            infer(&cx, &Raw::record_type(TYPES, [("held", Raw::var(TYPES, "Int"))]))
                .expect("the record type is a type")
                .0,
        ),
        (
            "a universe",
            Raw::universe(TYPES, Sort::ZERO),
            Term::universe(TYPES, Sort::One),
        ),
    ];
    for (name, written, ty) in questions {
        let term = check(&cx, &ty, &written).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(
            musa_calculus::canonical(&normalize_at(&cx, &ty, &term)),
            None,
            "{name} is not data"
        );
    }
}

// ---- structural eliminators ------------------------------------------------

/// `fn (t: Text) { 1 }` — one node counted.
fn counting_leaf() -> Raw {
    Raw::annotated_lam(TERMS, "t", Raw::var(TERMS, "Text"), Raw::lit(TERMS, int_lit(1)))
}

/// `fn (t: Text) { fn (n: Int) { int_add(n, 1) } }` — the children, plus this
/// one.
fn counting_branch() -> Raw {
    Raw::annotated_lam(
        TERMS,
        "t",
        Raw::var(TERMS, "Text"),
        Raw::annotated_lam(
            TERMS,
            "n",
            Raw::var(TERMS, "Int"),
            calls("int_add", [Raw::var(TERMS, "n"), Raw::lit(TERMS, int_lit(1))]),
        ),
    )
}

/// §5.8's second family: a traversal fires on the argument its registration
/// named, and the rewrite it answers is *evaluated* rather than taken as an
/// answer.
///
/// Three things at once, and each of them is out of δ's reach. Two of the
/// arguments are functions, so no δ condition could ever be met and the
/// traversal would never fire. The rewrite's answer is an application of one of
/// those functions — the host wrote down `branch label …` and does not know what
/// it computes to. And the answer holds recursive calls at the children, so the
/// traversal descends because the core kept evaluating what it was handed, not
/// because the host walked the tree itself.
#[test]
fn a_structural_eliminator_walks_the_literal_it_is_given() {
    let cx = host();
    let int_ty = int().term(TYPES);
    let subject = branch("a", vec![leaf("b"), branch("c", vec![leaf("d")])]);
    let program = calls(
        "tree_fold",
        [counting_leaf(), counting_branch(), Raw::lit(TERMS, tree_lit(subject))],
    );
    let term = check(&cx, &int_ty, &program).expect("the traversal checks");
    assert_eq!(
        normalize_at(&cx, &int_ty, &term),
        int_lit(4).term(TERMS),
        "four nodes, so every one of them was reached"
    );
}

/// A function argument is passed through as whatever it already is, and is never
/// forced.
///
/// Stated where it can be seen: the algebra here is a *variable*, so there is
/// nothing to force even in principle, and the traversal still fires and still
/// answers `f "b"`. A mechanism that had insisted on values for its whole spine
/// — δ's condition — would have left this blocked, and a mechanism that forced
/// its arguments would be asking for a strictness this calculus does not have.
#[test]
fn a_function_argument_survives_the_rewrite_unevaluated() {
    let cx = host();
    let algebra = arrow(text().term(TYPES), int().term(TYPES));
    let ty = arrow(algebra, int().term(TYPES));
    // λ(f : Text → Int). tree_fold f (fn (t) { fn (n) { … } }) (leaf "b")
    let program = Raw::annotated_lam(
        TERMS,
        "f",
        Raw::pi(TERMS, "_", Raw::var(TERMS, "Text"), Raw::var(TERMS, "Int")),
        calls(
            "tree_fold",
            [
                Raw::var(TERMS, "f"),
                counting_branch(),
                Raw::lit(TERMS, tree_lit(leaf("b"))),
            ],
        ),
    );
    let term = check(&cx, &ty, &program).expect("an open algebra checks");
    assert_eq!(
        normalize_at(&cx, &ty, &term),
        Term::lam(
            TERMS,
            "f",
            Term::app(TERMS, Term::var(TERMS, Index(0)), text_lit("b").term(TERMS)),
        ),
        "the traversal fired and handed the leaf to the algebra it was given"
    );
}

/// A rewrite builds a list from the vocabulary its registration gave it, at the
/// type parameter it names by index.
///
/// This is what [`Builtin::structural_with`] is for, seen end to end. The host
/// wrote `List.Cons A ⟨child⟩ ⟨rest⟩` for a family the core declared and the
/// rewrite could not otherwise have named; the core checked that list against
/// the branch's declared `List A`, evaluated the recursive calls inside it, and
/// handed the finished list to a branch that takes it apart with an ordinary
/// `match`. A vocabulary that had been resolved wrongly, or a member type read
/// off the first child instead of off the parameter, would fail at the
/// re-checker rather than answer a different number.
///
/// The tree is a left spine three deep with a second child at the root, so the
/// answer distinguishes "the leftmost child" from "the last child" and from "how
/// many children there are".
#[test]
fn a_rewrite_builds_a_list_from_its_registered_vocabulary() {
    let cx = host();
    let int_ty = int().term(TYPES);
    let subject = branch("a", vec![branch("b", vec![leaf("c")]), leaf("d")]);
    let term = check(&cx, &int_ty, &depth_program(subject)).expect("the traversal checks");
    assert_eq!(
        normalize_at(&cx, &int_ty, &term),
        int_lit(2).term(TERMS),
        "two groups above the leftmost leaf, each one `List.Cons` the rewrite wrote"
    );
}

/// Every vocabulary term is closed, so it means the same thing wherever the
/// rewrite splices it.
///
/// The one invariant [`Builtin::structural_with`] leaves to the host, stated
/// where it can be checked. A rewrite's answer is read in the environment of the
/// *spine's arguments*, so a vocabulary entry holding a free index would denote
/// an argument of whatever traversal happened to splice it — the same defect as
/// a payload carrying a de Bruijn index, one level up. Re-checking each entry in
/// a context with no assumptions is what rules it out: a free variable has
/// nothing to be.
#[test]
fn a_vocabulary_term_is_closed_and_so_means_the_same_everywhere() {
    let (_, declared) = declarations();
    let built = registry(&declared);
    let Some(Extern::Builtin(traversal)) = built.named("tree_depth") else {
        panic!("`tree_depth` is a registered builtin")
    };
    assert_eq!(
        traversal.vocabulary().len(),
        2,
        "`List.Empty` and `List.Cons`, in that order"
    );
    let cx = host();
    for (entry, name) in traversal.vocabulary().iter().zip(["List.Empty", "List.Cons"]) {
        let (resolved, _ty) = infer(&cx, &Raw::var(TYPES, name)).expect("the constructor is declared");
        assert_eq!(*entry, resolved, "the vocabulary holds the constant `{name}` denotes");
    }
}

/// Two hosts that register the same vocabulary reduce the same program to the
/// same normal form.
///
/// D3 restated for the new parameter. A vocabulary is data the host supplies, so
/// the question it raises is the one a captured closure would have raised: can
/// two compilers disagree? They cannot, and the reason is that the terms are
/// *values* fixed at registration rather than a lookup performed at reduction —
/// so two contexts built independently, each resolving `List.Empty` and
/// `List.Cons` from its own declaration, put the traversal in the same place.
#[test]
fn two_registries_with_the_same_vocabulary_reduce_alike() {
    let subject = branch("a", vec![branch("b", vec![leaf("c")]), leaf("d")]);
    let int_ty = int().term(TYPES);
    let (one, other) = (host(), host());
    let here = check(&one, &int_ty, &depth_program(subject.clone())).expect("the traversal checks");
    let there = check(&other, &int_ty, &depth_program(subject)).expect("the traversal checks");
    assert_eq!(
        normalize_at(&one, &int_ty, &here),
        normalize_at(&other, &int_ty, &there),
        "the reduction is a function of the vocabulary and not of which host registered it"
    );
}

/// `tree_depth Int (fn (t) { 0 }) (fn (t) { fn (kids) { … } }) ⟨subject⟩`.
///
/// The branch takes the list apart with an ordinary `match`, which is the point:
/// what the rewrite built is a value of a *declared* family, so the ordinary
/// eliminator consumes it and nothing about the traversal is special downstream.
fn depth_program(subject: Node) -> Raw {
    calls(
        "tree_depth",
        [
            Raw::var(TYPES, "Int"),
            Raw::lam(TERMS, "t", Raw::lit(TERMS, int_lit(0))),
            Raw::lam(
                TERMS,
                "t",
                Raw::lam(
                    TERMS,
                    "kids",
                    Raw::match_on(
                        TERMS,
                        [Raw::var(TERMS, "kids")],
                        vec![
                            RawArm {
                                patterns: vec![RawPattern::constructor(TERMS, "List.Empty", [])],
                                body: Raw::lit(TERMS, int_lit(0)),
                            },
                            RawArm {
                                patterns: vec![RawPattern::constructor(
                                    TERMS,
                                    "List.Cons",
                                    [RawPattern::bind(TERMS, "first"), RawPattern::bind(TERMS, "rest")],
                                )],
                                body: calls("int_add", [Raw::lit(TERMS, int_lit(1)), Raw::var(TERMS, "first")]),
                            },
                        ],
                    ),
                ),
            ),
            Raw::lit(TERMS, tree_lit(subject)),
        ],
    )
}

/// A target that is not a literal leaves an ordinary blocked spine.
///
/// The companion to the law above and the reason it is not vacuous: what the
/// rule waits for is the *target*, so a traversal over a bound variable is stuck
/// however many of its other arguments are values. Without this a traversal
/// could not appear in the body of a function over a tree, which is where every
/// real one appears.
#[test]
fn a_structural_eliminator_at_a_neutral_target_is_neutral() {
    let cx = host();
    let ty = arrow(tree().term(TYPES), int().term(TYPES));
    let program = Raw::annotated_lam(
        TERMS,
        "t",
        Raw::var(TERMS, "Tree"),
        calls("tree_fold", [counting_leaf(), counting_branch(), Raw::var(TERMS, "t")]),
    );
    let term = check(&cx, &ty, &program).expect("an open traversal checks");
    assert_eq!(
        normalize_at(&cx, &ty, &term),
        term,
        "stuck on the binder, so the normal form is the term itself"
    );
}

/// A structural step is charged, so a traversal that does not descend is refused
/// rather than run forever.
///
/// The pair matters more than either half: at one budget a finite traversal
/// completes and a non-descending one ends in exhaustion, which is what
/// separates "the meter is doing its job" from "the budget was too small for
/// anything". §5.8 leaves the descent obligation at the table, because no
/// signature shows it; what the core owes is that breaking it costs a judgment
/// rather than the machine.
#[test]
fn a_traversal_that_does_not_descend_is_refused() {
    let narrow = host_at(Budget::LANGUAGE.scaled(4));
    let int_ty = int().term(TYPES);
    let finite = calls(
        "tree_fold",
        [
            counting_leaf(),
            counting_branch(),
            Raw::lit(TERMS, tree_lit(branch("a", vec![leaf("b")]))),
        ],
    );
    let term = check(&narrow, &int_ty, &finite).expect("a finite traversal fits");
    assert_eq!(
        normalize_at(&narrow, &int_ty, &term),
        int_lit(2).term(TERMS),
        "two nodes, at the same budget the runaway one cannot afford"
    );

    // Elaboration accepts it — nothing in the signature is wrong, which is the
    // whole reason the descent obligation lives at the table. It is asking for
    // the normal form that has to end.
    let runaway = calls("tree_spin", [Raw::lit(TERMS, tree_lit(leaf("b")))]);
    let term = check(&narrow, &int_ty, &runaway).expect("a runaway traversal is well typed");
    assert!(
        matches!(normalize(&narrow, &int_ty, &term), Err(CoreError::Exhausted(_))),
        "a rewrite that reapplies itself to its own target ends the judgment"
    );
}

// ---- registration ----------------------------------------------------------

/// The three refusals a registration owes, each reached by a registry that earns
/// it.
#[test]
fn a_base_type_indexed_by_a_literal_is_finite_data() {
    // `Tagged : Int → Type 0`, and `tagged_of : Int → Tagged ⟨0⟩`. This is the
    // shape `Duration : Coordinate → Type 0` has in `musa-compiler`, and the
    // index is a literal rather than a constructor for D3's reason: a δ-rule is
    // a `fn` pointer, so the only type it can answer at is one it can build
    // without a context.
    let tagged = Base::new("Tagged", arrow(int().term(TYPES), Term::universe(TYPES, Sort::ZERO)));
    let at_zero = Term::app(TYPES, tagged.term(TYPES), int_lit(0).term(TYPES));
    Registry::new(
        vec![int(), tagged],
        vec![Builtin::new(
            "tagged_of",
            arrow(int().term(TYPES), at_zero),
            Family::Delta,
            |_| None,
        )],
    )
    .expect("a registered base type applied to a literal index is finite data");
}

#[test]
fn a_literal_is_data_only_where_a_base_type_indexes_on_it() {
    // The same literal, with nothing registered in front of it. The law is the
    // narrow one — admitted *under a registered base head* — so a signature
    // that names a literal outright is refused exactly as it was before the
    // index case had a caller.
    let refusal = Registry::new(
        vec![int()],
        vec![Builtin::new(
            "of_three",
            arrow(int_lit(3).term(TYPES), int().term(TYPES)),
            Family::Delta,
            |_| None,
        )],
    )
    .expect_err("a bare literal is not a type a δ-builtin may take");
    assert!(
        matches!(refusal, Refusal::NotFiniteData { .. }),
        "refused, but as `{refusal}`"
    );
}

/// The registry's two checks are about *rules*, and a [`Builtin::constructor`]
/// has none — so it passes both by never being asked.
///
/// Worth stating because the difference between an absence and an exemption is
/// invisible from the outside and load-bearing from the inside. One signature
/// below registers as a constructor and is refused twice over as anything that
/// computes: as a δ-builtin because D1 admits no arrow, and as a traversal
/// because the argument it would fire on is not a base type. Neither check has
/// been relaxed. A constructor's application *is* its value, so a δ signature it
/// does not have and a target it does not fire at are both questions about
/// nothing.
///
/// What a constructor does at a *use* site — stay neutral saturated or not, and
/// normalize to itself — is `musa-compiler`'s `registry::machine::laws`, where
/// the first eight of them live.
#[test]
fn a_constructor_is_asked_neither_registration_question() {
    let signature = || {
        arrow(
            arrow(int().term(TYPES), int().term(TYPES)),
            arrow(int().term(TYPES), int().term(TYPES)),
        )
    };
    let refused = Registry::new(
        vec![int()],
        vec![Builtin::new("wrapped", signature(), Family::Delta, |_| None)],
    )
    .expect_err("a δ-builtin may not take a function");
    assert!(
        matches!(refused, Refusal::HigherOrderDelta { .. }),
        "refused, but as `{refused}`"
    );

    let refused = Registry::new(
        vec![int()],
        vec![Builtin::structural("wrapped", signature(), 0, |_, _| None)],
    )
    .expect_err("a traversal may not fire on an argument that is not a base type");
    assert!(
        matches!(refused, Refusal::TargetNotABase { .. }),
        "refused, but as `{refused}`"
    );

    Registry::new(
        vec![int()],
        vec![Builtin::constructor("wrapped", signature(), Family::Machine)],
    )
    .expect("the same signature registers as a constructor, because it carries no rule to check");
}

#[test]
fn a_registration_is_refused_by_the_table_it_is_about() {
    for RefusedRegistry {
        name,
        outcome,
        expected,
    } in refused_registries()
    {
        let Err(refusal) = outcome else {
            panic!("{name}: the registry was admitted");
        };
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
    }
}

// ---- what the coverage gate consumes ---------------------------------------

/// A registry that must be refused, and the refusal it owes.
pub(crate) struct RefusedRegistry {
    pub(crate) name: &'static str,
    pub(crate) outcome: Result<Registry, Refusal>,
    pub(crate) expected: fn(&Refusal) -> bool,
}

/// The registrations §5.8 forbids.
///
/// Carried as outcomes rather than as programs because a registration is not a
/// term: `elaboration_laws.rs`'s gate reaches them the way it reaches a refused
/// trait declaration, and for the same reason.
pub(crate) fn refused_registries() -> Vec<RefusedRegistry> {
    let unregistered = Base::new("Ratio", Term::universe(TYPES, Sort::ZERO));
    vec![
        RefusedRegistry {
            name: "one name registered twice",
            outcome: Registry::new(vec![int(), int()], Vec::new()),
            expected: |refusal| matches!(refusal, Refusal::DuplicateExtern { .. }),
        },
        RefusedRegistry {
            name: "a base type and a builtin under one name",
            outcome: Registry::new(
                vec![int()],
                vec![Builtin::new(
                    "Int",
                    arrow(int().term(TYPES), int().term(TYPES)),
                    Family::Delta,
                    |_| None,
                )],
            ),
            expected: |refusal| matches!(refusal, Refusal::DuplicateExtern { .. }),
        },
        RefusedRegistry {
            name: "a δ-builtin taking a function",
            outcome: Registry::new(
                vec![int()],
                vec![Builtin::new(
                    "int_map",
                    arrow(
                        arrow(int().term(TYPES), int().term(TYPES)),
                        arrow(int().term(TYPES), int().term(TYPES)),
                    ),
                    Family::Delta,
                    |_| None,
                )],
            ),
            expected: |refusal| matches!(refusal, Refusal::HigherOrderDelta { .. }),
        },
        RefusedRegistry {
            name: "a δ signature over a base type nobody registered",
            outcome: Registry::new(
                vec![int()],
                vec![Builtin::new(
                    "int_of_ratio",
                    arrow(unregistered.term(TYPES), int().term(TYPES)),
                    Family::Delta,
                    |_| None,
                )],
            ),
            expected: |refusal| matches!(refusal, Refusal::UnknownBase { .. }),
        },
        RefusedRegistry {
            name: "a traversal firing on an argument its signature does not have",
            outcome: Registry::new(
                vec![tree(), int()],
                vec![Builtin::structural(
                    "tree_size",
                    arrow(tree().term(TYPES), int().term(TYPES)),
                    2,
                    |_, _| None,
                )],
            ),
            expected: |refusal| matches!(refusal, Refusal::TargetOutsideSignature { .. }),
        },
        RefusedRegistry {
            name: "a traversal firing on a type that is not a base type",
            outcome: Registry::new(
                vec![int()],
                vec![Builtin::structural(
                    "record_walk",
                    arrow(Term::record_type(TYPES, []), int().term(TYPES)),
                    0,
                    |_, _| None,
                )],
            ),
            expected: |refusal| matches!(refusal, Refusal::TargetNotABase { .. }),
        },
        RefusedRegistry {
            name: "a δ signature over a type that is not finite data",
            outcome: Registry::new(
                vec![int()],
                vec![Builtin::new(
                    "int_of_record",
                    arrow(Term::record_type(TYPES, []), int().term(TYPES)),
                    Family::Delta,
                    |_| None,
                )],
            ),
            expected: |refusal| matches!(refusal, Refusal::NotFiniteData { .. }),
        },
    ]
}

/// A program that must be refused, and the refusal it owes.
pub(crate) struct RefusedProgram {
    pub(crate) name: &'static str,
    pub(crate) raw: Raw,
    /// The type it is checked against, written raw and inferred here.
    pub(crate) ty: Raw,
    pub(crate) expected: fn(&Refusal) -> bool,
}

/// The context those programs are checked in: the worked registry, and nothing
/// else.
pub(crate) fn base_context() -> Cx {
    host()
}

/// The programs D1's second half forbids.
pub(crate) fn refused_programs() -> Vec<RefusedProgram> {
    vec![
        RefusedProgram {
            name: "a constructor pattern at a base type",
            raw: Raw::match_on(
                TERMS,
                [Raw::lit(TERMS, int_lit(3))],
                vec![RawArm {
                    patterns: vec![RawPattern::constructor(TERMS, "Succ", [RawPattern::bind(TERMS, "k")])],
                    body: Raw::lit(TERMS, int_lit(0)),
                }],
            ),
            ty: Raw::var(TERMS, "Int"),
            expected: |refusal| matches!(refusal, Refusal::BaseNotMatchable { .. }),
        },
        RefusedProgram {
            name: "a record pattern at a base type",
            raw: Raw::match_on(
                TERMS,
                [Raw::lit(TERMS, text_lit("c"))],
                vec![RawArm {
                    patterns: vec![RawPattern::record(TERMS, [("head", RawPattern::bind(TERMS, "h"))])],
                    body: Raw::lit(TERMS, int_lit(0)),
                }],
            ),
            ty: Raw::var(TERMS, "Int"),
            expected: |refusal| matches!(refusal, Refusal::BaseNotMatchable { .. }),
        },
    ]
}

/// The programs a δ-rule refuses, which checking accepts.
///
/// A separate list because they fail at a different moment, and the moment is
/// the point: `int_div 6 0 : Int` is a true typing judgment, so elaboration has
/// nothing to object to. The rule reads its arguments when it fires, and firing
/// is reduction — conversion, a type that mentions the application, or a host
/// asking what the program means.
pub(crate) fn refused_reductions() -> Vec<RefusedProgram> {
    vec![RefusedProgram {
        name: "a δ-rule refusing the arguments it was given",
        raw: calls("int_div", [Raw::lit(TERMS, int_lit(6)), Raw::lit(TERMS, int_lit(0))]),
        ty: Raw::var(TERMS, "Int"),
        expected: |refusal| matches!(refusal, Refusal::BuiltinRefused { .. }),
    }]
}

/// `term` at `ty`, normalized, or a panic naming what stopped it.
fn normalize_at(cx: &Cx, ty: &Term, term: &Term) -> Term {
    normalize(cx, ty, term).expect("normalization of a closed, well-typed term")
}
