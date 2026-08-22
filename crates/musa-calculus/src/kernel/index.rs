//! The index stratum: whether two index expressions denote the same quantity.
//!
//! `docs/rules/language/02-core-calculus.md` §1.5 admits index arguments on a
//! type — `Row(n)`, `Pc(12)`, `Bar(p + q)` — drawn from a fixed decidable
//! arithmetic domain, erased before evaluation, and decided **here** rather
//! than by conversion. This module is that decision.
//!
//! # Why it is a separate module and not an arm of [`crate::elaboration::convert`]
//!
//! §1.5's whole design is that two questions get two deciders. *Are these terms
//! equal* is normalization by evaluation over the semantic domain; *are these
//! indices equal* is arithmetic. The calculus this one replaced settled both by
//! unification, and the second is where the machinery and the cost lived, so
//! deleting one strand would have been subtraction rather than the separation
//! that was actually wanted.
//!
//! The separation is structural, and it is checkable by reading the imports:
//! **this module does not import [`crate::kernel::value`]**. It is handed a normalized
//! [`Expr`] and answers; it never evaluates, never forces, and never sees a
//! term. Reading an index position *into* an `Expr` — or refusing it — is
//! [`crate::elaboration::convert`]'s job, because that is the one place a comparison meets
//! one, and it is the only place that has to know the grammar. If this module
//! ever needs a `Value`, §1.5's stratification has been broken.
//!
//! # Normalization is the constructor's job
//!
//! An [`Expr`] is a **linear form**: a rational constant plus a finite set of
//! variables with nonzero rational coefficients, the variables ordered. Every
//! constructor here maintains that shape, so [`decide`] is structural equality
//! and there is no separate normalization pass to remember to call. `p + q` and
//! `q + p` build the same `Expr`, which is the whole gain over comparing the
//! terms.
//!
//! That is the fragment prompt 142d is scoped to, and it is not a general
//! Presburger procedure: there are no quantifiers, no divisibility, and no
//! disjunction, because equality of two linear forms settles every program note
//! 51 §3 exhibits. Anything outside the grammar is refused at the expression by
//! the reader, never approximated and never assumed.
//!
//! # Why exact arithmetic is written here rather than taken from a crate
//!
//! Roadmap §15 fixes this crate's dependency list at three, and says why
//! `num-rational` is not on it: "a core that reaches for `num-rational` has
//! started to know what a duration is". [`Exact`] is sixty lines of reduced
//! fraction with checked operations, private to this module, and knows nothing
//! about time. The alternative was a dependency whose presence would have been
//! read as the core owning a musical quantity.

use crate::kernel::budget::Meter;
use crate::kernel::error::CoreError;

/// The domain an index is drawn from.
///
/// Two arms and not §1.5's three, and the missing one is the interesting case.
/// A **finite literal enum** is admitted as an index domain by §1.5, and needs
/// no solver: two enum literals are equal or they are not, which conversion at
/// a base type already decides through [`crate::kernel::base::Payload::same`]. There is
/// nothing linear to normalize and nothing arithmetic to prove, so building an
/// arm for it would be machinery with no question to answer.
///
/// The two here are the sorts where two *different* expressions can denote one
/// quantity, which is exactly the condition for wanting a decider at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sort {
    /// A counting family — `Nat`. Its values reach this module as
    /// [`crate::kernel::family::Numeral`]'s count, which the core holds directly.
    Count,
    /// An exact rational — `Ratio`. Its values are base literals, opaque to
    /// this crate, so the host says how to read one
    /// ([`crate::kernel::base::Base::measuring`]).
    Rational,
}

/// An exact rational: a reduced fraction with a positive denominator.
///
/// Written here rather than depended on, for the reason the module header
/// gives. Every operation is checked: an index expression that would overflow
/// is a refusal at the expression, not a wrapped number that decides a type
/// wrongly. `i128` and `u128` are wide enough that a musical index reaches the
/// check only through deliberate abuse, and the check is what makes that
/// abuse a diagnostic instead of a silent wrong answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Exact {
    numerator: i128,
    /// Always nonzero, and always coprime with the numerator's magnitude.
    denominator: u128,
}

impl Exact {
    /// Zero.
    pub(crate) const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    /// One.
    pub(crate) const ONE: Self = Self {
        numerator: 1,
        denominator: 1,
    };

    /// `numerator / denominator`, reduced.
    ///
    /// # Errors
    ///
    /// `None` when the denominator is zero, which is the reader's refusal
    /// rather than this module's business — division is not in §1.5's grammar,
    /// so the only way to reach a zero denominator is a host that answered one.
    pub(crate) fn new(numerator: i128, denominator: i128) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        let sign = if denominator < 0 { -1 } else { 1 };
        let numerator = numerator.checked_mul(sign)?;
        let denominator = denominator.unsigned_abs();
        let divisor = gcd(numerator.unsigned_abs(), denominator);
        // `divisor` divides both, and dividing a nonzero magnitude by its own
        // divisor cannot overflow, so neither conversion below can fail.
        let divisor = i128::try_from(divisor).ok()?;
        Some(Self {
            numerator: numerator.checked_div(divisor)?,
            denominator: denominator.checked_div(divisor.unsigned_abs())?,
        })
    }

    /// A whole number.
    pub(crate) const fn whole(count: i128) -> Self {
        Self {
            numerator: count,
            denominator: 1,
        }
    }

    const fn is_zero(self) -> bool {
        self.numerator == 0
    }

    fn add(self, other: Self) -> Option<Self> {
        let left = self.numerator.checked_mul(i128::try_from(other.denominator).ok()?)?;
        let right = other.numerator.checked_mul(i128::try_from(self.denominator).ok()?)?;
        let denominator = self.denominator.checked_mul(other.denominator)?;
        Self::new(left.checked_add(right)?, i128::try_from(denominator).ok()?)
    }

    fn multiply(self, other: Self) -> Option<Self> {
        Self::new(
            self.numerator.checked_mul(other.numerator)?,
            i128::try_from(self.denominator.checked_mul(other.denominator)?).ok()?,
        )
    }
}

/// The greatest common divisor, by Euclid. Answers 1 for `gcd(0, 0)` so that
/// [`Exact::new`] never divides by zero at the zero numerator.
const fn gcd(left: u128, right: u128) -> u128 {
    let (mut left, mut right) = (left, right);
    while right != 0 {
        let Some(next) = left.checked_rem(right) else {
            break;
        };
        left = right;
        right = next;
    }
    if left == 0 { 1 } else { left }
}

/// Which variable a linear term stands for.
///
/// A de Bruijn **level**, not an index, for the reason every other level in
/// this crate is one: an expression built under one depth is compared against
/// one built under another, and a level is stable under both.
pub(crate) type Variable = u32;

/// A normalized index expression: `c + Σ kᵢ·xᵢ`.
///
/// Invariants, maintained by every constructor and relied on by [`decide`]:
/// `terms` is sorted by variable, holds each variable at most once, and holds
/// no zero coefficient. Two expressions denote the same quantity exactly when
/// these three fields agree, which is what makes deciding structural.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Expr {
    sort: Sort,
    constant: Exact,
    terms: Vec<(Variable, Exact)>,
}

impl Expr {
    /// The constant `value`.
    pub(crate) const fn literal(sort: Sort, value: Exact) -> Self {
        Self {
            sort,
            constant: value,
            terms: Vec::new(),
        }
    }

    /// The variable `variable`, standing alone.
    pub(crate) fn variable(sort: Sort, variable: Variable) -> Self {
        Self {
            sort,
            constant: Exact::ZERO,
            terms: vec![(variable, Exact::ONE)],
        }
    }

    /// Whether this expression is the constant `value`.
    ///
    /// What a caller asks when it wants the *number* rather than the form —
    /// a diagnostic naming `Row(12)`, or a check that an index is closed.
    pub(crate) fn as_constant(&self) -> Option<Exact> {
        self.terms.is_empty().then_some(self.constant)
    }

    /// `self + other`, renormalized.
    ///
    /// # Errors
    ///
    /// [`CoreError::Exhausted`] at the budget, or `Ok(None)` when the two are
    /// of different sorts or the arithmetic overflows. Both of those are the
    /// reader's refusal to report, because only the reader knows which written
    /// expression they came from.
    pub(crate) fn add(&self, meter: &mut Meter, other: &Self) -> Result<Option<Self>, CoreError> {
        if self.sort != other.sort {
            return Ok(None);
        }
        meter.step("index addition")?;
        let Some(constant) = self.constant.add(other.constant) else {
            return Ok(None);
        };
        let mut terms = Vec::with_capacity(self.terms.len().saturating_add(other.terms.len()));
        let (mut left, mut right) = (self.terms.iter().peekable(), other.terms.iter().peekable());
        loop {
            meter.step("index term")?;
            let combined = match (left.peek(), right.peek()) {
                (None, None) => break,
                (Some(&&(variable, coefficient)), None) => {
                    left.next();
                    (variable, coefficient)
                }
                (None, Some(&&(variable, coefficient))) => {
                    right.next();
                    (variable, coefficient)
                }
                (Some(&&(one, first)), Some(&&(other, second))) => match one.cmp(&other) {
                    core::cmp::Ordering::Less => {
                        left.next();
                        (one, first)
                    }
                    core::cmp::Ordering::Greater => {
                        right.next();
                        (other, second)
                    }
                    core::cmp::Ordering::Equal => {
                        left.next();
                        right.next();
                        let Some(sum) = first.add(second) else {
                            return Ok(None);
                        };
                        (one, sum)
                    }
                },
            };
            if !combined.1.is_zero() {
                terms.push(combined);
            }
        }
        Ok(Some(Self {
            sort: self.sort,
            constant,
            terms,
        }))
    }

    /// `self - other`, renormalized.
    ///
    /// # Errors
    ///
    /// As [`Self::add`].
    pub(crate) fn subtract(&self, meter: &mut Meter, other: &Self) -> Result<Option<Self>, CoreError> {
        let Some(negated) = other.scale(meter, Exact::whole(-1))? else {
            return Ok(None);
        };
        self.add(meter, &negated)
    }

    /// `factor · self`, renormalized.
    ///
    /// §1.5 admits multiplication **by a literal** only, so a caller has a
    /// number here and never a second expression. That restriction is what
    /// keeps the form linear, and keeping it in the signature is what stops a
    /// later caller from quietly leaving the fragment.
    ///
    /// # Errors
    ///
    /// As [`Self::add`].
    pub(crate) fn scale(&self, meter: &mut Meter, factor: Exact) -> Result<Option<Self>, CoreError> {
        meter.step("index scaling")?;
        if factor.is_zero() {
            return Ok(Some(Self::literal(self.sort, Exact::ZERO)));
        }
        let Some(constant) = self.constant.multiply(factor) else {
            return Ok(None);
        };
        let mut terms = Vec::with_capacity(self.terms.len());
        for &(variable, coefficient) in &self.terms {
            meter.step("index term")?;
            let Some(scaled) = coefficient.multiply(factor) else {
                return Ok(None);
            };
            terms.push((variable, scaled));
        }
        Ok(Some(Self {
            sort: self.sort,
            constant,
            terms,
        }))
    }
}

/// What the solver says about two index expressions.
///
/// Two arms and no third. There is deliberately no *unknown*: §1.5 refuses an
/// index the solver cannot read at the reader, so by the time a pair reaches
/// here both are in the fragment and the fragment is decidable. An arm for
/// "cannot tell" would be a postponement, and postponement is what note 51 §7
/// keeps deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// The two denote the same quantity.
    Same,
    /// They do not, at every assignment of their variables.
    Different,
}

/// Whether two index expressions denote the same quantity.
///
/// Structural, because normalization already happened: see the module header.
/// Two expressions of different sorts are [`Verdict::Different`] rather than a
/// refusal — a `Nat` index and a `Ratio` index standing in one position is a
/// type disagreement, which the caller reports as the type disagreement it is.
pub(crate) fn decide(left: &Expr, right: &Expr) -> Verdict {
    if left == right {
        Verdict::Same
    } else {
        Verdict::Different
    }
}

#[cfg(test)]
// A fixture these tests cannot build is a defect in this module rather than an
// outcome any law is about, so panicking is the correct behaviour there.
#[expect(clippy::unwrap_used, reason = "a broken fixture is a defect in this module")]
mod tests {
    use super::*;
    use crate::kernel::budget::Budget;

    fn meter() -> Meter {
        Meter::new(Budget::LANGUAGE)
    }

    fn nat(count: i128) -> Expr {
        Expr::literal(Sort::Count, Exact::whole(count))
    }

    #[test]
    fn a_fraction_is_reduced_at_construction() {
        assert_eq!(Exact::new(6, 8), Exact::new(3, 4));
        assert_eq!(Exact::new(-6, -8), Exact::new(3, 4));
        assert_eq!(Exact::new(6, -8), Exact::new(-3, 4));
        assert_eq!(Exact::new(1, 0), None);
    }

    #[test]
    fn zero_reduces_to_one_denominator() {
        assert_eq!(Exact::new(0, 7), Some(Exact::ZERO));
    }

    #[test]
    fn addition_is_commutative_on_the_form_and_not_only_on_the_answer() {
        let mut meter = meter();
        let p = Expr::variable(Sort::Count, 0);
        let q = Expr::variable(Sort::Count, 1);
        let left = p.add(&mut meter, &q).unwrap().unwrap();
        let right = q.add(&mut meter, &p).unwrap().unwrap();
        // Not merely `decide(..) == Same`: the two *build the same value*,
        // which is what makes deciding structural.
        assert_eq!(left, right);
        assert_eq!(decide(&left, &right), Verdict::Same);
    }

    #[test]
    fn like_terms_collect_and_a_cancelled_variable_disappears() {
        let mut meter = meter();
        let p = Expr::variable(Sort::Count, 0);
        let doubled = p.add(&mut meter, &p).unwrap().unwrap();
        assert_eq!(doubled, p.scale(&mut meter, Exact::whole(2)).unwrap().unwrap());
        let gone = p.subtract(&mut meter, &p).unwrap().unwrap();
        assert_eq!(gone, nat(0));
        assert_eq!(gone.as_constant(), Some(Exact::ZERO));
    }

    #[test]
    fn a_variable_expression_is_not_a_constant() {
        assert_eq!(Expr::variable(Sort::Count, 0).as_constant(), None);
        assert_eq!(nat(12).as_constant(), Some(Exact::whole(12)));
    }

    #[test]
    fn two_sorts_never_agree_and_never_combine() {
        let mut meter = meter();
        let counted = Expr::literal(Sort::Count, Exact::ONE);
        let rational = Expr::literal(Sort::Rational, Exact::ONE);
        assert_eq!(decide(&counted, &rational), Verdict::Different);
        assert_eq!(counted.add(&mut meter, &rational).unwrap(), None);
    }

    #[test]
    fn rational_arithmetic_is_exact_rather_than_rounded() {
        let mut meter = meter();
        let third = Expr::literal(Sort::Rational, Exact::new(1, 3).unwrap());
        let mut sum = Expr::literal(Sort::Rational, Exact::ZERO);
        for _ in 0..3 {
            sum = sum.add(&mut meter, &third).unwrap().unwrap();
        }
        assert_eq!(sum.as_constant(), Some(Exact::ONE));
    }

    #[test]
    fn overflow_answers_none_rather_than_wrapping_to_a_wrong_type() {
        let mut meter = meter();
        let huge = Expr::literal(Sort::Count, Exact::whole(i128::MAX));
        assert_eq!(huge.add(&mut meter, &huge).unwrap(), None);
    }

    #[test]
    fn the_budget_refuses_a_pathological_expression_rather_than_hanging() {
        // A budget narrow enough that the walk cannot finish, which is what
        // §4 asks of every loop in this crate: refuse, do not run long.
        let mut meter = Meter::new(Budget::LANGUAGE.scaled(u64::MAX));
        let mut wide = Expr::literal(Sort::Count, Exact::ZERO);
        let outcome = (0..64).try_fold(Ok(()), |_, variable| {
            match wide.add(&mut meter, &Expr::variable(Sort::Count, variable)) {
                Ok(Some(next)) => {
                    wide = next;
                    Some(Ok(()))
                }
                Ok(None) => Some(Ok(())),
                Err(error) => Some(Err(error)),
            }
        });
        assert!(matches!(outcome, Some(Err(CoreError::Exhausted(_)))));
    }
}
