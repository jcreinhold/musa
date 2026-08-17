//! A document's top-level definitions and instances, declared together.
//!
//! `docs/rules/language/02-core-calculus.md` §2.4's last paragraph is what this
//! module implements: "Top-level signatures are collected before bodies are
//! elaborated, so a later declaration may be referenced. From each body the
//! checker records an edge to every free named declaration. **Non-recursive**
//! declarations must form an acyclic graph, exactly as before; a *recursive*
//! declaration is the case §2.4 admits, and it is admitted through the measure
//! rather than through the graph."
//!
//! # Why a definition is a constant and not a binder
//!
//! The cheaper arrangement is a chain of [`RawShape::Let`](crate::RawShape),
//! and it cannot work: a `let` scopes forward only, so a body could never name
//! a definition written after it. The next cheapest is
//! [`Cx::define`](crate::Cx::define) with a name attached, and it cannot work
//! either, for a sharper reason — a de Bruijn binder refers *outward*, so the
//! last definition can see the first and never the other way round. Closing
//! that circle would need either substitution, which this crate deliberately
//! does not have (§3: "reduction is never performed on syntax"), or a fixed
//! point, which §1.3 refuses.
//!
//! So a definition is a **global name**, exactly as a family is, and a
//! [`Program`] sits beside the declared groups in a [`Cx`] rather than in
//! its binder environment. That also leaves α-equivalence alone: the
//! environment stays nameless, which is the property [`Cx::assume`]'s own
//! documentation is protecting.
//!
//! # A use is a reference, not a copy
//!
//! [`Shape::Def`](crate::Shape) is one node whose size does not depend on the
//! definition's body, and δ happens in [`crate::eval`] by handing back the
//! value the definition was elaborated to. Inlining the body at each use site
//! would type-check and would be wrong at scale: `stdlib/` is two thousand
//! definitions, every use would carry a copy, and a semantic hash would become
//! a function of what a definition happens to be written as rather than of what
//! it means.
//!
//! The value is stored and the *term* is not, which is what keeps the whole
//! arrangement free of the [`Arc`] cycle [`crate::family`] had to work around
//! with a declaration context. A [`Defined`] holds two [`Value`]s; nothing it
//! holds can point back at the group it is in, because evaluation has already
//! replaced every reference with the value it named.
//!
//! # The order is computed, and that is the mechanism
//!
//! Peyton Jones ch. 6 §6.2.8 is dependency analysis — sort definitions "into
//! minimal groups" and use `letrec` only "where it is actually necessary" — and
//! ch. 8 says when it has to run: "it is, however, important that the program
//! is subjected to the dependency analysis referred to in Section 6.2.8 before
//! type-checking", because a definition put into a recursive group it does not
//! belong in may fail to type-check at all. Here the analysis is the whole of
//! how §2.4's first sentence comes true: elaborating in dependency order is
//! what lets a body name a declaration written later, and the minimal groups it
//! finds are exactly the two cases §2.4 distinguishes — a group of one is an
//! ordinary definition or, with a self-edge, the recursive case the measure
//! admits, and a group of more than one is the cycle that has nowhere to go.
//!
//! The analysis runs over the document's `impl`s as well, for the reason
//! [`RawProgram`] states: a method body is an ordinary term that may name any
//! definition, and a definition that writes `x.m(y)` needs the instance at the
//! receiver's head to already be declared. Neither kind comes first, so neither
//! is declared first — the edges decide, one document at a time.

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::class::Instance;
use crate::context::Cx;
use crate::elab::Elaborator;
use crate::eval::eval;
use crate::origin::Origin;
use crate::raw::{RawImpl, RawPattern, RawProgram, RawShape, RawTopLevel};
use crate::refuse::{ElabError, Refusal};
use crate::scope::Scope;
use crate::term::{Name, Shape, Term};
use crate::value::{Env, Value};
use crate::visibility::{ModuleId, Visibility};

/// One elaborated top-level definition.
///
/// Its type and its value are [`Value`]s rather than [`Term`]s because that is
/// all a use needs — [`crate::eval`] hands back the value and
/// [`crate::recheck`] asks for the type — and because a stored term could hold
/// a [`Def`] pointing back at the group holding it. See the module
/// documentation.
pub(crate) struct Defined {
    /// The name it binds.
    pub(crate) name: Name,
    /// Whether it may be named outside its module.
    visibility: Visibility,
    /// The module it was written in, when the caller numbers modules.
    module: Option<ModuleId>,
    /// Its type.
    ty: Arc<Value>,
    /// Its value.
    value: Arc<Value>,
}

impl Defined {
    /// The module this definition is private to, when `viewer` may not name it.
    pub(crate) fn hidden_from(&self, viewer: Option<ModuleId>) -> Option<ModuleId> {
        (!self.visibility.visible_from(self.module, viewer)).then_some(self.module)?
    }
}

/// A definition shows as its name.
///
/// Written out rather than derived because a [`Value`] has no `Debug` — the
/// semantic domain holds closures — and because a definition's *body* is not
/// what a reader of a debug dump is looking for when a term mentions one.
impl core::fmt::Debug for Defined {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.debug_tuple("Defined").field(&self.name).finish()
    }
}

/// A document's top level, elaborated: its definitions and its instances.
///
/// Opaque for [`crate::Group`]'s reason: a caller brings it into scope with
/// [`Cx::defining`] — one call, both kinds — and then writes the names in
/// ordinary raw terms. It never assembles a definition's type itself, because
/// the elaboration that produced it is the only thing that knows what the type
/// came out as.
///
/// Both kinds together because [`crate::RawProgram`] holds both together and
/// for the same reason: the order they were elaborated in was computed from
/// what they name, and handing a caller two lists to re-combine would be handing
/// back the question this module exists to answer.
pub struct Program {
    members: Arc<[Arc<Defined>]>,
    instances: Arc<[Arc<Instance>]>,
}

impl Program {
    /// Every definition in the group, in the order they were written.
    pub(crate) fn members(&self) -> &[Arc<Defined>] {
        &self.members
    }

    /// Every instance in the group, in the order they were written.
    pub(crate) fn instances(&self) -> &[Arc<Instance>] {
        &self.instances
    }
}

/// A group's instances as the keys they answer, in the order they are held.
///
/// A wrapper rather than a `Debug` on [`Instance`], because what a reader of a
/// group's dump is looking for is which instances are in it and in what order,
/// and an instance's own derived dump is mostly its dictionary term.
struct Keyed<'a>(&'a [Arc<Instance>]);

impl core::fmt::Debug for Keyed<'_> {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.debug_list().entries(self.0.iter().map(|held| &held.key)).finish()
    }
}

/// Both kinds, in the order the document wrote them.
///
/// Two named lists rather than one bare list: the group holds two kinds now,
/// and a dump that showed only the definitions would be one a reader draws the
/// wrong conclusion from. What the order shows is the *written* order, which
/// [`declare_program`] restores after elaborating in the order it computed.
impl core::fmt::Debug for Program {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.debug_struct("Program")
            .field("definitions", &self.members)
            .field("instances", &Keyed(&self.instances))
            .finish()
    }
}

/// A use of a top-level definition.
///
/// One node. What it costs to write a definition's name is the same whatever
/// the definition is, which is the property the module documentation calls a
/// reference rather than a copy.
#[derive(Clone)]
pub struct Def(Arc<Defined>);

impl core::fmt::Debug for Def {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.debug_tuple("Def").field(&self.0.name).finish()
    }
}

/// Two uses are the same use when they name the same definition.
///
/// By name, not by pointer, for [`crate::Constant`]'s reason: a term elaborated
/// twice from one source must compare equal, and two elaborations build two
/// groups.
impl PartialEq for Def {
    fn eq(&self, other: &Self) -> bool {
        self.0.name == other.0.name
    }
}

impl Eq for Def {}

impl core::fmt::Display for Def {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.write_str(&self.0.name)
    }
}

impl Def {
    /// This use as a term.
    pub(crate) fn term(&self, origin: Origin) -> Term {
        Term::new(origin, Shape::Def(self.clone()))
    }

    /// The definition's type.
    pub(crate) fn ty(&self) -> Arc<Value> {
        Arc::clone(&self.0.ty)
    }

    /// The definition's value, which is what δ unfolds a use to.
    pub(crate) fn value(&self) -> Arc<Value> {
        Arc::clone(&self.0.value)
    }
}

/// Elaborate a document's top-level definitions *and instances*, in context
/// `cx`.
///
/// One call takes *all* of them, because §2.4 lets a body name a declaration
/// written later: nothing here is finished until the group is. The two kinds
/// travel together for [`RawProgram`]'s reason — each can name the other and
/// neither comes first — and what comes back holds both, brought into scope
/// with [`Cx::defining`] in one call.
///
/// Duplicate names are not this crate's to refuse. A caller that binds one name
/// twice has a source mistake with a diagnostic of its own — the second
/// declaration simply shadows the first here — and a second refusal beside it
/// would be the second path `02-core-calculus.md` §5's audit exists to catch.
///
/// # Errors
///
/// [`Refusal::DefinitionCycle`] for declarations that *name* each other, which
/// §2.4's graph rule forbids and no measure reaches. A cycle among method
/// spellings is not one of these: see [`Edge`] for why it is dropped, and
/// [`ordering`] for what the drop costs. [`Refusal::UntypedRecursion`] for a
/// self-recursive definition that wrote no type, since the measure is checked
/// against a type nobody else can supply; whatever
/// [`declare_impl`](crate::declare_impl) refuses, since an instance in the
/// group goes through it unchanged; and otherwise as [`crate::check`] — each
/// declaration is ordinary elaboration and fails in the ordinary ways.
pub(crate) fn declare_program(cx: &Cx, program: &RawProgram) -> Result<(Arc<Program>, crate::Spend), ElabError> {
    let nodes = graph(cx, program);
    let mut extended = cx.clone();
    // Positions rather than the values themselves, because the elaboration
    // order is not the written order and the written order is what comes back.
    let mut members: Vec<(usize, Arc<Defined>)> = Vec::with_capacity(program.definitions.len());
    let mut instances: Vec<(usize, Arc<Instance>)> = Vec::with_capacity(program.instances.len());
    let mut spent = crate::Spend::default();
    for node in ordering(&nodes)? {
        match node.held {
            Held::Definition(index, held) => {
                let (defined, spend) = elaborate(&extended, held, node.recursive)?;
                let defined = Arc::new(defined);
                spent = spent.and(spend);
                extended = extended.defining_one(&defined);
                members.push((index, defined));
            }
            Held::Instance(index, raw) => {
                let (declared, spend) = crate::dictionary::declare_impl(&extended, raw)?;
                spent = spent.and(spend);
                extended = extended.declaring_instance(&declared);
                instances.push((index, declared));
            }
        }
    }
    // Written order, not elaboration order: what a caller brings into scope is
    // the document's own list, and the order the analysis found is a fact about
    // this call rather than about the group.
    members.sort_by_key(|&(index, _)| index);
    instances.sort_by_key(|&(index, _)| index);
    let declared = Arc::new(Program {
        members: members.into_iter().map(|(_, defined)| defined).collect(),
        instances: instances.into_iter().map(|(_, declared)| declared).collect(),
    });
    Ok((declared, spent))
}

/// The dependency graph over a document's definitions and instances.
///
/// One index space, definitions first, so that an edge from either kind to
/// either kind is one number and [`ordering`] never has to ask which list it is
/// walking. `cx` supplies the trait declarations — an edge to an instance is
/// found by the *method spelling* a body writes, and only the classes know
/// which trait declares a given spelling.
fn graph<'a>(cx: &Cx, program: &'a RawProgram) -> Vec<Node<'a>> {
    let names: Vec<&Name> = program.definitions.iter().map(|held| &held.name).collect();
    let classes = cx.classes();
    // Which instance each trait name has here, so a method spelling reaches an
    // instance in two lookups rather than a scan per call site.
    let instanced: Vec<(&Name, usize)> = program
        .instances
        .iter()
        .enumerate()
        .map(|(index, raw)| (&raw.name, program.definitions.len().saturating_add(index)))
        .collect();
    // The soft half of the edges: every instance in this group of every trait
    // declaring a spelling the declaration writes. Sorted and deduplicated
    // because two spellings may reach one instance, and disjoint from the hard
    // half by construction — a hard edge is a definition's position and this
    // only ever answers an instance's.
    let implementing = |methods: &[Name]| {
        let mut found: Vec<usize> = Vec::new();
        for method in methods {
            for class in classes.declaring_method(method) {
                found.extend(
                    instanced
                        .iter()
                        .filter(|&&(name, _)| **name == **class)
                        .map(|&(_, index)| index),
                );
            }
        }
        found.sort_unstable();
        found.dedup();
        found.into_iter().map(|to| Edge { to, hard: false })
    };
    let naming = |mut found: Vec<usize>| {
        found.sort_unstable();
        found.dedup();
        found.into_iter().map(|to| Edge { to, hard: true })
    };
    let mut nodes: Vec<Node<'a>> = Vec::with_capacity(names.len().saturating_add(program.instances.len()));
    for (index, held) in program.definitions.iter().enumerate() {
        let mut written = Written::default();
        if let Some(ty) = &held.ty {
            written.walk(ty, &names);
        }
        written.walk(&held.value, &names);
        let recursive = written.found.contains(&index);
        nodes.push(Node {
            recursive,
            held: Held::Definition(index, held),
            edges: naming(written.found).chain(implementing(&written.methods)).collect(),
        });
    }
    for (index, raw) in program.instances.iter().enumerate() {
        let mut written = Written::default();
        for binder in &raw.params {
            written.walk(&binder.ty, &names);
        }
        for argument in &raw.args {
            written.walk(argument, &names);
        }
        for constraint in &raw.context {
            for argument in &constraint.args {
                written.walk(argument, &names);
            }
        }
        for method in &raw.methods {
            written.walk(&method.value, &names);
        }
        nodes.push(Node {
            // An instance is never `rec`: a dictionary is a closed term with no
            // name to tie itself back to, so a self-edge is dropped by
            // [`ordering`] and the method body refuses at the call it wrote.
            // Its own spelling reaches itself here, which is exactly the soft
            // edge that has to be droppable.
            recursive: false,
            held: Held::Instance(program.definitions.len().saturating_add(index), raw),
            edges: naming(written.found).chain(implementing(&written.methods)).collect(),
        });
    }
    nodes
}

/// What one declaration writes that the rest of the group might supply: the
/// definitions it names, and the method spellings it calls.
///
/// The two are collected in one walk because they are found in the same places,
/// and separately because they are resolved differently — a name is matched
/// against this group's definitions, and a spelling has to go through the
/// declared traits before it names anything at all.
#[derive(Default)]
struct Written {
    /// The positions in the group of the definitions it names.
    found: Vec<usize>,
    /// The method spellings it calls, each once.
    methods: Vec<Name>,
}

impl Written {
    /// Add what `raw` writes, with the group's definition names as `names`.
    fn walk(&mut self, raw: &crate::raw::Raw, names: &[&Name]) {
        let mut bound: Vec<Name> = Vec::new();
        free(raw, &mut bound, names, self);
    }
}

/// One definition, elaborated in a context holding everything it names.
///
/// The type is elaborated first and the value against it, which is the whole of
/// what "signatures are collected before bodies" means once the order has been
/// computed: by the time this runs, every definition this one names has been
/// through it already.
fn elaborate(cx: &Cx, held: &RawTopLevel, recursive: bool) -> Result<(Defined, crate::Spend), ElabError> {
    let inner = held.module.map_or_else(|| cx.clone(), |module| cx.in_module(module));
    let mut elaborator = Elaborator::new(&inner);
    let scope = Scope::new(&inner);
    let (ty, value) = match &held.ty {
        Some(written) => {
            let (ty, _) = elaborator.check_type(&scope, written)?;
            let expected = scope.eval(elaborator.meter(), &ty)?;
            // A definition that names itself is `rec f : A = e`, which is the
            // one form §2.4's measure is stated for. Building it here rather
            // than asking the caller to means the surface never has to decide
            // whether a `fn` is recursive — the dependency analysis already
            // knows, and it knows it the same way for every definition.
            let body = if recursive {
                crate::raw::Raw::rec(held.origin, Arc::clone(&held.name), written.clone(), held.value.clone())
            } else {
                held.value.clone()
            };
            let term = elaborator.run_check(&scope, &body, &expected)?;
            (expected, term)
        }
        None => {
            if recursive {
                return Err(Refusal::UntypedRecursion {
                    name: Arc::clone(&held.name),
                    at: held.origin,
                }
                .into());
            }
            let (term, ty) = elaborator.run_infer(&scope, &held.value)?;
            (scope.eval(elaborator.meter(), &ty)?, term)
        }
    };
    let value = eval(elaborator.meter(), &Env::EMPTY, &value)?;
    let defined = Defined {
        name: Arc::clone(&held.name),
        visibility: held.visibility,
        module: held.module,
        ty: Arc::new(ty),
        value: Arc::new(value),
    };
    Ok((defined, elaborator.spent()))
}

/// What `raw` names of the group around it, added to `found`.
///
/// `bound` is the binders standing over the sub-term being walked, so a local
/// `let` or a λ named after a top-level definition shadows it rather than
/// producing an edge. Over-approximating here would be worse than it sounds: a
/// spurious edge is a spurious *cycle*, and a cycle is refused.
///
/// A method spelling is collected in the same walk and is *not* shadowed by a
/// binder, because a method name is not a variable: `10-traits.md` §6 resolves
/// `x.m` by the receiver's type and never against the binders around it.
fn free(raw: &crate::raw::Raw, bound: &mut Vec<Name>, names: &[&Name], found: &mut Written) {
    let mut walk = |term: &crate::raw::Raw, bound: &mut Vec<Name>| free(term, bound, names, found);
    match raw.shape() {
        RawShape::Var(name) => {
            if !bound.iter().any(|binder| binder == name)
                && let Some(index) = names.iter().position(|declared| *declared == name)
            {
                found.found.push(index);
            }
        }
        // No edge: a hosted name is the reader's, resolved in the host's
        // namespaces, and cannot be the definition standing beside it.
        RawShape::Hosted(_) | RawShape::Lit(_) | RawShape::Numeral { .. } | RawShape::Universe(_) => {}
        RawShape::ConstrainedPi { constraint, codomain } => {
            for argument in &constraint.args {
                walk(argument, bound);
            }
            walk(codomain, bound);
        }
        RawShape::Pi {
            name, domain, codomain, ..
        } => {
            walk(domain, bound);
            under(bound, [Arc::clone(name)], |bound| free(codomain, bound, names, found));
        }
        RawShape::Lam { name, domain, body, .. } => {
            if let Some(domain) = domain {
                walk(domain, bound);
            }
            under(bound, [Arc::clone(name)], |bound| free(body, bound, names, found));
        }
        RawShape::App { function, argument, .. } => {
            walk(function, bound);
            walk(argument, bound);
        }
        RawShape::Call { function, arguments } => {
            walk(function, bound);
            for argument in arguments.iter() {
                walk(argument, bound);
            }
        }
        RawShape::RecordType(fields) | RawShape::Record(fields) => {
            for field in fields.iter() {
                walk(&field.term, bound);
            }
        }
        RawShape::Method { receiver, method } => {
            if !found.methods.iter().any(|seen| seen == method) {
                found.methods.push(Arc::clone(method));
            }
            free(receiver, bound, names, found);
        }
        RawShape::Project { record, .. } => walk(record, bound),
        RawShape::Update { record, updates } => {
            walk(record, bound);
            for update in updates.iter() {
                walk(&update.value, bound);
            }
        }
        RawShape::Id { ty, left, right } => {
            walk(ty, bound);
            walk(left, bound);
            walk(right, bound);
        }
        RawShape::Refl(witness) => walk(witness, bound),
        RawShape::J {
            ty,
            from,
            motive,
            base,
            to,
            proof,
        } => {
            for part in [ty, from, motive, base, to, proof] {
                walk(part, bound);
            }
        }
        RawShape::Let { name, ty, value, body } => {
            if let Some(ty) = ty {
                walk(ty, bound);
            }
            walk(value, bound);
            under(bound, [Arc::clone(name)], |bound| free(body, bound, names, found));
        }
        RawShape::Annot { term, ty } => {
            walk(term, bound);
            walk(ty, bound);
        }
        RawShape::Match { subjects, arms } => {
            for subject in subjects.iter() {
                walk(subject, bound);
            }
            for arm in arms.iter() {
                let mut binders = Vec::new();
                for pattern in &arm.patterns {
                    binders_of(pattern, &mut binders);
                }
                under(bound, binders, |bound| free(&arm.body, bound, names, found));
            }
        }
        RawShape::Rec { name, ty, body } => {
            walk(ty, bound);
            under(bound, [Arc::clone(name)], |bound| free(body, bound, names, found));
        }
    }
}

/// Run `walk` with `binders` in scope, and take them back out again.
fn under(bound: &mut Vec<Name>, binders: impl IntoIterator<Item = Name>, walk: impl FnOnce(&mut Vec<Name>)) {
    let depth = bound.len();
    bound.extend(binders);
    walk(bound);
    bound.truncate(depth);
}

/// Every name `pattern` binds, added to `binders`.
fn binders_of(pattern: &RawPattern, binders: &mut Vec<Name>) {
    match pattern {
        RawPattern::Bind { name, .. } => binders.push(Arc::clone(name)),
        RawPattern::Constructor { fields, .. } => {
            for field in fields {
                binders_of(field, binders);
            }
        }
        RawPattern::Record { fields, .. } => {
            for (_, field) in fields {
                binders_of(field, binders);
            }
        }
    }
}

/// What one node of the graph holds, and where in its own list it was written.
///
/// The position travels with the declaration because the order the analysis
/// finds is not the order the document wrote, and the caller gets the written
/// one back.
enum Held<'a> {
    /// A definition, at `.0` in [`RawProgram::definitions`].
    Definition(usize, &'a RawTopLevel),
    /// An instance, at `.0` in the combined index space — so subtracting the
    /// definition count gives its position in [`RawProgram::instances`].
    Instance(usize, &'a RawImpl),
}

impl Held<'_> {
    /// What to call this in a cycle: a definition's name, or the trait an
    /// instance implements.
    fn name(&self) -> &Name {
        match self {
            Self::Definition(_, held) => &held.name,
            Self::Instance(_, raw) => &raw.name,
        }
    }

    /// Where it was written.
    fn origin(&self) -> Origin {
        match self {
            Self::Definition(_, held) => held.origin,
            Self::Instance(_, raw) => raw.origin,
        }
    }
}

/// One declaration and the declarations it names, by position in one index
/// space over the whole group.
///
/// The graph and the declarations travel together because they are asked about
/// together: the order is computed from the edges and the elaboration reads the
/// declaration, and a second list to index into is a second thing that can be
/// indexed wrongly.
struct Node<'a> {
    held: Held<'a>,
    edges: Vec<Edge>,
    /// Whether it names itself — §2.4's recursive case, admitted through
    /// [`crate::rec`]'s measure rather than through the graph.
    ///
    /// Read off the *hard* edges and nothing else: a soft edge stands for "one
    /// of these" (see [`Edge`]), and a declaration that may or may not be its
    /// own dependency is not the thing `rec f : A = e` is written for.
    recursive: bool,
}

/// One dependency, and whether a cycle through it is a refusal.
///
/// The two kinds are 02-core-calculus.md §2.4's edge and an approximation of
/// it, and the difference is what each one *means*.
///
/// A **hard** edge is a free name: this declaration writes that definition's
/// name, so that one has to be elaborated first, and a cycle in hard edges is
/// [`Refusal::DefinitionCycle`] exactly as it was before instances joined the
/// group.
///
/// A **soft** edge is a method spelling: this declaration writes `x.m(y)`, and
/// `10-traits.md` §6 will resolve that against whichever instance sits at the
/// head of `x`'s type — which is not known until this declaration has been
/// elaborated. So the edge points at *every* instance in the group of *every*
/// trait declaring `m`, and it means "before one of these" rather than "before
/// that one". An over-approximation cannot be allowed to refuse: two instances
/// of one trait whose bodies both write that trait's own spelling name each
/// other under this rule and name each other in no other sense, and refusing
/// them as a cycle would reject a document a reader would order without
/// hesitating. So a soft back edge is dropped, and what it costs is stated in
/// [`ordering`].
///
/// Peyton Jones ch. 6 §6.2.8's minimal groups is the property being protected:
/// a declaration put into a recursive group it does not belong in may fail to
/// type-check something perfectly well typed, and an edge standing for "one of
/// these" is the standard way to manufacture that mistake.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Edge {
    /// The declaration this one is to be elaborated after.
    to: usize,
    /// Whether a cycle through this edge is a refusal.
    hard: bool,
}

/// One frame of the walk: a declaration, how far through its edges it is, and
/// how it was reached.
///
/// `soft` is the third field because breaking a cycle needs it: the walk has to
/// be able to say *which* of the declarations it is standing on was reached
/// through an edge that may be dropped.
struct Frame {
    node: usize,
    next: usize,
    soft: bool,
}

/// The order the declarations are elaborated in: everything one names, before
/// it.
///
/// A depth-first walk with an explicit stack rather than recursion, because the
/// depth here is a corpus's longest dependency chain and nothing bounds it.
///
/// # Where a cycle is broken
///
/// A back edge closes a cycle, and the question is only whether the cycle
/// contains an edge that may be dropped. A soft back edge is one, and is
/// dropped where it is found. A *hard* back edge is not — but the cycle it
/// closes may still have been entered through a soft edge, and that is the
/// shape a definition and an instance make when each names the other:
///
/// ```text
/// seed        = Nat.Zero.tag        // soft: before one of the Tagged instances
/// impl Tagged<Nat> { tag = seed }   // hard: before seed, and only seed
/// ```
///
/// Refusing that pair as a cycle would be the over-approximation refusing, and
/// dropping the *hard* edge instead would elaborate the instance first and
/// report an unknown name in its body — a sentence about the wrong half of the
/// program. So the walk unwinds to the deepest declaration on the cycle that
/// was reached softly and drops *that* edge, which leaves the definition first
/// and its call the thing that refuses. A dropped edge is remembered, so the
/// walk cannot take it again and the whole traversal is finite.
///
/// # What a dropped soft edge costs, said rather than hidden
///
/// The declaration is elaborated with that instance not yet in
/// [`crate::Classes`]. If it really did call that instance's method, the call
/// refuses with `NoMethodForType` **at the call**, naming the method and the
/// head type — which is a sentence about the program, and what a genuine mutual
/// dependency between a definition and a dictionary deserves. An instance is
/// never `rec`, so a cycle through one has nowhere to go even where the graph
/// admitted it. What must not happen instead is the same program refused as a
/// cycle among declarations that never named each other, which is what a hard
/// method edge produces and what [`Edge`] is for.
///
/// # Errors
///
/// [`Refusal::DefinitionCycle`], naming the declarations in the order they name
/// each other. Only a cycle of hard edges reaches it.
fn ordering<'a>(nodes: &'a [Node<'a>]) -> Result<Vec<&'a Node<'a>>, ElabError> {
    // Sets rather than a mark per position, so that the walk reads the node
    // list in one place and reads it by name.
    let mut walking = BTreeSet::new();
    let mut placed = BTreeSet::new();
    let mut dropped: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut order = Vec::with_capacity(nodes.len());
    for (start, _) in nodes.iter().enumerate() {
        if placed.contains(&start) {
            continue;
        }
        let mut stack = vec![Frame {
            node: start,
            next: 0,
            soft: false,
        }];
        walking.insert(start);
        while let Some(frame) = stack.last_mut() {
            let node = frame.node;
            let step = frame.next;
            frame.next = step.saturating_add(1);
            let Some(&edge) = nodes.get(node).and_then(|held| held.edges.get(step)) else {
                walking.remove(&node);
                placed.insert(node);
                order.extend(nodes.get(node));
                stack.pop();
                continue;
            };
            // A self-edge is the recursive case for a definition and dropped
            // for an instance, and neither is a cycle; an edge already placed
            // is already before this one in the order.
            if edge.to == node || placed.contains(&edge.to) || dropped.contains(&(node, edge.to)) {
                continue;
            }
            if walking.contains(&edge.to) {
                // An edge that meant "before one of these" is dropped rather
                // than refused, because which one it turns out to mean is not
                // this walk's to know.
                if !edge.hard {
                    continue;
                }
                let from = stack.iter().position(|frame| frame.node == edge.to).unwrap_or(0);
                let above = from.saturating_add(1);
                let softest = stack
                    .get(above..)
                    .and_then(|cycle| cycle.iter().rposition(|frame| frame.soft))
                    .map(|at| above.saturating_add(at));
                let Some(at) = softest else {
                    return Err(Refusal::DefinitionCycle {
                        names: stack
                            .get(from..)
                            .unwrap_or_default()
                            .iter()
                            .filter_map(|frame| nodes.get(frame.node))
                            .map(|held| Arc::clone(held.held.name()))
                            .collect(),
                        at: nodes.get(node).map_or(Origin::UNKNOWN, |held| held.held.origin()),
                    }
                    .into());
                };
                if let Some(entered) = stack.get(at.saturating_sub(1)).map(|frame| frame.node)
                    && let Some(reached) = stack.get(at).map(|frame| frame.node)
                {
                    dropped.insert((entered, reached));
                }
                for frame in stack.drain(at..) {
                    walking.remove(&frame.node);
                }
                continue;
            }
            walking.insert(edge.to);
            stack.push(Frame {
                node: edge.to,
                next: 0,
                soft: !edge.hard,
            });
        }
    }
    Ok(order)
}

/// The definition a single-member context extension holds.
pub(crate) fn one(defined: &Arc<Defined>) -> Def {
    Def(Arc::clone(defined))
}
