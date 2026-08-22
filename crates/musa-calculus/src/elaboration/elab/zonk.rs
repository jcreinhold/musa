//! Solutions written back into the term that is stored.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::elaboration::refuse::{ElabError, Refusal};
use crate::kernel::meta::{Meta, MetaSource};
use crate::kernel::term::{Binder, Level, Shape, Term};

use super::Elaborator;

impl Elaborator {
    /// `term` with every solved unknown written back as the term it solved to.
    ///
    /// Runs after [`Self::settled`] and before the term is *stored*, and that
    /// ordering is the point, not hygiene: an unknown's solution is a *value*,
    /// while the term it sits in is evaluated later under other environments —
    /// a caller's, the normalizer's. Only a term variable is re-read under a
    /// new environment, so the unknown has to become one here.
    ///
    /// **What is written in is the body, at the depth the occurrence sits at.**
    /// An occurrence is the unknown applied to its whole scope
    /// (`kernel::meta`) and its solution is a closed λ-chain over that same
    /// scope, so writing the λ-chain in where the occurrence stood would leave
    /// a β-redex — a term that says the right thing, reads like bookkeeping,
    /// and that [`crate::kernel::recheck`] cannot infer a type for, a λ having
    /// none of its own. Applying the solution to the variables it abstracted
    /// reduces that redex away, and quoting the result at the walk's own depth
    /// is what turns those variables back into indices that name the binders
    /// standing here.
    ///
    /// An unsolved unknown cannot reach here — [`Self::settled`] has already
    /// refused it — so one found is the audit's bug, reported as the refusal
    /// the audit would have given rather than a panic.
    pub(crate) fn zonk(&mut self, term: &Term) -> Result<Term, ElabError> {
        self.zonking(term, Level::ZERO)
    }

    /// The walk [`Self::zonk`] is the depth-0 case of.
    fn zonking(&mut self, term: &Term, depth: Level) -> Result<Term, ElabError> {
        let here = term.origin();
        let shape = match term.shape() {
            Shape::Meta(meta) => return self.filled(meta, depth),
            Shape::Var(_) | Shape::Lit(_) => return Ok(term.clone()),
            // Levels are written back here too, for the same reason metas are:
            // the stored term outlives the elaboration that solved them, and a
            // variable left standing in it would be one nothing can force
            // later. What survives is only what generalization claimed, which
            // is what makes a stored term's open variables exactly the
            // declaration's level parameters.
            Shape::Universe(level) => return Ok(Term::universe(here, level.forced())),
            Shape::Named { name, role, levels } => {
                return Ok(Term::named_at(here, Arc::clone(name), role.clone(), levels.forced()));
            }
            // One arm for all three binders: what differs between them is which
            // subterms sit outside the binder, and that is the `Binder`'s own
            // question rather than a reason for three copies of this walk.
            Shape::Bind { name, binder, body } => {
                let binder = match binder {
                    Binder::Lam => Binder::Lam,
                    Binder::Pi { filling, ty } => Binder::Pi {
                        filling: filling.clone(),
                        ty: self.zonking(ty, depth)?,
                    },
                    Binder::Let { ty, value } => Binder::Let {
                        ty: self.zonking(ty, depth)?,
                        value: self.zonking(value, depth)?,
                    },
                };
                Shape::Bind {
                    name: Arc::clone(name),
                    binder,
                    body: self.zonking(body, depth.deeper())?,
                }
            }
            Shape::App { function, argument } => Shape::App {
                function: self.zonking(function, depth)?,
                argument: self.zonking(argument, depth)?,
            },
            Shape::RecordType(fields) => {
                let mut zonked = Vec::with_capacity(fields.len());
                for (which, field) in fields.iter().enumerate() {
                    zonked.push(crate::kernel::term::Field {
                        name: Arc::clone(&field.name),
                        term: self.zonking(
                            &field.term,
                            Level(depth.0.saturating_add(u32::try_from(which).unwrap_or(0))),
                        )?,
                    });
                }
                Shape::RecordType(Arc::from(zonked))
            }
            Shape::Record(fields) => {
                let mut zonked = Vec::with_capacity(fields.len());
                for field in fields.iter() {
                    zonked.push(crate::kernel::term::Field {
                        name: Arc::clone(&field.name),
                        term: self.zonking(&field.term, depth)?,
                    });
                }
                Shape::Record(Arc::from(zonked))
            }
            Shape::Project { record, field } => Shape::Project {
                record: self.zonking(record, depth)?,
                field: Arc::clone(field),
            },
        };
        Ok(Term::new(here, shape))
    }

    /// The term a solved unknown stands for, at `depth`.
    ///
    /// The occurrence's own depth is what the answer is quoted at, and the
    /// scope check falls out of that: the body mentions only the variables the
    /// solution abstracted, every one of them a level below the arity, and
    /// [`crate::kernel::quote`] refuses a level the depth does not name. So a
    /// solution that escaped is an [`crate::kernel::error::Malformed`] here
    /// rather than a capture nobody notices.
    ///
    /// # Errors
    ///
    /// [`Refusal::Unsolved`] when the unknown has no solution, which
    /// [`Self::settled`] should already have reported; whatever opening the
    /// solution and reading it back spends.
    fn filled(&mut self, meta: &Meta, depth: Level) -> Result<Term, ElabError> {
        let Some((body, goal)) = crate::kernel::unify::opened_solution(&mut self.meter, meta)? else {
            return Err(Refusal::Unsolved {
                site: MetaSource::TypeParameter,
                created: meta.origin(),
                blocked: None,
            }
            .into());
        };
        Ok(crate::kernel::quote::quote(
            &mut self.meter,
            depth,
            crate::kernel::quote::Mode::Open,
            &goal,
            &body,
        )?)
    }
}
