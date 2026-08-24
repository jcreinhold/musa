//! An unknown's lifecycle: created, constrained, resolved, audited.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::elaboration::refuse::{ElabError, Refusal};
use crate::kernel::meta::{Meta, MetaSource};
use crate::kernel::origin::Origin;
use crate::kernel::quote::Mode;
use crate::kernel::scope::Scope;
use crate::kernel::term::{Level, Term};
use crate::kernel::value::{Env, Value};

use super::Elaborator;

impl Elaborator {
    /// The one finish point a declaration shares: drain what is waiting, and
    /// refuse the first parameter nothing in it determined.
    ///
    /// §2.1 puts the queue in the judgment and puts its end here: "elaboration
    /// of a declaration ends by draining the queue, and a constraint still
    /// unsolved at the end is an error naming what could not be determined.
    /// Nothing is defaulted, and nothing is left for a later declaration to
    /// settle." The order below is the order those clauses depend on each
    /// other in, and each step says why it is where it is.
    ///
    /// The report names the *earliest* unsolved parameter because that is the
    /// one the author's next edit is about.
    pub(crate) fn settled(&mut self) -> Result<(), ElabError> {
        // Levels first, and before the constraints below: a `Storable`
        // discharged here evaluates a term, and a term whose levels are still
        // open would carry an unknown into a stored value. See
        // [`super::levels`] for the rule this runs.
        self.default_levels()?;
        // The level equations conversion could not answer on the spot, read
        // once more now that defaulting has closed everything open. One place,
        // one message: postponing and then reporting from two places would be
        // two sentences about one disagreement.
        for equation in self.conversion.postponed_levels() {
            equation.settled()?;
        }
        // The postponed comparisons next, before the constraints and again
        // after them: a `Storable` discharge evaluates a term, and a term whose
        // parameters a queued comparison would have determined must not be
        // discharged with them still open. The second pass is what lets a
        // solution the discharge itself wrote unblock something.
        self.drain()?;
        // Constraints next: a `Storable` answered here is a meta solved, and
        // the parameter audit below should not call a constructor's own
        // parameters undetermined for having answered one.
        let waiting = core::mem::take(&mut self.constraints);
        for (constraint, scope, env, at, meta) in waiting {
            let cx = scope.cx().clone();
            let term = crate::elaboration::storable::discharge(self, &cx, &constraint, &env, at)?;
            // Abstracted over the unknown's telescope rather than evaluated in
            // `env`: `discharge` answers a term read under this scope, which is
            // the scope the unknown was created in, and a stored solution binds
            // that scope itself (`kernel::meta`). Evaluating it here would store
            // a value already standing at the occurrence's depth, and opening it
            // later would apply a non-function.
            let value = crate::kernel::unify::abstracted(&mut self.meter, &meta, term)?;
            meta.solve(&mut self.meter, value)
                .map_err(crate::kernel::error::CoreError::from)?;
        }
        self.drain()?;
        // Whatever is still waiting is §2.1's error, and it is reported before
        // the audit below because it can say more: the audit knows only that an
        // unknown was never determined, while a survivor also knows the
        // comparison that was waiting on it.
        let waiting = self.conversion.postponed().take();
        if let Some(entry) = waiting.first() {
            let blocked_on = entry.blocked_on(&mut self.meter)?;
            let site = blocked_on
                .as_ref()
                .map_or(MetaSource::TypeParameter(None), |meta| meta.source().clone());
            return Err(Refusal::Unsolved {
                site,
                created: blocked_on.map_or(entry.origin, |meta| meta.origin()),
                blocked: Some(entry.origin),
            }
            .into());
        }
        if let Some(meta) = self.created.iter().find(|meta| !meta.is_solved()) {
            return Err(Refusal::Unsolved {
                site: meta.source().clone(),
                created: meta.origin(),
                blocked: None,
            }
            .into());
        }
        Ok(())
    }

    /// Retry the postponed comparisons until a round settles nothing.
    ///
    /// §2.1 says a postponed constraint is "retried when a metavariable it
    /// mentions is solved". This is that, read as a fixpoint rather than as a
    /// subscription: a round retries everything and keeps whatever blocks
    /// again, and a round that shortened the queue is evidence something was
    /// solved, so another round is worth running. A round that shortens
    /// nothing has nothing new to tell any entry, and the loop stops.
    ///
    /// **It terminates without consulting the meter**, which matters because
    /// the meter is a resource limit and not a correctness argument: the queue
    /// is finite and each round either strictly shortens it or ends the loop.
    /// The meter is still charged — each retry is a full comparison and §4
    /// charges conversion work — so a queue whose *retries* are expensive
    /// exhausts, which is the budget law this prompt owes.
    fn drain(&mut self) -> Result<(), ElabError> {
        loop {
            let waiting = self.conversion.postponed().take();
            let before = waiting.len();
            if before == 0 {
                return Ok(());
            }
            for entry in waiting {
                self.conversion.retry(&mut self.meter, &entry)?;
            }
            if self.conversion.postponed().len() >= before {
                return Ok(());
            }
        }
    }

    /// A placeholder for a term the walk cannot yet determine, in `scope`.
    ///
    /// §2.1's `?m[σ]`, built the way `kernel::meta` requires: the type is
    /// `goal` abstracted over every binder `scope` holds, so the unknown is
    /// closed, and the occurrence handed back is that unknown applied to those
    /// binders. Both halves come out of one call because a caller that built
    /// one of them itself could build it differently, and the disagreement
    /// would not surface until the solution was read back somewhere else
    /// entirely.
    ///
    /// The telescope is read back with [`Mode::Keep`], not opened: it is the
    /// unknown's own bookkeeping, a definition in it is still resolvable under
    /// the table that travels with the meta, and unfolding the scope of every
    /// unknown would make creating one cost what normalizing the context costs.
    ///
    /// `named` is what the Π this unknown stands at called its binder, when the
    /// caller has one to give. It is carried for the diagnostic and read
    /// nowhere else: an unknown is identified by its cell, never by a name, and
    /// two unknowns standing at binders spelled alike are still two unknowns.
    ///
    /// # Errors
    ///
    /// Whatever reading the telescope back and evaluating it spends.
    pub(crate) fn fresh_meta(
        &mut self,
        scope: &Scope,
        here: Origin,
        goal: &Value,
        named: Option<crate::kernel::term::Name>,
    ) -> Result<Unknown, ElabError> {
        let telescope = scope.telescope();
        let arity = u32::try_from(telescope.len()).unwrap_or(u32::MAX);
        let mut ty = crate::kernel::quote::quote_type(&mut self.meter, Level(arity), Mode::Keep, goal)?;
        for (position, binder) in telescope.iter().enumerate().rev() {
            let depth = Level(u32::try_from(position).unwrap_or(0));
            let domain = crate::kernel::quote::quote_type(&mut self.meter, depth, Mode::Keep, &binder.ty)?;
            ty = Term::pi(here, Arc::clone(&binder.name), domain, ty);
        }
        let globals = scope.cx().globals().clone();
        let ty = crate::kernel::eval::eval(&mut self.meter, &Env::under(globals.clone()), &ty)?;
        let meta = Meta::new(
            self.next_meta,
            here,
            ty,
            arity,
            globals,
            MetaSource::TypeParameter(named),
        );
        self.next_meta = self.next_meta.saturating_add(1);
        self.created.push(meta.clone());
        let (term, value) = Self::occurrence(scope, &meta, here)?;
        Ok(Unknown { meta, term, value })
    }

    /// An occurrence of `meta` in `scope`: the unknown applied to what this
    /// scope's binders stand for.
    ///
    /// Asked a second time by the walk that has to make a placeholder agree
    /// with the argument that finally stood in its slot, which needs the same
    /// occurrence and must not build a different one.
    ///
    /// # Errors
    ///
    /// [`crate::kernel::error::Malformed::MetaTelescope`] when the scope no
    /// longer holds the binders the unknown was created under.
    pub(crate) fn occurrence(scope: &Scope, meta: &Meta, here: Origin) -> Result<(Term, Value), ElabError> {
        let arguments: Vec<Value> = scope.telescope().into_iter().map(|binder| binder.value).collect();
        Ok(crate::kernel::unify::occurrence(meta, here, &arguments)?)
    }
}

/// A fresh unknown, and the two ways it stands in what the walk is building.
///
/// Three fields rather than a meta and two constructions at each call site, for
/// the reason [`Elaborator::fresh_meta`] gives: the term and the value are one
/// fact, and two copies of it are free to disagree.
pub(crate) struct Unknown {
    /// The unknown itself, for a caller that has to solve or audit it.
    pub(crate) meta: Meta,
    /// Its occurrence as a term: the unknown applied to its whole scope.
    pub(crate) term: Term,
    /// The same occurrence as a value, for the type the walk carries on with.
    pub(crate) value: Value,
}
