//! Namespaced definitions, and the type-directed disambiguation that reads them.
//!
//! `01-surface.md` §1.4 and §1.5, after prompt 146 deleted the trait system.
//! What the traits were being used for was never dispatch — the corpus had six
//! traits, 82 call sites, and **zero** trait-constrained signatures — it was
//! *naming*: `equal` at five types, `act` at two, `fold_from_end` at two. This
//! module is that, and nothing more.
//!
//! # A namespace is a dotted name, and that is the whole mechanism
//!
//! `impl Pitch { fn act(subject: Pitch, operation: Interval) -> Pitch { … } }`
//! declares a definition called `Pitch.act`. There is no table beside the
//! program, no instance, no dictionary, and no key: the definition is an
//! ordinary top-level definition whose name happens to contain a dot, and
//! [`Cx::definition`](crate::context::Cx::definition) finds it the way it finds
//! every other name.
//!
//! Three spellings reach it, and all three are the same lookup:
//!
//! - `Pitch::act(p, i)` — the head is written, so the name is complete.
//! - `p.act(i)` — the head is the receiver's type's, read by [`head_name`].
//! - `p up i` — the operator table's spelling of the line above.
//!
//! # Disambiguation is a filter, not a search
//!
//! A bare `act` with no head in hand is the only case that needs deciding, and
//! it is decided by the type the site already has rather than by trying each
//! candidate and keeping whichever elaborated. [`declaring`] answers which
//! namespaces spell a member, [`head_name`] answers what the site's type is
//! headed by, and the intersection is either empty, one, or several — the three
//! outcomes being a refusal naming the type, the answer, and a refusal listing
//! the candidates. Nothing is tried and undone, so nothing can be tried in a
//! different order and answer differently.

use std::sync::Arc;

use crate::scope::Scope;
use crate::term::{Name, Role, Shape, Term};
use crate::value::{Form, Head, Value};

/// The separator between a namespace and one of its members.
pub(crate) const DOT: char = '.';

/// `Head.member` — the name a namespaced definition binds.
pub(crate) fn qualified(head: &str, member: &str) -> Name {
    Arc::from(format!("{head}{DOT}{member}"))
}

/// The namespace and member a dotted name names, when it is dotted.
pub(crate) fn split(name: &str) -> Option<(&str, &str)> {
    name.rsplit_once(DOT)
}

/// The name at the rigid head of a type, when it has one.
///
/// `None` for a type headed by a variable, a metavariable, a binder, or a
/// definition that still has to unfold — none of which names a namespace, and
/// each of which is a use the caller refuses rather than guesses at. A
/// definition is on that list for a reason of its own: it unfolds, so reading a
/// namespace off one would give two namespaces for one type.
pub(crate) fn head_name(term: &Term) -> Option<Name> {
    match term.shape() {
        Shape::Named {
            name,
            role: Role::TypeConstructor | Role::Constructor | Role::Recursor(_) | Role::Base,
        } => Some(Arc::clone(name)),
        // An indexed type shares its namespace with the type it indexes: the
        // index is a refinement of the same type and not a second one.
        Shape::Indexed { ty, .. } => head_name(ty),
        Shape::App { function, .. } => head_name(function),
        Shape::Named {
            role: Role::Defined | Role::Builtin,
            ..
        }
        | Shape::Var(_)
        | Shape::Universe(_)
        | Shape::Bind { .. }
        | Shape::RecordType(_)
        | Shape::Record(_)
        | Shape::Project { .. }
        | Shape::Lit(_)
        | Shape::Meta(_) => None,
    }
}

/// Whether `head` names a type here, which is what makes a dotted name a
/// *namespaced* one.
///
/// The dot is not enough on its own, and this is the distinction that keeps a
/// module from becoming a namespace. `import "a.musa" as high` binds that
/// document's `rise` as `high.rise`, which splits exactly like `Pitch.act` and
/// means something else entirely: an alias qualifies a *name*, and a namespace
/// qualifies a name **within a type**. A bare `rise` must stay unresolved under
/// that import, which is what the alias is for, so the candidates are filtered
/// by whether the head is a type before they are filtered by anything else.
fn names_a_type(cx: &crate::context::Cx, head: &str) -> bool {
    if matches!(cx.extern_named(head), Some(crate::base::Extern::Base(_))) {
        return true;
    }
    matches!(
        cx.declared(head),
        Some(crate::family::Found::Rigid(constant)) if constant.is_family()
    )
}

/// Every namespace in scope that declares a member spelled `member`.
///
/// In scope order, most recent first, and deduplicated: two definitions of
/// `Pitch.act` cannot both be in scope — the later one shadows the earlier by
/// the ordinary rule — so a namespace appears once however many times it was
/// written.
pub(crate) fn declaring(scope: &Scope, member: &str) -> Vec<Name> {
    let cx = scope.cx();
    let mut found: Vec<Name> = Vec::new();
    for name in cx.defined_names() {
        let Some((head, spelled)) = split(&name) else {
            continue;
        };
        if spelled != member || !names_a_type(cx, head) {
            continue;
        }
        let head: Name = Arc::from(head);
        if !found.iter().any(|seen| **seen == *head) {
            found.push(head);
        }
    }
    found
}

/// The name at the rigid head of a *value*, when it has one.
///
/// [`head_name`]'s twin, one representation over, and it exists for the one site
/// that reads a head off a type it did not quote: a bare member spelling checked
/// against an expected type. `None` for every flexible or structural form —
/// a Π, a universe, a record, an unsolved meta — none of which names a
/// namespace, so a site headed by one fixes nothing and the spelling stays
/// ambiguous.
///
/// A folded definition answers `None` for [`head_name`]'s reason: it unfolds, so
/// reading a namespace off one would give two namespaces for one type.
pub(crate) fn head_of(ty: &Value) -> Option<Name> {
    let Form::Neutral(neutral) = &ty.form else {
        return None;
    };
    match &neutral.head {
        Head::Const(constant, _) => Some(constant.name()),
        Head::Base(base, _) => Some(Arc::clone(base.name())),
        Head::Var(..) | Head::Builtin(..) | Head::Meta(_) | Head::Def(..) => None,
    }
}
