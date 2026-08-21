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
//! # A constraint is discharged by a hole resolved at declaration end
//!
//! A constraint met mid-expression needs a hole in the output term that
//! something later fills, so a use of `Eq.equal` elaborates to `?d.equal` with
//! `?d : Eq ?A`, registered by [`Elaborator::constrain`]. Nothing retries it:
//! [`Elaborator::settled`] resolves every registered constraint once, at the
//! end of the declaration, when matching has said everything it can about
//! `?A` — and `zonk` substitutes the answer away like any other solution.
//! Nothing new had to be built for the deferral, and a constraint whose head is
//! still unknown by then is reported by the machinery that already reports an
//! unsolved hole.
//!
//! # An instance's parameters are recovered by matching
//!
//! Lookup finds `impl<T> Eq<List<T>>` from the key `(Eq, List)` alone, and the
//! `T` still has to be found. It is found the way every other unknown in this
//! crate is: a hole per instance parameter, the instance's *written* arguments
//! evaluated in an environment of those holes, and §2.1's first-order match
//! against the arguments actually asked for. Writing a bespoke matcher here
//! instead would be a second conversion checker beside 134's, free to disagree
//! with it about exactly the cases — η, δ, a solved hole — where agreement is
//! what coherence rests on.

use std::sync::Arc;

use crate::class::{Classes, Constraint, Derived, Head, Instance, Key, Kind, Trait, head_of};
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
use crate::value::{Env, Form, Neutral, Telescope, Value};

/// Elaborate a `trait` declaration.
pub(crate) fn declare_trait(cx: &Cx, raw: &RawTrait) -> Result<(Arc<Trait>, crate::Spend), ElabError> {
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

    // §1's flat law: a trait's `where` clause was a supertrait, and a
    // supertrait is a dictionary obligation synthesized at every use — the
    // search `10-traits.md` §9 refuses. The methods alone are the fields.
    if let Some(constraint) = raw.context.first() {
        return Err(Refusal::SuperClass {
            at: constraint.origin,
            class: Arc::clone(&raw.name),
        }
        .into());
    }
    let mut fields: Vec<Field> = Vec::with_capacity(raw.methods.len());
    let mut inner = under_params;

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

    let params: Arc<[crate::family::Binder]> = Arc::from(params);
    let fields: Arc<[Field]> = Arc::from(fields);
    let dictionary = closed_lambda(here, &params, Term::new(here, Shape::RecordType(Arc::clone(&fields))));
    let declared = Trait {
        name: Arc::clone(&raw.name),
        package: cx.package(),
        params: Arc::clone(&params),
        dictionary,
        methods: Arc::from(methods),
        derived: Arc::from(Vec::new()),
    };
    let derived = derivations(&mut elaborator, cx, &declared, raw)?;
    elaborator.settled()?;
    // Everything below is stored: solutions are written back before storage,
    // for [`Elaborator::zonk`]'s reason.
    let mut zonked_fields = Vec::with_capacity(fields.len());
    for field in fields.iter() {
        zonked_fields.push(Field {
            name: Arc::clone(&field.name),
            term: elaborator.zonk(&field.term)?,
        });
    }
    let dictionary = closed_lambda(
        here,
        &params,
        Term::new(here, Shape::RecordType(Arc::from(zonked_fields))),
    );
    let mut zonked_derived = Vec::with_capacity(derived.len());
    for method in derived {
        zonked_derived.push(Derived {
            name: method.name,
            params: method.params,
            context: method.context,
            ty: elaborator.zonk(&method.ty)?,
            value: elaborator.zonk(&method.value)?,
        });
    }
    let class = Arc::new(Trait {
        dictionary,
        derived: Arc::from(zonked_derived),
        ..declared
    });
    Ok((class, elaborator.spent()))
}

/// `{q⃗} → τ`: a required method's own parameters, folded onto its type.
///
/// Type parameters, because a required method is reached by projecting it out
/// of the dictionary and applying it — `xs.fold_from_start(zero, step)` writes
/// no `B`. So the field's type is an ordinary parameter Π and §2's walk fills
/// it, with nothing here to know about. A **derived** method's parameters are
/// not folded in this way: [`method_at`] fills those itself, because the
/// constraints that follow them have to be resolved rather than unified.
fn quantified(method: &RawMethod) -> Raw {
    method.params.iter().rev().fold(method.ty.clone(), |body, binder| {
        Raw::parameter_pi(binder.ty.origin(), Arc::clone(&binder.name), binder.ty.clone(), body)
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
pub(crate) fn declare_impl(cx: &Cx, raw: &RawImpl) -> Result<(Arc<Instance>, crate::Spend), ElabError> {
    let here = raw.origin;
    let classes = cx.classes();
    if raw.name.as_ref() == STORABLE {
        return Err(Refusal::HandWrittenStorable { at: here }.into());
    }
    let Some(class) = classes.class(&raw.name) else {
        return Err(Refusal::UnknownName {
            name: Arc::clone(&raw.name),
            at: here,
            candidates: Vec::new(),
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

    // §4's flat law: an `impl` carries no `where` clause, because resolving one
    // would be synthesis from other instances — the recursion `10-traits.md`
    // §9 refuses. What the author would have written there is an ordinary
    // function taking the dictionaries as arguments, and §9's table says so.
    if let Some(constraint) = raw.context.first() {
        return Err(Refusal::ConstrainedInstance {
            at: constraint.origin,
            class: Arc::clone(&raw.name),
        }
        .into());
    }
    let inner = under_params.clone();

    // The dictionary itself: a record whose method fields are checked at the
    // types the trait declared for them.
    let value = dictionary_value(&mut elaborator, &inner, classes, &class, raw, &at_args, &dictionary_ty)?;

    elaborator.settled()?;
    // Stored, so written back first — [`Elaborator::zonk`]'s reason.
    let value = elaborator.zonk(&value)?;
    let args = args
        .into_iter()
        .map(|argument| elaborator.zonk(&argument))
        .collect::<Result<Vec<_>, _>>()?;
    let dictionary = closed_lambda(here, &params, value);
    let instance = Arc::new(Instance {
        origin: here,
        key,
        params: Arc::from(params),
        args: Arc::from(args),
        dictionary,
    });
    Ok((instance, elaborator.spent()))
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
    _classes: &Classes,
    class: &Trait,
    raw: &RawImpl,
    _at_args: &Env,
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
        let term = {
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

/// Resolution's three steps, once, at declaration end: a `where`-bound
/// dictionary, then `Storable` computed, then the one table entry.
///
/// Where the old calculus postponed a constraint whose head was not yet known,
/// this one is simply *run later* — [`Elaborator::settled`] is the only caller,
/// and by then matching has solved every parameter the expression determines.
/// What remains unknown then is the program's to answer for, and the refusals
/// below name it: a hole-headed constraint is [`Refusal::Unsolved`], a
/// variable-headed one is [`Refusal::UnconstrainedVariable`], a key with no
/// instance is [`Refusal::UnresolvedInstance`].
///
/// Returns the term that *is* the dictionary, so the caller stores it in the
/// hole that stands for it without asking what kind of answer it got.
pub(crate) fn resolve_at(
    elaborator: &mut Elaborator,
    scope: &Scope,
    constraint: &Constraint,
    env: &Env,
    at: Origin,
) -> Result<Term, ElabError> {
    // The arguments are read under the environment the constraint was written
    // against, and opened on quotation: a solved hole's solution is what the
    // lookup is about, never the hole.
    let depth = u32::try_from(env.iter().count()).unwrap_or(u32::MAX);
    let mut args = Vec::with_capacity(constraint.args.len());
    let mut values = Vec::with_capacity(constraint.args.len());
    for argument in constraint.args.iter() {
        let value = eval(elaborator.meter(), env, argument)?;
        args.push(crate::quote::quote_type(
            elaborator.meter(),
            crate::quote::Depth(depth),
            crate::quote::Mode::Open,
            &value,
        )?);
        values.push(value);
    }
    let needed = Constraint {
        origin: at,
        class: Arc::clone(&constraint.class),
        args: Arc::from(args),
    };

    let Some(first) = needed.args.first() else {
        return Err(Refusal::ClassArity {
            at,
            class: Arc::clone(&needed.class),
            wanted: 1,
            written: 0,
        }
        .into());
    };

    // `Storable` is not looked up at all: §1.2's fact about a type's shape,
    // computed where it is asked. It stands first because no source program
    // can write it, so no `where` clause or instance can be preferred to it.
    if &*needed.class == crate::storable::STORABLE {
        let Some(ty) = values.first() else {
            return Err(Refusal::ClassArity {
                at,
                class: Arc::clone(&needed.class),
                wanted: 1,
                written: 0,
            }
            .into());
        };
        // A port nothing determined is *undetermined*, not unstorable: the
        // refusal is the unsolved hole's, the same one the parameter audit
        // would have given.
        if let Some(opened) = crate::eval::opened(elaborator.meter(), ty)?
            && let crate::value::Form::Neutral(neutral) = &opened.form
            && let crate::value::Head::Hole(hole) = &neutral.head
            && !hole.is_solved()
        {
            return Err(Refusal::Unsolved {
                site: crate::meta::MetaSource::TypeParameter,
                created: hole.origin(),
                blocked: None,
            }
            .into());
        }
        return if crate::storable::is_storable(elaborator.meter(), scope.cx(), ty)? {
            Ok(Term::record(at, core::iter::empty()))
        } else {
            Err(Refusal::NotStorable { at, ty: first.clone() }.into())
        };
    }

    let Some(head) = head_of(first, depth) else {
        if matches!(first.shape(), crate::term::Shape::Hole(_)) {
            return Err(Refusal::Unsolved {
                site: MetaSource::Constraint,
                created: at,
                blocked: None,
            }
            .into());
        }
        if unkeyed(first) {
            return Err(Refusal::UnkeyedConstraint {
                at,
                class: Arc::clone(&needed.class),
                ty: first.clone(),
            }
            .into());
        }
        return Err(Refusal::UnconstrainedVariable {
            at,
            class: Arc::clone(&needed.class),
        }
        .into());
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
        return Ok(Term::var(at, level_index(scope.depth(), local.level)));
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
    let classes = scope.cx().classes().clone();
    let Some(instance) = classes.instance(&key) else {
        return Err(Refusal::UnresolvedInstance {
            at,
            class: Arc::clone(&needed.class),
            head: name,
        }
        .into());
    };
    let instance = Arc::clone(instance);
    apply_instance(elaborator, scope, &instance, &needed, &values)
}

/// Whether a constraint's argument is a type no instance could ever answer, as
/// against one whose head is not known *yet*.
///
/// Waiting for [`Elaborator::settled`] is for the second, and by the time this
/// runs the waiting is over. What is left is which sentence to report, and the
/// choice is worth making: calling an arrow merely unsolved sends the author
/// looking for a missing annotation instead of for the arrow they wrote, so
/// `true` here earns the true sentence — "an arrow has no instance" — and every
/// shape that might still have resolved answers `false`.
fn unkeyed(term: &Term) -> bool {
    match term.shape() {
        Shape::Hole(_) => false,
        Shape::App { function, .. } => unkeyed(function),
        // Keyed exactly when what it refines is: see `class::head_of`.
        Shape::Refine { ty, .. } => unkeyed(ty),
        // Canonical formers. None of them is a name, so no `impl` could ever be
        // keyed on one, and `02-core-calculus.md` §1.2 says so of the arrow in
        // particular.
        Shape::Pi { .. } | Shape::Universe(_) | Shape::RecordType(_) | Shape::Lam { .. } | Shape::Record(_) => true,
        // Neutral: stuck on a hole, and solving it is what waiting until
        // declaration end is for.
        Shape::Project { .. } | Shape::Let { .. } => false,
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
        Shape::Builtin(_) | Shape::Lit(_) | Shape::Numeral(_) => true,
    }
}

/// The dictionary an instance supplies, at the arguments a constraint asked for.
fn apply_instance(
    elaborator: &mut Elaborator,
    scope: &Scope,
    instance: &Instance,
    needed: &Constraint,
    wanted: &[Value],
) -> Result<Term, ElabError> {
    let at = needed.origin;
    // One hole per instance parameter, and the instance's head read in an
    // environment of them: the head match below is what solves them, which is
    // the whole of instance instantiation under §2.1 — first-order, and
    // determined by the constraint being answered.
    let mut env = Env::EMPTY;
    let mut supplied = Vec::with_capacity(instance.params.len());
    for binder in instance.params.iter() {
        let ty = eval(elaborator.meter(), &env, &binder.ty)?;
        let hole = elaborator.fresh_hole(at, &ty);
        env = env.push(Value::neutral(crate::value::Neutral::head(
            at,
            crate::value::Head::Hole(hole.clone()),
        )));
        supplied.push(Term::hole(at, hole));
    }
    for (written, wanted) in instance.args.iter().zip(wanted.iter()) {
        let written = eval(elaborator.meter(), &env, written)?;
        elaborator.unify_types(scope, at, &written, wanted)?;
    }

    // Its context is empty — the flat law: an `impl` carries no `where`
    // clause, so there is nothing to resolve here. The dictionary is the
    // impl's own value applied to the parameters the head match solved.
    Ok(applied(at, instance.dictionary.clone(), &supplied))
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
            candidates: Vec::new(),
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
/// resolved at declaration end, once its head is known, rather than from here.
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
        // The orphan rule asks where a type is *declared*, and a refinement
        // declares nothing: `Row(12)` is at home wherever `Row` is.
        Shape::Refine { ty, .. } => constant(ty),
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
        | Shape::Builtin(_)
        | Shape::Lit(_)
        | Shape::Numeral(_)
        | Shape::Hole(_)
        | Shape::Let { .. } => None,
        // A base type has no declaration and so no package. §3's orphan rule
        // asks whether an `impl` shares a package with the *declaration* of its
        // head type, and for a host-registered type there is none to share —
        // which `orphan` reads as "not at home here", the same conservative
        // answer it gives a variable.
        Shape::Base(_) => None,
    }
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
        let name: Name = Arc::clone(&elaborated.class);
        inner = assumed(elaborator, &inner, constraint.origin, &name, &ty)?;
        binders.push(Binder::written(name, ty));
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
        inner = inner.define(elaborator.meter(), Arc::clone(name), Arc::new(ty), value)?;
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
    // §1.2's one constraint shape, and not a trait: nothing looks it up, so
    // nothing has to declare it. Its dictionary is the empty record — the
    // evidence is that the type's shape was checked, which is why resolution
    // computes rather than reads.
    if raw.name.as_ref() == STORABLE {
        let [argument] = raw.args.as_slice() else {
            return Err(Refusal::ClassArity {
                at: raw.origin,
                class: Arc::clone(&raw.name),
                wanted: 1,
                written: raw.args.len(),
            }
            .into());
        };
        let (term, _) = elaborator.check_type(scope, argument)?;
        return Ok((
            Constraint {
                origin: raw.origin,
                class: Arc::clone(&raw.name),
                args: Arc::from([term]),
            },
            Term::record_type(raw.origin, core::iter::empty()),
        ));
    }
    let Some(class) = classes.class(&raw.name) else {
        return Err(Refusal::UnknownName {
            name: Arc::clone(&raw.name),
            at: raw.origin,
            candidates: Vec::new(),
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
        binders.push(Binder::written(Arc::clone(&binder.name), ty));
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

pub(crate) fn method_at(
    elaborator: &mut Elaborator,
    scope: &Scope,
    at: Origin,
    qualified: &Name,
) -> Result<Option<(Term, Value)>, ElabError> {
    let Some((class, method, kind)) = scope.cx().classes().method(qualified) else {
        return Ok(None);
    };
    let class = Arc::clone(class);

    // One hole per trait parameter, each read at the type the parameter was
    // declared with — which may mention the ones before it, so they are
    // evaluated through an environment of the holes already made. The
    // application walk solves them by matching; `Eq.equal` says which trait
    // and not at which type, and the receiver or the arguments say the rest.
    let mut env = Env::EMPTY;
    let mut args = Vec::with_capacity(class.params.len());
    for binder in class.params.iter() {
        let ty = eval(elaborator.meter(), &env, &binder.ty)?;
        let hole = elaborator.fresh_hole(at, &ty);
        env = env.push(Value::neutral(crate::value::Neutral::head(
            at,
            crate::value::Head::Hole(hole.clone()),
        )));
        args.push(Term::hole(at, hole));
    }
    let needed = Arc::new(Constraint {
        origin: at,
        class: Arc::clone(&class.name),
        args: Arc::from(args.clone()),
    });
    // The dictionary is a hole resolved at declaration end — the one place
    // resolution runs, for [`Elaborator::constrain`]'s reason.
    let dictionary_ty = scope.eval(elaborator.meter(), &applied(at, class.dictionary.clone(), &args))?;
    // The scope's own environment: the holes' solutions are quoted at the
    // resolution's depth, and a solution naming a parameter in scope here —
    // `Eq A` under a generic `A` — has to fit.
    let dictionary = elaborator.constrain(scope, needed, scope.env().clone(), at, &dictionary_ty);
    let dictionary_term = Term::hole(at, dictionary.clone());
    let dictionary_value = Value::neutral(Neutral::head(at, crate::value::Head::Hole(dictionary)));

    match kind {
        Kind::Required => {
            let Form::RecordType(telescope) = &dictionary_ty.form else {
                return Err(Refusal::NotARecord {
                    at,
                    ty: scope.quote_type(elaborator.meter(), &dictionary_ty)?,
                }
                .into());
            };
            let ty = field_type(elaborator, telescope, &dictionary_value, &method)?;
            Ok(Some((Term::project(at, dictionary_term, method), ty)))
        }
        // A derived method is `(p⃗ : Params) → (dict : Class p⃗) → (q⃗ : Own) →
        // (d⃗ : Ctx) → τ` applied to all four: the trait's parameters and the
        // method's own are holes the walk solves, and each constraint is
        // registered for the one resolution at declaration end.
        Kind::Derived => {
            let Some(derived) = class.derivation(&method) else {
                return Err(Refusal::UnknownName {
                    name: Arc::clone(qualified),
                    at,
                    candidates: Vec::new(),
                }
                .into());
            };
            let mut filled = args.clone();
            filled.push(dictionary_term);
            env = env.push(dictionary_value);
            for binder in derived.params.iter() {
                let ty = eval(elaborator.meter(), &env, &binder.ty)?;
                let hole = elaborator.fresh_hole(at, &ty);
                env = env.push(Value::neutral(Neutral::head(
                    at,
                    crate::value::Head::Hole(hole.clone()),
                )));
                filled.push(Term::hole(at, hole));
            }
            for constraint in derived.context.iter() {
                let wanted = instantiated(elaborator, scope, constraint, &env)?;
                let classes = scope.cx().classes().clone();
                let ty = dictionary_type(elaborator, scope, &classes, &wanted)?;
                let wanted = Arc::new(wanted);
                let hole = elaborator.constrain(scope, wanted, env.clone(), at, &ty);
                let term = Term::hole(at, hole.clone());
                env = env.push(Value::neutral(Neutral::head(at, crate::value::Head::Hole(hole))));
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
            Ok(Some((value, ty)))
        }
    }
}
