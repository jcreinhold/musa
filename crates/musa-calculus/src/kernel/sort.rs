//! Universe levels: the expressions, the normal form they are compared in, and
//! the unknowns a declaration generalizes.
//!
//! `02-core-calculus.md` §1's hierarchy is `Type ℓ` for a level expression
//!
//! ```text
//! ℓ ::= 0  |  u  |  ℓ + 1  |  max ℓ ℓ
//! ```
//!
//! and this module is that grammar together with the only thing that makes it
//! usable: a normal form. Every expression the four formers build is equal to
//! exactly one
//!
//! ```text
//! max(k, u₁+k₁, …, uₙ+kₙ)
//! ```
//!
//! — a constant, and at most one offset per level variable — so [`Sort`] stores
//! *that* and the formers are smart constructors over it. Equality is then
//! structural, which is the property §3 needs: conversion compares universes
//! with `==` and never with `<`, because the hierarchy is predicative and **not
//! cumulative**, and a `Type i` is not a `Type j`.
//!
//! **The normal form decides equality completely, and that is not true of every
//! level algebra.** Lean's is incomplete because it carries a fifth former for
//! impredicative Π, whose value depends on whether its right argument is zero
//! and so cannot be normalized away. Musa has no `Prop` and no impredicativity,
//! so §1 gives the grammar four formers and not five; over `0`, `+1`, `max` and
//! variables ranging over the naturals, two expressions denote the same function
//! of their variables exactly when their normal forms agree. So "convertible" is
//! decided here rather than approximated. §1 names the missing former and says
//! why a reader coming from Lean will look for it.
//!
//! **No ordering.** [`Sort`] deliberately has no [`Ord`]: an ordering would be
//! the shape cumulativity is written in, and a level expression with variables
//! is only partially ordered anyway. Formation rules take [`Sort::max`];
//! conversion takes `==`; nothing takes `<`.
//!
//! # A variable has two lifetimes and one representation
//!
//! [`SortVar`] is a shared cell, exactly as [`Meta`](crate::kernel::meta::Meta)
//! is and for the same reason: a variable embedded in ten types is one unknown,
//! and solving it there solves it in all ten. While a declaration is being
//! elaborated a variable is **open** — the level solver may assign it — and when
//! the declaration closes, whatever is still open and still reachable from what
//! is stored is **generalized** into one of the declaration's level parameters.
//! A parameter is an open variable nobody will ever solve, which is why one type
//! spells both: the difference is a fact about the declaration boundary, not
//! about the variable.

use std::sync::{Arc, OnceLock};

use crate::kernel::error::Malformed;
use crate::kernel::origin::Origin;

/// A level variable: an unknown while its declaration is open, one of that
/// declaration's parameters afterwards.
///
/// Cloning shares the cell. Equality, ordering and hashing are by id, so a
/// normal form can keep its terms sorted and deduplicated without ever looking
/// at a solution.
///
/// **Ids are a per-declaration numbering, not a global one.** Two declarations
/// each own a `u0`, and what keeps them apart is that neither ever meets the
/// other: a stored parameter is substituted away at every use before anything
/// compares it, and [`Sort::substitute`] is simultaneous so that a replacement
/// sharing an id with what it replaced terminates rather than recurring. See
/// `elaboration::elab::levels` for the invariant in full.
#[derive(Clone)]
pub struct SortVar(Arc<VarCell>);

struct VarCell {
    id: u32,
    origin: Origin,
    solution: OnceLock<Sort>,
}

impl SortVar {
    /// A fresh variable, identified by `id`, first written at `origin`.
    ///
    /// **`id` is a numbering, not a name.** Level variables are numbered per
    /// declaration — see `elaboration::elab::levels` — so a variable a host
    /// builds belongs to the host's own numbering and must not be mixed into a
    /// term the elaborator is still working on. Building one is for a host that
    /// hands the core a type with an explicit level in it, which §1 allows and
    /// does not document as ordinary usage.
    #[must_use]
    pub fn new(id: u32, origin: Origin) -> Self {
        Self(Arc::new(VarCell {
            id,
            origin,
            solution: OnceLock::new(),
        }))
    }

    /// Which variable this is, for a report and for the normal form's ordering.
    #[must_use]
    pub fn id(&self) -> u32 {
        self.0.id
    }

    /// Where the level it stands for was written.
    #[must_use]
    pub fn origin(&self) -> Origin {
        self.0.origin
    }

    /// Write its solution. Write-once, for
    /// [`Meta::solve`](crate::kernel::meta::Meta)'s reason: assignment is the
    /// only solver and it checks before it writes.
    ///
    /// # Errors
    ///
    /// [`Malformed::AlreadySolved`] on a second write, which is a solver defect
    /// rather than a program's fault.
    pub(crate) fn solve(&self, level: Sort) -> Result<(), Malformed> {
        self.0
            .solution
            .set(level)
            .map_err(|_| Malformed::AlreadySolved(self.0.id))
    }

    /// Its solution, if the level solver has found one.
    #[must_use]
    pub fn solution(&self) -> Option<&Sort> {
        self.0.solution.get()
    }
}

impl PartialEq for SortVar {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for SortVar {}

impl PartialOrd for SortVar {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SortVar {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.id.cmp(&other.0.id)
    }
}

impl core::hash::Hash for SortVar {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
    }
}

impl core::fmt::Debug for SortVar {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(out, "u{}", self.0.id)
    }
}

/// One `u + k` term of a normal form.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct Offset {
    var: SortVar,
    plus: u32,
}

/// A universe level: `Type ℓ`'s ℓ, in normal form.
///
/// Named `Sort` and not `Level` because [`crate::kernel::term::Level`] is a
/// position in an environment, and one crate cannot spell two unrelated
/// numberings the same way and expect the mix-up to be caught.
///
/// Build one with [`Sort::ZERO`], [`Sort::constant`], [`Sort::var`],
/// [`Sort::succ`] and [`Sort::max`] — §1's four formers, plus the constant
/// shorthand that repeated `succ`ing from zero would spell. Each answers a
/// normal form, so there is no separate normalization step to forget to call
/// and no un-normalized value to compare by mistake.
#[derive(Clone, Default)]
pub struct Sort {
    /// The `k` of `max(k, …)`.
    constant: u32,
    /// The `uᵢ+kᵢ` terms, sorted by variable, one entry per variable.
    ///
    /// [`None`] rather than an empty slice so that [`Sort::ZERO`] is a `const`
    /// and a closed level — which is nearly all of them — costs no allocation.
    vars: Option<Arc<[Offset]>>,
}

impl Sort {
    /// The lowest universe, `Type 0`.
    pub const ZERO: Self = Self {
        constant: 0,
        vars: None,
    };

    /// `Type 1`, where a type of types stands.
    pub const ONE: Self = Self {
        constant: 1,
        vars: None,
    };

    /// The closed level `k`.
    #[must_use]
    pub const fn constant(k: u32) -> Self {
        Self {
            constant: k,
            vars: None,
        }
    }

    /// The level variable `u`.
    #[must_use]
    pub fn var(var: SortVar) -> Self {
        var.solution().cloned().unwrap_or(Self {
            constant: 0,
            vars: Some(Arc::from(vec![Offset { var, plus: 0 }])),
        })
    }

    /// `ℓ + 1`: the universe that `Type ℓ` itself inhabits.
    ///
    /// Total, and that is the deliverable. The predecessor of this module
    /// answered `Option` here and the `None` was the ceiling — the refusal that
    /// said a declaration needed a universe above `Type 1` and there were two.
    /// There is no ceiling, so there is no `None`.
    #[must_use]
    pub fn succ(&self) -> Self {
        let forced = self.forced();
        Self {
            constant: forced.constant.saturating_add(1),
            vars: forced.vars.as_ref().map(|vars| {
                vars.iter()
                    .map(|offset| Offset {
                        var: offset.var.clone(),
                        plus: offset.plus.saturating_add(1),
                    })
                    .collect()
            }),
        }
    }

    /// `max ℓ ℓ'`, the level a formation rule assigns when it combines two.
    ///
    /// The only place the hierarchy goes *up* other than [`Self::succ`], and the
    /// only comparison in this module: `max` on the constant, and per variable
    /// the larger offset, which is the join of two normal forms because `max`
    /// distributes over itself and over `+1`.
    #[must_use]
    pub fn max(&self, other: &Self) -> Self {
        let (here, there) = (self.forced(), other.forced());
        let constant = here.constant.max(there.constant);
        let (Some(left), Some(right)) = (here.vars.as_deref(), there.vars.as_deref()) else {
            return Self {
                constant,
                vars: here.vars.or(there.vars),
            };
        };
        let mut merged: Vec<Offset> = Vec::with_capacity(left.len().saturating_add(right.len()));
        let (mut one, mut two) = (0_usize, 0_usize);
        while let (Some(this), Some(that)) = (left.get(one), right.get(two)) {
            match this.var.cmp(&that.var) {
                core::cmp::Ordering::Equal => {
                    merged.push(Offset {
                        var: this.var.clone(),
                        plus: this.plus.max(that.plus),
                    });
                    one = one.saturating_add(1);
                    two = two.saturating_add(1);
                }
                core::cmp::Ordering::Less => {
                    merged.push(this.clone());
                    one = one.saturating_add(1);
                }
                core::cmp::Ordering::Greater => {
                    merged.push(that.clone());
                    two = two.saturating_add(1);
                }
            }
        }
        merged.extend_from_slice(left.get(one..).unwrap_or(&[]));
        merged.extend_from_slice(right.get(two..).unwrap_or(&[]));
        Self {
            constant,
            vars: Some(Arc::from(merged)),
        }
    }

    /// This level with every solved variable replaced by its solution,
    /// transitively.
    ///
    /// Cheap when nothing is solved, which is the common case: the answer shares
    /// this level's own allocation. Transitive because a solution may itself
    /// mention a variable that was solved later, and the occurs check
    /// [`Conversion`](crate::elaboration::convert) makes before every assignment
    /// is what makes the recursion terminate.
    #[must_use]
    pub fn forced(&self) -> Self {
        let Some(vars) = self.vars.as_deref() else {
            return self.clone();
        };
        if vars.iter().all(|offset| offset.var.solution().is_none()) {
            return self.clone();
        }
        self.substitute(&|var: &SortVar| var.solution().map(Self::forced))
    }

    /// The closed level this is, if it mentions no variable.
    #[must_use]
    pub fn as_constant(&self) -> Option<u32> {
        let forced = self.forced();
        forced.vars.is_none().then_some(forced.constant)
    }

    /// The variable `u` this is, if it is one and nothing else.
    ///
    /// The one shape the level solver may assign to: `?u ≡ ℓ` has the single
    /// solution `ℓ`, and every other shape has none or several.
    #[must_use]
    pub fn as_var(&self) -> Option<SortVar> {
        let forced = self.forced();
        let [only] = forced.vars.as_deref()? else {
            return None;
        };
        (forced.constant == 0 && only.plus == 0).then(|| only.var.clone())
    }

    /// The `max(k, u+j)` this is, if exactly one variable occurs in it.
    ///
    /// The shape the level solver can still answer beyond [`Self::as_var`]:
    /// `max(k, u+j) ≡ c` for a closed `c` above `k` has the single solution
    /// `u = c - j`, and everything with two variables in it has several or
    /// none. Answers `(k, u, j)`.
    #[must_use]
    pub fn as_single_var(&self) -> Option<(u32, SortVar, u32)> {
        let forced = self.forced();
        let [only] = forced.vars.as_deref()? else {
            return None;
        };
        Some((forced.constant, only.var.clone(), only.plus))
    }

    /// Every variable this mentions, once each, in ascending order.
    #[must_use]
    pub fn vars(&self) -> Vec<SortVar> {
        self.forced()
            .vars
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|offset| offset.var.clone())
            .collect()
    }

    /// This level with each variable replaced by what `with` answers for it.
    ///
    /// The one operation both halves of §1's "generalized per declaration,
    /// instantiated at each use" are written in: generalization records which
    /// variables were still open, and a use site replaces those with the levels
    /// it is used at. `None` leaves a variable alone.
    ///
    /// **Simultaneous**, and that is load-bearing rather than a nuance: a
    /// replacement is used as it stands and is not itself substituted into.
    /// Variables are numbered per declaration, so a use site's `u0` and the
    /// declaration's `u0` are two variables that share an identity, and an
    /// iterated substitution would replace the answer with itself forever.
    #[must_use]
    pub fn substitute(&self, with: &impl Fn(&SortVar) -> Option<Self>) -> Self {
        let Some(vars) = self.vars.as_deref() else {
            return self.clone();
        };
        let mut built = Self::constant(self.constant);
        for offset in vars {
            let mut lifted = with(&offset.var).unwrap_or_else(|| Self {
                constant: 0,
                vars: Some(Arc::from(vec![Offset {
                    var: offset.var.clone(),
                    plus: 0,
                }])),
            });
            for _ in 0..offset.plus {
                lifted = lifted.succ();
            }
            built = built.max(&lifted);
        }
        built
    }

    /// Whether two normal forms already agree, without forcing either.
    fn same(&self, other: &Self) -> bool {
        self.constant == other.constant && self.vars.as_deref().unwrap_or(&[]) == other.vars.as_deref().unwrap_or(&[])
    }
}

impl PartialEq for Sort {
    /// Equality of the *forced* normal forms, which is equality of the levels:
    /// §3's conversion asks this and asks nothing weaker.
    fn eq(&self, other: &Self) -> bool {
        self.forced().same(&other.forced())
    }
}

impl Eq for Sort {}

impl core::hash::Hash for Sort {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        let forced = self.forced();
        forced.constant.hash(state);
        for offset in forced.vars.as_deref().unwrap_or(&[]) {
            offset.var.id().hash(state);
            offset.plus.hash(state);
        }
    }
}

impl core::fmt::Display for Sort {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let forced = self.forced();
        let vars = forced.vars.as_deref().unwrap_or(&[]);
        // `max(0, u)` is `u`, because a level is a natural number: the constant
        // is written only when it is the whole level or when it could win.
        let mut terms: Vec<String> = Vec::with_capacity(vars.len().saturating_add(1));
        if vars.is_empty() || forced.constant > 0 {
            terms.push(forced.constant.to_string());
        }
        for offset in vars {
            let var = offset.var.id();
            terms.push(if offset.plus == 0 {
                format!("u{var}")
            } else {
                format!("u{var}+{}", offset.plus)
            });
        }
        // Right-nested, because §1's `max` is binary and a reader should be able
        // to read the spelling back into the grammar.
        let mut written = terms.pop().unwrap_or_else(|| "0".to_owned());
        while let Some(term) = terms.pop() {
            written = format!("max {term} ({written})");
        }
        out.write_str(&written)
    }
}

/// A level shows as the level it *is*, which is the forced form.
///
/// Written out rather than derived because the derived form is the
/// representation — a constant and a list of offsets, each naming a variable
/// whose solution is somewhere else — and a debug dump that shows `u3` where
/// the level is `0` reports the elaborator's bookkeeping in place of the
/// program's meaning.
impl core::fmt::Debug for Sort {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self, out)
    }
}

/// The level arguments a use site instantiates a name at.
///
/// Empty for every name that is not level-polymorphic, which is nearly all of
/// them — hence [`None`] rather than an empty slice, so [`Levels::NONE`] is a
/// `const` and the common case costs no allocation.
///
/// This is the half of §1's "instantiated at each use" that lives in the
/// *term*. Without it the elaborated program would not record which levels a
/// polymorphic name was used at, and the re-checker could not verify that the
/// uses agree — it would have to infer them again, which is not re-checking.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct Levels(Option<Arc<[Sort]>>);

impl Levels {
    /// No level arguments: the name is not level-polymorphic.
    pub const NONE: Self = Self(None);

    /// The level arguments `levels`, in the order the declaration's parameters
    /// were generalized.
    #[must_use]
    pub fn of(levels: impl IntoIterator<Item = Sort>) -> Self {
        let levels: Vec<Sort> = levels.into_iter().collect();
        if levels.is_empty() {
            return Self::NONE;
        }
        Self(Some(Arc::from(levels)))
    }

    /// The arguments, in order.
    #[must_use]
    pub fn as_slice(&self) -> &[Sort] {
        self.0.as_deref().unwrap_or(&[])
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    /// Each argument with `with` applied to its variables.
    #[must_use]
    pub fn substitute(&self, with: &impl Fn(&SortVar) -> Option<Sort>) -> Self {
        if self.0.is_none() {
            return Self::NONE;
        }
        Self::of(self.as_slice().iter().map(|level| level.substitute(with)))
    }

    /// Each argument with every solved variable replaced by its solution.
    #[must_use]
    pub fn forced(&self) -> Self {
        if self.0.is_none() {
            return Self::NONE;
        }
        Self::of(self.as_slice().iter().map(Sort::forced))
    }

    /// Every variable the arguments mention, with repeats.
    #[must_use]
    pub fn vars(&self) -> Vec<SortVar> {
        self.as_slice().iter().flat_map(Sort::vars).collect()
    }
}

impl core::fmt::Display for Levels {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let levels = self.as_slice();
        if levels.is_empty() {
            return Ok(());
        }
        out.write_str(".{")?;
        for (position, level) in levels.iter().enumerate() {
            if position > 0 {
                out.write_str(", ")?;
            }
            write!(out, "{level}")?;
        }
        out.write_str("}")
    }
}
