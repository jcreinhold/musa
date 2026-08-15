//! Universe levels, and the metavariables that stand for the ones nobody wrote.
//!
//! `docs/rules/language/02-core-calculus.md` §1 gives levels their own sort,
//! `0 | succ l | max l l | ℓ`, where `ℓ` is a level metavariable the elaborator
//! solves. Prompt 133 could represent a level as a number because every level a
//! term held was closed, and a closed level built from `0`, `succ`, and `max`
//! *is* a natural number. Prompt 135 opens §2.1's third creation site — a family
//! parameterized by `(A : Type l)` is the first declaration that cannot write
//! its own levels — so a level is now a number *and* whatever unsolved arms it
//! is the maximum of.
//!
//! # The representation, and why it is normalized rather than a tree
//!
//! A level is `max(k, ℓ₁ + k₁, …, ℓₙ + kₙ)`: one constant, and at most one
//! offset per metavariable. Every level built by [`Level::succ`] and
//! [`Level::max`] is in that form already, because both operations maintain it —
//! `succ` adds one to the constant and to every offset, `max` merges two lists
//! keeping the larger offset per variable, and a constant no larger than some
//! arm's offset is dropped, since every level is at least `0` and so `max(1,
//! ℓ + 1)` *is* `ℓ + 1`.
//!
//! A syntax tree of `succ` and `max` nodes would have been the obvious
//! representation and is the wrong one. Conversion compares levels for
//! *equality* (universes are predicative and not cumulative, §1), and equality
//! on trees is not structural: `max(succ ℓ, 1)` and `succ (max ℓ, 0)` are the
//! same level and different trees. Deciding that would mean a normalizer, which
//! is this form, reached later and after every caller has had a chance to
//! compare two trees by mistake. Here the only way to build a level maintains
//! the normal form, so `==` is the answer.
//!
//! # What still computes and what gets stuck
//!
//! `succ` and `max` compute on the constant arm always. They get stuck on a
//! variable arm — `max(ℓ, 3)` is neither `ℓ` nor `3` until `ℓ` is known — which
//! is the change the prompt describes: **every place that reads a level has to
//! resolve it first**, because an arm may have been solved since the level was
//! built. [`Level::resolved`] is that operation and [`Level::is_closed`] is the
//! question a caller asks when it needs a number.
//!
//! [`Level::resolved`] answers a `Level` rather than an `Option<Level>` — the
//! shape [`crate::eval::force`] uses for values — because a closed level is
//! `{u32, None}` and cloning one allocates nothing. The `Option` exists there to
//! keep the common case free; here the common case already is.

use std::sync::Arc;
use std::sync::OnceLock;

/// A universe level: `max(k, ℓ₁ + k₁, …)`.
///
/// Ordered by [`Level::depth`] on the constant arm alone, which is a total
/// order on *closed* levels and nothing more. Conversion compares levels with
/// `==`, never with `<`: universes are predicative and not cumulative (§1), so
/// `Type 0` is not a `Type 1`.
#[derive(Clone, Debug)]
pub struct Level {
    /// The constant arm. `max` includes at least this numeral.
    ///
    /// Zero whenever some arm's offset already reaches it, because a level is at
    /// least `0` and so that arm dominates. Keeping the redundant constant would
    /// make `succ ?ℓ` and `max(1, ?ℓ + 1)` two values for one level.
    constant: u32,
    /// The `ℓ + offset` arms, sorted by metavariable id, one entry per variable.
    ///
    /// `None` for a closed level, which is the overwhelmingly common case and
    /// the one worth keeping free of an allocation: every level a `.musa`
    /// program writes today is closed, and so is every level in a term that has
    /// finished elaborating.
    vars: Option<Arc<[Arm]>>,
}

/// One `ℓ + offset` arm of a level's maximum.
#[derive(Clone, Debug)]
struct Arm {
    var: LevelMeta,
    offset: u32,
}

/// A level nobody wrote, to be determined by a constraint.
///
/// §2.1's third creation site. Solved write-once through a [`OnceLock`], the
/// same discipline term metavariables use, so a solution cannot be replaced by
/// a later constraint and acceptance cannot depend on the order they arrived
/// in.
#[derive(Clone, Debug)]
pub(crate) struct LevelMeta {
    id: u32,
    solution: Arc<OnceLock<Level>>,
}

impl LevelMeta {
    /// A fresh unsolved level metavariable with the stated id.
    #[must_use]
    pub(crate) fn new(id: u32) -> Self {
        Self {
            id,
            solution: Arc::new(OnceLock::new()),
        }
    }

    /// Whether a constraint has determined it.
    #[must_use]
    pub(crate) fn is_solved(&self) -> bool {
        self.solution.get().is_some()
    }

    /// Determine it, if it is not determined already.
    ///
    /// Answers whether this call is the one that set it. A second attempt with
    /// a *different* level is a unifier defect rather than a program error, and
    /// the caller that cares checks the stored solution instead.
    fn solve(&self, level: Level) -> bool {
        self.solution.set(level).is_ok()
    }

    /// The level it was determined to be.
    fn solution(&self) -> Option<&Level> {
        self.solution.get()
    }
}

impl Level {
    /// The lowest universe, `Type 0` — where every type a program writes lives.
    pub const ZERO: Self = Self {
        constant: 0,
        vars: None,
    };

    /// The level metavariable `meta`, standing alone.
    #[must_use]
    pub(crate) fn variable(meta: LevelMeta) -> Self {
        Self::built(0, vec![Arm { var: meta, offset: 0 }])
    }

    /// `succ l`, the level of the universe that `Type l` itself inhabits.
    ///
    /// Saturating rather than wrapping: a level past `u32::MAX` is not a
    /// program anybody wrote, and wrapping would make `Type (succ l)` inhabit
    /// `Type 0`, which is `Type : Type` reached by arithmetic. Saturation keeps
    /// the hierarchy monotone at the cost of collapsing two levels no source
    /// can name.
    #[must_use]
    pub fn succ(&self) -> Self {
        let Some(arms) = self.vars.as_ref() else {
            return Self {
                constant: self.constant.saturating_add(1),
                vars: None,
            };
        };
        Self::built(
            self.constant.saturating_add(1),
            arms.iter()
                .map(|arm| Arm {
                    var: arm.var.clone(),
                    offset: arm.offset.saturating_add(1),
                })
                .collect(),
        )
    }

    /// `max l l'`, the level a formation rule assigns when it combines two.
    ///
    /// The merge is what keeps the representation normal: one arm per
    /// metavariable, at the larger of the two offsets, because `max(ℓ+2, ℓ+5)`
    /// is `ℓ+5` whatever `ℓ` turns out to be.
    #[must_use]
    pub fn max(&self, other: &Self) -> Self {
        let constant = self.constant.max(other.constant);
        match (self.vars.as_ref(), other.vars.as_ref()) {
            (None, None) => Self { constant, vars: None },
            (Some(arms), None) | (None, Some(arms)) => Self::built(constant, arms.to_vec()),
            (Some(left), Some(right)) => Self::built(constant, merge(left, right)),
        }
    }

    /// A level from a constant and its arms, in normal form.
    ///
    /// The one constructor that builds an open level, so the invariant the
    /// module doc states — a constant no arm already dominates — holds by
    /// construction rather than by every caller remembering it.
    fn built(constant: u32, arms: Vec<Arm>) -> Self {
        if arms.is_empty() {
            return Self { constant, vars: None };
        }
        let dominated = arms.iter().any(|arm| arm.offset >= constant);
        Self {
            constant: if dominated { 0 } else { constant },
            vars: Some(Arc::from(arms)),
        }
    }

    /// This level with every solved metavariable replaced by its solution.
    ///
    /// The operation the module doc names: a level is built once and read many
    /// times, and an arm may have been solved in between. **Every place that
    /// reads a level calls this first**, which is what prompt 135 changed about
    /// every such place.
    #[must_use]
    pub(crate) fn resolved(&self) -> Self {
        let Some(arms) = self.vars.as_ref() else {
            return self.clone();
        };
        if !arms.iter().any(|arm| arm.var.is_solved()) {
            return self.clone();
        }
        let mut resolved = Self {
            constant: self.constant,
            vars: None,
        };
        for arm in arms.iter() {
            let solved = match arm.var.solution() {
                // A solution may itself mention a solved metavariable, so the
                // recursion is not decoration: constraints arrive in whatever
                // order elaboration reaches them.
                Some(level) => level.resolved(),
                None => Self::variable(arm.var.clone()),
            };
            resolved = resolved.max(&shifted(&solved, arm.offset));
        }
        resolved
    }

    /// Whether every arm is determined, so the level is a number.
    #[must_use]
    pub(crate) fn is_closed(&self) -> bool {
        self.vars.is_none()
    }

    /// The constant arm, for a diagnostic that has to print something.
    ///
    /// A *lower bound* on an open level rather than its value, which is why it
    /// is not the whole of [`Display`](core::fmt::Display) and why nothing
    /// decides anything with it.
    #[must_use]
    pub const fn depth(&self) -> u32 {
        self.constant
    }

    /// The single unsolved metavariable this level *is*, and by how much it is
    /// shifted, when that is all it is.
    ///
    /// `?ℓ + j` and nothing else — one arm, and no constant exceeding it. That
    /// is the shape a constraint can invert, because `succ` is injective on the
    /// naturals: `?ℓ + j ≡ c` has the one solution `c − j`. Anything with two
    /// arms is a `max` of two unknowns, which does not.
    ///
    /// The constant is `0` by [`Level::built`]'s invariant whenever an arm
    /// dominates it, so a single-arm level with a nonzero constant really is a
    /// `max` — `max(3, ?ℓ)` — and really is not invertible.
    fn as_shifted_variable(&self) -> Option<(&LevelMeta, u32)> {
        let arms = self.vars.as_ref()?;
        match (self.constant, arms.as_ref()) {
            (0, [arm]) => Some((&arm.var, arm.offset)),
            _ => None,
        }
    }

    /// Whether `meta` occurs in this level.
    fn mentions(&self, meta: &LevelMeta) -> bool {
        self.vars
            .as_ref()
            .is_some_and(|arms| arms.iter().any(|arm| arm.var.id == meta.id))
    }

    /// Make `self` and `other` the same level, answering whether they can be.
    ///
    /// Deliberately narrow, and the narrowness is §2.1's rather than an
    /// omission. Three cases are decided and everything else is refused:
    ///
    /// - two levels already equal, which is every constraint in a program that
    ///   wrote all its levels;
    /// - `?ℓ ≡ l`, solved outright;
    /// - `?ℓ + j ≡ c` for a closed `c`, solved to `c − j`, or refused when
    ///   `c < j` because no natural satisfies it. This is the shape a written
    ///   universe produces: `Type` elaborates to `Type ?ℓ`, whose own type is
    ///   `Type (succ ?ℓ)`, and checking that against `Type 1` asks exactly
    ///   `succ ?ℓ ≡ 1`.
    ///
    /// What is refused is `max` against `max` with different unknowns —
    /// `max(?ℓ₁, ?ℓ₂) ≡ 3` has three solutions and no principal one. A general
    /// solver for those constraints is semi-decidable at best, and prompt 130
    /// refused search: a level solver that guessed would decide which universe a
    /// program lives in on evidence the program did not give. The bound is worth
    /// stating rather than hiding, because it is where this will be widened if a
    /// program ever needs it — with a postponement queue like §2.1's, not with a
    /// search.
    pub(crate) fn determine(&self, other: &Self) -> bool {
        let left = self.resolved();
        let right = other.resolved();
        if left == right {
            return true;
        }
        for (open, against) in [(&left, &right), (&right, &left)] {
            let Some((variable, offset)) = open.as_shifted_variable() else {
                continue;
            };
            if against.mentions(variable) {
                // The occurs check. `?ℓ ≡ succ ?ℓ` has no solution, and solving
                // it anyway would build a level whose resolution does not end.
                return false;
            }
            if offset == 0 {
                return variable.solve(against.clone());
            }
            let Some(constant) = against.is_closed().then(|| against.constant.checked_sub(offset)) else {
                continue;
            };
            return constant.is_some_and(|constant| variable.solve(Self { constant, vars: None }));
        }
        false
    }
}

/// `level + offset`, which is `offset` applications of `succ`.
fn shifted(level: &Level, offset: u32) -> Level {
    let mut shifted = level.clone();
    for _ in 0..offset {
        shifted = shifted.succ();
    }
    shifted
}

/// Merge two sorted arm lists, keeping the larger offset per metavariable.
fn merge(left: &[Arm], right: &[Arm]) -> Vec<Arm> {
    let mut merged: Vec<Arm> = Vec::with_capacity(left.len().saturating_add(right.len()));
    let (mut here, mut there) = (left.iter(), right.iter());
    let (mut next_left, mut next_right) = (here.next(), there.next());
    loop {
        match (next_left, next_right) {
            (None, None) => break,
            (Some(arm), None) => {
                merged.push(arm.clone());
                next_left = here.next();
            }
            (None, Some(arm)) => {
                merged.push(arm.clone());
                next_right = there.next();
            }
            (Some(one), Some(two)) => match one.var.id.cmp(&two.var.id) {
                std::cmp::Ordering::Less => {
                    merged.push(one.clone());
                    next_left = here.next();
                }
                std::cmp::Ordering::Greater => {
                    merged.push(two.clone());
                    next_right = there.next();
                }
                std::cmp::Ordering::Equal => {
                    merged.push(Arm {
                        var: one.var.clone(),
                        offset: one.offset.max(two.offset),
                    });
                    next_left = here.next();
                    next_right = there.next();
                }
            },
        }
    }
    merged
}

/// Equality is on the normal form, and every constructor maintains it.
///
/// Hand-written rather than derived only because [`Arm`] holds a [`LevelMeta`],
/// whose `OnceLock` is identity rather than a value: two arms are the same arm
/// when they name the same metavariable at the same offset.
impl PartialEq for Level {
    /// Equality is on the **resolved** level, not the written one.
    ///
    /// A level is built once and read many times, and an arm may have been
    /// solved in between — so `?ℓ` solved to `0` and a written `0` are one
    /// level, and comparing the representations would make them two. That
    /// matters wherever a term carries a level a use site chose: a `match`
    /// compiles to a recursor at a solved metavariable and the same recursor
    /// written by hand carries the numeral, and §3 calls those convertible.
    ///
    /// The resolution is cheap where nothing was solved — [`Level::resolved`]
    /// answers a clone without walking — which is every comparison in a
    /// program that wrote its levels down.
    fn eq(&self, other: &Self) -> bool {
        Self::same(&self.resolved(), &other.resolved())
    }
}

impl Level {
    /// Whether two *resolved* levels are the same normal form.
    fn same(&self, other: &Self) -> bool {
        if self.constant != other.constant {
            return false;
        }
        match (self.vars.as_ref(), other.vars.as_ref()) {
            (None, None) => true,
            (Some(left), Some(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right.iter())
                        .all(|(one, two)| one.var.id == two.var.id && one.offset == two.offset)
            }
            (Some(_), None) | (None, Some(_)) => false,
        }
    }
}

impl Eq for Level {}

impl std::hash::Hash for Level {
    /// Resolved first, for the reason [`PartialEq`] is: a hash that disagreed
    /// with equality would put one level in two buckets.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let resolved = self.resolved();
        resolved.constant.hash(state);
        if let Some(arms) = resolved.vars.as_ref() {
            for arm in arms.iter() {
                arm.var.id.hash(state);
                arm.offset.hash(state);
            }
        }
    }
}

/// Printed as the level a reader would write, not as the representation.
///
/// A closed level is its numeral, a lone metavariable is `?lN`, and only a real
/// maximum is spelled `max(…)` — so the constant arm the normal form drops does
/// not reappear as `max(0, ?l0)` in a diagnostic about `?l0`.
impl core::fmt::Display for Level {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let Some(arms) = self.vars.as_ref() else {
            return write!(out, "{}", self.constant);
        };
        if let Some((var, offset)) = self.as_shifted_variable() {
            return match offset {
                0 => write!(out, "?l{}", var.id),
                _ => write!(out, "?l{} + {offset}", var.id),
            };
        }
        out.write_str("max(")?;
        write!(out, "{}", self.constant)?;
        for arm in arms.iter() {
            out.write_str(", ")?;
            if arm.offset == 0 {
                write!(out, "?l{}", arm.var.id)?;
            } else {
                write!(out, "?l{} + {}", arm.var.id, arm.offset)?;
            }
        }
        out.write_str(")")
    }
}

#[cfg(test)]
mod tests {
    use super::{Level, LevelMeta};

    #[test]
    fn max_is_the_join_of_two_levels() {
        let zero = Level::ZERO;
        let one = zero.succ();
        assert_eq!(zero.max(&one), one);
        assert_eq!(one.max(&zero), one);
        assert_eq!(one.max(&one), one);
    }

    #[test]
    fn the_hierarchy_is_strictly_increasing_and_never_wraps() {
        let mut level = Level::ZERO;
        for _ in 0..8 {
            let next = level.succ();
            assert!(level.depth() < next.depth(), "succ must climb");
            level = next;
        }
    }

    /// The reason the representation is normalized: two spellings of one level
    /// must be one value, or conversion would need a normalizer of its own.
    #[test]
    fn two_spellings_of_one_open_level_are_one_value() {
        let variable = Level::variable(LevelMeta::new(0));
        assert_eq!(
            variable.succ().max(&Level::ZERO.succ()),
            variable.max(&Level::ZERO).succ()
        );
        assert_eq!(variable.max(&variable), variable);
    }

    /// `succ` computes on the constant arm and is stuck on the variable one.
    #[test]
    fn a_level_with_an_unsolved_arm_is_not_closed() {
        let variable = Level::variable(LevelMeta::new(0));
        assert!(!variable.is_closed());
        assert!(!variable.succ().is_closed());
        assert!(Level::ZERO.succ().is_closed());
    }

    #[test]
    fn a_bare_metavariable_is_determined_by_the_level_it_meets() {
        let meta = LevelMeta::new(7);
        let variable = Level::variable(meta.clone());
        let two = Level::ZERO.succ().succ();
        assert!(variable.determine(&two));
        assert!(meta.is_solved());
        assert_eq!(variable.resolved(), two);
        // Write-once: meeting the same level again is agreement, not a re-solve.
        assert!(variable.determine(&two));
        assert!(!variable.determine(&Level::ZERO));
    }

    /// The constraint a written `Type` produces: `succ ?ℓ ≡ 1`, and `succ` is
    /// injective, so `?ℓ` is `0` and nothing was guessed to get there.
    #[test]
    fn a_shifted_metavariable_is_determined_against_a_closed_level() {
        let variable = Level::variable(LevelMeta::new(0));
        assert!(variable.succ().determine(&Level::ZERO.succ()));
        assert_eq!(variable.resolved(), Level::ZERO);
    }

    /// `succ ?ℓ ≡ 0` is unsatisfiable over the naturals rather than merely
    /// undecided, so it is refused rather than postponed.
    #[test]
    fn a_shift_past_the_level_it_meets_is_refused() {
        let meta = LevelMeta::new(0);
        let variable = Level::variable(meta.clone());
        assert!(!variable.succ().determine(&Level::ZERO));
        assert!(!meta.is_solved());
    }

    /// The bound [`Level::determine`] documents: a maximum of two unknowns has
    /// no principal solution, so nothing is solved and nothing is guessed.
    #[test]
    fn a_maximum_of_two_unknowns_is_refused_rather_than_guessed() {
        let (first, second) = (LevelMeta::new(0), LevelMeta::new(1));
        let joined = Level::variable(first.clone()).max(&Level::variable(second.clone()));
        assert!(!joined.determine(&Level::ZERO.succ()));
        assert!(!first.is_solved());
        assert!(!second.is_solved());
    }

    /// `?ℓ ≡ succ ?ℓ` has no solution, and building one would not terminate.
    #[test]
    fn a_level_that_would_contain_itself_is_refused() {
        let meta = LevelMeta::new(1);
        let variable = Level::variable(meta.clone());
        assert!(!variable.determine(&variable.succ()));
        assert!(!meta.is_solved());
    }

    /// Resolution follows a solution that was itself solved later.
    #[test]
    fn resolving_follows_a_chain_of_solutions() {
        let (first, second) = (LevelMeta::new(0), LevelMeta::new(1));
        let (one, two) = (Level::variable(first), Level::variable(second));
        assert!(one.determine(&two));
        assert!(two.determine(&Level::ZERO.succ()));
        assert_eq!(one.resolved(), Level::ZERO.succ());
    }
}
