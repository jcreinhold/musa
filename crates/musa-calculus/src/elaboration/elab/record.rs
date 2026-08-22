//! Record types, projection, and update.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::elaboration::raw::{Raw, RawField, RawUpdate};
use crate::elaboration::refuse::{ElabError, Refusal};
use crate::kernel::eval::{eval, field_type, opened};
use crate::kernel::origin::Origin;
use crate::kernel::scope::Scope;
use crate::kernel::sort::Sort;
use crate::kernel::term::{Field, Index, Name, Shape, Term};
use crate::kernel::value::{Form, Telescope, Value};

use super::{Elaborator, Typed};

impl Elaborator {
    /// `{ f : A, … } ⇒ Type (max …)`.
    pub(super) fn record_type(&mut self, scope: &Scope, here: Origin, fields: &[RawField]) -> Result<Typed, ElabError> {
        let mut level = Sort::ZERO;
        let mut inner = scope.clone();
        let mut elaborated = Vec::with_capacity(fields.len());
        for (position, field) in fields.iter().enumerate() {
            if let Some(previous) = fields.iter().take(position).find(|earlier| earlier.name == field.name) {
                return Err(Refusal::DuplicateField {
                    at: field.term.origin(),
                    previous: previous.term.origin(),
                    field: Arc::clone(&field.name),
                }
                .into());
            }
            let (term, field_level) = self.check_type(&inner, &field.term)?;
            level = level.max(field_level);
            let value = inner.eval(&mut self.meter, &term)?;
            inner = inner.assume(Some(Arc::clone(&field.name)), field.term.origin(), Arc::new(value));
            elaborated.push(Field {
                name: Arc::clone(&field.name),
                term,
            });
        }
        Ok(Typed {
            term: Term::new(here, Shape::RecordType(elaborated.into())),
            ty: Value::new(here, Form::Universe(level)),
        })
    }

    /// `e.f ⇒ A[e]`.
    pub(super) fn projection(
        &mut self,
        scope: &Scope,
        here: Origin,
        record: &Raw,
        field: &Name,
    ) -> Result<Typed, ElabError> {
        let inferred = self.infer(scope, record)?;
        let unfolded = opened(&mut self.meter, &inferred.ty)?;
        let record_ty = unfolded.as_ref().unwrap_or(&inferred.ty);
        let Form::RecordType(telescope) = &record_ty.form else {
            return Err(Refusal::NotARecord {
                at: here,
                ty: scope.quote_type(&mut self.meter, &inferred.ty)?,
            }
            .into());
        };
        let telescope = telescope.clone();
        if !telescope.fields.iter().any(|declared| &declared.name == field) {
            return Err(Refusal::NoSuchField {
                at: here,
                field: Arc::clone(field),
            }
            .into());
        }
        let subject = scope.eval(&mut self.meter, &inferred.term)?;
        Ok(Typed {
            ty: field_type(&mut self.meter, &telescope, &subject, field)?,
            term: Term::project(here, inferred.term, Arc::clone(field)),
        })
    }

    /// `e with { p⃗ = v, … } ⇒ A`, where `A` is `e`'s own type.
    ///
    /// The update is a rebuild, not a mutation: every field the paths do not
    /// name is carried over by projection, and every field they do is checked at
    /// the type the telescope gives it *after* the fields before it have been
    /// replaced. That is what makes an incoherent update — changing `n` in
    /// `{ n : Nat, xs : Vec A n }` and keeping `xs` — an ordinary type error
    /// rather than a rule this function has to state.
    pub(super) fn update(
        &mut self,
        scope: &Scope,
        here: Origin,
        record: &Raw,
        updates: &[RawUpdate],
    ) -> Result<Typed, ElabError> {
        overlapping(updates)?;
        let inferred = self.infer(scope, record)?;
        let unfolded = opened(&mut self.meter, &inferred.ty)?;
        let record_ty = Value::clone(unfolded.as_ref().unwrap_or(&inferred.ty));
        let Form::RecordType(telescope) = &record_ty.form else {
            return Err(Refusal::NotARecord {
                at: record.origin(),
                ty: scope.quote_type(&mut self.meter, &inferred.ty)?,
            }
            .into());
        };
        let telescope = telescope.clone();
        let ty_term = scope.quote_type(&mut self.meter, &record_ty)?;
        let subject = scope.eval(&mut self.meter, &inferred.term)?;
        // §9.1's "one `let` per segment". Without it the subject is written once
        // per field it carries over, and a record of eight fields updated at one
        // of them would evaluate the thing being updated eight times.
        let name: Name = Arc::from("with");
        let inner = scope.define(
            &mut self.meter,
            Arc::clone(&name),
            Arc::new(record_ty.clone()),
            subject.clone(),
        )?;
        let replacements: Vec<Replacement<'_>> = updates.iter().map(Replacement::of).collect();
        let literal = self.rebuilt(
            &inner,
            here,
            &Term::var(here, Index(0)),
            &subject,
            &telescope,
            &replacements,
        )?;
        Ok(Typed {
            term: Term::bind(here, name, ty_term, inferred.term, literal),
            ty: record_ty,
        })
    }

    /// The literal one segment of an update rebuilds.
    ///
    /// `subject` denotes the record being rebuilt *in `scope`*, and `value` is
    /// what it evaluates to. They are separate arguments because the term is a
    /// variable the caller just bound and the value is the record it was bound
    /// to: the type of a field carried over is read off the second, and the term
    /// that carries it over is a projection of the first.
    fn rebuilt(
        &mut self,
        scope: &Scope,
        here: Origin,
        subject: &Term,
        value: &Value,
        telescope: &Telescope,
        updates: &[Replacement<'_>],
    ) -> Result<Term, ElabError> {
        for update in updates {
            let Some(head) = update.path.first() else { continue };
            if !telescope.fields.iter().any(|declared| declared.name == *head) {
                return Err(Refusal::NoSuchField {
                    at: update.origin,
                    field: Arc::clone(head),
                }
                .into());
            }
        }
        let mut env = telescope.env.clone();
        let mut fields = Vec::with_capacity(telescope.fields.len());
        for declared in telescope.fields.iter() {
            let expected = eval(&mut self.meter, &env, &declared.term)?;
            let mine: Vec<Replacement<'_>> = updates
                .iter()
                .filter(|update| update.path.first() == Some(&declared.name))
                .filter_map(Replacement::rest)
                .collect();
            let term = match mine.split_first() {
                // Carried over. Its type is read at the *old* record and the
                // literal wants it at the new one, so the two are unified rather
                // than assumed equal — that is where a dependent field whose
                // type an earlier replacement invalidated reports.
                None => {
                    let term = Term::project(here, subject.clone(), Arc::clone(&declared.name));
                    let found = field_type(&mut self.meter, telescope, value, &declared.name)?;
                    self.conversion
                        .unify_types(&mut self.meter, scope.depth(), here, &expected, &found)?;
                    term
                }
                // The path ends here: the new value is checked at the field's
                // type, like any other field of any other literal.
                Some((update, rest)) if update.path.is_empty() && rest.is_empty() => {
                    self.check(scope, update.value, &expected)?
                }
                Some(_) => self.deeper(scope, here, subject, &declared.name, &expected, &mine)?,
            };
            env = env.push(scope.eval(&mut self.meter, &term)?);
            fields.push(Field {
                name: Arc::clone(&declared.name),
                term,
            });
        }
        Ok(Term::new(here, Shape::Record(fields.into())))
    }

    /// One field of an update whose paths reach through it.
    fn deeper(
        &mut self,
        scope: &Scope,
        here: Origin,
        subject: &Term,
        field: &Name,
        expected: &Value,
        updates: &[Replacement<'_>],
    ) -> Result<Term, ElabError> {
        let unfolded = opened(&mut self.meter, expected)?;
        let field_ty = Value::clone(unfolded.as_ref().unwrap_or(expected));
        let Form::RecordType(inner) = &field_ty.form else {
            let at = updates.first().map_or(here, |update| update.origin);
            return Err(Refusal::NotARecord {
                at,
                ty: scope.quote_type(&mut self.meter, expected)?,
            }
            .into());
        };
        let inner = inner.clone();
        let projected = Term::project(here, subject.clone(), Arc::clone(field));
        let projected_value = scope.eval(&mut self.meter, &projected)?;
        let ty_term = scope.quote_type(&mut self.meter, &field_ty)?;
        let under = scope.define(
            &mut self.meter,
            Arc::clone(field),
            Arc::new(field_ty),
            projected_value.clone(),
        )?;
        let body = self.rebuilt(
            &under,
            here,
            &Term::var(here, Index(0)),
            &projected_value,
            &inner,
            updates,
        )?;
        Ok(Term::bind(here, Arc::clone(field), ty_term, projected, body))
    }
}

/// One replacement of an update, as the rebuild of one segment sees it.
///
/// The path shortens by a segment at each level, which is why this borrows the
/// [`RawUpdate`]'s rather than owning a copy: the recursion is over suffixes of
/// one path and nothing needs a second.
#[derive(Clone, Copy)]
struct Replacement<'a> {
    origin: Origin,
    path: &'a [Name],
    value: &'a Raw,
}

impl<'a> Replacement<'a> {
    /// The whole replacement, at the outermost record.
    fn of(update: &'a RawUpdate) -> Self {
        Self {
            origin: update.origin,
            path: &update.path,
            value: &update.value,
        }
    }

    /// The same replacement, one segment further in.
    fn rest(&self) -> Option<Self> {
        Some(Self {
            path: self.path.split_first()?.1,
            ..*self
        })
    }
}

/// Refuse an update where one path is a prefix of another.
fn overlapping(updates: &[RawUpdate]) -> Result<(), ElabError> {
    for (position, update) in updates.iter().enumerate() {
        for earlier in updates.iter().take(position) {
            let shorter = earlier.path.len().min(update.path.len());
            if earlier.path.iter().take(shorter).eq(update.path.iter().take(shorter)) {
                return Err(Refusal::OverlappingUpdate {
                    at: update.origin,
                    previous: earlier.origin,
                }
                .into());
            }
        }
    }
    Ok(())
}
