//! What a bare name denotes: a binder, a declaration, a host item, a numeral.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::elaboration::refuse::{ElabError, Refusal};
use crate::kernel::eval::eval;
use crate::kernel::family::Found;
use crate::kernel::origin::Origin;
use crate::kernel::scope::Scope;
use crate::kernel::sort::Sort;
use crate::kernel::term::{Definition, Name, Shape, Term};
use crate::kernel::value::{Env, Form, Value};

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
            // The `rec` whose body this is, before everything below it: a
            // recursion is more local than any declaration, and its name is one
            // the author bound. Prompt 155a — the recursive call *is* a use of
            // the definition the `rec` was lifted to.
            if self.recursion(name).is_some() {
                return self.recursive_use(scope, here, name);
            }
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
                let def = crate::kernel::program::one(&defined);
                return self.used(scope, here, &def);
            }
            // A namespaced member before the general report, and only after
            // binders and declarations: `Pitch.act` lives in `Pitch`'s
            // namespace, so nothing here can shadow a name an author declared
            // themselves.
            if let Some(found) = self.member(scope, here, name, wanted)? {
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
        // found" — see [`crate::kernel::visibility`] for why that distinction is the
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
        let level = Sort::ZERO;
        let constant = found.at(level);
        let ty = constant.ty(&mut self.meter, scope.cx().globals())?;
        Ok(Typed {
            term: constant.term(here),
            ty,
        })
    }

    /// A bare member spelling, disambiguated by the type the site already has.
    ///
    /// `01-surface.md` §1.5's rule, and the whole of what replaced instance
    /// lookup. [`declaring`](crate::elaboration::namespace::declaring) answers which
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
    /// The disambiguation itself needs no elaborator: the whole rule is a
    /// comparison of names, so it spends no meter and creates no unknown. Only
    /// the answer does, because naming a definition instantiates its level
    /// parameters.
    pub(super) fn member(
        &mut self,
        scope: &Scope,
        here: Origin,
        member: &Name,
        wanted: Option<&Name>,
    ) -> Result<Option<Typed>, ElabError> {
        let candidates = crate::elaboration::namespace::declaring(scope, member);
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
        self.namespaced(scope, here, &crate::elaboration::namespace::qualified(head, member))
    }

    /// The recursion in progress, used: the lifted definition, applied to the
    /// context the lift abstracted.
    ///
    /// This is what a recursive call elaborates to, and the whole of what
    /// replaced the `#ih` rewrite. The definition stands at
    /// [`Body::Pending`](crate::kernel::program::Body) while its own body is
    /// read, so the spine is rigid: it type-checks, it carries the declared
    /// type, and it cannot compute — which is exactly what elaborating a
    /// recursive body needs and nothing more. Whether the call *descends* is
    /// not asked here; [`crate::kernel::terminate`] asks it of the finished
    /// tree.
    ///
    /// De Bruijn indices are relative, so the context binders are named from
    /// *this* depth: binder `p` of a context of `n` is `depth - 1 - p` steps
    /// out, wherever inside the body the call was written.
    fn recursive_use(&mut self, scope: &Scope, here: Origin, name: &Name) -> Result<Typed, ElabError> {
        let Some(recursion) = self.recursion(name) else {
            return self.unresolved(scope, here, name);
        };
        let global = Arc::clone(&recursion.global);
        let context: Vec<Value> = recursion.context.clone();
        let Some(defined) = scope.cx().definition(&global) else {
            return self.unresolved(scope, here, name);
        };
        let def = crate::kernel::program::one(&defined);
        let mut typed = self.used(scope, here, &def)?;
        let depth = scope.depth().0;
        for (position, argument) in context.into_iter().enumerate() {
            let position = u32::try_from(position).unwrap_or(u32::MAX);
            let opened = crate::kernel::eval::opened(&mut self.meter, &typed.ty)?;
            let ty = opened.as_ref().unwrap_or(&typed.ty);
            let Form::Pi { codomain, .. } = &ty.form else {
                return Err(Refusal::UncheckedRecursion {
                    at: here,
                    name: Arc::clone(name),
                }
                .into());
            };
            let codomain = codomain.clone();
            let steps_out = depth.saturating_sub(position).saturating_sub(1);
            typed = Typed {
                term: Term::app(here, typed.term, Term::var(here, crate::kernel::term::Index(steps_out))),
                ty: crate::kernel::eval::apply_closure(&mut self.meter, &codomain, argument)?,
            };
        }
        Ok(typed)
    }

    /// A top-level definition, used: its level parameters instantiated at
    /// fresh unknowns, and its type read at those.
    ///
    /// The one place a definition becomes a term, so the one place §1's "and a
    /// use site instantiates them" is spelled. A definition with no level
    /// parameters — which is every definition the language wrote before the
    /// hierarchy had any — takes the stored type unchanged and costs exactly
    /// what it cost before.
    pub(super) fn used(
        &mut self,
        scope: &Scope,
        here: Origin,
        def: &crate::kernel::program::Def,
    ) -> Result<Typed, ElabError> {
        let levels = self.instantiate_levels(here, def.levels().len());
        let (ty, _) = def.instance(&mut self.meter, scope.cx().globals(), &levels)?;
        Ok(Typed {
            term: def.term(here, levels),
            ty: Value::clone(&ty),
        })
    }

    /// A registered builtin or base type, and nothing else.
    fn registered(&mut self, scope: &Scope, here: Origin, name: &Name) -> Result<Typed, ElabError> {
        let Some(entry) = scope.cx().extern_named(name) else {
            return self.unresolved(scope, here, name);
        };
        let ty = eval(&mut self.meter, &Env::under(scope.cx().globals().clone()), entry.ty())?;
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
        if let Some(found) = self.member(scope, here, name, None)? {
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
                ty: constant.value(here, scope.cx().globals()),
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
            let ty = family.ty_term(&mut self.meter, scope.cx().globals())?;
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
        // parameters are metas for the position to solve, which is what makes
        // `fold(Nothing, step)` an ordinary call rather than a guessing game.
        // Two families declaring the case is the ambiguity the qualified
        // spelling exists for, and that stays a refusal.
        if let [only] = families.as_slice() {
            let qualified: Name = Arc::clone(only);
            let built = self.constant(scope, here, &qualified)?;
            let params = match built.term.shape() {
                Shape::Named { name, role, levels } => match scope.cx().globals().definition(name, role, levels) {
                    Definition::Declared(constant) => constant.group.params(),
                    Definition::Undeclared
                    | Definition::Defined(_)
                    | Definition::Compiled(_)
                    | Definition::Base(_)
                    | Definition::Builtin(_) => 0,
                },
                Shape::Meta(_)
                | Shape::Var(_)
                | Shape::Lit(_)
                | Shape::Universe(_)
                | Shape::Bind { .. }
                | Shape::App { .. } => 0,
            };
            return self.metas(scope, here, built, params);
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
