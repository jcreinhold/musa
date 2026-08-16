//! How a trait and an impl are made, and how a constraint is discharged.
//!
//! [`class`](crate::class) says what a trait and an instance *are*; this says
//! how one is built and how a use site reaches one. The split is
//! [`family`](crate::family)/[`declare`](crate::declare)'s, for the same reason:
//! the representation answers to evaluation, and this answers to
//! `docs/rules/language/10-traits.md` and to the author who wrote the
//! declaration wrong.
//!
//! # Every telescope is elaborated in the scope it will be read in
//!
//! A record type is a telescope — field `k`'s type is read under binders for
//! the `k` before it — and this crate has no substitution and no shift, by
//! design (`lib.rs`: "Reduction is never performed on syntax"). So a field type
//! cannot be elaborated under the parameters and then *placed* at position `k`;
//! it is elaborated in a scope that already has the earlier fields assumed, and
//! its de Bruijn indices come out right because nothing moved them. Every
//! telescope here is built that way, and it is why the super-constraint fields
//! come first: their types are terms this module assembles, so they must be
//! assembled at a depth that is known before any method is read.
//!
//! Where something built under one scope has to be read under a deeper one, it
//! crosses as a **[`Value`]**, never as a re-indexed [`Term`]. A value names its
//! free variables by de Bruijn *level*, so binders pushed after it was built
//! cannot disturb it — which is the same reason [`Cx`] keeps its binder types as
//! values, and it is what lets an impl's dictionary type be computed under the
//! instance parameters and then checked against under the `where` clause.
//!
//! # A constraint is discharged by solving a metavariable
//!
//! §4 postpones a constraint whose head is not yet known, and a postponed
//! constraint needs a hole in the output term that something later fills. That
//! is what a metavariable *is*, so a use of `Eq.equal` elaborates to
//! `?d.equal` with `?d : Eq ?A` recorded as pending; when `?A` is solved the
//! lookup runs and `?d` is solved to the instance's dictionary, and `zonk`
//! substitutes it away like any other. Nothing new had to be built for
//! postponement, and a constraint that is still blocked at the end is reported
//! by the machinery that already reports an unsolved hole.
//!
//! # An instance's parameters are recovered by unification
//!
//! Lookup finds `impl<T> Eq<List<T>>` from the key `(Eq, List)` alone, and the
//! `T` still has to be found. It is found the way every other unknown in this
//! crate is: a metavariable per instance parameter, the instance's *written*
//! arguments evaluated in an environment of those metas, and unification against
//! the arguments actually asked for. Writing a first-order matcher here instead
//! would be a second unifier beside 134's, free to disagree with it about
//! exactly the cases — η, δ, a solved meta — where agreement is what coherence
//! rests on.

use std::sync::Arc;

use crate::class::{Classes, Constraint, Derived, Head, Instance, Key, Kind, Trait, head_of, occurrences, size};
use crate::context::Cx;
use crate::elab::Elaborator;
use crate::eval::eval;
use crate::family::{Binder, Constant};
use crate::meta::MetaSource;
use crate::origin::Origin;
use crate::raw::{Raw, RawBinder, RawConstraint, RawImpl, RawMethod, RawTrait};
use crate::refuse::{ElabError, Refusal};
use crate::scope::Scope;
use crate::storable::STORABLE;
use crate::term::{Field, Index, Name, Shape, Term};
use crate::value::{Env, Form, Telescope, Value};

/// Elaborate a `trait` declaration.
pub(crate) fn declare_trait(cx: &Cx, raw: &RawTrait) -> Result<Arc<Trait>, ElabError> {
    let here = raw.origin;
    if raw.name.as_ref() == STORABLE {
        return Err(Refusal::ReservedClass {
            at: here,
            class: Arc::clone(&raw.name),
        }
        .into());
    }
    duplicate_method(raw)?;
    let closed = cx.closed();
    let mut elaborator = Elaborator::new(cx);
    let outer = Scope::new(&closed);
    let (params, under_params) = telescope(&mut elaborator, &outer, &raw.params)?;
    if params.is_empty() {
        return Err(Refusal::HeadlessClass {
            at: here,
            class: Arc::clone(&raw.name),
        }
        .into());
    }

    // The super-constraint fields, then the required-method fields, each read in
    // the scope the ones before it built — see the module doc for why the order
    // is not a preference.
    let mut fields: Vec<Field> = Vec::with_capacity(raw.context.len().saturating_add(raw.methods.len()));
    let mut supers = Vec::with_capacity(raw.context.len());
    let mut inner = under_params;
    for constraint in &raw.context {
        let (elaborated, ty) = constraint_at(&mut elaborator, &inner, cx.classes(), constraint)?;
        let name = Trait::super_field(&elaborated.class);
        inner = assumed(&mut elaborator, &inner, constraint.origin, &name, &ty)?;
        fields.push(Field { name, term: ty });
        supers.push(elaborated);
    }

    let mut methods = Vec::with_capacity(raw.methods.len());
    for method in &raw.methods {
        if method.body.is_some() {
            methods.push((Arc::clone(&method.name), Kind::Derived));
            continue;
        }
        if let Some(constraint) = method.context.first() {
            return Err(Refusal::ConstrainedField {
                at: constraint.origin,
                class: Arc::clone(&raw.name),
                method: Arc::clone(&method.name),
            }
            .into());
        }
        let (ty, _) = elaborator.check_type(&inner, &quantified(method))?;
        inner = assumed(&mut elaborator, &inner, method.origin, &method.name, &ty)?;
        fields.push(Field {
            name: Arc::clone(&method.name),
            term: ty,
        });
        methods.push((Arc::clone(&method.name), Kind::Required));
    }

    let dictionary = closed_lambda(here, &params, Term::new(here, Shape::RecordType(Arc::from(fields))));
    let declared = Trait {
        name: Arc::clone(&raw.name),
        package: cx.package(),
        params: Arc::from(params),
        supers: Arc::from(supers),
        dictionary,
        methods: Arc::from(methods),
        derived: Arc::from(Vec::new()),
    };
    let derived = derivations(&mut elaborator, cx, &declared, raw)?;
    elaborator.settled()?;
    Ok(Arc::new(Trait {
        derived: Arc::from(derived),
        ..declared
    }))
}

/// `{q⃗} → τ`: a required method's own parameters, folded onto its type.
///
/// Implicit, because a required method is reached by projecting it out of the
/// dictionary and applying it — `xs.fold_from_start(zero, step)` writes no `B`.
/// So the field's type is an ordinary implicit Π and §2's insertion rule fills
/// it, with nothing here to know about. A **derived** method's parameters are
/// not folded in this way: [`method_at`] fills those itself, because the
/// constraints that follow them have to be resolved rather than unified.
fn quantified(method: &RawMethod) -> Raw {
    method.params.iter().rev().fold(method.ty.clone(), |body, binder| {
        Raw::implicit_pi(binder.ty.origin(), Arc::clone(&binder.name), binder.ty.clone(), body)
    })
}

/// Refuse a trait that declares one method name twice.
fn duplicate_method(raw: &RawTrait) -> Result<(), ElabError> {
    for (which, method) in raw.methods.iter().enumerate() {
        if let Some(previous) = raw
            .methods
            .iter()
            .take(which)
            .find(|earlier| earlier.name == method.name)
        {
            return Err(Refusal::DuplicateMethod {
                at: method.origin,
                previous: previous.origin,
                class: Arc::clone(&raw.name),
                method: Arc::clone(&method.name),
            }
            .into());
        }
    }
    Ok(())
}

/// Elaborate an `impl` declaration, and run every check §2, §3, and §4 make at
/// the declaration.
pub(crate) fn declare_impl(cx: &Cx, raw: &RawImpl) -> Result<Arc<Instance>, ElabError> {
    let here = raw.origin;
    let classes = cx.classes();
    if raw.name.as_ref() == STORABLE {
        return Err(Refusal::HandWrittenStorable { at: here }.into());
    }
    let Some(class) = classes.class(&raw.name) else {
        return Err(Refusal::UnknownName {
            name: Arc::clone(&raw.name),
            at: here,
        }
        .into());
    };
    let class = Arc::clone(class);
    let closed = cx.closed();
    let mut elaborator = Elaborator::new(cx);
    let outer = Scope::new(&closed);
    let (params, under_params) = telescope(&mut elaborator, &outer, &raw.params)?;

    let args = arguments(&mut elaborator, &under_params, &class, here, &raw.args)?;
    let Some(Head::Rigid(head)) = args.first().and_then(|first| head_of(first, under_params.depth())) else {
        return Err(Refusal::BlanketInstance {
            at: here,
            class: Arc::clone(&raw.name),
        }
        .into());
    };
    let key = Key::rigid(&raw.name, &head);
    if let Some(previous) = classes.clash(&key) {
        return Err(Refusal::DuplicateInstance {
            at: here,
            previous: previous.origin,
            class: Arc::clone(&raw.name),
            head: Arc::clone(&head),
        }
        .into());
    }
    orphan(cx, here, &class, &raw.name, &head, args.first())?;

    // The dictionary's *type*, and the arguments the trait's own constraints are
    // read at: both computed here under the parameters and carried into the
    // deeper scope as **values** — see the module doc. The arguments in
    // particular cannot travel as terms, because the `where` clause below pushes
    // a binder per constraint and every index in them would then name the wrong
    // thing.
    let dictionary_ty = under_params.eval(elaborator.meter(), &applied(here, class.dictionary.clone(), &args))?;
    let mut at_args = Env::EMPTY;
    for argument in &args {
        at_args = at_args.push(under_params.eval(elaborator.meter(), argument)?);
    }

    // The `where` clause: each constraint read under the parameters and the
    // dictionaries before it, each measured against the head, and each recorded
    // as a local so that §4 step 1 can prefer it to a global instance.
    let mut context = Vec::with_capacity(raw.context.len());
    let mut inner = under_params.clone();
    let head_size = args.first().map_or(0, size);
    for constraint in &raw.context {
        let (elaborated, ty) = constraint_at(&mut elaborator, &inner, classes, constraint)?;
        decreasing(&elaborated, head_size, args.first(), under_params.depth(), &params)?;
        if let Some(key) = discharges(&elaborated, &inner) {
            let written = valued(&mut elaborator, &inner, &elaborated)?;
            inner = inner.discharging(key, inner.depth(), written);
        }
        inner = assumed(
            &mut elaborator,
            &inner,
            constraint.origin,
            &Trait::super_field(&elaborated.class),
            &ty,
        )?;
        context.push(elaborated);
    }

    // The dictionary itself: a record whose super fields are *resolved* and
    // whose method fields are checked at the types the trait declared for them.
    let value = dictionary_value(&mut elaborator, &inner, classes, &class, raw, &at_args, &dictionary_ty)?;

    let dictionary = closed_lambda(
        here,
        &params,
        context.iter().rev().fold(value, |body, constraint| {
            Term::lam(here, Trait::super_field(&constraint.class), body)
        }),
    );
    elaborator.settled()?;
    Ok(Arc::new(Instance {
        origin: here,
        key,
        params: Arc::from(params),
        args: Arc::from(args),
        context: Arc::from(context),
        dictionary,
    }))
}

/// The record an impl's dictionary is.
///
/// Written out here rather than assembled as a [`Raw`] record and handed to the
/// elaborator, and the reason is the super fields: they are already core terms,
/// because resolving a constraint *produces* one. A raw record could only carry
/// them by inventing a raw form that embeds a finished term, which would be a
/// hole in the "raw is what an author wrote" invariant for the sake of reusing
/// twelve lines. The telescope walk below is those twelve lines.
fn dictionary_value(
    elaborator: &mut Elaborator,
    scope: &Scope,
    classes: &Classes,
    class: &Trait,
    raw: &RawImpl,
    at_args: &Env,
    dictionary_ty: &Value,
) -> Result<Term, ElabError> {
    let here = raw.origin;
    supplied_methods(class, raw)?;
    let Form::RecordType(telescope) = &dictionary_ty.form else {
        return Err(Refusal::NotARecord {
            at: here,
            ty: scope.quote_type(elaborator.meter(), dictionary_ty)?,
        }
        .into());
    };
    let telescope = telescope.clone();

    // The trait's own constraints are written under *its* parameters, so they
    // are read at this instance's arguments through `at_args`, an environment of
    // them built where those arguments were elaborated.
    let mut env = telescope.env.clone();
    let mut fields = Vec::with_capacity(telescope.fields.len());
    for declared in telescope.fields.iter() {
        let field_ty = eval(elaborator.meter(), &env, &declared.term)?;
        let term = match class
            .supers
            .iter()
            .find(|constraint| Trait::super_field(&constraint.class) == declared.name)
        {
            Some(constraint) => {
                let needed = instantiated(elaborator, scope, constraint, at_args)?;
                resolve(elaborator, scope, classes, &needed)?
            }
            None => {
                let supplied = raw
                    .methods
                    .iter()
                    .find(|supplied| supplied.name == declared.name)
                    .ok_or_else(|| {
                        ElabError::from(Refusal::MissingMethod {
                            at: here,
                            class: Arc::clone(&raw.name),
                            method: Arc::clone(&declared.name),
                        })
                    })?;
                elaborator.check_open(scope, &supplied.value, &field_ty)?
            }
        };
        env = env.push(scope.eval(elaborator.meter(), &term)?);
        fields.push(Field {
            name: Arc::clone(&declared.name),
            term,
        });
    }
    Ok(Term::new(here, Shape::Record(fields.into())))
}

/// Refuse an impl's method that the trait derives or does not declare.
///
/// Before anything is checked, because "`Eq` has no method `equals`" is a better
/// first sentence than a type error inside a definition that was never going to
/// be used.
fn supplied_methods(class: &Trait, raw: &RawImpl) -> Result<(), ElabError> {
    for supplied in &raw.methods {
        match class.method(&supplied.name) {
            Some(Kind::Required) => {}
            Some(Kind::Derived) => {
                return Err(Refusal::DerivedMethod {
                    at: supplied.origin,
                    class: Arc::clone(&raw.name),
                    method: Arc::clone(&supplied.name),
                }
                .into());
            }
            None => {
                return Err(Refusal::NoSuchMethod {
                    at: supplied.origin,
                    class: Arc::clone(&raw.name),
                    method: Arc::clone(&supplied.name),
                    methods: class.methods.iter().map(|(name, _)| Arc::clone(name)).collect(),
                }
                .into());
            }
        }
    }
    Ok(())
}

/// A constraint written under one binding, read at the arguments in `at`.
pub(crate) fn instantiated(
    elaborator: &mut Elaborator,
    scope: &Scope,
    constraint: &Constraint,
    at: &Env,
) -> Result<Constraint, ElabError> {
    let mut args = Vec::with_capacity(constraint.args.len());
    for argument in constraint.args.iter() {
        let value = eval(elaborator.meter(), at, argument)?;
        args.push(scope.quote_type(elaborator.meter(), &value)?);
    }
    Ok(Constraint {
        origin: constraint.origin,
        class: Arc::clone(&constraint.class),
        args: Arc::from(args),
    })
}

/// §4's three steps: the local dictionaries, then the one table entry, then
/// postponement — and no fourth.
///
/// Returns the term that *is* the dictionary, so a caller projects a method out
/// of it or stores it in a field without asking what kind of answer it got.
pub(crate) fn resolve(
    elaborator: &mut Elaborator,
    scope: &Scope,
    classes: &Classes,
    needed: &Constraint,
) -> Result<Term, ElabError> {
    if let Some(dictionary) = lookup(elaborator, scope, classes, needed)? {
        return Ok(dictionary);
    }
    // Step 3: the head is not known yet, so the answer is a hole and a note to
    // come back — see [`discharge`].
    let at = needed.origin;
    let ty = dictionary_type(elaborator, scope, classes, needed)?;
    let hole = elaborator.fresh_meta(scope, at, MetaSource::Dictionary, &ty)?;
    elaborator.postpone(scope, needed, &hole, &ty, None);
    Ok(hole)
}

/// §4 steps 1 and 2, with `None` for the head that is not known yet.
///
/// Separate from [`resolve`] because a use site does not want the dictionary —
/// it wants a method read out of one — and the hole it leaves has to stand at
/// the *method's* type. See [`method_at`].
fn lookup(
    elaborator: &mut Elaborator,
    scope: &Scope,
    classes: &Classes,
    needed: &Constraint,
) -> Result<Option<Term>, ElabError> {
    let at = needed.origin;
    let Some(first) = needed.args.first() else {
        return Err(Refusal::ClassArity {
            at,
            class: Arc::clone(&needed.class),
            wanted: 1,
            written: 0,
        }
        .into());
    };

    let Some(head) = head_of(first, scope.depth()) else {
        if unkeyed(first) {
            return Err(Refusal::UnkeyedConstraint {
                at,
                class: Arc::clone(&needed.class),
            }
            .into());
        }
        return Ok(None);
    };
    let key = Key {
        class: Arc::clone(&needed.class),
        head: head.clone(),
    };

    // Step 1. Innermost first, so an inner `where` shadows an outer one the way
    // every other binder does. Its written arguments are unified with the ones
    // asked for, which is §1's "with it every other parameter" — the same thing
    // `apply_instance` does for a global, and needed for the same reason.
    if let Some(local) = scope.discharged(&key) {
        let local = local.clone();
        for (written, wanted) in local.args.iter().zip(needed.args.iter()) {
            let wanted = scope.eval(elaborator.meter(), wanted)?;
            elaborator.unify_types(scope, at, written, &wanted)?;
        }
        return Ok(Some(Term::var(at, level_index(scope.depth(), local.level))));
    }

    // A variable head can never be answered globally, and saying so is a
    // different sentence from "nothing implements this": the repair is a
    // constraint on the signature, not an instance somebody has to write.
    let Head::Rigid(name) = head else {
        return Err(Refusal::UnconstrainedVariable {
            at,
            class: Arc::clone(&needed.class),
        }
        .into());
    };

    // Step 2, and the last one: a single table read, whose `None` is the
    // refusal.
    let Some(instance) = classes.instance(&key) else {
        return Err(Refusal::UnresolvedInstance {
            at,
            class: Arc::clone(&needed.class),
            head: name,
        }
        .into());
    };
    let instance = Arc::clone(instance);
    Ok(Some(apply_instance(elaborator, scope, classes, &instance, needed)?))
}

/// One constraint §4 postponed, and the hole standing for its dictionary.
///
/// The scope travels with it because §4 step 1 is a lookup in the *local*
/// dictionaries, and a retry that had lost them would prefer a global instance
/// where the author's own `where` clause was in scope — which is the one thing
/// local-beats-global exists to stop.
pub(crate) struct Postponed {
    /// The binders it was written under, and the local dictionaries among them.
    pub(crate) scope: Scope,
    /// What it asks for.
    pub(crate) needed: Constraint,
    /// The metavariable the retry solves.
    pub(crate) hole: Term,
    /// The type that metavariable stands at, which the retry solves it at.
    pub(crate) ty: Value,
    /// What the dictionary is *for*, when it is not the answer itself.
    pub(crate) wanted: Option<Wanted>,
}

/// A method waiting on the dictionary it is read out of.
///
/// The hole a use site leaves stands at the **method's** type rather than at the
/// dictionary's, and that is not a preference. A term is only useful downstream
/// if the independent re-checker (prompt 134) accepts it, and `{ … }.equal` is
/// not a term it can accept: §2 gives a record literal no inference rule, so a
/// projection out of one has nothing to infer through. Solving a metavariable
/// makes the *value* of the projection the solution, and quotation reads back a
/// normal form — `Nat.Zero`, which infers.
///
/// That is half of what the re-checker needs, and the half this module can
/// supply. A normal form at a **Π** is η-long, so a method that is a function —
/// which every method `10-traits.md` §5 gives an operator is — reads back as a
/// λ, and the call around it is an application of an introduction form. Writing
/// the method's type down is what answers that, and it is done where the
/// read-back happens rather than here: see [`crate::elab`]'s `stated`.
pub(crate) struct Wanted {
    /// The hole standing for the dictionary, solved alongside the method's.
    pub(crate) dictionary: Term,
    /// Its type.
    pub(crate) dictionary_ty: Value,
    /// The method, written over that hole: a projection, or the trait's derived
    /// definition applied to it.
    pub(crate) read: Term,
}

/// Retry every postponed constraint until none of them can move.
///
/// The loop is what makes postponement a mechanism rather than a delay:
/// answering one constraint solves metavariables, which can determine the head
/// of another, so a single pass would answer the constraints in the order they
/// happened to be written. It terminates because every round either solves a
/// hole — and there are finitely many — or answers nothing and stops.
///
/// What is left over is not an error here. A constraint still blocked at the end
/// of the declaration is a hole nobody filled, and [`Elaborator::settled`]
/// reports it with every other unsolved hole rather than inventing a second
/// report for the same fact.
pub(crate) fn discharge(elaborator: &mut Elaborator) -> Result<(), ElabError> {
    loop {
        let waiting = elaborator.waiting();
        if waiting.is_empty() {
            return Ok(());
        }
        let mut answered = false;
        let mut blocked = Vec::new();
        for item in waiting {
            let mut args = Vec::with_capacity(item.needed.args.len());
            for argument in item.needed.args.iter() {
                args.push(elaborator.resolved(&item.scope, argument)?);
            }
            let needed = Constraint {
                args: Arc::from(args),
                ..item.needed
            };
            let classes = item.scope.cx().classes().clone();
            let Some(dictionary) = lookup(elaborator, &item.scope, &classes, &needed)? else {
                blocked.push(Postponed { needed, ..item });
                continue;
            };
            let at = needed.origin;
            let supplied = match &item.wanted {
                None => item.scope.eval(elaborator.meter(), &dictionary)?,
                Some(wanted) => {
                    // Solve the dictionary's own hole first, so that evaluating
                    // the method reaches through it to the instance.
                    let stood = item.scope.eval(elaborator.meter(), &wanted.dictionary)?;
                    let found = item.scope.eval(elaborator.meter(), &dictionary)?;
                    elaborator.unify_at(&item.scope, at, &wanted.dictionary_ty, &stood, &found)?;
                    item.scope.eval(elaborator.meter(), &wanted.read)?
                }
            };
            let hole = item.scope.eval(elaborator.meter(), &item.hole)?;
            // At the hole's *type* rather than as types, because a dictionary is
            // a record value and a method may be one too: §3 performs η at a
            // record during quotation, and a unification that did not say what
            // type it was at could not perform it.
            elaborator.unify_at(&item.scope, at, &item.ty, &hole, &supplied)?;
            answered = true;
        }
        elaborator.keep_waiting(blocked);
        if !answered {
            return Ok(());
        }
    }
}

/// Whether a constraint's argument is a type no instance could ever answer, as
/// against one whose head is not known *yet*.
///
/// §4's postponement is for the second. Spending it on the first trades a true
/// sentence now — "an arrow has no instance" — for an unsolved hole reported at
/// the end of the declaration, which sends the author looking for a missing
/// annotation instead of for the arrow they wrote.
fn unkeyed(term: &Term) -> bool {
    match term.shape() {
        Shape::App { function, .. } => unkeyed(function),
        // Canonical formers. None of them is a name, so no `impl` could ever be
        // keyed on one, and `02-core-calculus.md` §1.2 says so of the arrow in
        // particular.
        Shape::Pi { .. }
        | Shape::Universe(_)
        | Shape::RecordType(_)
        | Shape::Lam { .. }
        | Shape::Record(_)
        | Shape::Id { .. }
        | Shape::Refl(_) => true,
        // Neutral: stuck on a metavariable, and solving it is what postponement
        // is for.
        Shape::Meta(_) | Shape::Project { .. } | Shape::J { .. } | Shape::Let { .. } => false,
        // Reached only when `head_of` answered, so unreachable here. `false`
        // keeps the answer conservative rather than inventing a refusal. A base
        // type is on this list for the same reason: `head_of` keys on it.
        Shape::Var(_) | Shape::Const(_) | Shape::Base(_) => false,
        // A definition unfolds, so a constraint headed by one is stuck on
        // nothing an author can fix by annotating — but it is also not a
        // canonical former, and `false` keeps the answer conservative here for
        // the reason the line above does.
        Shape::Def(_) => false,
        // A builtin and a literal are terms, not type constructors. Neither is
        // a name an `impl` could be keyed on, which is what `true` says.
        Shape::Builtin(_) | Shape::Lit(_) => true,
    }
}

/// The dictionary an instance supplies, at the arguments a constraint asked for.
fn apply_instance(
    elaborator: &mut Elaborator,
    scope: &Scope,
    classes: &Classes,
    instance: &Instance,
    needed: &Constraint,
) -> Result<Term, ElabError> {
    let at = needed.origin;
    // One metavariable per instance parameter, and the instance's own terms read
    // in an environment of them — which is what turns `impl<T> Eq<List<T>>` into
    // `Eq (List ?T)` without substituting on syntax.
    let mut env = Env::EMPTY;
    let mut supplied = Vec::with_capacity(instance.params.len());
    for binder in instance.params.iter() {
        let ty = eval(elaborator.meter(), &env, &binder.ty)?;
        let term = elaborator.fresh_meta(scope, at, MetaSource::Dictionary, &ty)?;
        env = env.push(scope.eval(elaborator.meter(), &term)?);
        supplied.push(term);
    }
    for (written, wanted) in instance.args.iter().zip(needed.args.iter()) {
        let written = eval(elaborator.meter(), &env, written)?;
        let wanted = scope.eval(elaborator.meter(), wanted)?;
        elaborator.unify_types(scope, at, &written, &wanted)?;
    }

    // Its own context, resolved at those arguments. This terminates because
    // `decreasing` refused every instance whose context is not smaller than its
    // head, checked when the instance was declared rather than here.
    let mut dictionary = applied(at, instance.dictionary.clone(), &supplied);
    for constraint in instance.context.iter() {
        let inner = instantiated(elaborator, scope, constraint, &env)?;
        let inner = Constraint { origin: at, ..inner };
        let argument = resolve(elaborator, scope, classes, &inner)?;
        dictionary = Term::app(at, dictionary, argument);
    }
    Ok(dictionary)
}

/// The type a constraint's dictionary has, as a value in `scope`.
fn dictionary_type(
    elaborator: &mut Elaborator,
    scope: &Scope,
    classes: &Classes,
    needed: &Constraint,
) -> Result<Value, ElabError> {
    let Some(class) = classes.class(&needed.class) else {
        return Err(Refusal::UnknownName {
            name: Arc::clone(&needed.class),
            at: needed.origin,
        }
        .into());
    };
    let applied = applied(needed.origin, class.dictionary.clone(), &needed.args);
    Ok(scope.eval(elaborator.meter(), &applied)?)
}

/// The key a `where` constraint discharges, when its head is one lookup can key
/// on.
///
/// `None` for a constraint whose first argument has no head at all, which is not
/// a defect: such a dictionary is still *bound*, and a use that needs it is
/// postponed until its head is known rather than answered from here.
pub(crate) fn discharges(constraint: &Constraint, scope: &Scope) -> Option<Key> {
    let head = head_of(constraint.args.first()?, scope.depth())?;
    Some(Key {
        class: Arc::clone(&constraint.class),
        head,
    })
}

/// A constraint's arguments, as the local dictionary it becomes will hold them.
///
/// Values rather than terms because a local outlives the scope it was written
/// in: §4 step 1 reads it while checking a body nested arbitrarily deep inside
/// the `where`, and a term would have to be re-indexed at every one of those
/// depths. A value is closed over its own environment and is read at any of
/// them, which is [`Local::level`](crate::scope::Local::level)'s argument
/// applied to the arguments too.
///
/// # Errors
///
/// As [`crate::eval`].
pub(crate) fn valued(
    elaborator: &mut Elaborator,
    scope: &Scope,
    constraint: &Constraint,
) -> Result<Arc<[Value]>, ElabError> {
    let mut args = Vec::with_capacity(constraint.args.len());
    for argument in constraint.args.iter() {
        args.push(scope.eval(elaborator.meter(), argument)?);
    }
    Ok(Arc::from(args))
}

/// The index a variable at `level` has, read at `depth`.
fn level_index(depth: u32, level: u32) -> Index {
    Index(depth.saturating_sub(level).saturating_sub(1))
}

/// §3: an impl lives with its trait or with its head type's declaration.
fn orphan(
    cx: &Cx,
    at: Origin,
    class: &Trait,
    name: &Name,
    head: &Name,
    argument: Option<&Term>,
) -> Result<(), ElabError> {
    let Some(here) = cx.package() else {
        return Ok(());
    };
    if class.package.is_none_or(|package| package == here) {
        return Ok(());
    }
    let of_head = argument.and_then(constant).and_then(|constant| constant.group.package);
    if of_head.is_none_or(|package| package == here) {
        return Ok(());
    }
    Err(Refusal::OrphanInstance {
        at,
        class: Arc::clone(name),
        head: Arc::clone(head),
    }
    .into())
}

/// The declared constant a type application is headed by.
fn constant(term: &Term) -> Option<&Constant> {
    match term.shape() {
        Shape::Const(constant) => Some(constant),
        Shape::App { function, .. } => constant(function),
        // A head that is not a declared constant is not one §3 can place, and
        // `orphan` reads that as "not at home here" rather than guessing.
        Shape::Var(_)
        | Shape::Def(_)
        | Shape::Universe(_)
        | Shape::Pi { .. }
        | Shape::Lam { .. }
        | Shape::RecordType(_)
        | Shape::Record(_)
        | Shape::Project { .. }
        | Shape::Id { .. }
        | Shape::Refl(_)
        | Shape::J { .. }
        | Shape::Meta(_)
        | Shape::Builtin(_)
        | Shape::Lit(_)
        | Shape::Let { .. } => None,
        // A base type has no declaration and so no package. §3's orphan rule
        // asks whether an `impl` shares a package with the *declaration* of its
        // head type, and for a host-registered type there is none to share —
        // which `orphan` reads as "not at home here", the same conservative
        // answer it gives a variable.
        Shape::Base(_) => None,
    }
}

/// §4's measure, checked at the instance and never at a use.
fn decreasing(
    constraint: &Constraint,
    head_size: u32,
    head: Option<&Term>,
    depth: u32,
    params: &[Binder],
) -> Result<(), ElabError> {
    let Some(first) = constraint.args.first() else {
        return Ok(());
    };
    if size(first) >= head_size {
        return Err(Refusal::UnboundedInstance {
            at: constraint.origin,
            class: Arc::clone(&constraint.class),
            reason: "it is not smaller than the head",
        }
        .into());
    }
    let Some(head) = head else { return Ok(()) };
    let count = u32::try_from(params.len()).unwrap_or(u32::MAX);
    for which in 0..count {
        let level = depth.saturating_sub(count).saturating_add(which);
        if occurrences(first, depth, level) > occurrences(head, depth, level) {
            return Err(Refusal::UnboundedInstance {
                at: constraint.origin,
                class: Arc::clone(&constraint.class),
                reason: "a type variable occurs in it more often than in the head",
            }
            .into());
        }
    }
    Ok(())
}

/// The derived methods of a trait, each a definition over the dictionary.
fn derivations(elaborator: &mut Elaborator, cx: &Cx, class: &Trait, raw: &RawTrait) -> Result<Vec<Derived>, ElabError> {
    let mut built = Vec::new();
    for method in &raw.methods {
        let Some(body) = method.body.as_ref() else {
            continue;
        };
        let here = method.origin;
        let closed = cx.closed();
        let outer = Scope::new(&closed);
        let (_, under_params) = telescope(elaborator, &outer, &raw.params)?;
        let dictionary_ty = applied(here, class.dictionary.clone(), &parameters(here, class.params.len()));
        let ty_value = under_params.eval(elaborator.meter(), &dictionary_ty)?;
        let name: Name = Arc::from("dictionary");
        let under_dictionary = under_params.assume(Some(Arc::clone(&name)), here, Arc::new(ty_value.clone()));

        // The method's own parameters, then its own constraints, in that order
        // and not the other: `where Buildable<D, B>` mentions `D`, and nothing
        // in a parameter's type can mention a dictionary.
        let (own, under_own) = telescope(elaborator, &under_dictionary, &method.params)?;
        let (context, required, inner) = requirements(elaborator, cx, &under_own, method)?;

        // The type is checked *here* rather than deeper, and that is what lets
        // it be used as written: the Π below binds exactly these binders in
        // exactly this order, so its indices are already the ones it needs. The
        // `let`s that follow are why the old form had to quote the type back.
        let (stated, _) = elaborator.check_type(&inner, &method.ty)?;
        let goal = inner.eval(elaborator.meter(), &stated)?;

        // The trait's own required methods are in scope in a derived body, bound
        // to projections out of that dictionary. §1 calls a derived method "an
        // ordinary function that takes the dictionary", and this is what makes
        // the form worth having: `map` is written in terms of `fold`, by name.
        let deeper = own.len().saturating_add(required.len());
        let dictionary = Index(u32::try_from(deeper).unwrap_or(u32::MAX));
        let (with_methods, wrappers) = methods_in_scope(elaborator, &inner, class, &ty_value, dictionary, here)?;
        let definition = elaborator.check_open(&with_methods, body, &goal)?;

        // The `let`s that rebuilt the scope have to appear in the term, because
        // the body's indices were read under them.
        let definition = wrappers
            .iter()
            .rev()
            .fold(definition, |body, (field, field_ty, value)| {
                Term::bind(here, Arc::clone(field), field_ty.clone(), value.clone(), body)
            });
        built.push(Derived {
            name: Arc::clone(&method.name),
            params: Arc::from(own.clone()),
            context: Arc::from(context),
            ty: closed_pi(
                here,
                &class.params,
                Term::pi(
                    here,
                    Arc::clone(&name),
                    dictionary_ty,
                    closed_pi(here, &own, closed_pi(here, &required, stated)),
                ),
            ),
            value: closed_lambda(
                here,
                &class.params,
                Term::lam(
                    here,
                    Arc::clone(&name),
                    closed_lambda(here, &own, closed_lambda(here, &required, definition)),
                ),
            ),
        });
    }
    Ok(built)
}

/// A derived method's own `where` clause: the constraints, the binders they
/// become, and the scope in which they are answered.
///
/// Each is **discharged** into the scope as well as assumed, so a body that
/// writes `push` reaches the `Buildable` binder standing right there rather than
/// a global instance — §4 step 1, and the same call `declare_impl` makes for an
/// instance's own `where`. That is the whole reason a derived method may have
/// one: `map` is written at a `D` no instance can be declared for, so the only
/// dictionary that could ever answer is the one its caller supplies.
fn requirements(
    elaborator: &mut Elaborator,
    cx: &Cx,
    scope: &Scope,
    method: &RawMethod,
) -> Result<(Vec<Constraint>, Vec<Binder>, Scope), ElabError> {
    let mut context = Vec::with_capacity(method.context.len());
    let mut binders = Vec::with_capacity(method.context.len());
    let mut inner = scope.clone();
    for constraint in &method.context {
        let (elaborated, ty) = constraint_at(elaborator, &inner, cx.classes(), constraint)?;
        if let Some(key) = discharges(&elaborated, &inner) {
            let args = valued(elaborator, &inner, &elaborated)?;
            inner = inner.discharging(key, inner.depth(), args);
        }
        let name = Trait::super_field(&elaborated.class);
        inner = assumed(elaborator, &inner, constraint.origin, &name, &ty)?;
        binders.push(Binder::explicit(name, ty));
        context.push(elaborated);
    }
    Ok((context, binders, inner))
}

/// The scope a derived body is written in, and the `let`s that rebuild it.
///
/// One definition per required method, bound to the projection of it out of the
/// dictionary. They are δ-definitions rather than assumptions, so a body that
/// calls `equal` and a body that writes `dictionary.equal` elaborate to
/// convertible terms — which is what keeps the form a spelling rather than a
/// second way for a method to mean something.
fn methods_in_scope(
    elaborator: &mut Elaborator,
    scope: &Scope,
    class: &Trait,
    dictionary_ty: &Value,
    dictionary: Index,
    at: Origin,
) -> Result<(Scope, Vec<(Name, Term, Term)>), ElabError> {
    let Form::RecordType(telescope) = &dictionary_ty.form else {
        return Ok((scope.clone(), Vec::new()));
    };
    let telescope = telescope.clone();
    let subject = scope.eval(elaborator.meter(), &Term::var(at, dictionary))?;
    let mut inner = scope.clone();
    let mut wrappers: Vec<(Name, Term, Term)> = Vec::new();
    for (name, kind) in class.methods.iter() {
        if *kind != Kind::Required {
            continue;
        }
        let ty = field_type(elaborator, &telescope, &subject, name)?;
        let ty_term = inner.quote_type(elaborator.meter(), &ty)?;
        // The dictionary has moved out by one binder per wrapper already bound.
        let position = u32::try_from(wrappers.len()).unwrap_or(u32::MAX);
        let reached = Index(dictionary.0.saturating_add(position));
        let value_term = Term::project(at, Term::var(at, reached), Arc::clone(name));
        let value = inner.eval(elaborator.meter(), &value_term)?;
        wrappers.push((Arc::clone(name), ty_term, value_term));
        inner = inner.define(Arc::clone(name), Arc::new(ty), value);
    }
    Ok((inner, wrappers))
}

/// The type a telescope gives `field`, at the record `subject`.
fn field_type(
    elaborator: &mut Elaborator,
    telescope: &Telescope,
    subject: &Value,
    field: &Name,
) -> Result<Value, ElabError> {
    let mut env = telescope.env.clone();
    for declared in telescope.fields.iter() {
        let ty = eval(elaborator.meter(), &env, &declared.term)?;
        if declared.name == *field {
            return Ok(ty);
        }
        env = env.push(crate::eval::project(
            elaborator.meter(),
            subject.origin,
            subject.clone(),
            &declared.name,
        )?);
    }
    Err(Refusal::NoSuchField {
        at: subject.origin,
        field: Arc::clone(field),
    }
    .into())
}

/// Elaborate a constraint, answering it and the type its dictionary has here.
///
/// The type comes back **β-normal**. `Class a⃗` is `(λp⃗. { … }) a⃗`, and a redex
/// standing as a binder's domain is a term the re-checker cannot infer — it
/// reads a λ only against a Π it was given, which a domain position does not
/// supply. Reducing it here is what makes every binder built from this an
/// ordinary record-typed one.
pub(crate) fn constraint_at(
    elaborator: &mut Elaborator,
    scope: &Scope,
    classes: &Classes,
    raw: &RawConstraint,
) -> Result<(Constraint, Term), ElabError> {
    let Some(class) = classes.class(&raw.name) else {
        return Err(Refusal::UnknownName {
            name: Arc::clone(&raw.name),
            at: raw.origin,
        }
        .into());
    };
    let class = Arc::clone(class);
    let args = arguments(elaborator, scope, &class, raw.origin, &raw.args)?;
    let applied = applied(raw.origin, class.dictionary.clone(), &args);
    let value = scope.eval(elaborator.meter(), &applied)?;
    let ty = scope.quote_type(elaborator.meter(), &value)?;
    Ok((
        Constraint {
            origin: raw.origin,
            class: Arc::clone(&raw.name),
            args: Arc::from(args),
        },
        ty,
    ))
}

/// A constraint's arguments, one per trait parameter and checked at its type.
fn arguments(
    elaborator: &mut Elaborator,
    scope: &Scope,
    class: &Trait,
    at: Origin,
    raw: &[Raw],
) -> Result<Vec<Term>, ElabError> {
    if raw.len() != class.params.len() {
        return Err(Refusal::ClassArity {
            at,
            class: Arc::clone(&class.name),
            wanted: class.params.len(),
            written: raw.len(),
        }
        .into());
    }
    // A later parameter's type may mention an earlier one, so the types are read
    // through an environment of the arguments already checked.
    let mut env = Env::EMPTY;
    let mut args = Vec::with_capacity(raw.len());
    for (binder, argument) in class.params.iter().zip(raw) {
        let ty = eval(elaborator.meter(), &env, &binder.ty)?;
        let term = elaborator.check_open(scope, argument, &ty)?;
        env = env.push(scope.eval(elaborator.meter(), &term)?);
        args.push(term);
    }
    Ok(args)
}

/// Elaborate a telescope of binders, answering them and the scope they build.
fn telescope(elaborator: &mut Elaborator, scope: &Scope, raw: &[RawBinder]) -> Result<(Vec<Binder>, Scope), ElabError> {
    let mut binders = Vec::with_capacity(raw.len());
    let mut inner = scope.clone();
    for binder in raw {
        let (ty, _) = elaborator.check_type(&inner, &binder.ty)?;
        inner = assumed(elaborator, &inner, binder.ty.origin(), &binder.name, &ty)?;
        binders.push(Binder::explicit(Arc::clone(&binder.name), ty));
    }
    Ok((binders, inner))
}

fn assumed(elaborator: &mut Elaborator, scope: &Scope, at: Origin, name: &Name, ty: &Term) -> Result<Scope, ElabError> {
    let value = scope.eval(elaborator.meter(), ty)?;
    Ok(scope.assume(Some(Arc::clone(name)), at, Arc::new(value)))
}

/// `head a₁ … aₙ`.
fn applied(at: Origin, head: Term, args: &[Term]) -> Term {
    args.iter()
        .fold(head, |function, argument| Term::app(at, function, argument.clone()))
}

/// `λb₀ … λbₙ. body`.
fn closed_lambda(at: Origin, binders: &[Binder], body: Term) -> Term {
    binders
        .iter()
        .rev()
        .fold(body, |inner, binder| Term::lam(at, Arc::clone(&binder.name), inner))
}

/// `(b₀ : B₀) → … → body`.
fn closed_pi(at: Origin, binders: &[Binder], body: Term) -> Term {
    binders.iter().rev().fold(body, |codomain, binder| {
        Term::pi(at, Arc::clone(&binder.name), binder.ty.clone(), codomain)
    })
}

/// The variables standing for `count` binders just introduced, outermost first.
fn parameters(at: Origin, count: usize) -> Vec<Term> {
    (0..count)
        .map(|which| {
            let back = u32::try_from(count.saturating_sub(which)).unwrap_or(u32::MAX);
            Term::var(at, Index(back.saturating_sub(1)))
        })
        .collect()
}

/// `Class.method` at a use site: the dictionary its constraint needs, the
/// method read out of it, and its type.
///
/// §4's lookup with nothing before it and nothing after it. The trait's
/// arguments are unknown here — `Eq.equal` says which trait and not at which
/// type — so each becomes a metavariable, and whether the constraint can be
/// answered *now* is then the same question as whether that metavariable is
/// solved. A required method is a projection out of the dictionary; a derived
/// one is the trait's definition applied to it, which is the whole difference
/// between §1's two kinds.
///
/// The type is read off the trait here rather than inferred from the term by the
/// caller, and that is not a shortcut: the term is a projection whose record may
/// be an unsolved metavariable spine, and inferring through one would force the
/// very constraint §4 is entitled to postpone.
pub(crate) fn method_at(
    elaborator: &mut Elaborator,
    scope: &Scope,
    at: Origin,
    qualified: &Name,
) -> Result<Option<(Term, Value)>, ElabError> {
    let classes = scope.cx().classes().clone();
    let Some((class, method, kind)) = classes.method(qualified) else {
        return Ok(None);
    };
    let class = Arc::clone(class);

    // One metavariable per trait parameter, each read at the type the parameter
    // was declared with — which may mention the ones before it, so they are
    // evaluated through an environment of the metas already made.
    let mut env = Env::EMPTY;
    let mut args = Vec::with_capacity(class.params.len());
    for binder in class.params.iter() {
        let ty = eval(elaborator.meter(), &env, &binder.ty)?;
        let term = elaborator.fresh_meta(scope, at, MetaSource::Dictionary, &ty)?;
        env = env.push(scope.eval(elaborator.meter(), &term)?);
        args.push(term);
    }
    let needed = Constraint {
        origin: at,
        class: Arc::clone(&class.name),
        args: Arc::from(args.clone()),
    };
    // The dictionary is a hole rather than a lookup, *always*, even where the
    // lookup would succeed at once. The trait's arguments here are
    // metavariables — `Eq.equal` says which trait and not at which type — so the
    // head is unknown by construction and §4 postpones; making the immediate
    // case a second path would be a path no program takes.
    let dictionary_ty = scope.eval(elaborator.meter(), &applied(at, class.dictionary.clone(), &args))?;
    let dictionary = elaborator.fresh_meta(scope, at, MetaSource::Dictionary, &dictionary_ty)?;

    let (read, ty) = match kind {
        Kind::Required => {
            let Form::RecordType(telescope) = &dictionary_ty.form else {
                return Err(Refusal::NotARecord {
                    at,
                    ty: scope.quote_type(elaborator.meter(), &dictionary_ty)?,
                }
                .into());
            };
            let subject = scope.eval(elaborator.meter(), &dictionary)?;
            let ty = field_type(elaborator, telescope, &subject, &method)?;
            (Term::project(at, dictionary.clone(), method), ty)
        }
        // A derived method is
        // `(p⃗ : Params) → (dict : Class p⃗) → (q⃗ : Own) → (d⃗ : Ctx) → τ`
        // applied to all four, so its type is that Π instantiated the same way
        // its value is. The four groups are filled by three different rules and
        // that is the point of the walk: the trait's arguments and the method's
        // own parameters are metavariables, the trait's dictionary is a hole
        // §4 postpones, and each of the method's own constraints goes through
        // §4's lookup at the parameters just made.
        Kind::Derived => {
            let Some(derived) = class.derivation(&method) else {
                return Err(Refusal::UnknownName {
                    name: Arc::clone(qualified),
                    at,
                }
                .into());
            };
            let mut filled = args.clone();
            filled.push(dictionary.clone());
            env = env.push(scope.eval(elaborator.meter(), &dictionary)?);
            for binder in derived.params.iter() {
                let ty = eval(elaborator.meter(), &env, &binder.ty)?;
                let term = elaborator.fresh_meta(scope, at, MetaSource::ImplicitArgument, &ty)?;
                env = env.push(scope.eval(elaborator.meter(), &term)?);
                filled.push(term);
            }
            for constraint in derived.context.iter() {
                let wanted = instantiated(elaborator, scope, constraint, &env)?;
                let term = resolve(elaborator, scope, &classes, &wanted)?;
                env = env.push(scope.eval(elaborator.meter(), &term)?);
                filled.push(term);
            }

            let value = applied(at, derived.value.clone(), &filled);
            // Its type is a Π, so it is *instantiated* rather than applied: a Π
            // is a type and `Term::app` of one would be a term no rule accepts.
            let mut ty = scope.eval(elaborator.meter(), &derived.ty)?;
            for argument in &filled {
                let Form::Pi { codomain, .. } = &ty.form else {
                    return Err(Refusal::NotAFunction {
                        at,
                        ty: scope.quote_type(elaborator.meter(), &ty)?,
                    }
                    .into());
                };
                let codomain = codomain.clone();
                let argument = scope.eval(elaborator.meter(), argument)?;
                ty = crate::eval::apply_closure(elaborator.meter(), &codomain, argument)?;
            }
            (value, ty)
        }
    };

    let hole = elaborator.fresh_meta(scope, at, MetaSource::Dictionary, &ty)?;
    elaborator.postpone(
        scope,
        &needed,
        &hole,
        &ty,
        Some(Wanted {
            dictionary,
            dictionary_ty,
            read,
        }),
    );
    Ok(Some((hole, ty)))
}
