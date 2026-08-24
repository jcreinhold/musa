//! One instantiation walk: implicits filled, constraints noted, arguments placed.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::elaboration::raw::{Raw, RawField};
use crate::elaboration::refuse::{ElabError, Refusal};
use crate::kernel::eval::{apply_closure, apply_closure_read, opened};
use crate::kernel::origin::Origin;
use crate::kernel::scope::Scope;
use crate::kernel::term::{Definition, Filling, Name, Role, Shape, Term};
use crate::kernel::value::{Form, Value};

use super::{Elaborator, Typed};

impl Elaborator {
    /// §2's constructor rule in a checking position: `C a⃗ ⇐ N p⃗`.
    ///
    /// A bare case name resolves against the family the expected type names —
    /// the whole of what "a constructor is checked" means; [`case_named`]
    /// owns the two spellings and the one coincidence §1.3 allows. The
    /// family's parameters are no longer read off and applied here: they are
    /// the constructor's implicit binders, so the application pass matches
    /// them out of `ty` at the end, which is the same rule §2.1 states for
    /// every other call.
    /// `head` applied to one constructor parameter the expected type carried.
    ///
    /// The parameter is a *value*: it was read off the type, not written, so
    /// there is no [`Raw`] to elaborate and nothing to infer — the term is the
    /// value quoted back at the scope's depth. [`crate::kernel::quote::Mode::Keep`]
    /// because the type is the author's and any definition it folds stays
    /// folded in what they read back.
    pub(super) fn given(
        &mut self,
        scope: &Scope,
        here: Origin,
        head: Typed,
        param: &Value,
    ) -> Result<Typed, ElabError> {
        let unfolded = crate::kernel::eval::opened(&mut self.meter, &head.ty)?;
        let function_ty = unfolded.as_ref().unwrap_or(&head.ty);
        let Form::Pi { domain, codomain, .. } = &function_ty.form else {
            return Err(Refusal::NotAFunction {
                at: here,
                ty: scope.quote_type(&mut self.meter, &head.ty)?,
            }
            .into());
        };
        let (domain, codomain) = (Arc::clone(domain), codomain.clone());
        let term = crate::kernel::quote::quote(
            &mut self.meter,
            scope.depth(),
            crate::kernel::quote::Mode::Keep,
            &domain,
            param,
        )?;
        Ok(Typed {
            term: Term::app(here, head.term, term),
            ty: apply_closure(&mut self.meter, &codomain, param.clone())?,
        })
    }

    /// `head` applied to `count` fresh metas — a constructor's parameters when
    /// no expected type named the family. The fields solve them through §2.1's
    /// ordinary matching, and [`Elaborator::settled`] audits what they could
    /// not.
    pub(super) fn metas(
        &mut self,
        scope: &Scope,
        here: Origin,
        mut built: Typed,
        count: u32,
    ) -> Result<Typed, ElabError> {
        for _ in 0..count {
            let unfolded = crate::kernel::eval::opened(&mut self.meter, &built.ty)?;
            let function_ty = unfolded.as_ref().unwrap_or(&built.ty);
            let Form::Pi {
                name, domain, codomain, ..
            } = &function_ty.form
            else {
                return Err(Refusal::NotAFunction {
                    at: here,
                    ty: scope.quote_type(&mut self.meter, &built.ty)?,
                }
                .into());
            };
            let named = Some(Arc::clone(name));
            let (domain, codomain) = (Arc::clone(domain), codomain.clone());
            let unknown = self.fresh_meta(scope, here, &domain, named)?;
            built = Typed {
                term: Term::app(here, built.term, unknown.term),
                ty: apply_closure(&mut self.meter, &codomain, unknown.value)?,
            };
        }
        Ok(built)
    }

    /// `head` applied to fresh metas until its type is no longer a Π — a
    /// *family*'s parameters, where a record literal named the family and no
    /// expected type said what it stands at.
    ///
    /// [`Self::metas`] takes a count because a constructor's is known from the
    /// declaration; a family's is however many Π stand between it and the
    /// universe it lands in, and reading that off the type is the same walk
    /// without the bookkeeping. The fields solve what comes out, through §2.1's
    /// ordinary matching, and [`Elaborator::settled`] audits what they could
    /// not.
    pub(super) fn saturated(&mut self, scope: &Scope, here: Origin, mut built: Typed) -> Result<Typed, ElabError> {
        loop {
            let unfolded = opened(&mut self.meter, &built.ty)?;
            let Form::Pi {
                name, domain, codomain, ..
            } = &unfolded.as_ref().unwrap_or(&built.ty).form
            else {
                return Ok(built);
            };
            let named = Some(Arc::clone(name));
            let (domain, codomain) = (Arc::clone(domain), codomain.clone());
            let unknown = self.fresh_meta(scope, here, &domain, named)?;
            built = Typed {
                term: Term::app(here, built.term, unknown.term),
                ty: apply_closure(&mut self.meter, &codomain, unknown.value)?,
            };
        }
    }

    /// §2.1's instantiation pass: apply `head` to the written arguments, then
    /// match what remains against `expected` when the call is in a checking
    /// position.
    ///
    /// One left-to-right walk over the spine, and the discipline is the
    /// document's: an implicit parameter becomes a [meta](crate::kernel::meta::Meta)
    /// that the first argument to mention it solves; a constraint waits until
    /// the walk has said everything matching can say, and is then resolved
    /// once, by lookup, never postponed. The domain and the argument's written
    /// form together decide what happens to it, in three cases rather than
    /// two:
    ///
    /// - **Settled** — the argument is *checked* against it. So is an argument
    ///   that annotates its own binder, because checking is what makes the
    ///   annotation and the domain agree ([`Raw::annotates_its_binder`]).
    /// - **Still quantified, and the argument can be inferred** — the argument
    ///   teaches the parameter, and what it inferred is matched against the
    ///   domain.
    /// - **Still quantified, and the argument is a checking-only form that
    ///   describes nothing** — the argument is *deferred*. Nothing can infer a
    ///   bare `λ`, so a placeholder stands in the slot, the rest of the spine is
    ///   walked — which is what solves the domain — and the argument is checked
    ///   afterwards against the type it turned out to have. A λ that annotates
    ///   its own binder is not here: it says what its parameter is, so it is one
    ///   of the arguments the deferred ones are waiting for rather than one of
    ///   the ones waiting.
    ///
    /// **Why the queue prompt 153 built does not replace this.** §2.1 says an
    /// argument like that "is checked after the rest of the spine has
    /// constrained its slot", and adds that with a real constraint queue this
    /// needs no special discipline. The first sentence is what this walk does;
    /// the second is true of everything a *comparison* can carry and false of
    /// the thing that actually breaks. `01-surface.md` §1.5 names the breakage
    /// in advance: a method resolves by exact receiver in one step, a receiver
    /// still undetermined after the spine walk is *refused* rather than
    /// postponed, and "§2.1's two-pass spine is what now makes a receiver's
    /// type known in the cases that used to need it". Elaborating
    /// `fn (p) { p.act(P8) }` against an undetermined domain binds `p` at an
    /// unknown, and a receiver at an unknown has no method — "no method" is a
    /// name that did not resolve rather than a constraint waiting on a
    /// solution. There is nothing to postpone. The host's index-acceptance
    /// rule, which reads two *literal* indices or does not apply, is the same
    /// shape. What has to wait is therefore the argument's **elaboration**,
    /// not its constraints, and that is what this is. Deleting the deferral
    /// turns 57 of `musa-compiler`'s laws red;
    /// `argument_order_laws::an_un_annotated_lambda_is_typed_by_an_argument_written_after_it`
    /// and `quotation_laws::a_spliced_value_arrives_where_the_splice_stood` are
    /// the two that name the mechanism, and prompt 153's repair records it.
    ///
    /// Deferral is not postponement. Each argument is elaborated exactly once,
    /// there is no queue and no retry, and both the deferred set and the order
    /// they are revisited in are fixed by the written argument order, so the
    /// answer cannot depend on which branch ran first. A domain that is *still*
    /// unsolved when the walk ends is checked against anyway, and prompt 153's
    /// η-expansion is what that now means: a λ read against an unknown makes it
    /// a function type rather than refusing.
    ///
    /// The emitted term is unaffected: the arguments are elaborated in one
    /// order and the spine is built in the written one, so evaluation order is
    /// exactly what was written.
    pub(super) fn apply_spine(
        &mut self,
        scope: &Scope,
        here: Origin,
        head: Typed,
        arguments: &[&Raw],
        expected: Option<&Value>,
    ) -> Result<Typed, ElabError> {
        self.supplied_spine(scope, here, head, arguments, &mut Supplied::none(), expected)
    }

    /// [`Self::apply_spine`] for a call that also supplies type parameters by
    /// name — `01-surface.md` §1's `{ IDENT = expr }` argument form.
    ///
    /// `supplied` is threaded rather than consumed up front because a named
    /// argument is placed *where its binder stands*, which only the walk knows:
    /// the binder's domain may mention the parameters before it, so the
    /// argument cannot be elaborated until the walk reaches it. What is left
    /// unplaced when the walk ends is the caller's to refuse, since naming the
    /// callee is the caller's knowledge and not the walk's.
    pub(super) fn supplied_spine(
        &mut self,
        scope: &Scope,
        here: Origin,
        head: Typed,
        arguments: &[&Raw],
        supplied: &mut Supplied<'_>,
        expected: Option<&Value>,
    ) -> Result<Typed, ElabError> {
        // A *bare* constructor reference — no written fields — has nothing
        // for its family parameters to be learned from, so each becomes a
        // meta (§2.1): `None` is `None<?>` wherever it stands, and the slot it
        // is checked against solves the meta by ordinary first-order matching.
        // Without this a bare constructor at an undetermined slot would lend
        // the slot its Π-scheme, and the program that then drew a value from
        // the slot would meet a function type where its data was. A written
        // field does the same job later in the walk, so this is the
        // no-arguments case only.
        let bare = match head.term.shape() {
            Shape::Named {
                name,
                role: role @ Role::Constructor,
                levels,
            } if arguments.is_empty() => match scope.cx().globals().definition(name, role, levels) {
                Definition::Declared(constant) => Some(constant.group.params()),
                Definition::Undeclared
                | Definition::Defined(_)
                | Definition::Compiled(_)
                | Definition::Base(_)
                | Definition::Builtin(_) => None,
            },
            Shape::Named { .. }
            | Shape::Meta(_)
            | Shape::Var(_)
            | Shape::Lit(_)
            | Shape::Universe(_)
            | Shape::Bind { .. }
            | Shape::App { .. } => None,
        };
        let head = match bare {
            Some(params) => self.metas(scope, here, head, params)?,
            None => head,
        };
        let mut walk = Walk::default();
        let mut waiting: Vec<Waiting<'_>> = Vec::new();
        let mut ty = head.ty.clone();
        // The stopping rule applies to the *last* advance only: a binder with a
        // written argument still to its right is one the walk has to get past,
        // whatever the call is checked against.
        let ending = Keeping::at_the_end(expected);
        let mut left = arguments.len();
        self.advance(scope, &mut ty, &mut walk, supplied, Keeping::after(left, ending))?;
        for argument in arguments {
            let unfolded = opened(&mut self.meter, &ty)?;
            let current = unfolded.as_ref().unwrap_or(&ty);
            let Form::Pi {
                name, domain, codomain, ..
            } = &current.form
            else {
                return Err(Refusal::NotAFunction {
                    at: here,
                    ty: scope.quote_type(&mut self.meter, &ty)?,
                }
                .into());
            };
            let named = Some(Arc::clone(name));
            let (domain, codomain) = (Arc::clone(domain), codomain.clone());
            let at = argument.origin();
            let (slot, value) = if !crate::kernel::unify::mentions_unsolved(&domain)
                || (argument.checks_only() && argument.annotates_its_binder())
            {
                // Checked: the argument is read against the domain, either
                // because the domain is settled or because the argument
                // annotates its own binder and checking is what makes the two
                // agree.
                let term = self.check(scope, argument, &domain)?;
                (Slot::Argument(term.clone()), Reading::Written(term))
            } else if argument.checks_only() {
                // Deferred: a placeholder holds the slot so the rest of the
                // walk can proceed, and the argument is checked below against
                // whatever the rest of the walk made the domain be. The
                // placeholder is an ordinary meta, so a codomain that reads the
                // argument's *value* gets a value to read, and anything the
                // walk learns about it is a solution the second pass then
                // agrees with rather than overwrites.
                let unknown = self.fresh_meta(scope, at, &domain, named)?;
                waiting.push(Waiting {
                    argument,
                    domain: Arc::clone(&domain),
                    meta: unknown.meta,
                    at,
                });
                (Slot::Deferred(unknown.term), Reading::Ready(unknown.value))
            } else {
                // Inferred — but an inferred head can still quantify over
                // parameters the domain determines (`identity` used unapplied):
                // the empty walk peels those into metas and does the matching,
                // which is §2.1's one rule rather than a second path here.
                let inferred = self.infer(scope, argument)?;
                let term = self
                    .apply_spine(scope, argument.origin(), inferred, &[], Some(&domain))?
                    .term;
                (Slot::Argument(term.clone()), Reading::Written(term))
            };
            walk.slots.push(slot);
            // The walk needs the *codomain* here, and the argument's value only
            // where the codomain reads it: see [`apply_closure_read`].
            ty = apply_closure_read(&mut self.meter, &codomain, |meter| value.read(scope, meter))?;
            left = left.saturating_sub(1);
            self.advance(scope, &mut ty, &mut walk, supplied, Keeping::after(left, ending))?;
        }
        if let Some(expected) = expected {
            // Checking position: the rest of the type is matched against what
            // the position wants, which is where a bare constructor's family
            // parameters — and any argument's still-unsolved ones — are
            // learned. The position is the mismatch's *expected*: it is the
            // type the author wrote and the walked type the one found.
            //
            // Before the deferred arguments and not after, because this is the
            // last thing that can teach a domain anything: the expected type is
            // where `fold(combine, seed, xs)`'s result type comes from when the
            // seed is the lambda. Idris2 checks the rest of the spine including
            // its target for the same reason.
            self.conversion
                .unify_types(&mut self.meter, scope.depth(), here, expected, &ty)?;
        }
        // The second pass, in written order. `domain` is the same value the
        // walk skipped, and it needs no re-derivation: a meta is shared, so a
        // domain the walk solved is already solved here.
        let mut deferred = Vec::with_capacity(waiting.len());
        for Waiting {
            argument,
            domain,
            meta,
            at,
        } in waiting
        {
            let term = self.check(scope, argument, &domain)?;
            let value = scope.eval(&mut self.meter, &term)?;
            let (_, stood) = Self::occurrence(scope, &meta, at)?;
            // Assignment when the placeholder is still free, conversion when
            // the walk already decided what stood there — one call, because
            // those are the same procedure (see [`crate::elaboration::convert`]).
            self.conversion
                .unify(&mut self.meter, scope.depth(), at, &domain, &stood, &value)?;
            deferred.push(term);
        }
        Ok(Self::finish_walk(here, head.term, ty, &walk, deferred))
    }

    /// Skip the binders §2.1 fills rather than the author: an implicit
    /// parameter becomes a fresh meta, a constraint is noted for the walk's
    /// end.
    ///
    /// `keeping` is prompt 154's stopping rule, and it is why insertion is not
    /// unconditional. A value used *as a value* at a type that quantifies the
    /// same way keeps its own scheme: `identity` checked at `{X} → X → X` is
    /// `identity`, not `λX. identity X`. Without it, an implicit the term's
    /// type carries and its body cannot determine is refused rather than
    /// matched — `{A} → {B} → A → A` at its own type reports that `B` could not
    /// be determined, because the λ that was wrapped around the term took the
    /// expected `B` away before the inserted `?B` could meet it.
    pub(super) fn advance(
        &mut self,
        scope: &Scope,
        ty: &mut Value,
        walk: &mut Walk,
        supplied: &mut Supplied<'_>,
        keeping: Keeping,
    ) -> Result<(), ElabError> {
        loop {
            let unfolded = opened(&mut self.meter, ty)?;
            let current = unfolded.as_ref().unwrap_or(ty);
            let Form::Pi {
                filling,
                name,
                domain,
                codomain,
            } = &current.form
            else {
                return Ok(());
            };
            // Recorded before the arms decide anything, so a refusal can list
            // the names the callee bears whichever way the walk then leaves.
            if *filling == Filling::Parameter {
                supplied.bears(name);
            }
            match filling {
                Filling::Written => return Ok(()),
                // A written argument first, and before the stopping rule:
                // keeping the scheme is what the *elaborator* does with a
                // binder nobody spoke for, and this author spoke for it.
                Filling::Parameter if supplied.holds(name) => {
                    let Some(written) = supplied.take(name) else {
                        return Ok(());
                    };
                    let (domain, codomain) = (Arc::clone(domain), codomain.clone());
                    let term = self.check(scope, written, &domain)?;
                    walk.slots.push(Slot::Parameter(term.clone()));
                    *ty = apply_closure_read(&mut self.meter, &codomain, |meter| {
                        scope.eval(meter, &term).map_err(ElabError::from)
                    })?;
                }
                Filling::Parameter if keeping == Keeping::TheScheme => return Ok(()),
                Filling::Parameter => {
                    let named = Some(Arc::clone(name));
                    let (domain, codomain, origin) = (Arc::clone(domain), codomain.clone(), current.origin);
                    let unknown = self.fresh_meta(scope, origin, &domain, named)?;
                    walk.slots.push(Slot::Parameter(unknown.term));
                    *ty = apply_closure(&mut self.meter, &codomain, unknown.value)?;
                }
                Filling::Constraint(constraint) => {
                    let constraint = Arc::clone(constraint);
                    let (domain, codomain, origin) = (Arc::clone(domain), codomain.clone(), current.origin);
                    // The codomain reads the evidence off its binder; a meta
                    // stands for it, and [`Self::settled`] writes the computed
                    // evidence in — the one place a constraint is answered.
                    let unknown = self.fresh_meta(scope, origin, &domain, None)?;
                    self.constraints
                        .push((constraint, scope.clone(), codomain.env.clone(), origin, unknown.meta));
                    walk.slots.push(Slot::Evidence(unknown.term));
                    *ty = apply_closure(&mut self.meter, &codomain, unknown.value)?;
                }
            }
        }
    }

    /// The walk's end: build the spine, and leave the residual type with the
    /// metas it still mentions — solved or not, which [`Self::settled`]
    /// audits.
    pub(super) fn finish_walk(here: Origin, head: Term, ty: Value, walk: &Walk, deferred: Vec<Term>) -> Typed {
        // Consumed in order, which is what makes the emitted spine the written
        // one: the deferred slots stand in `walk.slots` in the order they were
        // written, and the second pass checked them in that same order.
        let mut deferred = deferred.into_iter();
        let mut term = head;
        for slot in &walk.slots {
            let argument = match slot {
                Slot::Parameter(term) | Slot::Evidence(term) | Slot::Argument(term) => term.clone(),
                // The placeholder is the fallback rather than a panic because
                // it is a *correct* term: the second pass solved it to the
                // argument's value, so a spine built from it says the same
                // thing with the argument read back instead of as written.
                Slot::Deferred(placeholder) => deferred.next().unwrap_or_else(|| placeholder.clone()),
            };
            term = Term::app(here, term, argument);
        }
        Typed { term, ty }
    }
}

/// The type parameters a call supplies by name, and which of them the walk has
/// placed.
///
/// A list rather than a map: a call writes one or two of these, the callee's
/// telescope is walked once, and a hash of two entries costs more than the
/// scan. The `placed` flags are kept beside the fields rather than removing
/// entries, so [`Self::unplaced`] can report the *first written* leftover
/// rather than whichever one a removal happened to leave behind.
pub(super) struct Supplied<'raw> {
    /// The named arguments, in the order written.
    fields: &'raw [RawField],
    /// Whether each has been placed at its binder yet.
    placed: Vec<bool>,
    /// Every inferred binder the walk has met, in order — the list a refusal
    /// shows the author when a name matches none of them.
    borne: Vec<Name>,
}

impl<'raw> Supplied<'raw> {
    /// A call that supplies nothing, which is every call but the new form.
    pub(super) const fn none() -> Self {
        Self {
            fields: &[],
            placed: Vec::new(),
            borne: Vec::new(),
        }
    }

    /// The named arguments of one call.
    pub(super) fn of(fields: &'raw [RawField]) -> Self {
        Self {
            fields,
            placed: vec![false; fields.len()],
            borne: Vec::new(),
        }
    }

    /// Note that the callee's telescope bears `name`.
    fn bears(&mut self, name: &Name) {
        self.borne.push(Arc::clone(name));
    }

    /// Whether an unplaced argument names this binder.
    fn holds(&self, name: &Name) -> bool {
        self.fields
            .iter()
            .zip(&self.placed)
            .any(|(field, placed)| !placed && field.name == *name)
    }

    /// The unplaced argument for `name`, marked placed.
    fn take(&mut self, name: &Name) -> Option<&'raw Raw> {
        let at = self
            .fields
            .iter()
            .zip(&self.placed)
            .position(|(field, placed)| !placed && field.name == *name)?;
        *self.placed.get_mut(at)? = true;
        self.fields.get(at).map(|field| &field.term)
    }

    /// The first named argument the walk never placed, and the binders it
    /// could have named.
    pub(super) fn unplaced(&self) -> Option<(&'raw RawField, Vec<Name>)> {
        let at = self.placed.iter().position(|placed| !placed)?;
        self.fields.get(at).map(|field| (field, self.borne.clone()))
    }
}

/// Whether [`Elaborator::advance`] fills an inferred binder or leaves it.
///
/// Prompt 154's stopping rule, as a type rather than a `bool` because the two
/// answers are asymmetric and a caller reading `false` cannot tell which one it
/// asked for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Keeping {
    /// Fill every inferred binder met, which is what an application does.
    Nothing,
    /// Leave the inferred binders standing: the position this call is checked
    /// against quantifies the same way, so the scheme is the answer.
    TheScheme,
}

impl Keeping {
    /// What the *last* advance of a walk checked against `expected` does.
    ///
    /// The rule reads the expected type and nothing else: a call whose result
    /// is asked for at an inferred Π is a call whose inferred binders the
    /// position wants back.
    fn at_the_end(expected: Option<&Value>) -> Self {
        match expected.map(|ty| &ty.form) {
            Some(Form::Pi {
                filling: Filling::Parameter,
                ..
            }) => Self::TheScheme,
            _ => Self::Nothing,
        }
    }

    /// `ending` when no written argument is left, and [`Self::Nothing`] while
    /// any still is.
    const fn after(left: usize, ending: Self) -> Self {
        if left == 0 { ending } else { Self::Nothing }
    }
}

/// One instantiation pass's local state — see [`Elaborator::apply_spine`].
///
/// Just the spine: constraints are the elaborator's business, resolved at
/// declaration end, and an argument's term is pushed as it is elaborated.
#[derive(Default)]
pub(super) struct Walk {
    /// The spine slots to fill when the walk ends, in walk order.
    pub(super) slots: Vec<Slot>,
}

/// One argument [`Elaborator::apply_spine`] skipped, and where to put it back.
///
/// Not a postponed constraint: it is one argument, one slot, and one pass over
/// the list, all of it inside the call that made it. Nothing outlives the spine.
struct Waiting<'raw> {
    /// The argument, still unelaborated.
    argument: &'raw Raw,
    /// The domain it will be checked against — the same value, whose metas the
    /// rest of the walk may since have solved.
    domain: Arc<Value>,
    /// The placeholder that stood in the slot, so the checked argument can be
    /// made to agree with anything the walk decided about it.
    meta: crate::kernel::meta::Meta,
    /// Where the argument was written, for the placeholder and the agreement.
    at: Origin,
}

/// One spine slot of a [`Walk`].
///
/// Every arm holds a *term*, including the three that stand at an unknown: an
/// unknown's occurrence is itself applied to its whole scope
/// (`kernel::meta`), so there is no longer a spelling of it short enough to
/// rebuild from the meta alone, and rebuilding it would be the second copy
/// [`Elaborator::fresh_meta`] exists to prevent.
/// What the walk has for an argument, for the one thing it needs it for.
///
/// The walk pushes each argument's value into the codomain and reads the type
/// that comes back. A written argument arrives as a *term*, and evaluating one
/// costs the whole subterm — so it is held unevaluated here and evaluated by
/// [`apply_closure_read`] only where the codomain can read it. An implicit the
/// walk minted has its value already and nothing is saved by delaying it.
enum Reading {
    /// An argument elaborated to a term, not yet evaluated.
    Written(Term),
    /// A value the walk already has.
    Ready(Value),
}

impl Reading {
    /// The value, evaluating the term if that is what this is.
    ///
    /// # Errors
    ///
    /// As [`Scope::eval`].
    fn read(self, scope: &Scope, meter: &mut crate::kernel::budget::Meter) -> Result<Value, ElabError> {
        match self {
            Self::Written(term) => scope.eval(meter, &term).map_err(ElabError::from),
            Self::Ready(value) => Ok(value),
        }
    }
}

pub(super) enum Slot {
    /// An implicit parameter: the unknown stands in the term whether or not the
    /// walk solved it, and [`Elaborator::settled`] audits at declaration end.
    Parameter(Term),
    /// A constraint's evidence, computed at declaration end.
    Evidence(Term),
    /// An argument the author wrote, elaborated.
    Argument(Term),
    /// An argument the walk deferred, standing at the placeholder that held
    /// its slot. [`Elaborator::finish_walk`] fills it from the second pass.
    Deferred(Term),
}
