//! Base types, literals, and compiler-owned builtins.
//!
//! `docs/rules/language/02-core-calculus.md` §5.8 admits a **conservative
//! extension** to §1's calculus: a base type with no eliminator, together with a
//! finite set of builtins over the extended base set. §1's grammar is the pure
//! core and says nothing about either, because neither is part of it — they are
//! what the extension adds, and §5.8 re-derives §5's matrix over the result.
//!
//! **The core owns the mechanism; the host owns the table.** Nothing in this
//! crate knows what a pitch is, and nothing may learn. §5.8's corollary is the
//! reason: "A later musical domain needs no new proof — it needs a base type
//! with no eliminator, builtin signatures containing no arrow, and a discharge
//! of D1–D4." A core that enumerated the base types would make every new domain
//! a core amendment and falsify that corollary outright. So a [`Registry`] is
//! built by the caller, checked once, and carried on [`Cx`](crate::Cx) beside
//! the declared groups and the trait table, which are already exactly this
//! shape.
//!
//! **Three things, each with one job.**
//!
//! - A [`Base`] is a base type: a name and the kind it inhabits. It has no
//!   constructor, no eliminator, and contributes no ι-rule. That is D1
//!   *inertness* stated as a representation rather than as a rule to obey —
//!   there is no arm anywhere in this crate that inspects a base type's value,
//!   because there is nothing to inspect it with.
//! - A [`Literal`] is a closed value of a base type, opaque to the core.
//!   Conversion is §5.8's own sentence: two closed values of an inert base type
//!   are convertible iff they are the same constant.
//! - A [`Builtin`] is a compiler-owned operation: a name, a declared type, a
//!   [`Family`], and a δ-rule. Its *typing* needs no new rule — a builtin is a
//!   constant of a declared type and application is application — so the only
//!   new arm anywhere is reduction.
//!
//! **A literal's payload is opaque, and the alternative was refused.** The core
//! needs three things from a literal and no more: which type it inhabits,
//! whether it is the same literal as another, and how to show it in a
//! diagnostic. A closed payload universe — integer, rational, text, bytes —
//! would answer all three without `dyn`, and was refused twice over: `Syntax` is
//! a tree, so every phase operation would encode and every read would decode;
//! and a list of the host's data shapes inside a leaf calculus is the
//! enumeration the corollary forbids, wearing a different hat. [`Payload`] has
//! three methods with one reason each, and the downcast is the host's own
//! concern at the host's own δ-rule.
//!
//! **D3 is enforced by the type.** A δ-rule is a `fn` pointer and not a closure,
//! so it cannot capture host state and "the result is a function of the argument
//! values alone" is checked by the Rust compiler rather than reviewed by a
//! reader. A rule that wants ambient context cannot be written down.
//!
//! **Which half of D1–D4 is checked here.** [`Registry::new`] checks what the
//! signature makes visible: every name registered once, no arrow anywhere in a
//! δ-builtin's signature, and every base type a δ signature mentions registered
//! as a base type. D2 totality, D3's order-independence, and D4's size bound are
//! properties of the host's *functions over the host's domains*, and the host's
//! own law suite samples them where the table lives — `crates/musa-compiler`'s
//! `BUILTIN_OWNERSHIP` suite, since prompt 127ca. Do not look for D2 here; it is
//! not checkable from a signature.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::origin::Origin;
use crate::refuse::Refusal;
use crate::term::{Name, Shape, Term};

/// A base type: a name, and the kind it inhabits.
///
/// The kind is a term rather than a level because a base type may take
/// parameters — `Syntax` is indexed by its category and `EventTrack` by its time
/// and payload — so `Nat : Type 0` and `Syntax : Type 0 → Type 0` are both base
/// types and only a type can tell them apart. A base type with parameters is
/// still inert: it is applied, never eliminated.
///
/// Cheap to clone: everything is behind one [`Arc`], because a base type is
/// named at every occurrence of its type and copied into every value of it.
#[derive(Clone, Debug)]
pub struct Base(Arc<BaseDeclaration>);

#[derive(Debug)]
struct BaseDeclaration {
    name: Name,
    kind: Term,
}

/// Two base types are the same when they have the same name.
///
/// By name, not by pointer, for the reason [`crate::family::Constant`] gives:
/// one host may build its registry twice — a second `Cx`, a second compilation —
/// and two closed values of one base type have to stay convertible across that.
/// A registry refuses a duplicate name, so within one registry the name decides.
impl PartialEq for Base {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.name == other.0.name
    }
}

impl Eq for Base {}

impl fmt::Display for Base {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0.name)
    }
}

impl Base {
    /// A base type named `name`, inhabiting `kind`.
    ///
    /// `kind` is `Type l` for an unparameterized base type and a Π ending in one
    /// for a parameterized one. It is read in the empty context: a base type is
    /// closed, which is what lets [`Registry`] check a signature without an
    /// environment.
    #[must_use]
    pub fn new(name: impl Into<Name>, kind: Term) -> Self {
        Self(Arc::new(BaseDeclaration {
            name: name.into(),
            kind,
        }))
    }

    /// Its name.
    #[must_use]
    pub fn name(&self) -> &Name {
        &self.0.name
    }

    /// The kind it inhabits.
    #[must_use]
    pub fn kind(&self) -> &Term {
        &self.0.kind
    }

    /// This base type as a term.
    #[must_use]
    pub fn term(&self, origin: Origin) -> Term {
        Term::new(origin, Shape::Base(self.clone()))
    }
}

/// A literal's value, owned by the host and opaque to the core.
///
/// Three methods, one reason each, and no fourth: [`Self::same`] is what
/// conversion at a base type calls, [`Self::shown`] is what a diagnostic prints,
/// and [`Self::as_any`] is how a host's own δ-rule reads back what it put in.
/// Nothing in this crate downcasts.
///
/// `Send + Sync` because a literal rides inside a [`Term`], and terms cross
/// threads wherever the host compiles more than one document at once.
pub trait Payload: fmt::Debug + Send + Sync + 'static {
    /// Whether this is the same value as `other`.
    ///
    /// The host decides, because the host knows what equality means for its own
    /// data. It is called only on two payloads of the *same* base type — the
    /// core compares types first — so an implementation may downcast to its own
    /// type and answer `false` when the downcast fails.
    ///
    /// It must be an equivalence relation. Conversion is decidable only if this
    /// is, and §5's matrix rests on that.
    fn same(&self, other: &dyn Payload) -> bool;

    /// How to show this value in a diagnostic.
    fn shown(&self) -> String;

    /// This value, for the host's own δ-rule to downcast.
    fn as_any(&self) -> &dyn Any;
}

/// A closed value of a base type.
///
/// It carries its whole type rather than just its [`Base`], because a
/// parameterized base type's literals are at different types: a `Syntax Expr` is
/// not a `Syntax Pattern`, and conversion has to be able to say so before it
/// reaches [`Payload::same`]. The type is closed, so it needs no environment.
#[derive(Clone, Debug)]
pub struct Literal {
    ty: Term,
    payload: Arc<dyn Payload>,
}

/// Two literals are the same when they are at the same type and the host says
/// their payloads agree.
///
/// This is §5.8's sentence and nothing more: "an inert base type contributes no
/// ι-rule, so two closed values of it are convertible iff they are the same
/// constant."
impl PartialEq for Literal {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty && self.payload.same(other.payload.as_ref())
    }
}

impl Eq for Literal {}

impl fmt::Display for Literal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.payload.shown())
    }
}

impl Literal {
    /// A literal of type `ty` carrying `payload`.
    ///
    /// `ty` is a closed type headed by a [`Base`]. Nothing checks that here: a
    /// literal whose type is not a base type is a host defect, and the
    /// re-checker is where a term the host built wrong is caught.
    #[must_use]
    pub fn new(ty: Term, payload: Arc<dyn Payload>) -> Self {
        Self { ty, payload }
    }

    /// Its type.
    #[must_use]
    pub fn ty(&self) -> &Term {
        &self.ty
    }

    /// Its value, for the host that put it there.
    #[must_use]
    pub fn payload(&self) -> &dyn Payload {
        self.payload.as_ref()
    }

    /// This literal as a term.
    #[must_use]
    pub fn term(&self, origin: Origin) -> Term {
        Term::new(origin, Shape::Lit(self.clone()))
    }
}

/// Which of §5.8's four families a builtin belongs to.
///
/// Disjoint and exhaustive is a checked law over there, where the table is; here
/// it is one field, so a builtin cannot be classified twice and cannot be
/// classified none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    /// Every argument type and the result type is a base type or a finite
    /// constructor over base types, with no arrow anywhere in the signature.
    /// D1–D4 apply to exactly these.
    Delta,
    /// A generated recursor or a derived traversal over one.
    Eliminator,
    /// A constructor or controlled transform of §5.7's event tracks.
    Track,
    /// A constructor of `../across-stages/03-machine-calculus.md` §2's machines.
    Machine,
}

impl fmt::Display for Family {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match *self {
            Self::Delta => "δ-builtin",
            Self::Eliminator => "structural eliminator",
            Self::Track => "track builtin",
            Self::Machine => "machine builtin",
        })
    }
}

/// What a builtin does to its arguments.
///
/// A `fn` pointer rather than a boxed closure, and that is D3 rather than a
/// micro-optimization: a function pointer cannot capture host state, so "the
/// result is a function of the argument values alone, with no ambient context,
/// evaluation-order dependence, hash-iteration order, or diagnostic emission" is
/// a property of the type rather than a promise a reader has to audit.
///
/// `None` means *this rule does not apply to these arguments*, and the
/// application stays a neutral spine. For a δ-builtin at closed literal
/// arguments of its declared types that answer is a host defect, which D2
/// forbids and [`crate::Refusal::BuiltinStuck`] reports.
pub type Rule = fn(&[&Literal]) -> Option<Literal>;

/// A compiler-owned operation: what it is called, what type it has, which family
/// it belongs to, and what it computes.
///
/// Cheap to clone for the same reason [`Base`] is.
#[derive(Clone, Debug)]
pub struct Builtin(Arc<BuiltinDeclaration>);

#[derive(Debug)]
struct BuiltinDeclaration {
    name: Name,
    ty: Term,
    family: Family,
    arity: usize,
    rule: Rule,
}

/// Two builtins are the same when they have the same name, for the reason
/// [`Base`]'s equality gives.
impl PartialEq for Builtin {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.name == other.0.name
    }
}

impl Eq for Builtin {}

impl fmt::Display for Builtin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0.name)
    }
}

impl Builtin {
    /// A builtin named `name`, of type `ty`, in `family`, computed by `rule`.
    ///
    /// The arity is read off `ty` rather than passed: it is the number of Π
    /// binders the signature has, and a second copy would be free to disagree
    /// with the first. `ty` is read in the empty context, because a builtin is
    /// closed.
    #[must_use]
    pub fn new(name: impl Into<Name>, ty: Term, family: Family, rule: Rule) -> Self {
        let arity = arity_of(&ty);
        Self(Arc::new(BuiltinDeclaration {
            name: name.into(),
            ty,
            family,
            arity,
            rule,
        }))
    }

    /// Its name.
    #[must_use]
    pub fn name(&self) -> &Name {
        &self.0.name
    }

    /// Its declared type.
    #[must_use]
    pub fn ty(&self) -> &Term {
        &self.0.ty
    }

    /// Which family it belongs to.
    #[must_use]
    pub fn family(&self) -> Family {
        self.0.family
    }

    /// How many arguments it takes before it can compute.
    #[must_use]
    pub fn arity(&self) -> usize {
        self.0.arity
    }

    /// This builtin as a term.
    #[must_use]
    pub fn term(&self, origin: Origin) -> Term {
        Term::new(origin, Shape::Builtin(self.clone()))
    }

    /// What it computes at `arguments`, or `None` when it does not apply.
    ///
    /// Called only at exactly [`Self::arity`] arguments, every one of them a
    /// literal: an application short of the arity, or one whose argument has not
    /// reduced to a literal, is a blocked spine and never reaches here.
    pub(crate) fn reduce(&self, arguments: &[&Literal]) -> Option<Literal> {
        (self.0.rule)(arguments)
    }
}

/// How many Π binders a signature has.
fn arity_of(ty: &Term) -> usize {
    let mut arity = 0usize;
    let mut rest = ty;
    while let Shape::Pi { codomain, .. } = rest.shape() {
        arity = arity.saturating_add(1);
        rest = codomain;
    }
    arity
}

/// A name the host has registered: a base type or a builtin.
///
/// One enum rather than two lookups, because a caller that resolves a name wants
/// to know what it found in one question, and because a name may be either but
/// never both.
#[derive(Clone, Debug)]
pub enum Extern {
    /// A base type.
    Base(Base),
    /// A compiler-owned operation.
    Builtin(Builtin),
}

impl Extern {
    /// Its type: the kind for a base type, the signature for a builtin.
    #[must_use]
    pub fn ty(&self) -> &Term {
        match *self {
            Self::Base(ref base) => base.kind(),
            Self::Builtin(ref builtin) => builtin.ty(),
        }
    }

    /// This name as a term.
    #[must_use]
    pub fn term(&self, origin: Origin) -> Term {
        match *self {
            Self::Base(ref base) => base.term(origin),
            Self::Builtin(ref builtin) => builtin.term(origin),
        }
    }
}

/// The host's base types and builtins, checked once and then immutable.
///
/// Immutable on purpose, and the alternative is worth naming because it is the
/// obvious one: a registry that interned new literals as evaluation produced
/// them would need interior mutability, and a table that changes while a term is
/// being normalized is an evaluation-order dependence — the exact D3 this module
/// exists to enforce. A [`Literal`] therefore carries its own payload and no
/// table is consulted to compare two.
#[derive(Debug, Default)]
pub struct Registry {
    names: HashMap<Name, Extern>,
}

impl Registry {
    /// The registry holding `bases` and `builtins`, or why it is not one.
    ///
    /// Checks the half of §5.8 a signature makes visible, and says in the module
    /// doc which half that is:
    ///
    /// - **one name, one meaning** — no name registered twice, whether as two
    ///   base types, two builtins, or one of each;
    /// - **no arrow in a δ signature** (D1) — a δ-builtin's argument and result
    ///   types hold no Π anywhere, at any depth, which is what makes a δ-builtin
    ///   a first-order operation over data rather than a higher-order one;
    /// - **δ arguments are over registered base types** — every [`Base`] a δ
    ///   signature mentions is a base type of this registry, so "its base types
    ///   have no eliminator" is true because they are inert here rather than
    ///   because someone checked elsewhere.
    ///
    /// # Errors
    ///
    /// [`Refusal::DuplicateExtern`], [`Refusal::HigherOrderDelta`], or
    /// [`Refusal::UnknownBase`].
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
        Ok(registry)
    }

    /// What `name` names here, if anything.
    #[must_use]
    pub fn named(&self, name: &str) -> Option<&Extern> {
        self.names.get(name)
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
                self.check_first_order(builtin, &argument)?;
            }
        }
        Ok(())
    }

    /// One argument or result type of a δ-builtin: no arrow at any depth, and
    /// every base type it names registered here.
    fn check_first_order(&self, builtin: &Builtin, ty: &Term) -> Result<(), Refusal> {
        let mut pending = vec![ty.clone()];
        while let Some(part) = pending.pop() {
            match part.shape() {
                Shape::Pi { .. } => {
                    return Err(Refusal::HigherOrderDelta {
                        name: Arc::clone(builtin.name()),
                        at: builtin.ty().origin(),
                    });
                }
                Shape::Base(base) => {
                    if self.named(base.name()).is_none() {
                        return Err(Refusal::UnknownBase {
                            name: Arc::clone(base.name()),
                            at: part.origin(),
                        });
                    }
                }
                Shape::App { function, argument } => {
                    pending.push(function.clone());
                    pending.push(argument.clone());
                }
                Shape::RecordType(fields) | Shape::Record(fields) => {
                    pending.extend(fields.iter().map(|field| field.term.clone()));
                }
                Shape::Project { record, .. } => pending.push(record.clone()),
                Shape::Lam { body, .. } => pending.push(body.clone()),
                Shape::Refl(witness) => pending.push(witness.clone()),
                Shape::Id { ty, left, right } => {
                    pending.push(ty.clone());
                    pending.push(left.clone());
                    pending.push(right.clone());
                }
                Shape::J {
                    ty,
                    from,
                    motive,
                    base,
                    to,
                    proof,
                } => {
                    pending.extend([
                        ty.clone(),
                        from.clone(),
                        motive.clone(),
                        base.clone(),
                        to.clone(),
                        proof.clone(),
                    ]);
                }
                Shape::Let { ty, value, body, .. } => {
                    pending.extend([ty.clone(), value.clone(), body.clone()]);
                }
                // A variable, a universe, a declared constant, another builtin,
                // a literal, or a metavariable: none of them is an arrow and
                // none of them holds one.
                Shape::Var(_)
                | Shape::Universe(_)
                | Shape::Const(_)
                | Shape::Builtin(_)
                | Shape::Lit(_)
                | Shape::Meta(_) => {}
            }
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

/// A signature's argument types and its result type, in order.
fn signature_parts(ty: &Term) -> Vec<Term> {
    let mut parts = Vec::new();
    let mut rest = ty.clone();
    while let Shape::Pi { domain, codomain, .. } = rest.shape() {
        parts.push(domain.clone());
        let next = codomain.clone();
        rest = next;
    }
    parts.push(rest);
    parts
}
