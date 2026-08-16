//! A document's top-level definitions, declared together.
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
//! So a definition is a **global name**, exactly as a family is, and
//! [`Definitions`] sits beside the declared groups in a [`Cx`] rather than in
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

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::context::Cx;
use crate::elab::Elaborator;
use crate::eval::eval;
use crate::origin::Origin;
use crate::raw::{RawPattern, RawProgram, RawShape, RawTopLevel};
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

/// A document's top-level definitions, elaborated.
///
/// Opaque for [`crate::Group`]'s reason: a caller brings it into scope with
/// [`Cx::defining`] and then writes the names in ordinary raw terms. It never
/// assembles a definition's type itself, because the elaboration that produced
/// it is the only thing that knows what the type came out as.
pub struct Definitions {
    members: Arc<[Arc<Defined>]>,
}

impl Definitions {
    /// Every definition in the group, in the order they were written.
    pub(crate) fn members(&self) -> &[Arc<Defined>] {
        &self.members
    }
}

impl core::fmt::Debug for Definitions {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.debug_list().entries(self.members.iter()).finish()
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

/// Elaborate a document's top-level definitions, in context `cx`.
///
/// One call takes *all* of them, because §2.4 lets a body name a declaration
/// written later: no definition here is finished until the group is. What comes
/// back is brought into scope with [`Cx::defining`].
///
/// Duplicate names are not this crate's to refuse. A caller that binds one name
/// twice has a source mistake with a diagnostic of its own — the second
/// declaration simply shadows the first here — and a second refusal beside it
/// would be the second path `02-core-calculus.md` §5's audit exists to catch.
///
/// # Errors
///
/// [`Refusal::DefinitionCycle`] for definitions that name each other, which
/// §2.4's graph rule forbids and no measure reaches;
/// [`Refusal::UntypedRecursion`] for a self-recursive definition that wrote no
/// type, since the measure is checked against a type nobody else can supply;
/// and otherwise as [`crate::check`] — each definition is ordinary elaboration
/// and fails in the ordinary ways.
pub(crate) fn declare_program(cx: &Cx, program: &RawProgram) -> Result<Arc<Definitions>, ElabError> {
    let names: Vec<&Name> = program.definitions.iter().map(|held| &held.name).collect();
    let nodes: Vec<Node<'_>> = program
        .definitions
        .iter()
        .enumerate()
        .map(|(index, held)| {
            let edges = edges(held, &names);
            Node {
                recursive: edges.contains(&index),
                held,
                edges,
            }
        })
        .collect();
    let mut extended = cx.clone();
    let mut members: Vec<Arc<Defined>> = Vec::with_capacity(nodes.len());
    for node in ordering(&nodes)? {
        let defined = Arc::new(elaborate(&extended, node.held, node.recursive)?);
        extended = extended.defining_one(&defined);
        members.push(defined);
    }
    // Written order, not elaboration order: what a caller brings into scope is
    // the document's own list, and the order the analysis found is a fact about
    // this call rather than about the group.
    members.sort_by_key(|member| {
        program
            .definitions
            .iter()
            .position(|held| held.name == member.name)
            .unwrap_or(usize::MAX)
    });
    Ok(Arc::new(Definitions {
        members: members.into(),
    }))
}

/// One definition, elaborated in a context holding everything it names.
///
/// The type is elaborated first and the value against it, which is the whole of
/// what "signatures are collected before bodies" means once the order has been
/// computed: by the time this runs, every definition this one names has been
/// through it already.
fn elaborate(cx: &Cx, held: &RawTopLevel, recursive: bool) -> Result<Defined, ElabError> {
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
    Ok(Defined {
        name: Arc::clone(&held.name),
        visibility: held.visibility,
        module: held.module,
        ty: Arc::new(ty),
        value: Arc::new(value),
    })
}

/// Every definition in this group that `held` names, by index.
///
/// Both halves of the declaration are walked, because a type may name a
/// definition as readily as a body can: `fn play(v : Voicing) -> Sounding` is
/// an edge to `Sounding` if `Sounding` is one of these.
fn edges(held: &RawTopLevel, names: &[&Name]) -> Vec<usize> {
    let mut found = Vec::new();
    let mut bound: Vec<Name> = Vec::new();
    if let Some(ty) = &held.ty {
        free(ty, &mut bound, names, &mut found);
    }
    free(&held.value, &mut bound, names, &mut found);
    found.sort_unstable();
    found.dedup();
    found
}

/// The indices of `names` that `raw` mentions free, added to `found`.
///
/// `bound` is the binders standing over the sub-term being walked, so a local
/// `let` or a λ named after a top-level definition shadows it rather than
/// producing an edge. Over-approximating here would be worse than it sounds: a
/// spurious edge is a spurious *cycle*, and a cycle is refused.
fn free(raw: &crate::raw::Raw, bound: &mut Vec<Name>, names: &[&Name], found: &mut Vec<usize>) {
    let mut walk = |term: &crate::raw::Raw, bound: &mut Vec<Name>| free(term, bound, names, found);
    match raw.shape() {
        RawShape::Var(name) => {
            if !bound.iter().any(|binder| binder == name)
                && let Some(index) = names.iter().position(|declared| *declared == name)
            {
                found.push(index);
            }
        }
        RawShape::Lit(_) | RawShape::Universe(_) => {}
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
        RawShape::RecordType(fields) | RawShape::Record(fields) => {
            for field in fields.iter() {
                walk(&field.term, bound);
            }
        }
        RawShape::Method { receiver, .. } | RawShape::Project { record: receiver, .. } => walk(receiver, bound),
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

/// One definition and the definitions it names, by position in the same list.
///
/// The graph and the definitions travel together because they are asked about
/// together: the order is computed from the edges and the elaboration reads the
/// definition, and a second list to index into is a second thing that can be
/// indexed wrongly.
struct Node<'a> {
    held: &'a RawTopLevel,
    edges: Vec<usize>,
    /// Whether it names itself — §2.4's recursive case, admitted through
    /// [`crate::rec`]'s measure rather than through the graph.
    recursive: bool,
}

/// The order the definitions are elaborated in: everything a definition names,
/// before it.
///
/// A depth-first walk with an explicit stack rather than recursion, because the
/// depth here is a corpus's longest dependency chain and nothing bounds it.
/// A back edge to a definition still on the stack is a cycle, and the stack
/// from that definition up is the cycle to name.
///
/// # Errors
///
/// [`Refusal::DefinitionCycle`], naming the definitions in the order they call
/// each other.
fn ordering<'a>(nodes: &'a [Node<'a>]) -> Result<Vec<&'a Node<'a>>, ElabError> {
    // Sets rather than a mark per position, so that the walk reads the node
    // list in one place and reads it by name.
    let mut walking = BTreeSet::new();
    let mut placed = BTreeSet::new();
    let mut order = Vec::with_capacity(nodes.len());
    for (start, _) in nodes.iter().enumerate() {
        if placed.contains(&start) {
            continue;
        }
        let mut stack = vec![(start, 0_usize)];
        walking.insert(start);
        while let Some(&mut (node, ref mut next)) = stack.last_mut() {
            let step = *next;
            *next = next.saturating_add(1);
            let Some(&edge) = nodes.get(node).and_then(|held| held.edges.get(step)) else {
                walking.remove(&node);
                placed.insert(node);
                order.extend(nodes.get(node));
                stack.pop();
                continue;
            };
            // A self-edge is the recursive case and not a cycle; an edge
            // already placed is already before this one in the order.
            if edge == node || placed.contains(&edge) {
                continue;
            }
            if walking.contains(&edge) {
                let from = stack.iter().position(|&(held, _)| held == edge).unwrap_or(0);
                return Err(Refusal::DefinitionCycle {
                    names: stack
                        .get(from..)
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|&(held, _)| nodes.get(held))
                        .map(|held| Arc::clone(&held.held.name))
                        .collect(),
                    at: nodes.get(node).map_or(Origin::UNKNOWN, |held| held.held.origin),
                }
                .into());
            }
            walking.insert(edge);
            stack.push((edge, 0));
        }
    }
    Ok(order)
}

/// The definition a single-member context extension holds.
pub(crate) fn one(defined: &Arc<Defined>) -> Def {
    Def(Arc::clone(defined))
}
