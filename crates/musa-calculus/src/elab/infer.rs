//! The inference judgment: the forms that compute a type.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::class::{Head, Key, head_of};
use crate::eval::{apply_closure, eval, opened};
use crate::level::Level;
use crate::origin::Origin;
use crate::raw::{Raw, RawConstraint, RawShape};
use crate::refuse::{ElabError, Refusal};
use crate::scope::Scope;
use crate::term::{Name, Plicity, Shape, Term};
use crate::value::{Env, Form, Value};

use super::spine::{Slot, Walk};
use super::{Elaborator, Typed};

impl Elaborator {
    /// `Γ ⊢ raw ⇒ ty ⇝ t`.
    pub(super) fn infer(&mut self, scope: &Scope, raw: &Raw) -> Result<Typed, ElabError> {
        let here = raw.origin();
        match raw.shape() {
            RawShape::Var(name) => {
                // §2.1's uniqueness rule: a name no binder answers to may be
                // the one case exactly one family declares. Tried before the
                // declaration chain so that the family reading is the one an
                // argument position can still determine the parameters of.
                if scope.lookup(name).is_none()
                    && let Some(built) = self.constructed_open(scope, raw)?
                {
                    return Ok(built);
                }
                let Some(found) = scope.lookup(name) else {
                    return self.constant(scope, here, name);
                };
                Ok(Typed {
                    term: Term::var(here, found.index),
                    ty: Value::clone(&found.ty),
                })
            }
            RawShape::Hosted(name) => self.hosted(scope, here, name),
            // A literal carries the base type it inhabits, so it infers rather
            // than checks: the host wrote the type down when it made the
            // literal, and reading it off anything else would be guessing at
            // what the host already said.
            RawShape::Lit(literal) => Ok(Typed {
                term: literal.term(here),
                ty: eval(&mut self.meter, &Env::EMPTY, literal.ty())?,
            }),
            RawShape::Numeral { family, count } => self.numeral(scope, here, family, *count),
            RawShape::Universe(written) => {
                // §1: two universes, and a bare `Type` is `Type 0`.
                let level = written.as_ref().copied().unwrap_or(Level::ZERO);
                let Some(above) = level.succ() else {
                    return Err(Refusal::BeyondUniverses { at: here }.into());
                };
                Ok(Typed {
                    ty: Value::new(here, Form::Universe(above)),
                    term: Term::universe(here, level),
                })
            }
            RawShape::Pi {
                plicity,
                name,
                domain,
                codomain,
            } => self.function_type(scope, here, plicity.clone(), name, domain, codomain),
            RawShape::ConstrainedPi { constraint, codomain } => {
                self.constrained_function_type(scope, here, constraint, codomain)
            }
            RawShape::Lam {
                plicity,
                name,
                domain,
                body,
            } => self.infer_lambda(scope, here, plicity.clone(), name, domain.as_ref(), body),
            RawShape::App {
                plicity,
                function,
                argument,
            } => match self.constructed_open(scope, raw)? {
                Some(built) => Ok(built),
                None => self.application(scope, here, plicity, function, argument),
            },
            // The same two steps the arm above takes, because a constructor
            // written with its fields is the form an author actually writes and
            // `Succ(fewer)` is one: `written_spine` reads both forms, so the
            // constructor rule sees the same head and the same arguments here.
            RawShape::Call { function, arguments } => match self.constructed_open(scope, raw)? {
                Some(built) => Ok(built),
                None => self.complete_call(scope, here, function, arguments, None),
            },
            RawShape::RecordType(fields) => self.record_type(scope, here, fields),
            // §2: a record literal is an introduction form, so it checks. The
            // type it "obviously" has is a guess rather than a principal type —
            // `{ ty = {}, val = {} }` inhabits both `{ ty : Type 0, val : ty }`
            // and `{ ty : Type 0, val : {} }` — and picking one would be the
            // unifier's forbidden habit of trying the solution that comes to
            // hand. So there is no inference rule, and an author who wants to
            // project out of a literal writes the type it should have.
            RawShape::Record(_) => Err(Refusal::Uninferable { at: here }.into()),
            RawShape::Method { receiver, method } => self.method(scope, here, receiver, method),
            RawShape::Project { record, field } => self.projection(scope, here, record, field),
            RawShape::Update { record, updates } => self.update(scope, here, record, updates),
            RawShape::Let {
                name,
                ty: written,
                value,
                body,
            } => {
                let bound = self.definition(scope, name, written.as_ref(), value)?;
                let inferred = self.infer(&bound.scope, body)?;
                Ok(Typed {
                    term: Term::bind(here, Arc::clone(name), bound.ty_term, bound.value_term, inferred.term),
                    ty: inferred.ty,
                })
            }
            // §2's `Annot`, the other mode-switch rule: an author writes a type
            // and the term is checked against it.
            RawShape::Annot { term, ty } => {
                let (ty_term, _) = self.check_type(scope, ty)?;
                let ty_value = scope.eval(&mut self.meter, &ty_term)?;
                Ok(Typed {
                    term: self.check(scope, term, &ty_value)?,
                    ty: ty_value,
                })
            }
            // §6.2's `match` and §2.4's `rec` check and never infer, for the
            // same reason a record literal does not: the type is what decides
            // the elaboration, and guessing it from an arm or from a body would
            // make the answer depend on which one was written first.
            RawShape::Match { .. } | RawShape::Rec { .. } => Err(Refusal::Uninferable { at: here }.into()),
        }
    }

    /// `(x : A) → B ⇒ Type (max l l')`.
    fn function_type(
        &mut self,
        scope: &Scope,
        here: Origin,
        plicity: Plicity,
        name: &Name,
        domain: &Raw,
        codomain: &Raw,
    ) -> Result<Typed, ElabError> {
        let (domain_term, domain_level) = self.check_type(scope, domain)?;
        let domain_value = scope.eval(&mut self.meter, &domain_term)?;
        let inner = scope.assume(Some(Arc::clone(name)), here, Arc::new(domain_value));
        let (codomain_term, codomain_level) = self.check_type(&inner, codomain)?;
        Ok(Typed {
            term: Term::function(here, plicity, Arc::clone(name), domain_term, codomain_term),
            ty: Value::new(here, Form::Universe(domain_level.max(codomain_level))),
        })
    }

    /// `[Class a⃗] → B ⇒ Type (max l l')` — `01-surface.md` §1.4's `where`.
    ///
    /// It adds no term to the calculus, which is §1.4's own claim: what this
    /// builds is the Π that was already there, at a domain the author did not
    /// have to write because the trait and its arguments determine it. The
    /// level is asked of the assembled dictionary type rather than read off a
    /// raw one, since there is no raw one — [`crate::dictionary`] hands back a
    /// term, and `Class a⃗` β-reduces to the record type whose universe is the
    /// answer.
    ///
    /// The binder takes the trait's own name, which is what makes a body's
    /// `Eq` and the dictionary it stands at the same word in a diagnostic.
    fn constrained_function_type(
        &mut self,
        scope: &Scope,
        here: Origin,
        raw: &RawConstraint,
        codomain: &Raw,
    ) -> Result<Typed, ElabError> {
        let classes = scope.cx().classes().clone();
        let (constraint, domain_term) = crate::dictionary::constraint_at(self, scope, &classes, raw)?;
        let domain_value = scope.eval(&mut self.meter, &domain_term)?;
        let domain_level = Term::level_of(&domain_term)?;
        let name: Name = Arc::clone(&constraint.class);
        let constraint = Arc::new(constraint);
        // Discharged as well as assumed, for [`Self::discharging`]'s reason: a
        // codomain that mentions the trait's own methods is answered by the
        // binder standing right there.
        let inner = self.discharging(scope, &Plicity::Constraint(Arc::clone(&constraint)), scope.env())?;
        let inner = inner.assume(Some(Arc::clone(&name)), here, Arc::new(domain_value));
        let (codomain_term, codomain_level) = self.check_type(&inner, codomain)?;
        Ok(Typed {
            term: Term::constrained_pi(here, constraint, name, domain_term, codomain_term),
            ty: Value::new(here, Form::Universe(domain_level.max(codomain_level))),
        })
    }

    /// `λx. e ⇒ (x : A) → B`, where `A` is the annotation §2 asks for.
    fn infer_lambda(
        &mut self,
        scope: &Scope,
        here: Origin,
        plicity: Plicity,
        name: &Name,
        domain: Option<&Raw>,
        body: &Raw,
    ) -> Result<Typed, ElabError> {
        let domain_value = match domain {
            Some(written) => {
                let (term, _) = self.check_type(scope, written)?;
                Arc::new(scope.eval(&mut self.meter, &term)?)
            }
            None => {
                // §2: a binder the checking type did not describe must be
                // annotated. No hole stands here, because nothing downstream
                // of an inferred λ ever determines one — the annotation is the
                // program saying what it means.
                return Err(Refusal::Uninferable { at: here }.into());
            }
        };
        let inner = scope.assume(Some(Arc::clone(name)), here, Arc::clone(&domain_value));
        let inferred = self.infer(&inner, body)?;
        // The body's type is read under the binder, so the Π that reports it has
        // to be built from a term rather than from the value directly.
        let codomain = inner.quote_type(&mut self.meter, &inferred.ty)?;
        let domain_term = scope.quote_type(&mut self.meter, &domain_value)?;
        let ty = scope.eval(
            &mut self.meter,
            &Term::function(here, plicity, Arc::clone(name), domain_term, codomain),
        )?;
        Ok(Typed {
            term: Term::lam(here, Arc::clone(name), inferred.term),
            ty,
        })
    }

    /// §1.3's arity law: a `Call` supplies every declared parameter, and this
    /// is where the count is checked.
    ///
    /// **Measured on the function, not on the result.** §1.3's word is
    /// "declared", and [`declared_parameters`] is what reads it: the named
    /// explicit binders of `f`'s own type, taken before an argument has solved
    /// anything. Counting what stands after the last argument instead would
    /// count a *result* that happens to be a function, which is what a generic
    /// answer becomes the moment a caller instantiates it.
    ///
    /// Over-application is left to the walk. §1.3 names one direction, and
    /// "supplies every declared parameter" is silent about an argument list
    /// that runs past them into a result that takes more — which either
    /// applies, or earns [`Refusal::NotAFunction`] from the argument that
    /// could not.
    ///
    /// `expected` is the type the call stands at, and it is [`None`] unless
    /// the caller is [`Elaborator::check`] with an argument the walk may
    /// defer. It reaches the walk for one reason: a call all of whose
    /// informative arguments are bare lambdas has nothing *inside* it to say
    /// what they are, and the position it stands in does. See
    /// [`Elaborator::apply_spine`].
    pub(super) fn complete_call(
        &mut self,
        scope: &Scope,
        here: Origin,
        function: &Raw,
        arguments: &[Raw],
        expected: Option<&Value>,
    ) -> Result<Typed, ElabError> {
        let head = self.infer(scope, function)?;
        let stated = scope.quote_type(&mut self.meter, &head.ty)?;
        let declared = declared_parameters(&head.term, &stated);
        if let Some(missing) = declared.get(arguments.len()..).filter(|rest| !rest.is_empty()) {
            return Err(Refusal::Underapplied {
                at: here,
                function: crate::show::head_spelled(&head.term),
                wanted: declared.len(),
                written: arguments.len(),
                missing: missing.to_vec(),
            }
            .into());
        }
        self.apply_spine(scope, here, head, &arguments.iter().collect::<Vec<_>>(), expected)
    }

    /// `f a ⇒ B[a]`.
    fn application(
        &mut self,
        scope: &Scope,
        here: Origin,
        plicity: &Plicity,
        function: &Raw,
        argument: &Raw,
    ) -> Result<Typed, ElabError> {
        let inferred = self.infer(scope, function)?;
        if *plicity == Plicity::Implicit {
            // The one written-implicit rule: the argument fills the next
            // binder when that binder is the implicit one — the spelling the
            // host's schemes and the suite's fixtures use for a type argument
            // a use site writes out — and is refused otherwise, which is the
            // refusal's own sentence.
            let unfolded = crate::eval::opened(&mut self.meter, &inferred.ty)?;
            let ty = unfolded.as_ref().unwrap_or(&inferred.ty);
            let Form::Pi {
                plicity: Plicity::Implicit,
                domain,
                codomain,
                ..
            } = &ty.form
            else {
                return Err(Refusal::PlicityMismatch { at: argument.origin() }.into());
            };
            let argument_term = self.check(scope, argument, &Arc::clone(domain))?;
            let value = scope.eval(&mut self.meter, &argument_term)?;
            return Ok(Typed {
                term: Term::app(here, inferred.term, argument_term),
                ty: apply_closure(&mut self.meter, codomain, value)?,
            });
        }
        self.apply_spine(scope, here, inferred, &[argument], None)
    }

    /// `x.m` — `10-traits.md` §6's method syntax, resolved by exact receiver.
    ///
    /// Three steps and no search. The receiver is inferred, the head of its
    /// type is read, and the traits that declare a method spelled `m` are
    /// intersected with the ones that have a dictionary at that head. Exactly
    /// one survivor is the call; none and two are the two refusals §6 names.
    ///
    /// What makes this a lookup rather than a search is that both operands are
    /// tables: [`Classes::declaring_method`](crate::class::Classes::declaring_method)
    /// is an index read and the dictionary test is the same keyed read §4 step 2
    /// already was. Nothing is tried and undone, so nothing can be tried in a
    /// different order and answer differently.
    ///
    /// The survivor is then elaborated as if the author had written
    /// `Class.m(x)`: the same [`method_at`](crate::dictionary::method_at) a
    /// qualified name goes through, applied to the receiver already in hand.
    /// That is what makes "a method is a spelling" true of the elaboration and
    /// not only of the prose — `x.m(y)` and `Class.m(x, y)` are one term.
    fn method(&mut self, scope: &Scope, here: Origin, receiver: &Raw, method: &Name) -> Result<Typed, ElabError> {
        let inferred = self.infer(scope, receiver)?;
        let unfolded = opened(&mut self.meter, &inferred.ty)?;
        let receiver_ty = unfolded.as_ref().unwrap_or(&inferred.ty);
        let stated = scope.quote_type(&mut self.meter, receiver_ty)?;
        // A local head is §6's generic parameter and `None` is a type with no
        // name at its head. Neither is a key, and the message is the same
        // because the repair is: write the trait.
        let Some(Head::Rigid(head)) = head_of(&stated, scope.depth()) else {
            return Err(Refusal::MethodOnVariable {
                at: here,
                method: Arc::clone(method),
            }
            .into());
        };
        let classes = scope.cx().classes().clone();
        let candidates: Vec<Name> = classes
            .declaring_method(method)
            .iter()
            .filter(|class| {
                let key = Key::rigid(class, &head);
                scope.discharged(&key).is_some() || classes.instance(&key).is_some()
            })
            .map(Arc::clone)
            .collect();
        let [class] = candidates.as_slice() else {
            return Err(if candidates.is_empty() {
                Refusal::NoMethodForType {
                    at: here,
                    head,
                    method: Arc::clone(method),
                }
            } else {
                Refusal::AmbiguousMethod {
                    at: here,
                    head,
                    method: Arc::clone(method),
                    classes: candidates,
                }
            }
            .into());
        };
        let qualified: Name = Arc::from(format!("{class}.{method}"));
        let Some((term, ty)) = crate::dictionary::method_at(self, scope, here, &qualified)? else {
            return Err(Refusal::NoMethodForType {
                at: here,
                head,
                method: Arc::clone(method),
            }
            .into());
        };
        self.receiving(scope, here, Typed { term, ty }, &inferred)
    }

    /// A method applied to the receiver it was found for.
    ///
    /// The receiver is elaborated already, so this is [`Self::application`]
    /// with its argument arriving as a term rather than as syntax — and it is a
    /// separate function rather than a parameter on that one because the two
    /// differ in what they do with the domain: there the argument is *checked*
    /// against it, here the two types are unified, which is what solves the
    /// trait arguments `method_at` left as metavariables.
    fn receiving(
        &mut self,
        scope: &Scope,
        here: Origin,
        function: Typed,
        receiver: &Typed,
    ) -> Result<Typed, ElabError> {
        let mut walk = Walk::default();
        let mut ty = function.ty.clone();
        self.advance(scope, &mut ty, &mut walk)?;
        let unfolded = opened(&mut self.meter, &ty)?;
        let function_ty = unfolded.as_ref().unwrap_or(&ty);
        let Form::Pi { domain, codomain, .. } = &function_ty.form else {
            return Err(Refusal::NotAFunction {
                at: here,
                ty: scope.quote_type(&mut self.meter, &ty)?,
            }
            .into());
        };
        let (domain, codomain) = (Arc::clone(domain), codomain.clone());
        // Matching mode: the receiver is where the method's parameters are
        // learned — `xs.fold_from_end` reads `A` off `xs`.
        self.unifier
            .unify_types(&mut self.meter, scope.depth(), here, &domain, &receiver.ty)?;
        walk.slots.push(Slot::Argument(receiver.term.clone()));
        let value = scope.eval(&mut self.meter, &receiver.term)?;
        let ty = apply_closure(&mut self.meter, &codomain, value)?;
        Self::finish_walk(here, function.term, ty, &walk, Vec::new())
    }
}

/// The parameters a call to `head`, whose type is `ty`, must supply — in the
/// order they were declared.
///
/// Explicit binders only, and the implicit and constraint ones are skipped
/// rather than collected: neither is written at a call site, so neither is a
/// parameter an argument list can be measured against. The walk stops at the
/// first form that is not a Π, so a generic answer `A` contributes nothing: it
/// is a parameter's worth of nothing until a caller instantiates it, and a
/// caller that instantiates it to a function did not thereby leave an argument
/// out.
///
/// # Why the head decides how the type is read
///
/// §1.3's word is "declared", and there are two kinds of declaration.
///
/// A *source* definition writes a parameter list, and the names in it survive
/// into the binders — so a named binder is a declared parameter and
/// [`ARROW_BINDER`](crate::raw::ARROW_BINDER) is the mark of one that is not.
/// The distinction is load-bearing rather than cosmetic: `walked(items:
/// StaffItem) -> (Position<τ> → Result<…>)` declares one parameter and returns
/// a function, a two-parameter definition has the same type, and only the
/// names tell them apart. Without this the corpus would be refused for writing
/// down the functions it returns.
///
/// A *registered* signature has no such second reading. There is no surface
/// declaration beside it to disagree with, so the arrow it was built from **is**
/// its parameter list, names or none — which is `base.rs`'s law again, the
/// shape of the table being the host's to state. That is why `transpose(P8)` is
/// under-applied even though the host wrote its binders anonymously.
fn declared_parameters(head: &Term, ty: &Term) -> Vec<Name> {
    let registered = matches!(head.shape(), Shape::Builtin(_));
    let mut declared = Vec::new();
    let mut rest = ty;
    while let Shape::Pi {
        plicity,
        name,
        codomain,
        ..
    } = rest.shape()
    {
        if *plicity == Plicity::Explicit {
            if !registered && &**name == crate::raw::ARROW_BINDER {
                break;
            }
            declared.push(Arc::clone(name));
        }
        rest = codomain;
    }
    declared
}
