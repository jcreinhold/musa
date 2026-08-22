//! The inference judgment: the forms that compute a type.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::elaboration::namespace;
use crate::elaboration::raw::{Raw, RawShape};
use crate::elaboration::refuse::{ElabError, Refusal};
use crate::kernel::eval::{apply_closure, eval, opened};
use crate::kernel::origin::Origin;
use crate::kernel::scope::Scope;
use crate::kernel::sort::Sort;
use crate::kernel::term::{Binder, Filling, Name, Role, Shape, Term};
use crate::kernel::value::{Env, Form, Value};

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
                ty: eval(&mut self.meter, &Env::under(scope.cx().globals().clone()), literal.ty())?,
            }),
            RawShape::Numeral { family, count } => self.numeral(scope, here, family, *count),
            RawShape::Universe(written) => {
                // §1: two universes, and a bare `Type` is `Type 0`.
                let level = written.as_ref().copied().unwrap_or(Sort::ZERO);
                let Some(above) = level.succ() else {
                    return Err(Refusal::BeyondUniverses { at: here }.into());
                };
                Ok(Typed {
                    ty: Value::new(here, Form::Universe(above)),
                    term: Term::universe(here, level),
                })
            }
            RawShape::Pi {
                filling,
                name,
                domain,
                codomain,
            } => self.function_type(scope, here, filling.clone(), name, domain, codomain),
            RawShape::Indexed { ty, index } => self.indexed_type_formation(scope, here, ty, index),
            RawShape::Lam {
                filling,
                name,
                domain,
                body,
            } => self.infer_lambda(scope, here, filling.clone(), name, domain.as_ref(), body),
            RawShape::App {
                filling,
                function,
                argument,
            } => match self.constructed_open(scope, raw)? {
                Some(built) => Ok(built),
                None => self.application(scope, here, filling, function, argument),
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
            // conversion checker's forbidden habit of trying the solution that comes to
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

    /// `T(i) ⇒ Type l` — a type refined by an index (§1.5).
    ///
    /// The refined type carries the whole thing's universe, because the
    /// index adds no inhabitants and no size: `Row(12)` is `Row` under a
    /// wrapper that quotation drops, so a universe read off the index would be
    /// reading a level off something erased.
    ///
    /// **§1.5's grammar is checked here**, which is where §1.5 sites it: an
    /// index position holding anything else "is the refusal below, named at the
    /// expression". Prompt 142d checked it at the comparison instead, and the
    /// cost was that `≡` stopped being reflexive — two indices nothing can read
    /// were answered "different", so `Row(mystery n)` was not the same type as
    /// itself and a term could fail to check against its own type.
    ///
    /// It is still read once. [`crate::elaboration::convert::reads_as_index`] is conversion's
    /// own reader, asked one stage earlier rather than reimplemented, so there
    /// is one grammar rather than two obliged to agree.
    fn indexed_type_formation(
        &mut self,
        scope: &Scope,
        here: Origin,
        ty: &Raw,
        index: &Raw,
    ) -> Result<Typed, ElabError> {
        // The *head*, so §1.5's arity check is not asked of a type whose index
        // is one line away — [`Elaborator::formed_type`] is `check_type` without
        // it, and this is its one caller.
        let (ty, level) = self.formed_type(scope, ty)?;
        // §1.5's other half of the arity check: an index written on a type that
        // declares none. `Nat(12)` is refused here, and it has to be refused
        // somewhere — a form nothing declares is a form nothing can be erased
        // from, so admitting it would leave a type whose read-back is a
        // different type for no reason anything recorded.
        let Some(binder) = ty.declared_index(scope.cx().globals()) else {
            return Err(Refusal::NotIndexed {
                ty: crate::elaboration::show::head_spelled(&ty),
                at: here,
            }
            .into());
        };
        // **Checked, never inferred, at the sort the head declares.** This is
        // the bidirectional rule the rest of the elaborator follows, and the
        // reason it matters here is `Pc(3/4)`: inferring answers "an exact
        // fraction" and has nothing to compare that against, so a type declared
        // over whole numbers silently carries a rational and two use sites that
        // disagree about what `Pc` counts are both accepted.
        //
        // The sort is evaluated under this scope even though it was elaborated
        // closed, which is sound because §1.5's sorts *are* closed —
        // [`crate::elaboration::declare`]'s check on the telescope is what guarantees it.
        let sort = scope.eval(&mut self.meter, &binder.ty)?;
        let index = Typed {
            term: self.check(scope, index, &sort)?,
            ty: sort,
        };
        // §1.5's grammar is checked *here*, where the type is formed and the
        // expression that broke it is still on the page. Two things follow, and
        // both are the reason the check is not left to the comparison:
        //
        // - the message names `Row(f x)`, which is what §1.5 asks for and what
        //   a comparison could not produce — by then there are two types and no
        //   written expression;
        // - an unreadable index never reaches [`crate::elaboration::convert`], so
        //   `index::decide` is total on what does and a type is convertible with
        //   itself. Refusing at the comparison instead answers "different" for
        //   two indices nothing can read, which is not an answer about them.
        //
        // The reader is `convert`'s own, asked one stage earlier rather than
        // reimplemented. A second reader would be two grammars obliged to agree.
        let value = scope.eval(&mut self.meter, &index.term)?;
        // A meta is §2.1's question and not this one: an index still mentioning
        // one has not been determined by the call yet, and §2.1 already refuses
        // an index variable no written argument determines. Refusing it here
        // would be postponement's mirror image — a complaint raised before the
        // information that answers it arrives.
        if !crate::elaboration::convert::mentions_unsolved(&value)
            && !crate::elaboration::convert::reads_as_index(&mut self.meter, scope.cx().globals(), &value)?
        {
            return Err(Refusal::UnreadableIndex {
                shown: crate::elaboration::show::spelled(&index.term),
                at: index.term.origin(),
            }
            .into());
        }
        Ok(Typed {
            term: Term::indexed(here, ty, index.term),
            ty: Value::new(here, Form::Universe(level)),
        })
    }

    /// `(x : A) → B ⇒ Type (max l l')`.
    fn function_type(
        &mut self,
        scope: &Scope,
        here: Origin,
        filling: Filling,
        name: &Name,
        domain: &Raw,
        codomain: &Raw,
    ) -> Result<Typed, ElabError> {
        let (domain_term, domain_level) = self.check_type(scope, domain)?;
        let domain_value = scope.eval(&mut self.meter, &domain_term)?;
        let inner = scope.assume(Some(Arc::clone(name)), here, Arc::new(domain_value));
        let (codomain_term, codomain_level) = self.check_type(&inner, codomain)?;
        Ok(Typed {
            term: Term::function(here, filling, Arc::clone(name), domain_term, codomain_term),
            ty: Value::new(here, Form::Universe(domain_level.max(codomain_level))),
        })
    }

    /// `λx. e ⇒ (x : A) → B`, where `A` is the annotation §2 asks for.
    fn infer_lambda(
        &mut self,
        scope: &Scope,
        here: Origin,
        filling: Filling,
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
                // annotated. No meta stands here, because nothing downstream
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
            &Term::function(here, filling, Arc::clone(name), domain_term, codomain),
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
                function: crate::elaboration::show::head_spelled(&head.term),
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
        filling: &Filling,
        function: &Raw,
        argument: &Raw,
    ) -> Result<Typed, ElabError> {
        let inferred = self.infer(scope, function)?;
        if *filling == Filling::Parameter {
            // The one written-implicit rule: the argument fills the next
            // binder when that binder is the implicit one — the spelling the
            // host's schemes and the suite's fixtures use for a type argument
            // a use site writes out — and is refused otherwise, which is the
            // refusal's own sentence.
            let unfolded = crate::kernel::eval::opened(&mut self.meter, &inferred.ty)?;
            let ty = unfolded.as_ref().unwrap_or(&inferred.ty);
            let Form::Pi {
                filling: Filling::Parameter,
                domain,
                codomain,
                ..
            } = &ty.form
            else {
                return Err(Refusal::FillingMismatch { at: argument.origin() }.into());
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

    /// `x.m` — `01-surface.md` §1.5's method syntax, resolved by exact receiver.
    ///
    /// Two steps and no search. The receiver is inferred, the rigid head of its
    /// type is read, and `Head.m` is looked up among the definitions in scope.
    /// A hit is the call; a miss is [`Refusal::NoMethodForType`] naming the type
    /// and the member, and a receiver whose type has no rigid head at all is
    /// [`Refusal::MethodOnVariable`].
    ///
    /// What makes this a lookup rather than a search is that there is one
    /// candidate by construction: the head names the namespace, the namespace
    /// and the member spell one name, and a name resolves to one definition.
    /// Nothing is tried and undone, so nothing can be tried in a different
    /// order and answer differently.
    ///
    /// The hit is then elaborated as if the author had written `Head::m(x)`:
    /// the same definition a qualified path reaches, applied to the receiver
    /// already in hand. That is what makes "a method is a spelling" true of the
    /// elaboration and not only of the prose — `x.m(y)` and `Head::m(x, y)` are
    /// one term.
    fn method(&mut self, scope: &Scope, here: Origin, receiver: &Raw, method: &Name) -> Result<Typed, ElabError> {
        let inferred = self.infer(scope, receiver)?;
        let unfolded = opened(&mut self.meter, &inferred.ty)?;
        let receiver_ty = unfolded.as_ref().unwrap_or(&inferred.ty);
        let stated = scope.quote_type(&mut self.meter, receiver_ty)?;
        // A variable head is §1.5's generic receiver and `None` is a type with
        // no name at its head. Neither names a namespace, and the message is
        // the same because the repair is: write the type.
        let Some(head) = namespace::head_name(&stated) else {
            return Err(Refusal::MethodOnVariable {
                at: here,
                method: Arc::clone(method),
            }
            .into());
        };
        let qualified = crate::elaboration::namespace::qualified(&head, method);
        let Some(found) = Self::namespaced(scope, here, &qualified)? else {
            return Err(Refusal::NoMethodForType {
                at: here,
                head,
                method: Arc::clone(method),
            }
            .into());
        };
        self.receiving(scope, here, found, &inferred)
    }

    /// The definition a namespaced name denotes, when one is in scope.
    ///
    /// The same visibility check every other name gets, for the same reason: a
    /// private `Pitch.act` is *private* rather than absent, and a diagnostic
    /// that said "no such member" would send its reader looking for a typo.
    /// No `self`: a namespaced name is *looked up* and never elaborated
    /// against, which is the whole claim §1.5 makes about the mechanism. A rule
    /// that needed the elaborator would need metas, and a meta here would be
    /// the trial elaboration this design does not have.
    pub(super) fn namespaced(scope: &Scope, here: Origin, qualified: &Name) -> Result<Option<Typed>, ElabError> {
        let Some(defined) = scope.cx().definition(qualified) else {
            return Ok(None);
        };
        if let Some(module) = defined.hidden_from(scope.cx().module()) {
            return Err(Refusal::Private {
                name: Arc::clone(qualified),
                module,
                at: here,
            }
            .into());
        }
        let def = crate::kernel::program::one(defined);
        let ty = Value::clone(&def.ty());
        Ok(Some(Typed {
            term: def.term(here),
            ty,
        }))
    }

    /// A method applied to the receiver it was found for.
    ///
    /// The receiver is elaborated already, so this is [`Self::application`]
    /// with its argument arriving as a term rather than as syntax — and it is a
    /// separate function rather than a parameter on that one because the two
    /// differ in what they do with the domain: there the argument is *checked*
    /// against it, here the two types are unified, which is what solves the
    /// definition's own type parameters from the receiver.
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
        self.conversion
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
/// [`ARROW_BINDER`](crate::elaboration::raw::ARROW_BINDER) is the mark of one that is not.
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
    let registered = matches!(
        head.shape(),
        Shape::Named {
            role: Role::Builtin,
            ..
        }
    );
    let mut declared = Vec::new();
    let mut rest = ty;
    while let Shape::Bind {
        name,
        binder: Binder::Pi { filling, .. },
        body,
    } = rest.shape()
    {
        if *filling == Filling::Written {
            if !registered && &**name == crate::elaboration::raw::ARROW_BINDER {
                break;
            }
            declared.push(Arc::clone(name));
        }
        rest = body;
    }
    declared
}
