//! What a bare name denotes: a binder, a declaration, a host item, a numeral.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::eval::eval;
use crate::family::Found;
use crate::level::Level;
use crate::origin::Origin;
use crate::refuse::{ElabError, Refusal};
use crate::scope::Scope;
use crate::term::{Name, Shape, Term};
use crate::value::{Env, Value};

use super::{Elaborator, Typed};

impl Elaborator {
    /// A name no binder in scope answers to, which a declaration may.
    ///
    /// Constants are looked up *after* binders rather than merged with them, so
    /// a local binder named `Nat` shadows the family — the ordinary rule, and the
    /// one an author expects from every other name in the language.
    ///
    /// A recursor gets its motive universe here, and here is the only place it
    /// can: §1.3 admits no universe polymorphism beyond level metavariables, so
    /// the level is one per *use site* and this is what a use site is.
    pub(super) fn constant(&mut self, scope: &Scope, here: Origin, name: &Name) -> Result<Typed, ElabError> {
        self.constant_at(scope, here, name, None)
    }

    /// The same, with the head the site fixes.
    ///
    /// `wanted` reaches exactly one rule — [`Self::member`]'s filter — and every
    /// other reading below ignores it. That is §1.5's scope for the expected
    /// type, stated as a signature: a binder, a declaration, a definition, and a
    /// registered name all resolve without asking what the site wants, and only
    /// a *bare member spelling* consults it.
    pub(super) fn constant_at(
        &mut self,
        scope: &Scope,
        here: Origin,
        name: &Name,
        wanted: Option<&Name>,
    ) -> Result<Typed, ElabError> {
        let Some(found) = scope.declared(name) else {
            // A top-level definition (§2.4), after declarations for the reason
            // declarations come after binders — the more local answer wins —
            // and before the two below because both of those are the host's
            // namespaces rather than the author's.
            if let Some(defined) = scope.cx().definition(name) {
                if let Some(module) = defined.hidden_from(scope.cx().module()) {
                    return Err(Refusal::Private {
                        name: Arc::clone(name),
                        module,
                        at: here,
                    }
                    .into());
                }
                let def = crate::program::one(defined);
                let ty = Value::clone(&def.ty());
                return Ok(Typed {
                    term: def.term(here),
                    ty,
                });
            }
            // A namespaced member before the general report, and only after
            // binders and declarations: `Pitch.act` lives in `Pitch`'s
            // namespace, so nothing here can shadow a name an author declared
            // themselves.
            if let Some(found) = Self::member(scope, here, name, wanted)? {
                return Ok(found);
            }
            // Last, and last on purpose: the host's registry is consulted only
            // where nothing the author wrote answers, so a declaration always
            // shadows a base type or a builtin of the same spelling rather than
            // the other way round. A registry that won would let the host
            // silently redefine a name in a program it never read.
            return self.registered(scope, here, name);
        };
        // Found, and possibly not for this reader. The check is here rather
        // than inside the lookup so that the answer is "private" and not "not
        // found" — see [`crate::visibility`] for why that distinction is the
        // whole of `01-surface.md` §1.3's value.
        if let Some(module) = found.hidden_from(scope.cx().module()) {
            return Err(Refusal::Private {
                name: Arc::clone(name),
                module,
                at: here,
            }
            .into());
        }
        // A recursor's motive universe rides on the use site; a bare reference
        // is always one whose goal is an ordinary type. The match compiler
        // computes its own from the goal (`case.rs`) and never comes through
        // here.
        let level = Level::ZERO;
        let constant = found.at(level);
        let ty = constant.ty(&mut self.meter)?;
        Ok(Typed {
            term: constant.term(here),
            ty,
        })
    }

    /// A bare member spelling, disambiguated by the type the site already has.
    ///
    /// `01-surface.md` §1.5's rule, and the whole of what replaced instance
    /// lookup. [`declaring`](crate::namespace::declaring) answers which
    /// namespaces spell `member`; `wanted` is the rigid head of the type the
    /// site is checking against, when the site has one. Three outcomes:
    ///
    /// - **No namespace spells it.** `Ok(None)`, so the caller goes on to the
    ///   host's registry — a bare name that is not a member is not this rule's
    ///   business.
    /// - **One survives the filter.** That is the answer, and it is the same
    ///   definition `Head::member` would have named.
    /// - **Several survive.** [`Refusal::AmbiguousMethod`] listing every
    ///   candidate, which is strictly more than "no instance found" said: the
    ///   reader is told what the choices were and can write the qualified path.
    ///
    /// Nothing is elaborated in order to be discarded. The filter compares two
    /// names, so the candidates cannot be reordered into a different answer.
    /// No `self`, for [`Self::namespaced`]'s reason and with more force: the
    /// whole rule is a comparison of names, so it needs no meter, no holes, and
    /// no elaborator. A disambiguation that needed one would be a search.
    pub(super) fn member(
        scope: &Scope,
        here: Origin,
        member: &Name,
        wanted: Option<&Name>,
    ) -> Result<Option<Typed>, ElabError> {
        let candidates = crate::namespace::declaring(scope, member);
        if candidates.is_empty() {
            return Ok(None);
        }
        let narrowed: Vec<Name> = match wanted {
            Some(head) => candidates
                .iter()
                .filter(|candidate| ***candidate == **head)
                .map(Arc::clone)
                .collect(),
            None => candidates.clone(),
        };
        let [head] = narrowed.as_slice() else {
            // An empty narrowing is still ambiguous rather than absent: the
            // member exists, the expected type ruled every spelling out, and
            // the reader needs to see the list either way.
            return Err(Refusal::AmbiguousMethod {
                at: here,
                head: wanted.map_or_else(|| Arc::clone(member), Arc::clone),
                method: Arc::clone(member),
                candidates,
            }
            .into());
        };
        Self::namespaced(scope, here, &crate::namespace::qualified(head, member))
    }

    /// A registered builtin or base type, and nothing else.
    fn registered(&mut self, scope: &Scope, here: Origin, name: &Name) -> Result<Typed, ElabError> {
        let Some(entry) = scope.cx().extern_named(name) else {
            return self.unresolved(scope, here, name);
        };
        let ty = eval(&mut self.meter, &Env::EMPTY, entry.ty())?;
        Ok(Typed {
            term: entry.term(here),
            ty,
        })
    }

    /// [`RawShape::Hosted`]: a name the *reader* wrote, resolved in the host's
    /// namespaces only.
    ///
    /// Declarations then the registry, and neither binders nor top-level
    /// definitions. [`Self::constant`]'s order is the author's — the more local
    /// answer wins, so a program may name a value `sounded` and mean its own —
    /// and this one is the reader's, which is why the two orders differ. A
    /// desugaring that went through the author's namespace would let an
    /// ordinary binding change what `music { c5/1 }` means, and would make two
    /// definitions that never mentioned each other look like a cycle.
    pub(super) fn hosted(&mut self, scope: &Scope, here: Origin, name: &Name) -> Result<Typed, ElabError> {
        if scope.declared(name).is_some() {
            return self.constant(scope, here, name);
        }
        // A namespaced member, for the reason [`Self::constant`] gives: it
        // lives in its type's namespace, which is the host's here as well.
        if let Some(found) = Self::member(scope, here, name, None)? {
            return Ok(found);
        }
        self.registered(scope, here, name)
    }

    /// [`RawShape::Numeral`]: a number at the family the reader named.
    ///
    /// Resolved in the host's namespaces, like [`Self::hosted`] and for the same
    /// reason: the family is the reader's word, so a binding that happens to
    /// spell `Nat` cannot change what a written number means. It infers rather
    /// than checks, because the raw term already says which type it is at — the
    /// same argument [`RawShape::Lit`] makes, one namespace over.
    pub(super) fn numeral(
        &mut self,
        scope: &Scope,
        here: Origin,
        family: &Name,
        count: u64,
    ) -> Result<Typed, ElabError> {
        let Some(Found::Rigid(constant)) = scope.declared(family) else {
            return self.unresolved(scope, here, family);
        };
        let Some(reason) = constant.uncounted() else {
            return Ok(Typed {
                term: Term::numeral(here, &constant, count),
                ty: constant.value(here),
            });
        };
        Err(Refusal::NotANumeralFamily {
            at: here,
            name: Arc::clone(family),
            reason,
        }
        .into())
    }

    /// Why a name resolved to nothing, as precisely as the context can say.
    ///
    /// Three different mistakes wear the same spelling, and telling them apart
    /// is the whole value of the report. `Tying.Tied` reached a namespace that
    /// exists and has no such case; a bare `Untied` is a case of something, and
    /// this position does not say of what; anything else is a name nobody
    /// declared.
    fn unresolved(&mut self, scope: &Scope, here: Origin, name: &Name) -> Result<Typed, ElabError> {
        if let Some(family) = scope.cx().stranger(name) {
            let ty = family.ty_term(&mut self.meter)?;
            return Err(Refusal::NoSuchConstructor {
                at: here,
                name: Arc::clone(name),
                ty,
                cases: family.cases(),
            }
            .into());
        }
        let families = scope.cx().cases(name);
        // §2.1's uniqueness rule: exactly one family in scope declares the
        // case, so the bare name *is* that constructor — written without its
        // family because there is nothing to disambiguate against. Its
        // parameters are holes for the position to solve, which is what makes
        // `fold(Nothing, step)` an ordinary call rather than a guessing game.
        // Two families declaring the case is the ambiguity the qualified
        // spelling exists for, and that stays a refusal.
        if let [only] = families.as_slice() {
            let qualified: Name = Arc::clone(only);
            let built = self.constant(scope, here, &qualified)?;
            let params = match &built.term.shape() {
                Shape::Const(constant) => constant.group.params(),
                Shape::Hole(_)
                | Shape::Var(_)
                | Shape::Def(_)
                | Shape::Numeral(_)
                | Shape::Base(_)
                | Shape::Lit(_)
                | Shape::Builtin(_)
                | Shape::Universe(_)
                | Shape::Pi { .. }
                | Shape::Lam { .. }
                | Shape::App { .. }
                | Shape::RecordType(_)
                | Shape::Record(_)
                | Shape::Project { .. }
                | Shape::Indexed { .. }
                | Shape::Let { .. } => 0,
            };
            return self.holes(scope, here, built, params);
        }
        if !families.is_empty() {
            return Err(Refusal::BareConstructor {
                at: here,
                name: Arc::clone(name),
                families,
            }
            .into());
        }
        Err(Refusal::UnknownName {
            name: Arc::clone(name),
            at: here,
            candidates: scope.nameable(),
        }
        .into())
    }
}
