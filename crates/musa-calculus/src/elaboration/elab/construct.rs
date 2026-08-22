//! A constructor applied, with or without an expected type naming its family.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::elaboration::raw::{Raw, RawShape};
use crate::elaboration::refuse::ElabError;
use crate::kernel::scope::Scope;
use crate::kernel::term::{Filling, Name};
use crate::kernel::value::Value;

use super::{Elaborator, Typed};

impl Elaborator {
    pub(super) fn constructed(&mut self, scope: &Scope, raw: &Raw, ty: &Value) -> Result<Option<Typed>, ElabError> {
        let here = raw.origin();
        let Some((head, arguments)) = written_spine(raw) else {
            return Ok(None);
        };
        let Some((case, fields, params)) = self.case_of(scope, head, ty)? else {
            return Ok(None);
        };
        if arguments.len() > fields {
            return Ok(None);
        }
        // Through the ordinary constant rule, so that a case a module keeps to
        // itself is refused here the same way it is refused when its qualified
        // name is written out.
        let mut built = self.infer(scope, &Raw::var(here, case))?;
        built = match params {
            Params::Read(values) => {
                for param in &values {
                    built = self.given(scope, here, built, param)?;
                }
                built
            }
            Params::Metas(count) => self.metas(scope, here, built, count)?,
        };
        Ok(Some(self.apply_spine(scope, here, built, &arguments, Some(ty))?))
    }

    /// The qualified case `head` denotes, and how many fields it takes.
    ///
    /// The expected type first, because that is the rule §2 states and the
    /// only one a bare case name can use — and it applies to a name from
    /// either namespace, since which namespace a name came from decides how it
    /// *resolves* and not what it may be checked against.
    ///
    /// The written registry second, and only for [`RawShape::Hosted`]. An
    /// author's `Option.Some Int` is `Some` at the parameter `Int`, still
    /// wanting its field; a reader's is `Some` holding the field `Int` at a
    /// parameter the application pass will solve. Both are well-typed readings
    /// of the same spine, so the namespace is what tells them apart — see
    /// [`Self::constructed_open`], which draws the same line where there is no
    /// expected type at all.
    fn case_of(&mut self, scope: &Scope, head: &Raw, ty: &Value) -> Result<Option<(String, usize, Params)>, ElabError> {
        let (RawShape::Var(name) | RawShape::Hosted(name)) = head.shape() else {
            return Ok(None);
        };
        if let Some(element) = crate::kernel::family::element(&mut self.meter, ty)?
            && let Some(declared) = element.group.family_at(element.family)
            && let Some((case, fields)) = case_named(scope, name, declared)
        {
            return Ok(Some((
                format!("{}.{case}", declared.name),
                fields,
                Params::Read(element.params),
            )));
        }
        if !matches!(head.shape(), RawShape::Hosted(_)) {
            return Ok(None);
        }
        let Some((fields, params)) = written_case(scope, name) else {
            return Ok(None);
        };
        Ok(Some((name.to_string(), fields, Params::Metas(params))))
    }

    /// §2's constructor rule reached from the other direction: `C a⃗ ⇒ N ?p⃗`,
    /// where the `?p⃗` are the application pass's metas.
    ///
    /// Three shapes arrive here. A **reader-written** constructor (a
    /// [`RawShape::Hosted`] name) knows its family from the name itself. A
    /// **bare** case name — `Nothing`, not applied — answers to no binder and
    /// no declaration, and then §2.1's rule is uniqueness: exactly one family
    /// may declare the case, which is what makes the reading determined rather
    /// than guessed. Any other shape is not a constructor and answers `None`,
    /// leaving the name to the ordinary variable rule and its refusal.
    ///
    /// A case that is still *applied* afterwards keeps its remaining fields as
    /// an ordinary Π: `Some` alone is a function value, and §1.3's arity law
    /// is about calls, not names.
    pub(super) fn constructed_open(&mut self, scope: &Scope, raw: &Raw) -> Result<Option<Typed>, ElabError> {
        let here = raw.origin();
        let (head, arguments) = written_spine(raw).unwrap_or((raw, Vec::new()));
        match head.shape() {
            RawShape::Hosted(name) => {
                let Some((fields, params)) = written_case(scope, name) else {
                    return Ok(None);
                };
                if arguments.len() > fields {
                    return Ok(None);
                }
                let built = self.infer(scope, &Raw::var(here, name.to_string()))?;
                let built = self.metas(scope, here, built, params)?;
                Ok(Some(self.apply_spine(scope, here, built, &arguments, None)?))
            }
            RawShape::Var(_)
            | RawShape::Numeral { .. }
            | RawShape::Lit(_)
            | RawShape::Universe(_)
            | RawShape::Pi { .. }
            | RawShape::Indexed { .. }
            | RawShape::Lam { .. }
            | RawShape::App { .. }
            | RawShape::Call { .. }
            | RawShape::RecordType(_)
            | RawShape::Record(_)
            | RawShape::Method { .. }
            | RawShape::Project { .. }
            | RawShape::Update { .. }
            | RawShape::Let { .. }
            | RawShape::Annot { .. }
            | RawShape::Match { .. }
            | RawShape::Rec { .. } => Ok(None),
        }
    }
}

/// A constructor's family parameters, for [`Elaborator::constructed`]: read
/// off the expected type, or — where no expected type has named the family —
/// one meta each, solved by the fields as they are written.
enum Params {
    /// The values the expected type carries, in family order.
    Read(Vec<Value>),
    /// How many the family's telescope declares.
    Metas(u32),
}

/// How many fields the constructor `name` names takes, and how many parameters
/// its family has — or [`None`] for a name that is not a declared case.
///
/// The written name's half of [`Elaborator::case_of`], read out so that
/// [`Elaborator::constructed_open`] asks the same question in the direction that
/// has no expected type to ask it of.
fn written_case(scope: &Scope, name: &Name) -> Option<(usize, u32)> {
    let crate::kernel::family::Found::Rigid(constant) = scope.declared(name)? else {
        return None;
    };
    let crate::kernel::family::Role::Constructor(which) = constant.role else {
        return None;
    };
    let fields = constant
        .group
        .family_at(constant.family)
        .and_then(|declared| declared.constructor_at(which))
        .map(|constructor| constructor.fields.len())?;
    Some((fields, constant.group.params()))
}

/// The case of `declared` that a written name denotes, and how many fields it
/// takes — the count being what tells a missing parameter from a missing field.
///
/// Two spellings, and the difference between them is who wrote the family's
/// name. `Option.Some` says it, and the only question is whether the family is
/// the one expected. `Some` does not, and then §1.3's rule applies: the word is
/// read in the expected type's namespace, but only after a binder and a
/// declaration have both declined it, so nothing an author named themselves can
/// be taken for a constructor.
///
/// # The family answers to its own name, and that is not a declaration declining
///
/// `01-surface.md` §1.3 blesses `enum Beats { Beats(Nat) }` in as many words, so
/// a word that names a family and one of its cases is an ordinary program and
/// not a collision. Counting the family as "a declaration answered" would make
/// every such case unwritable in checking position, which is the position §1.3
/// says a bare constructor is *for*. The exception is exactly as wide as the
/// coincidence: the case wins only over the family the expected type already
/// names, and reading the word as that family is never what this position
/// wanted, because a family is a type and an element of one is what is being
/// checked. Any other declaration — a `let`, a function, a different family —
/// still declines, and a binder still declines first.
fn case_named(scope: &Scope, name: &Name, declared: &crate::kernel::family::Declared) -> Option<(Name, usize)> {
    let case = match name.split_once('.') {
        Some((family, case)) if family == &*declared.name => case,
        Some(_) => return None,
        None => {
            if scope.lookup(name).is_some() || answered_apart_from(scope, name, declared) {
                return None;
            }
            name
        }
    };
    declared
        .constructors
        .iter()
        .find(|constructor| *constructor.name == *case)
        .map(|constructor| (Arc::clone(&constructor.name), constructor.fields.len()))
}

/// Whether a declaration other than `declared` itself answers to `name`.
///
/// [`case_named`]'s "a declaration declined it" test, with the one coincidence
/// §1.3 allows taken out — see the note there.
fn answered_apart_from(scope: &Scope, name: &Name, declared: &crate::kernel::family::Declared) -> bool {
    match scope.declared(name) {
        Some(crate::kernel::family::Found::Rigid(constant)) => {
            !(constant.is_family() && *constant.name() == *declared.name)
        }
        Some(crate::kernel::family::Found::Recursor(..)) => true,
        None => false,
    }
}

/// A raw term as a head and the arguments the author wrote after it.
///
/// [`None`] where any of them is implicit: an author supplying an implicit
/// argument is telling the elaborator which binder they mean, and
/// [`Elaborator::constructed`] would be filling a different one.
///
/// Both application forms, because the question here is what the *author*
/// wrote and [`RawShape::Call`] is the form they wrote it in: `Succ(fewer)`
/// reaches [`Elaborator::constructed`] as one node with its arguments beside
/// it, and a reader that only knew the iterated [`RawShape::App`] would see a
/// head it could not name and leave the constructor to infer — which is
/// [`Refusal::BareConstructor`](crate::Refusal::BareConstructor), for a term
/// whose family the expected type was holding all along.
fn written_spine(raw: &Raw) -> Option<(&Raw, Vec<&Raw>)> {
    let mut arguments = Vec::new();
    let mut head = raw;
    loop {
        if let RawShape::App {
            filling,
            function,
            argument,
        } = head.shape()
        {
            if *filling != Filling::Written {
                return None;
            }
            arguments.push(argument);
            head = function;
            continue;
        }
        // Pushed in reverse because the whole vector is reversed below, which
        // is what lets the two forms nest in either order.
        if let RawShape::Call {
            function,
            arguments: written,
        } = head.shape()
        {
            arguments.extend(written.iter().rev());
            head = function;
            continue;
        }
        break;
    }
    arguments.reverse();
    Some((head, arguments))
}
