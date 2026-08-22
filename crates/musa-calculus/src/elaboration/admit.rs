//! Whether the host's registrations are admissible (§5.8).
//!
//! [`Registry`] itself is data and lives in [`crate::kernel::base`]; this is the
//! judgment that decides whether a table of registrations may exist at all. It
//! is here rather than there because it answers a
//! [`Refusal`](crate::elaboration::refuse::Refusal) — the host wrote a
//! signature, and a signature that D1 does not admit is a mistake reported to
//! whoever wrote it, in the same vocabulary a source program's mistakes are
//! reported in.
//!
//! What it checks, and why each is checked here:
//!
//! - **one name, one meaning** — no name registered twice, whether as two base
//!   types, two builtins, or one of each;
//! - **a δ signature is finite data** (D1);
//! - **a structural eliminator has a target it could fire on**;
//! - **an index-reading base declares an index of a sort the solver knows**.

use std::collections::HashMap;
use std::sync::Arc;

use crate::elaboration::refuse::Refusal;
use crate::kernel::base::{Base, Builtin, Extern, Family, Registry};
use crate::kernel::term::{Binder, Name, Role, Shape, Term};

impl Registry {
    /// The registry holding `bases` and `builtins`, or why it is not one.
    ///
    /// Checks the half of §5.8 a signature makes visible, and says in the module
    /// doc which half that is:
    ///
    /// - **one name, one meaning** — no name registered twice, whether as two
    ///   base types, two builtins, or one of each;
    /// - **a δ signature is finite data** (D1) — every argument type and the
    ///   result type is a registered base type, or a declared family applied to
    ///   argument types that are themselves this, at any depth. Stated
    ///   positively rather than as "no arrow", because "no arrow" also admitted
    ///   the record types, universes, and bare variables D1 never meant, and
    ///   because a rule can only *read* and *answer* what [`Datum`] can say;
    /// - **a structural eliminator has a target it could fire on** — the index
    ///   [`Builtin::structural`] wrote down names an argument of the signature,
    ///   and that argument's type is headed by a base type of this registry.
    ///
    /// D1 is checked over δ-builtins and nowhere else, which is where §5.8
    /// states it. A structural eliminator's signature holds an arrow by
    /// definition — a traversal takes an algebra — so a registry that applied D1
    /// to the whole table would refuse the family it is registering. What
    /// replaces it for that family is the target check: a rewrite whose target
    /// is a *declared* type would be a second ι-rule for something that already
    /// has one, and the second path is what the audits keep looking for.
    ///
    /// An arrow keeps its own diagnostic inside the positive check, because a Π
    /// where data was wanted is the D1 violation a table author actually writes,
    /// and [`Refusal::HigherOrderDelta`] says the specific thing.
    ///
    /// **A [`Builtin::constructor`] is checked by neither, and that is the
    /// absence of a check rather than an exemption from one.** Both checks ask
    /// about a *reduction*: D1 bounds what a δ-rule may read and answer, and the
    /// target check bounds what a rewrite may fire on. A form with no reduction
    /// has neither question to answer, so what is left is the duplicate-name
    /// check above, which applies to everything registered. §2's machine forms
    /// are why: their signatures are full of arrows and universes, and holding
    /// them to a rule about `fn` pointers would refuse the family for a property
    /// no rule of theirs has.
    ///
    /// # Errors
    ///
    /// [`Refusal::DuplicateExtern`], [`Refusal::HigherOrderDelta`],
    /// [`Refusal::NotFiniteData`], [`Refusal::UnknownBase`],
    /// [`Refusal::TargetOutsideSignature`], or [`Refusal::TargetNotABase`].
    pub fn new(bases: Vec<Base>, builtins: Vec<Builtin>) -> Result<Self, Refusal> {
        let mut names: HashMap<Name, Extern> = HashMap::with_capacity(bases.len().saturating_add(builtins.len()));
        for base in bases {
            claim(&mut names, Arc::clone(base.name()), Extern::Base(base))?;
        }
        for builtin in builtins {
            claim(&mut names, Arc::clone(builtin.name()), Extern::Builtin(builtin))?;
        }
        let registry = Self { names };
        registry.check_delta_signatures()?;
        registry.check_structural_targets()?;
        Ok(registry)
    }

    /// Every structural eliminator's target: an argument of its own signature,
    /// at a base type this registry declared inert.
    fn check_structural_targets(&self) -> Result<(), Refusal> {
        for entry in self.names.values() {
            let Extern::Builtin(builtin) = entry else {
                continue;
            };
            let Some((target, _)) = builtin.structural_rule() else {
                continue;
            };
            // `signature_parts` ends with the result type, and a rule cannot
            // fire on what it produces, so the bound is the arity rather than
            // the number of parts.
            let domain = signature_parts(builtin.ty())
                .into_iter()
                .nth(target)
                .filter(|_| target < builtin.arity())
                .ok_or_else(|| Refusal::TargetOutsideSignature {
                    name: Arc::clone(builtin.name()),
                    at: builtin.ty().origin(),
                })?;
            let Some(base) = head_base(&domain) else {
                return Err(Refusal::TargetNotABase {
                    name: Arc::clone(builtin.name()),
                    at: domain.origin(),
                });
            };
            if self.named(base).is_none() {
                return Err(Refusal::UnknownBase {
                    name: Arc::clone(base),
                    at: domain.origin(),
                });
            }
        }
        Ok(())
    }

    /// D1's signature half, over every δ-builtin at once.
    fn check_delta_signatures(&self) -> Result<(), Refusal> {
        for entry in self.names.values() {
            let Extern::Builtin(builtin) = entry else {
                continue;
            };
            if builtin.family() != Family::Delta {
                continue;
            }
            for argument in signature_parts(builtin.ty()) {
                self.check_finite_data(builtin, &argument)?;
            }
        }
        Ok(())
    }

    /// One argument or result type of a δ-builtin: finite data, at any depth.
    ///
    /// Two shapes and no third. A registered base type, applied to whatever it
    /// takes, is data because §5.8 declares it inert. A declared family applied
    /// to more data is data because prompt 141 proved `List`, `Option`, and
    /// `Result` are ordinary declarations — their values are constructor
    /// applications, which is exactly what [`Datum::Case`] says.
    ///
    /// **A literal is data where a base type indexes on one, and nowhere else.**
    /// This is the case the check reserved and prompt 141e brought a caller for:
    /// `Duration ⟨written⟩` and `Syntax ⟨token-tree⟩` are how the compiler keeps
    /// a written beat from adding to a number of seconds and an expression from
    /// standing where a token tree belongs, and a δ-rule is a `fn` pointer, so
    /// the *only* index it can write is one it can build without a context —
    /// which is a literal and is not a constructor. The law is the narrow one:
    /// admitted under a registered base head, so a declared family applied to a
    /// literal is still refused and `Vec Nat 3` is still not a δ signature.
    ///
    /// Everything else is refused, including the ones that would be *harmless*
    /// to admit, for the reason the literal case was refused until it had a
    /// caller: a check that admits what nothing writes is a check nobody has
    /// read.
    fn check_finite_data(&self, builtin: &Builtin, ty: &Term) -> Result<(), Refusal> {
        let mut head = ty;
        let mut arguments = Vec::new();
        while let Shape::App { function, argument } = head.shape() {
            arguments.push(argument);
            head = function;
        }
        let mut indexed = false;
        match head.shape() {
            // A Π keeps its own diagnostic: it is what a table author writes
            // when they reach for a higher-order operation, and "not finite
            // data" would be a true sentence about the wrong problem.
            Shape::Bind {
                binder: Binder::Pi { .. },
                ..
            } => {
                return Err(Refusal::HigherOrderDelta {
                    name: Arc::clone(builtin.name()),
                    at: builtin.ty().origin(),
                });
            }
            Shape::Named { name, role: Role::Base } => {
                if self.named(name).is_none() {
                    return Err(Refusal::UnknownBase {
                        name: Arc::clone(name),
                        at: head.origin(),
                    });
                }
                indexed = true;
            }
            Shape::Named {
                role: Role::TypeConstructor,
                ..
            } => {}
            Shape::Var(_)
            | Shape::Universe(_)
            | Shape::Named { .. }
            | Shape::App { .. }
            | Shape::RecordType(_)
            | Shape::Record(_)
            | Shape::Project { .. }
            // A λ or a `let`, the Π above having taken its own diagnostic.
            | Shape::Bind { .. }
            | Shape::Lit(_)
            | Shape::Meta(_) => {
                return Err(Refusal::NotFiniteData {
                    name: Arc::clone(builtin.name()),
                    at: head.origin(),
                });
            }
        }
        for argument in arguments {
            if indexed && matches!(*argument.shape(), Shape::Lit(_)) {
                continue;
            }
            self.check_finite_data(builtin, argument)?;
        }
        Ok(())
    }
}

/// Claim a name for one meaning, or refuse because something else has it.
fn claim(names: &mut HashMap<Name, Extern>, name: Name, entry: Extern) -> Result<(), Refusal> {
    // The declared type is the only piece of a registration that came from
    // anywhere, so it is where a diagnostic about the registration points.
    let at = entry.ty().origin();
    if names.contains_key(&name) {
        return Err(Refusal::DuplicateExtern { name, at });
    }
    names.insert(name, entry);
    Ok(())
}

/// The base type a type is an application of, if it is one of anything.
///
/// A base type may take parameters, so `Syntax Expr` is at `Syntax` and the
/// spine says which one. Anything else — a variable, a declared family, a record
/// type — is not a base type and has no answer here.
fn head_base(ty: &Term) -> Option<&Name> {
    let mut head = ty;
    while let Shape::App { function, .. } = head.shape() {
        head = function;
    }
    if let Shape::Named { name, role: Role::Base } = head.shape() {
        Some(name)
    } else {
        None
    }
}

/// A signature's argument types and its result type, in order.
fn signature_parts(ty: &Term) -> Vec<Term> {
    let mut parts = Vec::new();
    let mut rest = ty.clone();
    while let Shape::Bind {
        binder: Binder::Pi { ty, .. },
        body,
        ..
    } = rest.shape()
    {
        parts.push(ty.clone());
        let next = body.clone();
        rest = next;
    }
    parts.push(rest);
    parts
}
