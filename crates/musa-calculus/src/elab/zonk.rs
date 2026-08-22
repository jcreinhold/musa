//! Solutions written back into the term that is stored.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::meta::MetaSource;
use crate::refuse::{ElabError, Refusal};
use crate::term::{Binder, Level, Shape, Term};

use super::Elaborator;

impl Elaborator {
    /// `term` with every solved meta written back as the term it solved to.
    ///
    /// Runs after [`Self::settled`] and before the term is *stored*, and that
    /// ordering is the point, not hygiene: a meta's solution is a *value*, read
    /// at the levels of the scope that created it, while the term it sits in is
    /// evaluated later under other environments — a caller's, the normalizer's.
    /// Only a term variable is re-read under a new environment, so the meta has
    /// to become one here: quoting the solution at the binder depth the meta
    /// sits at turns the value back into indices, which is exactly what makes
    /// the stored term mean the same thing everywhere it is evaluated.
    ///
    /// An unsolved meta cannot reach here — [`Self::settled`] has already
    /// refused it — so one found is the audit's bug, reported as the refusal
    /// the audit would have given rather than a panic.
    pub(crate) fn zonk(&mut self, term: &Term) -> Result<Term, ElabError> {
        self.zonking(term, Level::ZERO)
    }

    /// The walk [`Self::zonk`] is the depth-0 case of.
    fn zonking(&mut self, term: &Term, depth: Level) -> Result<Term, ElabError> {
        let here = term.origin();
        let shape = match term.shape() {
            Shape::Meta(meta) => {
                let Some(solution) = meta.solution() else {
                    return Err(Refusal::Unsolved {
                        site: MetaSource::TypeParameter,
                        created: meta.origin(),
                        blocked: None,
                    }
                    .into());
                };
                let solution = solution.clone();
                return Ok(crate::quote::quote(
                    &mut self.meter,
                    depth,
                    crate::quote::Mode::Open,
                    meta.ty(),
                    &solution,
                )?);
            }
            Shape::Var(_) | Shape::Named { .. } | Shape::Lit(_) | Shape::Universe(_) => return Ok(term.clone()),
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
            // Both halves. An index is an ordinary term (§1.5), so a meta
            // standing in one is solved and unfolded exactly as anywhere else —
            // which is what `Row(n)` at a call that solved `n` depends on.
            Shape::Indexed { ty, index } => Shape::Indexed {
                ty: self.zonking(ty, depth)?,
                index: self.zonking(index, depth)?,
            },
            Shape::RecordType(fields) => {
                let mut zonked = Vec::with_capacity(fields.len());
                for (which, field) in fields.iter().enumerate() {
                    zonked.push(crate::term::Field {
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
                    zonked.push(crate::term::Field {
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
}
