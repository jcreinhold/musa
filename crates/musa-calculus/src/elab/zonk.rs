//! Solutions written back into the term that is stored.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::meta::MetaSource;
use crate::refuse::{ElabError, Refusal};
use crate::term::{Shape, Term};

use super::Elaborator;

impl Elaborator {
    /// `term` with every solved hole written back as the term it solved to.
    ///
    /// Runs after [`Self::settled`] and before the term is *stored*, and that
    /// ordering is the point, not hygiene: a hole's solution is a *value*, read
    /// at the levels of the scope that created it, while the term it sits in is
    /// evaluated later under other environments — a caller's, the normalizer's.
    /// Only a term variable is re-read under a new environment, so the hole has
    /// to become one here: quoting the solution at the binder depth the hole
    /// sits at turns the value back into indices, which is exactly what makes
    /// the stored term mean the same thing everywhere it is evaluated.
    ///
    /// An unsolved hole cannot reach here — [`Self::settled`] has already
    /// refused it — so one found is the audit's bug, reported as the refusal
    /// the audit would have given rather than a panic.
    pub(crate) fn zonk(&mut self, term: &Term) -> Result<Term, ElabError> {
        self.zonking(term, 0)
    }

    /// The walk [`Self::zonk`] is the depth-0 case of.
    fn zonking(&mut self, term: &Term, depth: u32) -> Result<Term, ElabError> {
        let here = term.origin();
        let shape = match term.shape() {
            Shape::Hole(hole) => {
                let Some(solution) = hole.solution() else {
                    return Err(Refusal::Unsolved {
                        site: MetaSource::TypeParameter,
                        created: hole.origin(),
                        blocked: None,
                    }
                    .into());
                };
                let solution = solution.clone();
                return Ok(crate::quote::quote(
                    &mut self.meter,
                    crate::quote::Depth(depth),
                    crate::quote::Mode::Open,
                    hole.ty(),
                    &solution,
                )?);
            }
            Shape::Var(_)
            | Shape::Const(_)
            | Shape::Def(_)
            | Shape::Numeral(_)
            | Shape::Base(_)
            | Shape::Lit(_)
            | Shape::Builtin(_)
            | Shape::Universe(_) => return Ok(term.clone()),
            Shape::Pi {
                filling,
                name,
                domain,
                codomain,
            } => {
                let domain = self.zonking(domain, depth)?;
                let codomain = self.zonking(codomain, depth.saturating_add(1))?;
                Shape::Pi {
                    filling: filling.clone(),
                    name: Arc::clone(name),
                    domain,
                    codomain,
                }
            }
            Shape::Lam { name, body } => {
                let body = self.zonking(body, depth.saturating_add(1))?;
                Shape::Lam {
                    name: Arc::clone(name),
                    body,
                }
            }
            Shape::App { function, argument } => Shape::App {
                function: self.zonking(function, depth)?,
                argument: self.zonking(argument, depth)?,
            },
            // Both halves. An index is an ordinary term (§1.5), so a hole
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
                        term: self.zonking(&field.term, depth.saturating_add(u32::try_from(which).unwrap_or(0)))?,
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
            Shape::Let { name, ty, value, body } => Shape::Let {
                name: Arc::clone(name),
                ty: self.zonking(ty, depth)?,
                value: self.zonking(value, depth)?,
                body: self.zonking(body, depth.saturating_add(1))?,
            },
        };
        Ok(Term::new(here, shape))
    }
}
