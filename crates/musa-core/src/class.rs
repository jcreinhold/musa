//! Traits and their instances: `docs/rules/language/10-traits.md`, elaborated.
//!
//! The module is named for Wadler and Blott's *type class* because `trait` is a
//! Rust keyword and this file would otherwise have to be spelled `r#trait.rs`.
//! Everything it holds is a trait under Musa's name for it.
//!
//! # Nothing here reaches the core
//!
//! §1's claim — "a trait is a record of methods, an impl is a value of that
//! record" — is implemented literally, and that is what keeps `lib.rs`'s promise
//! that the core is complete. A trait elaborates to an ordinary **term**:
//!
//! ```text
//! trait Eq<A> { fn equal(x: A, y: A) -> Bool; }
//!   ⇝   Eq = λ(A : Type ℓ). { equal : A → A → Bool }
//! ```
//!
//! so the type of an `Eq<τ>` dictionary is the application `Eq τ`, which β-
//! reduces to the record type by the evaluator that was already there. No new
//! `Term` shape, no new conversion rule, no unfolding step written for traits.
//! An impl is a record literal checked against that type, and a use site is a
//! projection out of whichever dictionary lookup found. The whole mechanism is
//! elaboration.
//!
//! # Lookup is a table read, and the types make it one
//!
//! §4 is three steps and no backtracking, and §9's first row refuses search
//! outright — so the code has to be shaped so that search is not expressible,
//! not merely not written. [`Key`] is that shape: a `(trait, head constructor)`
//! pair with `Eq`/`Hash`, and [`Instances`] is a map from it. There is no
//! candidate list to filter, no ordering to break a tie with, and nothing to
//! retry, because a failed lookup returns `None` and `None` is the refusal.
//!
//! A **local** dictionary — one bound by an enclosing `where` — is found first
//! (§4 step 1), and its head may be a type *variable* rather than a constructor,
//! which is why [`Head`] has two arms. Under coherence the two lookups can never
//! disagree; what the rule buys is determinacy, so that instantiating a
//! parameter later cannot reroute a call that was already elaborated.
//!
//! # What is checked where
//!
//! At the **declaration**, and never at a use site: coherence ([`Refusal::DuplicateInstance`]),
//! the orphan rule ([`Refusal::OrphanInstance`]), the termination measure
//! ([`Refusal::UnboundedInstance`]), a trait parameter its head cannot determine,
//! a missing method, a replaced derived method, and a hand-written `Storable`.
//! §4 gives the reason and it is the same one every time: a use site is the
//! wrong place to learn that a library cannot answer, because the author reading
//! the message is not the author who can fix it.

use std::collections::HashMap;
use std::sync::Arc;

use crate::family::Binder;
use crate::origin::Origin;
use crate::term::{DbLevel, Name, Term};

/// The package a declaration was written in, as the caller numbers it.
///
/// `Origin`'s bargain a third time, after [`ModuleId`]: a number the caller
/// assigns, compared only for equality and never interpreted. §3's orphan rule
/// is the one question in this crate that needs a boundary *wider* than a
/// module — an impl belongs with its trait or with its head type, and those live
/// in the same package as a rule but rarely in the same module — and this is the
/// least this crate can learn in order to ask it.
///
/// `None` on a context is inside every package, which is what leaves every
/// caller that has no packages unchanged, exactly as [`ModuleId`]'s absence
/// does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackageId(u32);

impl PackageId {
    /// The package the caller numbers `id`.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

/// What a constraint's first argument is headed by.
///
/// The two arms are §4's two lookups. A [`Self::Rigid`] head is a declared type
/// constructor and keys the global table; a [`Self::Local`] head is a type
/// *variable*, which no global instance can ever be declared for and which only
/// an enclosing `where` can discharge. A head that is neither — a metavariable —
/// is not representable here on purpose: that constraint is postponed rather
/// than looked up, and a `Head` value would be a guess about which lookup it
/// will eventually take.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Head {
    /// A declared constant: `List`, `Tying`, `Ratio`.
    Rigid(Name),
    /// A variable in scope, by the de Bruijn level it stands at.
    Local(DbLevel),
}

/// What a lookup is keyed on: a trait and the head its first argument has.
///
/// §1: "A trait has one or more parameters, and the first is the head." Later
/// parameters are determined by the instance rather than searched for, so they
/// are deliberately *not* in the key — putting them there would make lookup a
/// search over a product of heads, which is §9's last-but-two refusal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    /// The trait's name.
    pub(crate) class: Name,
    /// The head of its first argument.
    pub(crate) head: Head,
}

impl Key {
    /// The key a constraint on `class` at a rigid head has.
    pub(crate) fn rigid(class: &Name, head: &Name) -> Self {
        Self {
            class: Arc::clone(class),
            head: Head::Rigid(Arc::clone(head)),
        }
    }
}

/// One elaborated constraint: a trait, its arguments, and where it was written.
///
/// The arguments are core terms read under whatever binders the constraint was
/// written under — a trait's own context under its parameters, an instance's
/// under the instance parameters, a function's `where` under its type
/// parameters.
#[derive(Clone, Debug)]
pub(crate) struct Constraint {
    /// Where it was written.
    pub(crate) origin: Origin,
    /// The trait's name.
    pub(crate) class: Name,
    /// One argument per trait parameter.
    pub(crate) args: Arc<[Term]>,
}

/// Which of §1's two kinds a method is, and how a use site reaches it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A field of the dictionary: a use projects it out.
    Required,
    /// A function of the dictionary: a use applies it.
    Derived,
}

/// One method a trait derives: a definition, written once, that an impl may not
/// replace.
///
/// Its own parameters and constraints are carried beside the type rather than
/// only inside it, because a use site fills the two by different mechanisms: a
/// parameter becomes a metavariable and a constraint goes through §4's lookup.
/// Reading them back off a Π chain would mean guessing which binder was which.
#[derive(Debug)]
pub(crate) struct Derived {
    /// Its name, unqualified.
    pub(crate) name: Name,
    /// Its own type parameters, read under the trait's and the dictionary.
    pub(crate) params: Arc<[Binder]>,
    /// Its own constraints, read under all of those and its own parameters.
    pub(crate) context: Arc<[Constraint]>,
    /// Its type: `(p⃗ : Params) → (dict : Class p⃗) → (q⃗ : Own) → (d⃗ : Ctx) → τ`.
    pub(crate) ty: Term,
    /// Its definition: `λp⃗. λdict. λq⃗. λd⃗. e`.
    pub(crate) value: Term,
}

/// A `trait` declaration, elaborated.
#[derive(Debug)]
pub struct Trait {
    /// Its name.
    pub(crate) name: Name,
    /// The package that declares it, for §3's orphan rule.
    pub(crate) package: Option<PackageId>,
    /// Its parameters, in declaration order. The first is the head.
    pub(crate) params: Arc<[Binder]>,
    /// Its own constraints, as the dictionary fields they became.
    pub(crate) supers: Arc<[Constraint]>,
    /// The dictionary type as a function of the parameters: `λp⃗. { … }`.
    ///
    /// A closed term, so the type of a dictionary at arguments `τ⃗` is the
    /// application of this to them and nothing has to be substituted by hand.
    pub(crate) dictionary: Term,
    /// Every method, in dictionary-field order for the required ones.
    pub(crate) methods: Arc<[(Name, Kind)]>,
    /// The derived definitions, keyed by the names above.
    pub(crate) derived: Arc<[Derived]>,
}

impl Trait {
    /// Which kind of method `name` is, if the trait has one.
    pub(crate) fn method(&self, name: &str) -> Option<Kind> {
        self.methods
            .iter()
            .find_map(|(method, kind)| (**method == *name).then_some(*kind))
    }

    /// The derived definition named `name`.
    pub(crate) fn derivation(&self, name: &str) -> Option<&Derived> {
        self.derived.iter().find(|derived| *derived.name == *name)
    }

    /// The field a super-constraint on `class` occupies.
    ///
    /// Named after the trait it constrains, which is what makes §1's "reaching
    /// `Eq` from `Ord` is one field read" true of the representation and not
    /// only of the prose.
    pub(crate) fn super_field(class: &Name) -> Name {
        Arc::clone(class)
    }
}

/// An `impl` declaration, elaborated.
#[derive(Debug)]
pub struct Instance {
    /// Where it was written.
    pub(crate) origin: Origin,
    /// What it answers.
    pub(crate) key: Key,
    /// The type variables it abstracts over.
    pub(crate) params: Arc<[Binder]>,
    /// Its head arguments, under those parameters.
    pub(crate) args: Arc<[Term]>,
    /// Its own constraints, under those parameters, each smaller than the head.
    pub(crate) context: Arc<[Constraint]>,
    /// The dictionary: `λp⃗. λd⃗. { … }`, one `d` per constraint above.
    pub(crate) dictionary: Term,
}

/// Every trait and instance in scope.
///
/// A map rather than a list, so that §4 step 2 is a table read in the code as
/// well as in the specification. It is rebuilt on each declaration rather than
/// mutated, because a [`Cx`](crate::Cx) is persistent — an elaborator descends
/// into two branches from one context and neither may see the other's
/// declarations — and a declaration is rare where a lookup is not.
#[derive(Clone, Debug)]
pub(crate) struct Classes {
    traits: HashMap<Name, Arc<Trait>>,
    instances: HashMap<Key, Arc<Instance>>,
    /// Which traits declare a method of each unqualified name, sorted.
    ///
    /// §6's method syntax asks "which trait has an `m` for this head", and the
    /// honest answer without this index is a scan of every trait in scope —
    /// which is the shape of the search §9 refuses, even where the code that
    /// performs it stops at the first hit. Kept beside the map it summarizes so
    /// a use site reads one bucket, and rebuilt with it, since a trait
    /// declaration is rare where a method call is not.
    by_method: HashMap<Name, Vec<Name>>,
}

/// Every context starts with `Storable` in scope and nothing else.
///
/// `02-core-calculus.md` §1.2 makes storability a fact about a declaration
/// rather than a claim, so there is no declaration that puts the trait in scope
/// and nothing for a caller to remember to do. A signature may require it from
/// the first line of the first module.
impl Default for Classes {
    fn default() -> Self {
        let storable = crate::storable::class();
        let mut built = Self {
            traits: HashMap::new(),
            instances: HashMap::new(),
            by_method: HashMap::new(),
        };
        built.insert(&storable);
        built
    }
}

impl Classes {
    /// The trait named `name`.
    pub(crate) fn class(&self, name: &str) -> Option<&Arc<Trait>> {
        self.traits.get(name)
    }

    /// The trait a qualified method name `Class.method` reaches, with the kind
    /// of method it names.
    pub(crate) fn method(&self, qualified: &str) -> Option<(&Arc<Trait>, Name, Kind)> {
        let (class, method) = qualified.rsplit_once('.')?;
        let declared = self.traits.get(class)?;
        let kind = declared.method(method)?;
        Some((declared, Arc::from(method), kind))
    }

    /// The one instance answering `key`, or `None`.
    ///
    /// The whole of §4 step 2. There is no second candidate to consider,
    /// because [`Self::declaring_instance`] refused the second declaration.
    pub(crate) fn instance(&self, key: &Key) -> Option<&Arc<Instance>> {
        self.instances.get(key)
    }

    /// Whatever instance already answers `key`, for the coherence report.
    pub(crate) fn clash(&self, key: &Key) -> Option<&Arc<Instance>> {
        self.instances.get(key)
    }

    /// Which traits in scope declare a method spelled `method`.
    ///
    /// Sorted, so a report that names two of them names them in the same order
    /// twice — a hash map's iteration order would make an ambiguity message
    /// depend on the allocator.
    pub(crate) fn declaring_method(&self, method: &str) -> &[Name] {
        self.by_method.get(method).map_or(&[], Vec::as_slice)
    }

    /// These classes with `declared` added.
    pub(crate) fn declaring_class(&self, declared: &Arc<Trait>) -> Self {
        let mut built = self.clone();
        built.insert(declared);
        built
    }

    /// Record `declared` in both the name map and the method index.
    fn insert(&mut self, declared: &Arc<Trait>) {
        self.traits.insert(Arc::clone(&declared.name), Arc::clone(declared));
        for (method, _) in declared.methods.iter() {
            let bucket = self.by_method.entry(Arc::clone(method)).or_default();
            if let Err(at) = bucket.binary_search(&declared.name) {
                bucket.insert(at, Arc::clone(&declared.name));
            }
        }
    }

    /// These classes with `instance` added.
    ///
    /// Coherence is checked by the caller rather than here, because the refusal
    /// names *both* declarations and this operation would have to invent an
    /// error type to say so.
    pub(crate) fn declaring_instance(&self, instance: &Arc<Instance>) -> Self {
        let mut built = self.clone();
        built.instances.insert(instance.key.clone(), Arc::clone(instance));
        built
    }

    /// These classes with every one of `instances` added.
    ///
    /// One clone for a whole declaration group rather than one per family: a
    /// group of six mutually recursive families generates six instances, and
    /// folding [`Self::declaring_instance`] over them would copy both maps six
    /// times to reach the same table.
    pub(crate) fn declaring_instances(&self, instances: &[Arc<Instance>]) -> Self {
        let mut built = self.clone();
        for instance in instances {
            built.instances.insert(instance.key.clone(), Arc::clone(instance));
        }
        built
    }
}

/// The size §4's termination measure counts: type constructors and variables,
/// repeats included.
///
/// A structural walk and nothing cleverer, because the measure has to be one an
/// author can compute in their head from the declaration they wrote. Binders
/// count as the constructor they are: `A → B` is one more than the sum of its
/// sides, which is what makes a constraint on a function type larger than one on
/// either half.
pub(crate) fn size(term: &Term) -> u32 {
    use crate::term::Shape;
    let inner = match term.shape() {
        Shape::Var(_) | Shape::Const(_) | Shape::Universe(_) | Shape::Meta(_) => 0,
        Shape::Pi { domain, codomain, .. } => size(domain).saturating_add(size(codomain)),
        Shape::Lam { body, .. } => size(body),
        Shape::App { function, argument, .. } => size(function).saturating_add(size(argument)),
        Shape::RecordType(fields) | Shape::Record(fields) => fields
            .iter()
            .fold(0_u32, |total, field| total.saturating_add(size(&field.term))),
        Shape::Project { record, .. } => size(record),
        Shape::Id { ty, left, right } => size(ty).saturating_add(size(left)).saturating_add(size(right)),
        Shape::Refl(witness) => size(witness),
        Shape::J {
            ty,
            from,
            motive,
            base,
            to,
            proof,
        } => [from, motive, base, to, proof]
            .iter()
            .fold(size(ty), |total, part| total.saturating_add(size(part))),
        Shape::Let { ty, value, body, .. } => size(ty).saturating_add(size(value)).saturating_add(size(body)),
    };
    inner.saturating_add(1)
}

/// How many times the variable at `level` occurs in `term`.
///
/// The second half of §4's measure. Counting occurrences rather than merely
/// noticing one is what refuses `impl<A> C<F<A>> where D<G<A, A>>`, whose
/// argument is smaller and still duplicates its variable — the classic
/// non-terminating context that a size test alone admits.
pub(crate) fn occurrences(term: &Term, depth: u32, level: u32) -> u32 {
    use crate::term::Shape;
    let deeper = |term: &Term, by: u32| occurrences(term, depth.saturating_add(by), level);
    match term.shape() {
        Shape::Var(index) => u32::from(depth.checked_sub(index.0.saturating_add(1)) == Some(level)),
        Shape::Const(_) | Shape::Universe(_) | Shape::Meta(_) => 0,
        Shape::Pi { domain, codomain, .. } => deeper(domain, 0).saturating_add(deeper(codomain, 1)),
        Shape::Lam { body, .. } => deeper(body, 1),
        Shape::App { function, argument, .. } => deeper(function, 0).saturating_add(deeper(argument, 0)),
        Shape::RecordType(fields) => fields.iter().enumerate().fold(0, |total, (which, field)| {
            total.saturating_add(deeper(&field.term, u32::try_from(which).unwrap_or(u32::MAX)))
        }),
        Shape::Record(fields) => fields
            .iter()
            .fold(0, |total, field| total.saturating_add(deeper(&field.term, 0))),
        Shape::Project { record, .. } => deeper(record, 0),
        Shape::Id { ty, left, right } => deeper(ty, 0)
            .saturating_add(deeper(left, 0))
            .saturating_add(deeper(right, 0)),
        Shape::Refl(witness) => deeper(witness, 0),
        Shape::J {
            ty,
            from,
            motive,
            base,
            to,
            proof,
        } => [from, motive, base, to, proof]
            .iter()
            .fold(deeper(ty, 0), |total, part| total.saturating_add(deeper(part, 0))),
        Shape::Let { ty, value, body, .. } => deeper(ty, 0)
            .saturating_add(deeper(value, 0))
            .saturating_add(deeper(body, 1)),
    }
}

/// The head a constraint's first argument has, when it has a rigid one.
///
/// `None` for a head that is a metavariable, which §4 postpones rather than
/// looks up, and for one that is not a type application at all.
pub(crate) fn head_of(term: &Term, depth: u32) -> Option<Head> {
    use crate::term::Shape;
    match term.shape() {
        Shape::Const(constant) => Some(Head::Rigid(constant.name())),
        Shape::Var(index) => depth
            .checked_sub(index.0.saturating_add(1))
            .map(|level| Head::Local(DbLevel(level))),
        Shape::App { function, .. } => head_of(function, depth),
        // Everything else is a type no instance can be keyed on: a universe, a
        // binder, a record, an eliminator, or a metavariable — and the last is
        // §4's postponement rather than a failure.
        Shape::Universe(_)
        | Shape::Pi { .. }
        | Shape::Lam { .. }
        | Shape::RecordType(_)
        | Shape::Record(_)
        | Shape::Project { .. }
        | Shape::Id { .. }
        | Shape::Refl(_)
        | Shape::J { .. }
        | Shape::Meta(_)
        | Shape::Let { .. } => None,
    }
}
