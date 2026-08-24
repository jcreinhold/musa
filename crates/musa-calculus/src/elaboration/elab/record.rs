//! Projection and update, over the one-constructor family a `record` is.
//!
//! See the `elab` module docs for the judgments these rules belong to.
//!
//! There is no record *type* rule here and no record *shape* in the core.
//! `01-surface.md` §1.2 makes a `record` a one-constructor inductive family, so
//! a literal is that constructor applied to its fields, a projection is an
//! application of the family's generated accessor
//! ([`Role::Projection`](crate::Role)), and `with` is the `let`-and-literal
//! rebuild §9.1 always described. What this module still owns is the two rules
//! that have to *find* the family — a projection is written at a field name and
//! not at a constructor, and an update is written at a path.

use std::sync::Arc;

use crate::elaboration::raw::{Raw, RawUpdate};
use crate::elaboration::refuse::{ElabError, Refusal};
use crate::kernel::eval::{apply, apply_closure, opened};
use crate::kernel::family::{Product, product};
use crate::kernel::origin::Origin;
use crate::kernel::scope::Scope;
use crate::kernel::term::{Index, Name, Role, Shape, Term};
use crate::kernel::value::{Form, Value};

use super::{Elaborator, Typed};

/// The generated accessor for `product`'s field at `position`, as a term
/// already applied to the parameters the subject's type stands at.
///
/// [`Product`] is the kernel's reading of a one-constructor family, shared so
/// that η, conversion, and these two rules cannot disagree about what a record
/// is. What it does not have is a way to write a *term*, which is this
/// module's business and not the kernel's.
fn accessor(
    elaborator: &mut Elaborator,
    scope: &Scope,
    product: &Product,
    here: Origin,
    position: u32,
) -> Result<Term, ElabError> {
    let mut term = product.projection(position).term(here);
    for param in &product.params {
        term = Term::app(here, term, scope.quote_type(&mut elaborator.meter, param)?);
    }
    Ok(term)
}

impl Elaborator {
    /// The record `ty` is, or `None` where it is not one.
    ///
    /// Through the kernel's [`product`], so that "is this a record" has exactly
    /// one answer in this crate and the accessor a projection emits is
    /// generated for exactly the types η expands.
    pub(crate) fn product(&mut self, ty: &Value) -> Result<Option<Product>, ElabError> {
        let unfolded = opened(&mut self.meter, ty)?;
        Ok(product(&mut self.meter, unfolded.as_ref().unwrap_or(ty))?)
    }

    /// [`Self::product`], or [`Refusal::NotARecord`] at `at`.
    fn record_of(&mut self, scope: &Scope, at: Origin, ty: &Value) -> Result<Product, ElabError> {
        match self.product(ty)? {
            Some(product) => Ok(product),
            None => Err(Refusal::NotARecord {
                at,
                ty: scope.quote_type(&mut self.meter, ty)?,
            }
            .into()),
        }
    }

    /// `R { f = e, … } ⇒ R ?p⃗`.
    ///
    /// The head is elaborated by the ordinary constant rule, applied to a
    /// metavariable per family parameter, and the literal is then *checked*
    /// against the type that makes — so there is one rule that builds a record
    /// value ([`Elaborator::literal`]) and this one only says what type it is
    /// being built at. The fields solve the parameters through §2.1's ordinary
    /// matching; one they do not solve is the ordinary unsolved-metavariable
    /// refusal, raised where every other unsolved one is.
    ///
    /// A head whose type is not a universe once its parameters are filled is
    /// not a record type, and is refused here rather than reaching
    /// [`Elaborator::product`] as a goal it would answer `None` to.
    pub(super) fn headed(&mut self, scope: &Scope, raw: &Raw, head: &Raw) -> Result<Typed, ElabError> {
        let here = raw.origin();
        let built = self.infer(scope, head)?;
        let built = self.saturated(scope, here, built)?;
        let unfolded = opened(&mut self.meter, &built.ty)?;
        if !matches!(unfolded.as_ref().unwrap_or(&built.ty).form, Form::Universe(_)) {
            return Err(Refusal::NotARecord {
                at: head.origin(),
                ty: scope.quote_type(&mut self.meter, &built.ty)?,
            }
            .into());
        }
        let ty = scope.eval(&mut self.meter, &built.term)?;
        let term = self.check(scope, raw, &ty)?;
        Ok(Typed { term, ty })
    }

    /// The head a literal wrote and the family its goal names are the same
    /// family, or neither is silently ignored.
    ///
    /// Before the head rode on the raw term it rode in an annotation, and a
    /// disagreement came out as a conversion mismatch. Dropping the annotation
    /// without this would make `let c: Cell<Nat> = Other { … };` elaborate as
    /// `Cell` and say nothing about the word the author wrote — a real mistake
    /// answered with silence, which is worse than the defect this whole rule
    /// exists to fix.
    pub(super) fn head_agrees(
        &mut self,
        scope: &Scope,
        head: Option<&Raw>,
        product: &Product,
    ) -> Result<(), ElabError> {
        let Some(head) = head else { return Ok(()) };
        let written = self.infer(scope, head)?;
        let Shape::Named {
            name,
            role: Role::TypeConstructor,
            ..
        } = written.term.shape()
        else {
            return Err(Refusal::NotARecord {
                at: head.origin(),
                ty: scope.quote_type(&mut self.meter, &written.ty)?,
            }
            .into());
        };
        let expected = crate::kernel::family::Constant::family(&product.group, product.family).name();
        if *name == expected {
            return Ok(());
        }
        Err(Refusal::RecordHead {
            at: head.origin(),
            expected,
            found: Arc::clone(name),
        }
        .into())
    }

    /// `e.f ⇒ A[e]`.
    ///
    /// The term is `N.f p⃗ e` — the family's generated accessor, applied to the
    /// parameters its type stands at and then to the value read from — and the
    /// type is that accessor's own type at the same arguments. Both come off
    /// the same constant rather than being assembled here, so a projection and the ι
    /// rule that reduces it cannot disagree about what field `f` is.
    pub(super) fn projection(
        &mut self,
        scope: &Scope,
        here: Origin,
        record: &Raw,
        field: &Name,
    ) -> Result<Typed, ElabError> {
        let inferred = self.infer(scope, record)?;
        let product = self.record_of(scope, here, &inferred.ty)?;
        let Some(position) = product.fields.iter().position(|declared| declared.name == *field) else {
            return Err(Refusal::NoSuchField {
                at: here,
                field: Arc::clone(field),
            }
            .into());
        };
        let position = u32::try_from(position).unwrap_or(u32::MAX);
        let constant = product.projection(position);
        let globals = scope.cx().globals().clone();
        let mut ty = constant.ty(&mut self.meter, &globals)?;
        for param in product.params.clone() {
            ty = instantiated(&mut self.meter, here, &ty, param)?;
        }
        let subject = scope.eval(&mut self.meter, &inferred.term)?;
        let ty = instantiated(&mut self.meter, here, &ty, subject)?;
        let term = accessor(self, scope, &product, here, position)?;
        Ok(Typed {
            ty,
            term: Term::app(here, term, inferred.term),
        })
    }

    /// `e with { p⃗ = v, … } ⇒ A`, where `A` is `e`'s own type.
    ///
    /// The update is a rebuild, not a mutation: every field the paths do not
    /// name is carried over by projection, and every field they do is checked at
    /// the type the constructor's telescope gives it *after* the fields before
    /// it have been replaced. That is what makes an incoherent update — changing
    /// `n` in `record Frame { n: Nat; held: Vect<Nat>(n); }` and keeping `held` —
    /// an ordinary type error rather than a rule this function has to state.
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
        let product = self.record_of(scope, record.origin(), &record_ty)?;
        let ty_term = scope.quote_type(&mut self.meter, &record_ty)?;
        let subject = scope.eval(&mut self.meter, &inferred.term)?;
        // §9.1's "one `let` per segment". Without it the subject is written once
        // per field it carries over, and a record of eight fields updated at one
        // of them would evaluate the thing being updated eight times.
        let name: Name = Arc::from("with");
        let inner = scope.define(&mut self.meter, Arc::clone(&name), Arc::new(record_ty.clone()), subject)?;
        let replacements: Vec<Replacement<'_>> = updates.iter().map(Replacement::of).collect();
        let literal = self.rebuilt(&inner, here, &Term::var(here, Index(0)), &product, &replacements)?;
        Ok(Typed {
            term: Term::bind(here, name, ty_term, inferred.term, literal),
            ty: record_ty,
        })
    }

    /// The literal one segment of an update rebuilds.
    ///
    /// `subject` denotes the record being rebuilt *in `scope`* — a variable the
    /// caller just bound to it — and the fields carried over are projections of
    /// that variable rather than of the expression, which is what evaluates the
    /// subject once.
    fn rebuilt(
        &mut self,
        scope: &Scope,
        here: Origin,
        subject: &Term,
        product: &Product,
        updates: &[Replacement<'_>],
    ) -> Result<Term, ElabError> {
        for update in updates {
            let Some(head) = update.path.first() else { continue };
            if !product.fields.iter().any(|declared| declared.name == *head) {
                return Err(Refusal::NoSuchField {
                    at: update.origin,
                    field: Arc::clone(head),
                }
                .into());
            }
        }
        let globals = scope.cx().globals().clone();
        let constructor = product.constructor();
        let mut built = constructor.term(here);
        let mut ty = constructor.ty(&mut self.meter, &globals)?;
        for param in product.params.clone() {
            built = Term::app(here, built, scope.quote_type(&mut self.meter, &param)?);
            ty = instantiated(&mut self.meter, here, &ty, param)?;
        }
        let names: Vec<Name> = product
            .fields
            .iter()
            .map(|declared| Arc::clone(&declared.name))
            .collect();
        for (position, declared) in names.iter().enumerate() {
            let unfolded = opened(&mut self.meter, &ty)?;
            let Form::Pi { domain, codomain, .. } = &unfolded.as_ref().unwrap_or(&ty).form else {
                return Err(Refusal::NotAFunction {
                    at: here,
                    ty: scope.quote_type(&mut self.meter, &ty)?,
                }
                .into());
            };
            let (expected, codomain) = (Value::clone(domain), codomain.clone());
            let mine: Vec<Replacement<'_>> = updates
                .iter()
                .filter(|update| update.path.first() == Some(declared))
                .filter_map(Replacement::rest)
                .collect();
            let position = u32::try_from(position).unwrap_or(u32::MAX);
            let term = match mine.split_first() {
                // Carried over: the accessor applied to the bound subject. Its
                // type is read at the *old* record and the literal wants it at
                // the new one, so the two are unified rather than assumed
                // equal — that is where a dependent field whose type an earlier
                // replacement invalidated reports.
                None => {
                    let carried = Term::app(here, accessor(self, scope, product, here, position)?, subject.clone());
                    let was = scope.eval(&mut self.meter, subject)?;
                    let (_, found) = read(product, self, here, &globals, position, &was)?;
                    self.conversion
                        .unify_types(&mut self.meter, scope.depth(), here, &expected, &found)?;
                    carried
                }
                // The path ends here: the new value is checked at the field's
                // type, like any other argument of any other constructor.
                Some((update, rest)) if update.path.is_empty() && rest.is_empty() => {
                    self.check(scope, update.value, &expected)?
                }
                Some(_) => self.deeper(scope, here, subject, product, position, &expected, &mine)?,
            };
            let value = scope.eval(&mut self.meter, &term)?;
            ty = apply_closure(&mut self.meter, &codomain, value)?;
            built = Term::app(here, built, term);
        }
        Ok(built)
    }

    /// One field of an update whose paths reach through it.
    #[expect(
        clippy::too_many_arguments,
        reason = "one segment's context, and every part of it is used"
    )]
    fn deeper(
        &mut self,
        scope: &Scope,
        here: Origin,
        subject: &Term,
        outer: &Product,
        position: u32,
        expected: &Value,
        updates: &[Replacement<'_>],
    ) -> Result<Term, ElabError> {
        let at = updates.first().map_or(here, |update| update.origin);
        let unfolded = opened(&mut self.meter, expected)?;
        let field_ty = Value::clone(unfolded.as_ref().unwrap_or(expected));
        let inner = self.record_of(scope, at, &field_ty)?;
        let projector = accessor(self, scope, outer, here, position)?;
        let projected = Term::app(here, projector, subject.clone());
        let projected_value = scope.eval(&mut self.meter, &projected)?;
        let ty_term = scope.quote_type(&mut self.meter, &field_ty)?;
        let name: Name = outer
            .fields
            .get(usize::try_from(position).unwrap_or(usize::MAX))
            .map_or_else(|| Arc::from("field"), |declared| Arc::clone(&declared.name));
        let under = scope.define(&mut self.meter, Arc::clone(&name), Arc::new(field_ty), projected_value)?;
        let body = self.rebuilt(&under, here, &Term::var(here, Index(0)), &inner, updates)?;
        Ok(Term::bind(here, name, ty_term, projected, body))
    }
}

/// The codomain of a Π type, at `argument`.
///
/// A generated constant's *type* is instantiated by its own closure and never
/// by [`apply`], which applies a lambda: `(t : Cell) → Nat` is a
/// [`Form::Pi`](crate::kernel::value::Form::Pi) and applying it as a function is
/// the malformed-core report. Every rule here walks an accessor's or a
/// constructor's telescope one argument at a time, so the one walk lives here.
///
/// # Errors
///
/// [`Refusal::NotAFunction`] where the type has fewer Π than the walk has
/// arguments, which is a generated constant disagreeing with the declaration it
/// was generated from.
pub(super) fn instantiated(
    meter: &mut crate::kernel::budget::Meter,
    here: Origin,
    ty: &Value,
    argument: Value,
) -> Result<Value, ElabError> {
    let unfolded = opened(meter, ty)?;
    let Form::Pi { codomain, .. } = &unfolded.as_ref().unwrap_or(ty).form else {
        return Err(Refusal::NotAFunction {
            at: here,
            ty: crate::kernel::quote::quote_type(
                meter,
                crate::kernel::term::Level::ZERO,
                crate::kernel::quote::Mode::Open,
                ty,
            )?,
        }
        .into());
    };
    Ok(apply_closure(meter, codomain, argument)?)
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

/// Field `position` of `subject`, as a value and the type it stands at.
///
/// What splitting a record pattern needs: the generated accessor applied to the
/// parameters and then to the subject, which ι reduces the moment the subject
/// is a literal, together with that accessor's own result type at the same
/// arguments. Both come off the same [`Constant`] the projection rule emits, so
/// a pattern and a `.f` cannot read a field differently.
///
/// # Errors
///
/// As the accessor's type: a budget exhausted assembling or instantiating it.
pub(crate) fn read(
    product: &Product,
    elaborator: &mut Elaborator,
    here: Origin,
    globals: &crate::kernel::context::Globals,
    position: u32,
    subject: &Value,
) -> Result<(Value, Value), ElabError> {
    let constant = product.projection(position);
    let mut ty = constant.ty(elaborator.meter(), globals)?;
    let mut value = constant.value(here, globals);
    for param in &product.params {
        ty = instantiated(elaborator.meter(), here, &ty, param.clone())?;
        value = apply(elaborator.meter(), here, value, param.clone())?;
    }
    let ty = instantiated(elaborator.meter(), here, &ty, subject.clone())?;
    let value = apply(elaborator.meter(), here, value, subject.clone())?;
    Ok((value, ty))
}
