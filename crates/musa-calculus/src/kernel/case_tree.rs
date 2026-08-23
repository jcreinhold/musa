//! A compiled case tree: what a `match` is, before it is a term.
//!
//! `docs/rules/language/02-core-calculus.md` §6.2: "Surface `match` is compiled
//! to a **case tree** and then to nested applications of the generated
//! eliminators, which is where coverage is decided." Both halves are here — the
//! tree is [`CaseTree`], the emission is [`CaseTree::emitted`], and coverage is
//! [`CaseTree::uncovered`], asked of the *finished* tree rather than of the
//! builder's bookkeeping.
//!
//! # Why the tree is a value and not a shape of the recursion
//!
//! The builder in [`crate::elaboration::case`] already walked a tree; what it
//! did not do was ever hold one. Coverage was then a fact about the order the
//! builder happened to visit constructors in, which is the kind of invariant
//! that is true until someone reorders a loop. With the tree in hand it is
//! re-derived from the declaration group, and the two answers have to agree.
//!
//! It is also what prompt 157 needs: a record projection is "a generated
//! function whose body is a one-branch case tree" (§1.1), and a body is a value
//! somebody stores.
//!
//! # Three nodes
//!
//! [`CaseTree::Answer`] is a leaf, [`CaseTree::Split`] is a case analysis, and
//! [`CaseTree::Impossible`] is the branch index unification ruled out. Idris2's
//! `Core/Case/CaseTree.idr` carries a **fourth** node, for a `match` that falls
//! off the end with no alternative matching. Musa forbids exactly that case, so
//! there is no node for it and no name for one here: coverage cannot fail while
//! a program runs, because [`CaseTree::uncovered`] decides it before one does.
//!
//! `Impossible` has **no producer until prompt 156**. With no indices there is
//! nothing for unification to refute, so a tree this crate builds today holds
//! none, and emission reports one as a defect rather than inventing a term.
//!
//! # A tree is also a body, and then it reduces
//!
//! §1's `Definition` list says a name's reduction behaviour may be "a compiled
//! case tree". [`Compiled`] is that: the binders a definition abstracts, and the
//! tree beneath them. Given as many arguments as there are binders, [reduction]
//! builds the environment from them, forces the scrutinee, takes the
//! alternative its constructor names, and evaluates that answer — and a
//! scrutinee that is not canonical leaves the name neutral, which is what makes
//! a body behave like the eliminator it stands beside rather than like a runtime
//! `switch`.
//!
//! [reduction]: Compiled::reduce
//!
//! The binders ride *beside* the tree rather than around it because
//! [`Form::Lam`](crate::kernel::value::Form) holds a closure over a **`Term`**,
//! and a tree is not a term — §1's seven shapes have no case node, which is the
//! whole reason a tree lives in a `Definition` at all. Idris2 writes the same
//! pair as `PMDef args tree` (`Core/Context.idr`) for the same reason.
//!
//! An [`Alternative`] carries its fields and its hypotheses apart, and that is
//! load-bearing here. Emission binds both, because a method's type is
//! `(a⃗ : Fields) → (ih⃗) → R (c a⃗)` and the arms were elaborated under all of
//! them. Reduction binds the fields to what the constructor was applied to and
//! every hypothesis to a **placeholder**, because since prompt 155a retired the
//! `#ih` rewrite no body can name one: recursion in a tree body is the
//! definition's own name, not a bound hypothesis. That is `iota.rs`'s `unread`
//! argument with its question settled statically rather than per call, and the
//! placeholder is `Type 0` for `unread`'s reason — a universe standing where a
//! proof belongs is wrong in a way the next conversion says out loud.

use std::sync::Arc;

use crate::kernel::budget::Meter;
use crate::kernel::context::Globals;
use crate::kernel::error::{CoreError, Malformed};
use crate::kernel::family::{Constant, Group, constructed};
use crate::kernel::origin::Origin;
use crate::kernel::sort::{Sort, SortVar};
use crate::kernel::term::{Binder, Index, Name, Role, Shape, Term};
use crate::kernel::value::{Elim, Env, Form, Value};

/// A `match`, compiled.
pub(crate) enum CaseTree {
    /// The branch is this term, elaborated under the binders the splits above
    /// it introduced.
    Answer(Term),
    /// A case analysis. Boxed because a [`Split`] holds a whole telescope and
    /// most nodes are leaves.
    Split(Box<Split>),
    /// Index unification refuted this branch, so it has no body and needs none.
    ///
    /// **No producer until prompt 156**: with no indices there is nothing for
    /// unification to refute. Every walk over a tree answers for it anyway,
    /// because the arm that cannot happen is the one a reader checks last.
    Impossible,
}

/// One case analysis: what is analysed, and what each case answers.
pub(crate) struct Split {
    /// Where the `match` was written (§7).
    pub(crate) origin: Origin,
    /// The declaration group the analysed family belongs to.
    pub(crate) group: Arc<Group>,
    /// Which family of the group is analysed.
    pub(crate) family: u32,
    /// The family's parameters, as terms at the depth this split stands at.
    pub(crate) params: Arc<[Term]>,
    /// The **subject's** indices, as terms at the depth this split stands at.
    ///
    /// Not the family's index telescope, which is a list of binders: these are
    /// the arguments the subject's own type stood at, and they are what the
    /// eliminator is applied to just before the subject. Empty for a family
    /// with no indices.
    pub(crate) indices: Arc<[Term]>,
    /// One motive per family of the group, each `λ(i⃗). λ(t : N p⃗ i⃗). …`
    /// (§1.1).
    pub(crate) motives: Arc<[Term]>,
    /// The universe the motives land in, chosen per use site (§1.3).
    pub(crate) level: Sort,
    /// The subject, as a term at the depth this split stands at.
    pub(crate) on: Term,
    /// One alternative per constructor of **every** family of the group, in
    /// declaration order — the order the generated eliminator takes its methods
    /// in, so emission is a fold rather than a search.
    pub(crate) alternatives: Arc<[Alternative]>,
}

/// One constructor's case.
pub(crate) struct Alternative {
    /// The constructor this is the case for, qualified by its family.
    pub(crate) constructor: Name,
    /// One binder per field of the constructor, in declaration order.
    ///
    /// The names are for a reader of the emitted term; what a row's body may
    /// *write* is bound by the builder from that row's own pattern.
    pub(crate) fields: Arc<[Name]>,
    /// One binder per recursive field, standing after every field.
    ///
    /// Kept apart from [`Self::fields`] because the two are bound differently:
    /// emission binds both, since the arm was elaborated under a method's whole
    /// telescope, and reduction binds these to a placeholder because no body can
    /// name one. See the module documentation.
    pub(crate) hypotheses: Arc<[Name]>,
    /// What this case answers.
    pub(crate) body: CaseTree,
}

impl CaseTree {
    /// The first constructor this tree fails to analyse, if there is one.
    ///
    /// **Re-derived from the declaration group**, not from the builder: the
    /// question is "does the group have a constructor no alternative names",
    /// and the group is the only thing that can answer it. A builder that
    /// silently dropped a case, reordered its loops, or matched a name loosely
    /// is caught here rather than by the emitted term happening to type-check.
    pub(crate) fn uncovered(&self) -> Option<Name> {
        let Self::Split(split) = self else {
            return None;
        };
        for wanted in split.constructors() {
            if !split
                .alternatives
                .iter()
                .any(|alternative| *alternative.constructor == *wanted)
            {
                return Some(wanted);
            }
        }
        split
            .alternatives
            .iter()
            .find_map(|alternative| alternative.body.uncovered())
    }

    /// The tree as a term: nested applications of the generated eliminators
    /// (§6.2).
    ///
    /// An [`Self::Impossible`] node emits the **identity**, `λx. x`. That is not
    /// a placeholder: since prompt 156 the motive of a split that refuted a
    /// branch answers `Π(_ : G). G` off the subject's own index (see
    /// `elaboration::case`'s `filtered`), so the refuted method's type *is* an
    /// identity type, and the eliminator wants a term there like it wants one
    /// for every other constructor. The branch is unreachable, so which
    /// inhabitant it gets says nothing; that it has one is what lets §6.2's
    /// emission stay total.
    pub(crate) fn emitted(&self, origin: Origin) -> Result<Term, CoreError> {
        match self {
            Self::Answer(term) => Ok(term.clone()),
            Self::Impossible => Ok(Term::lam(origin, "impossible", Term::var(origin, Index(0)))),
            Self::Split(split) => split.emitted(),
        }
    }
}

impl CaseTree {
    /// This tree with its level parameters replaced — see
    /// [`Compiled::substitute_levels`].
    fn substitute_levels(&self, with: &impl Fn(&SortVar) -> Option<Sort>) -> Self {
        match self {
            Self::Answer(term) => Self::Answer(term.substitute_levels(with)),
            Self::Impossible => Self::Impossible,
            Self::Split(split) => Self::Split(Box::new(Split {
                origin: split.origin,
                group: Arc::clone(&split.group),
                family: split.family,
                params: split.params.iter().map(|term| term.substitute_levels(with)).collect(),
                indices: split.indices.iter().map(|term| term.substitute_levels(with)).collect(),
                motives: split.motives.iter().map(|term| term.substitute_levels(with)).collect(),
                level: split.level.substitute(with),
                on: split.on.substitute_levels(with),
                alternatives: split
                    .alternatives
                    .iter()
                    .map(|alternative| Alternative {
                        constructor: Arc::clone(&alternative.constructor),
                        fields: Arc::clone(&alternative.fields),
                        hypotheses: Arc::clone(&alternative.hypotheses),
                        body: alternative.body.substitute_levels(with),
                    })
                    .collect(),
            })),
        }
    }
}

impl Split {
    /// Every constructor of every family of the group, qualified, in the order
    /// the eliminator takes its methods in.
    fn constructors(&self) -> Vec<Name> {
        let mut wanted = Vec::new();
        for family in 0..self.group.arity() {
            let count = self
                .group
                .family_at(family)
                .map_or(0, |declared| declared.constructors.len());
            for which in 0..count {
                let which = u32::try_from(which).unwrap_or(u32::MAX);
                wanted.push(Constant::constructor(&self.group, family, which).name());
            }
        }
        wanted
    }

    /// `N.elim p⃗ motives methods… subject`.
    fn emitted(&self) -> Result<Term, CoreError> {
        let mut applied = Constant::recursor(&self.group, self.family, self.level.clone()).term(self.origin);
        for param in self.params.iter() {
            applied = Term::app(self.origin, applied, param.clone());
        }
        for motive in self.motives.iter() {
            applied = Term::app(self.origin, applied, motive.clone());
        }
        for alternative in self.alternatives.iter() {
            applied = Term::app(self.origin, applied, alternative.emitted(self.origin)?);
        }
        // The subject's indices stand between the methods and the subject,
        // which is the order the generated eliminator's own telescope puts them
        // in — see `family::constant`'s `recursor_type`.
        for index in self.indices.iter() {
            applied = Term::app(self.origin, applied, index.clone());
        }
        Ok(Term::app(self.origin, applied, self.on.clone()))
    }
}

impl Alternative {
    /// The method this alternative is: its body, under one λ per binder.
    fn emitted(&self, origin: Origin) -> Result<Term, CoreError> {
        let body = self.body.emitted(origin)?;
        Ok(self
            .fields
            .iter()
            .chain(self.hypotheses.iter())
            .rev()
            .fold(body, |built, name| Term::lam(origin, Arc::clone(name), built)))
    }
}

/// A definition's body: the binders it abstracts, and the tree beneath them.
///
/// §1's `Definition` list, second arm. See the module documentation for why the
/// binders ride beside the tree rather than around it.
pub(crate) struct Compiled {
    /// The definition's own arguments, outermost first.
    pub(crate) binders: Arc<[Name]>,
    /// What it answers, once it has them.
    pub(crate) tree: CaseTree,
}

impl Compiled {
    /// How many arguments this body needs before it can decide anything.
    pub(crate) fn arity(&self) -> usize {
        self.binders.len()
    }

    /// δ: reduce this body, named `name`, applied to `arguments`, or `None`
    /// when it stays blocked.
    ///
    /// Blocked for either of two reasons, and neither is an error: too few
    /// arguments have arrived, or the scrutinee of a split is not canonical. A
    /// definition applied to a variable is exactly as stuck as an eliminator
    /// applied to one, which is the property that keeps a body a *definition*
    /// rather than a runtime `switch`.
    ///
    /// **A tail call is a jump, and that is a measurement rather than a
    /// flourish.** §4.1's nesting metric bounds what the machine spends stack
    /// on, and a level is charged per evaluation. The generated eliminator this
    /// replaces spent *no* levels on recursion depth: ι computes a method's
    /// induction hypothesis before it enters the method, so the whole chain runs
    /// through `apply`, which nests nothing. A tree whose arm evaluated its own
    /// recursive call would spend one level per step, and
    /// `crates/musa-compiler`'s staff package — a fold `walk rest (step built
    /// first)`, which is the shape every forward fold in the language has —
    /// needed 272 of the 256 levels §4.1 fixes. Recognizing the saturated
    /// self-call at an arm's answer and looping instead of recursing spends no
    /// stack, so nothing is charged and the same programs compile as before.
    /// A recursion that is *not* in tail position still costs a level a step,
    /// which is exactly what its evaluation costs the machine.
    ///
    /// # Errors
    ///
    /// [`Malformed::UnreachableAlternative`] for an [`CaseTree::Impossible`]
    /// node reached by reduction, and otherwise as [`eval`].
    pub(crate) fn reduce(
        &self,
        meter: &mut Meter,
        globals: &Globals,
        here: Origin,
        name: &Name,
        arguments: &[Value],
    ) -> Result<Option<Value>, CoreError> {
        let arity = self.arity();
        let Some(taken) = arguments.get(..arity) else {
            return Ok(None);
        };
        let mut taken: Vec<Value> = taken.to_vec();
        let mut answer = loop {
            let env = taken
                .iter()
                .fold(Env::under(globals.clone()), |env, argument| env.push(argument.clone()));
            match descend(meter, &env, &self.tree, name, arity)? {
                None => return Ok(None),
                Some(Descended::Answered(answer)) => break answer,
                Some(Descended::Again(again)) => taken = again,
            }
        };
        // A definition may be applied to more than it abstracts — `f x y` where
        // `f` splits on `x` and answers a function. The tree decided at `x`; the
        // rest is ordinary application.
        for argument in arguments.get(arity..).unwrap_or_default() {
            answer = crate::kernel::eval::apply(meter, here, answer, argument.clone())?;
        }
        Ok(Some(answer))
    }

    /// This body with its level parameters replaced.
    ///
    /// A tree-bodied definition is generalized over the levels its type mentions
    /// exactly as any other is, so instantiation has to reach the terms the tree
    /// holds. It is a walk rather than a substitution on values for
    /// [`Defined`](crate::kernel::program::Defined)'s reason: a closure is not
    /// something a level substitution can see inside.
    pub(crate) fn substitute_levels(&self, with: &impl Fn(&SortVar) -> Option<Sort>) -> Self {
        Self {
            binders: Arc::clone(&self.binders),
            tree: self.tree.substitute_levels(with),
        }
    }
}

/// What walking the tree reached: an answer, or the next turn of a tail call.
enum Descended {
    /// The value the body answers with.
    Answered(Value),
    /// A saturated call to the same definition, and the arguments it passes.
    /// See [`Compiled::reduce`] for why this is not simply evaluated.
    Again(Vec<Value>),
}

/// Walk the tree in `env`, or `None` when a split is blocked.
fn descend(
    meter: &mut Meter,
    env: &Env,
    tree: &CaseTree,
    name: &Name,
    arity: usize,
) -> Result<Option<Descended>, CoreError> {
    match tree {
        CaseTree::Answer(term) => answered(meter, env, term, name, arity).map(Some),
        CaseTree::Impossible => Err(Malformed::UnreachableAlternative.into()),
        CaseTree::Split(split) => {
            // A split's subject is a variable by construction: `case.rs` names
            // anything else with a `let`, and a tree body refuses one that did.
            // Read out of the environment rather than evaluated, because §4.1's
            // nesting metric charges a level per `eval` and a chain of columns
            // would otherwise pay one per column for a lookup that nests
            // nothing. The generated eliminator paid nothing here either — it
            // reached its subject through `apply`, which is not a nesting level
            // — so this keeps the two reducers charging the same depth for the
            // same program.
            let on = if let Shape::Var(index) = split.on.shape() {
                env.get(index.0)
                    .cloned()
                    .ok_or_else(|| CoreError::from(Malformed::UnboundVariable(*index)))?
            } else {
                crate::kernel::eval::eval(meter, env, &split.on)?
            };
            let on = crate::kernel::eval::opened(meter, &on)?.unwrap_or(on);
            let Some((constructor, fields)) = analysed(&on) else {
                return Ok(None);
            };
            let Some(alternative) = split
                .alternatives
                .iter()
                .find(|alternative| *alternative.constructor == *constructor)
            else {
                return Ok(None);
            };
            let mut inner = env.clone();
            for field in &fields {
                inner = inner.push(field.clone());
            }
            // One placeholder per hypothesis binder. Nothing can name one — see
            // the module documentation — and the arm's de Bruijn indices were
            // read under them, so the slots have to be there.
            for _ in alternative.hypotheses.iter() {
                inner = inner.push(Value::new(on.origin, Form::Universe(Sort::ZERO)));
            }
            descend(meter, &inner, &alternative.body, name, arity)
        }
    }
}

/// An arm's answer: the value it evaluates to, or the tail call it *is*.
///
/// The `let`s are the ones `elaboration::case` wrapped the arm's pattern
/// bindings in. They are peeled without being evaluated first, because peeling
/// only decides whether the answer is a self-call; when it is not, the term is
/// evaluated whole, exactly as it would have been.
fn answered(meter: &mut Meter, env: &Env, term: &Term, name: &Name, arity: usize) -> Result<Descended, CoreError> {
    let mut bindings = 0_usize;
    let mut at = term;
    while let Shape::Bind {
        binder: Binder::Let { .. },
        body,
        ..
    } = at.shape()
    {
        bindings = bindings.saturating_add(1);
        at = body;
    }
    let (head, spine) = called(at);
    let jumps = matches!(
        head.shape(),
        Shape::Named {
            name: called,
            role: Role::Defined,
            ..
        } if called == name
    ) && spine.len() == arity;
    if !jumps {
        return Ok(Descended::Answered(crate::kernel::eval::eval(meter, env, term)?));
    }
    // The jump saves *stack*, and nothing else. Evaluating the answer whole
    // would have charged §4's step metric for every node this path walks past
    // instead of into — each `let`, each application in the spine, the head, and
    // one [`crate::kernel::eval::apply`] per argument — so those steps are
    // charged here, in the order that walk would have charged them. A reduction
    // that got cheaper by being written differently would be a budget that
    // decides acceptance by implementation detail, which is what §4 forbids.
    let mut inner = env.clone();
    let mut at = term;
    for _ in 0..bindings {
        let Shape::Bind {
            binder: Binder::Let { value, .. },
            body,
            ..
        } = at.shape()
        else {
            break;
        };
        meter.step("evaluation")?;
        let value = crate::kernel::eval::eval(meter, &inner, value)?;
        inner = inner.push(value);
        at = body;
    }
    for _ in 0..spine.len().saturating_add(1) {
        meter.step("evaluation")?;
    }
    let mut again = Vec::with_capacity(spine.len());
    for argument in spine {
        again.push(crate::kernel::eval::eval(meter, &inner, argument)?);
        meter.step("function application")?;
    }
    Ok(Descended::Again(again))
}

/// A term's application spine: the head, and its arguments outermost first.
fn called(term: &Term) -> (&Term, Vec<&Term>) {
    let mut arguments = Vec::new();
    let mut at = term;
    while let Shape::App { function, argument } = at.shape() {
        arguments.push(argument);
        at = function;
    }
    arguments.reverse();
    (at, arguments)
}

/// The constructor a canonical value was built by, and what it was applied to.
///
/// `None` for anything that is not canonical at a declared family, which is what
/// leaves a split blocked.
///
/// A numeral answers from its **count** rather than by being unfolded into a
/// spine, which is `iota.rs`'s tower-avoiding decrement restated as this
/// reducer's own rule: the two reducers ask the question of different things and
/// share no code, so a split on a large numeral would otherwise cost the number
/// the author wrote.
fn analysed(value: &Value) -> Option<(Name, Vec<Value>)> {
    match value.form {
        Form::Numeral(ref numeral) => {
            let counting = numeral.family.counting()?;
            let which = counting.case_of(numeral.count);
            let below = numeral
                .family
                .below(numeral.count)
                .map(|below| Value::new(value.origin, Form::Numeral(below)));
            let name = Constant::constructor(&numeral.family.group, numeral.family.family, which).name();
            Some((name, below.into_iter().collect()))
        }
        Form::Neutral(ref neutral) => {
            let (name, params) = constructed(neutral)?;
            let mut fields = Vec::with_capacity(neutral.spine.len().saturating_sub(params));
            for elimination in neutral.spine.iter().skip(params) {
                let Elim::App { ref argument, .. } = *elimination;
                fields.push(Value::clone(argument));
            }
            Some((name, fields))
        }
        Form::Universe(_) | Form::Pi { .. } | Form::Lam(_) | Form::Lit(_) => None,
    }
}
