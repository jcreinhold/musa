//! Structural descent over a compiled case tree: §2.4's termination rule.
//!
//! `docs/rules/language/02-core-calculus.md` §2.4's second paragraph says what
//! changed and why this module exists: descent "used to fall out of the
//! generated eliminator", and with `match` compiled to a tree "the checker
//! walks the tree instead". §1.3's refusal list says how far it goes —
//! "Structural descent over the case tree is the whole rule". There is no
//! size-change analysis here, no lexicographic order, and no way for an author
//! to assert totality.
//!
//! # The rule, stated before it is implemented
//!
//! A [`Compiled`] body abstracts `n` binders and answers a tree beneath them. A
//! **smaller** term is a variable that a [`Split`](crate::kernel::case_tree)
//! bound as a *field* of the pattern it matched, transitively along the path
//! from the root: the fields of a split whose subject is binder `p` descend
//! from `p`, and so do the fields of a split whose subject already descends
//! from `p`. Nothing else is smaller — not a constructor applied to a field,
//! not the subject itself, not a field of a split on something the definition
//! did not receive.
//!
//! A definition is admitted when there is **one position `p`** such that every
//! recursive call in the tree passes, at `p`, a variable descending from binder
//! `p`. The position is fixed for the whole definition, and that is the load-
//! bearing half. "Some argument is smaller" is *not* the rule, because it
//! admits a definition that does not terminate:
//!
//! ```text
//! f x y = match x { Zero => match y { Zero => 0; Succ k => f y k }
//!                 ; Succ j => f j y }
//! ```
//!
//! Each call passes something a split bound — `k` in one arm, `j` in the other
//! — and they descend in *different* positions, so `f 1 1` reaches `f 1 0`,
//! `f 0 1`, and back. Requiring one position for all of them refuses it. That
//! is Coq's guarded-fixpoint condition and nothing larger.
//!
//! What the other arguments do is **not** constrained, and that too is
//! measured. `musa-compiler/src/prelude.rs`'s `list_from_start` calls
//! `walk rest (step built first)`: the second argument is an accumulator the
//! author computed, and every forward fold in the language is shaped that way.
//! A rule that also demanded "and none is larger" would refuse the corpus,
//! while under a fixed position it buys nothing — the measure is the size of
//! the argument at `p` alone.
//!
//! # How the position is found
//!
//! Not by trying each in turn. The walk collects, for **each** recursive call,
//! the set of positions at which that call passes a descending variable. A call
//! whose set is empty is refused where it stands, naming itself; otherwise the
//! sets are intersected, and an empty intersection refuses the definition at the
//! first call. One pass, and the refusal names a call rather than the whole
//! body.
//!
//! A use of the definition that is not a call at all — passed along, partially
//! applied past `p`, or standing as a value — has an empty set by construction,
//! which is the same answer `rec.rs` gave it before this module existed: a core
//! with no fixed point has nothing to hand over.

use std::collections::BTreeMap;

use crate::kernel::case_tree::{CaseTree, Compiled};
use crate::kernel::origin::Origin;
use crate::kernel::term::{Binder, Level, Name, Role, Shape, Term};

/// A recursive call the rule cannot see descend, and where it was written.
///
/// One origin rather than a list: the report is
/// [`Refusal::UncheckedRecursion`](crate::Refusal), which names *the* call, and
/// a reader fixes the first one before the rest can be judged.
pub(crate) struct Undescending(pub(crate) Origin);

/// Whether every recursive call in `body` descends at one fixed position.
///
/// `name` is the definition's own name, which is how a call is recognized: a
/// recursive body names itself through the globals table (see
/// [`Body::Pending`](crate::kernel::program::Body)), so a call is an
/// application spine headed by [`Shape::Named`] at [`Role::Defined`] bearing
/// this name.
///
/// `None` when the rule holds, and a definition with no recursive call at all
/// holds it vacuously.
pub(crate) fn descends(body: &Compiled, name: &Name) -> Option<Undescending> {
    let arity = u32::try_from(body.arity()).unwrap_or(u32::MAX);
    // The definition's own binders, each standing at its own position and none
    // of them smaller than itself. Seeded rather than special-cased so that a
    // `let` naming one, and a split on one, are read by the same two lines that
    // read everything else.
    let mut from = Descent::new();
    for position in 0..arity {
        from.insert(
            Level(position),
            Descended {
                position,
                smaller: false,
            },
        );
    }
    let mut measure = Measure { name, agreed: None };
    measure.tree(&body.tree, Level(arity), &from)
}

/// The intersection so far, and what a call is measured against.
struct Measure<'a> {
    name: &'a Name,
    /// The positions every call seen so far descends at. `None` until the first
    /// call, which is what makes a non-recursive body vacuously admitted.
    agreed: Option<Vec<u32>>,
}

/// What one variable in scope is, as far as the measure is concerned.
#[derive(Clone, Copy)]
struct Descended {
    /// Which of the definition's own binders it came from.
    ///
    /// Per-position, because knowing that `k` is smaller is not enough: the
    /// checker has to know it is smaller *than the argument at `p`*.
    position: u32,
    /// Whether a split made it smaller, or it is merely that binder again.
    smaller: bool,
}

/// What each variable in scope descends from.
type Descent = BTreeMap<Level, Descended>;

impl Measure<'_> {
    /// Walk one node at `depth`, with `from` saying what is already smaller.
    ///
    /// `Some(Undescending)` short-circuits the whole walk: the first call the
    /// rule cannot see descend is the one reported.
    fn tree(&mut self, tree: &CaseTree, depth: Level, from: &Descent) -> Option<Undescending> {
        match tree {
            // Nothing to answer, so nothing to descend.
            CaseTree::Impossible => None,
            CaseTree::Answer(term) => self.term(term, depth, from),
            CaseTree::Split(split) => {
                // The subject decides whether this split *makes* anything
                // smaller. A subject that is not a variable — or is one the
                // definition did not receive and no split bound — leaves its
                // fields unmeasured, which is what refuses a recursion on a
                // value the body computed.
                let descending =
                    variable(&split.on, depth).and_then(|level| from.get(&level).map(|held| held.position));
                for alternative in split.alternatives.iter() {
                    let mut inner = from.clone();
                    let mut at = depth;
                    for _ in alternative.fields.iter() {
                        if let Some(position) = descending {
                            inner.insert(
                                at,
                                Descended {
                                    position,
                                    smaller: true,
                                },
                            );
                        }
                        at = at.deeper();
                    }
                    // A hypothesis binder is a placeholder nothing can name
                    // (`case_tree.rs`), so it occupies a level and is never
                    // smaller.
                    for _ in alternative.hypotheses.iter() {
                        at = at.deeper();
                    }
                    if let Some(refused) = self.tree(&alternative.body, at, &inner) {
                        return Some(refused);
                    }
                }
                None
            }
        }
    }

    /// Walk one answer, measuring every recursive call it holds.
    fn term(&mut self, term: &Term, depth: Level, from: &Descent) -> Option<Undescending> {
        let (head, arguments) = spine(term);
        if self.recursive(head) {
            if let Some(refused) = self.call(term.origin(), &arguments, depth, from) {
                return Some(refused);
            }
            // The head is the name itself and holds nothing; the arguments are
            // walked so that a call nested inside one is measured too.
            for argument in arguments {
                if let Some(refused) = self.term(argument, depth, from) {
                    return Some(refused);
                }
            }
            return None;
        }
        match term.shape() {
            Shape::Meta(_) | Shape::Var(_) | Shape::Named { .. } | Shape::Lit(_) | Shape::Universe(_) => None,
            Shape::Bind { binder, body, .. } => {
                for outer in binder.outer() {
                    if let Some(refused) = self.term(outer, depth, from) {
                        return Some(refused);
                    }
                }
                // A `let` naming a variable is that variable — see the module
                // documentation for why every arm of every tree has some.
                let aliased = match *binder {
                    Binder::Let { ref value, .. } => variable(value, depth).and_then(|level| from.get(&level).copied()),
                    Binder::Lam | Binder::Pi { .. } => None,
                };
                match aliased {
                    Some(held) => {
                        let mut inner = from.clone();
                        inner.insert(depth, held);
                        self.term(body, depth.deeper(), &inner)
                    }
                    None => self.term(body, depth.deeper(), from),
                }
            }
            Shape::App { function, argument } => self
                .term(function, depth, from)
                .or_else(|| self.term(argument, depth, from)),
            Shape::RecordType(fields) => {
                // A record *type* is a telescope: a later field's type stands
                // one binder deeper than the one before it.
                let mut at = depth;
                for field in fields.iter() {
                    if let Some(refused) = self.term(&field.term, at, from) {
                        return Some(refused);
                    }
                    at = at.deeper();
                }
                None
            }
            Shape::Record(fields) => fields.iter().find_map(|field| self.term(&field.term, depth, from)),
            Shape::Project { record, .. } => self.term(record, depth, from),
        }
    }

    /// One recursive call, narrowing the agreed positions.
    fn call(&mut self, at: Origin, arguments: &[&Term], depth: Level, from: &Descent) -> Option<Undescending> {
        let descending: Vec<u32> = arguments
            .iter()
            .enumerate()
            .filter_map(|(position, argument)| {
                let position = u32::try_from(position).ok()?;
                let level = variable(argument, depth)?;
                let held = from.get(&level)?;
                (held.smaller && held.position == position).then_some(position)
            })
            .collect();
        if descending.is_empty() {
            return Some(Undescending(at));
        }
        let agreed = match self.agreed.take() {
            Some(agreed) => agreed
                .into_iter()
                .filter(|position| descending.contains(position))
                .collect(),
            None => descending,
        };
        if agreed.is_empty() {
            return Some(Undescending(at));
        }
        self.agreed = Some(agreed);
        None
    }

    /// Whether this head is the definition being checked.
    fn recursive(&self, head: &Term) -> bool {
        matches!(head.shape(), Shape::Named { name, role: Role::Defined, .. } if name == self.name)
    }
}

/// The level a term names, when it is a bare variable.
fn variable(term: &Term, depth: Level) -> Option<Level> {
    let Shape::Var(index) = term.shape() else {
        return None;
    };
    depth.0.checked_sub(index.0)?.checked_sub(1).map(Level)
}

/// A written application spine: its head, and what is applied to it, outermost
/// argument last.
fn spine(term: &Term) -> (&Term, Vec<&Term>) {
    let mut arguments = Vec::new();
    let mut head = term;
    while let Shape::App { function, argument } = head.shape() {
        arguments.push(argument);
        head = function;
    }
    arguments.reverse();
    (head, arguments)
}
