//! Deterministic resource accounting for conversion.
//!
//! `docs/rules/language/02-core-calculus.md` §4 states one meter over checking,
//! conversion, and evaluation, and the three-outcome law it obeys:
//!
//! ```text
//! accepted(t)  |  refused(Diagnostic)  |  exhausted(ResourceError)
//! ```
//!
//! This crate produces the first and third. `refused` is a *typing* verdict and
//! arrives with prompt 134's elaborator; nothing here decides whether a term is
//! well typed. What matters for the law is that exhaustion is its own outcome:
//! [`convertible`](crate::convertible) answering `false` and
//! [`convertible`](crate::convertible) running out of budget are different
//! answers, and a `bool` return could not tell them apart.
//!
//! These are language acceptance limits, not wall-clock timeouts. No charge
//! reads the clock, the allocator, or machine load, so the same terms exhaust at
//! the same operation on every host — which is what makes acceptance a property
//! of the language rather than of the machine that ran it.
//!
//! **A budget may end a conversion and may never change one that finished.**
//! That is §4's independence law, and `tests/suite/budget_laws.rs` is where it
//! is stated as a test rather than as a hope.

use crate::error::CoreError;

/// The metric a charge is spent against.
///
/// Five, and each is here because §4 names it. The compiler's own meter has
/// six; the other three — logical value bytes, instantiated prelude entries,
/// and estimated occurrences — are about values with musical payloads, and this
/// crate does not know what a payload is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    /// Evaluation steps: β, δ, ι, and projection.
    Steps,
    /// Nodes written by quotation, which §4 calls "quoted nodes charged during
    /// a `Switch`".
    QuotedNodes,
    /// How far inside itself an evaluation currently is. The one metric that
    /// goes back down.
    Nesting,
    /// Metavariables created during one elaboration (§4, §2.1).
    Metavariables,
    /// Retries of a postponed constraint (§4, §2.1).
    ///
    /// Charged rather than merely bounded by the loop's own progress argument,
    /// because "each retry either solves something or changes nothing" bounds
    /// the *rounds*, and a round is quadratic in the queue.
    Retries,
}

impl Metric {
    /// What a refusal prints for this metric.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Steps => "reduction steps",
            Self::QuotedNodes => "quoted nodes",
            Self::Nesting => "nested evaluation levels",
            Self::Metavariables => "metavariables",
            Self::Retries => "postponed-constraint retries",
        }
    }
}

/// The limits a meter is spent against.
///
/// `Copy` and small on purpose: a context carries one, and extending a context
/// must not fork a *spend*. Limits are inherited; spending is per operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    steps: u64,
    quoted_nodes: u64,
    nesting: u64,
    metavariables: u64,
    retries: u64,
}

impl Budget {
    /// How many evaluator frames may stand inside one another (§4.1).
    ///
    /// 256, the same constant the existing evaluator uses, and for the same
    /// reason: past anything a person writes and short of anything a host
    /// cannot hold. Normalization by evaluation adds a *second* descent to the
    /// one §4.1 was written for — `quote` walks a value the way `eval` walks a
    /// term — so this limit now bounds both.
    pub const NESTING: u64 = 256;

    /// The language budget.
    ///
    /// 200,000 reduction steps, matching the existing evaluator's, because a
    /// program refused at one count and accepted at another is a program two
    /// compilers disagree about, and the two evaluators become one at prompt
    /// 142.
    ///
    /// **Quoted nodes, metavariables, and retries are charged and not
    /// limited.** §4 says so in as many words: "Conversion and metavariable
    /// metrics have no defaults yet: prompt 144 measures the new checker and
    /// sets them, and until it does, the checker charges them and reports them
    /// without a limit." The charge paths are live and tested through
    /// [`Self::scaled`]; only the defaults are open.
    pub const LANGUAGE: Self = Self {
        steps: 200_000,
        quoted_nodes: u64::MAX,
        nesting: Self::NESTING,
        metavariables: u64::MAX,
        retries: u64::MAX,
    };

    /// This budget with quotation forbidden outright.
    ///
    /// §3's conversion decides by walking two *values*, and the only thing that
    /// reads a value back is the path that builds a mismatch's message. This
    /// exists so that can be stated as a law rather than left as a comment: a
    /// conversion that answers `true` under this budget read nothing back, and
    /// one that exhausts did. `conversion_laws.rs` is the caller, and prompt 144
    /// is the one that turns the measurement into a real limit.
    #[must_use]
    pub const fn without_quotation(self) -> Self {
        Self {
            quoted_nodes: 0,
            ..self
        }
    }

    /// The language budget with every limit divided by `divisor`.
    ///
    /// This exists for one caller and it is a real one: §4's independence law
    /// is normative, and the only way to state it is to run the same terms
    /// under two budgets and check that a narrower one either exhausted or
    /// agreed. Prompt 148's audit consumes those tests.
    ///
    /// It is not a knob the compiler turns. [`crate::Cx::new`] uses
    /// [`Self::LANGUAGE`] and nothing in the pipeline narrows it, because a
    /// budget the caller could lower would make acceptance a property of the
    /// invocation rather than of the language.
    #[must_use]
    pub const fn scaled(self, divisor: u64) -> Self {
        const fn share(limit: u64, divisor: u64) -> u64 {
            match limit.checked_div(divisor) {
                Some(share) => share,
                None => limit,
            }
        }
        Self {
            steps: share(self.steps, divisor),
            quoted_nodes: share(self.quoted_nodes, divisor),
            nesting: share(self.nesting, divisor),
            metavariables: share(self.metavariables, divisor),
            retries: share(self.retries, divisor),
        }
    }

    const fn limit(self, metric: Metric) -> u64 {
        match metric {
            Metric::Steps => self.steps,
            Metric::QuotedNodes => self.quoted_nodes,
            Metric::Nesting => self.nesting,
            Metric::Metavariables => self.metavariables,
            Metric::Retries => self.retries,
        }
    }
}

/// Which operation crossed which limit, and by how much.
///
/// No span: this crate has no source. Prompt 134 attaches the position, because
/// it is the stage that knows one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{operation} exceeded the budget for {metric} at {attempted} of {limit}", metric = self.metric.name())]
pub struct ResourceError {
    /// The operation the diagnostic names, so a refusal says what was asked for
    /// rather than which counter overflowed.
    pub operation: &'static str,
    /// The metric that ran out.
    pub metric: Metric,
    /// The limit it ran out against.
    pub limit: u64,
    /// What the charge would have taken the total to.
    pub attempted: u64,
}

/// What one facade call charged, by metric.
///
/// The four counters that only go up. [`Metric::Nesting`] is not among them
/// because it is a depth rather than a total: it comes back down on the way out,
/// so "how deep did this go" is not a charge and adding it to one would be a
/// number nothing means.
///
/// Reported because a caller may have a budget of its own to keep.
/// `26-language-design-decision.md` §3.5 gives the expansion phase four
/// counters, two of which are this crate's work — an adapter that is expensive
/// to check is expensive whether or not the region it reads is small — and a
/// phase that had to estimate them would be keeping a second opinion about work
/// this crate already counted exactly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spend {
    /// [`Metric::Steps`].
    pub steps: u64,
    /// [`Metric::QuotedNodes`].
    pub quoted_nodes: u64,
    /// [`Metric::Metavariables`].
    pub metavariables: u64,
    /// [`Metric::Retries`].
    pub retries: u64,
}

impl Spend {
    /// The two spends of one operation done in two calls, added.
    #[must_use]
    pub const fn and(self, later: Self) -> Self {
        Self {
            steps: self.steps.saturating_add(later.steps),
            quoted_nodes: self.quoted_nodes.saturating_add(later.quoted_nodes),
            metavariables: self.metavariables.saturating_add(later.metavariables),
            retries: self.retries.saturating_add(later.retries),
        }
    }
}

/// One operation's spend.
///
/// Built per facade call rather than carried in the context: limits are
/// inherited by every conversion under a context, but a *spend* belongs to the
/// run that made it, and a meter shared across runs would make one conversion's
/// answer depend on how many ran before it.
pub(crate) struct Meter {
    budget: Budget,
    steps: u64,
    quoted_nodes: u64,
    nesting: u64,
    metavariables: u64,
    retries: u64,
}

impl Meter {
    pub(crate) const fn new(budget: Budget) -> Self {
        Self {
            budget,
            steps: 0,
            quoted_nodes: 0,
            nesting: 0,
            metavariables: 0,
            retries: 0,
        }
    }

    /// Charge one reduction.
    ///
    /// # Errors
    ///
    /// [`CoreError::Exhausted`] when the charge would cross the limit.
    pub(crate) fn step(&mut self, operation: &'static str) -> Result<(), CoreError> {
        self.steps = self.charge(Metric::Steps, operation, self.steps)?;
        Ok(())
    }

    /// Charge one node written by quotation.
    ///
    /// # Errors
    ///
    /// [`CoreError::Exhausted`] when the charge would cross the limit.
    pub(crate) fn quoted_node(&mut self, operation: &'static str) -> Result<(), CoreError> {
        self.quoted_nodes = self.charge(Metric::QuotedNodes, operation, self.quoted_nodes)?;
        Ok(())
    }

    /// Charge one metavariable.
    ///
    /// # Errors
    ///
    /// [`CoreError::Exhausted`] when the charge would cross the limit.
    pub(crate) fn metavariable(&mut self, operation: &'static str) -> Result<(), CoreError> {
        self.metavariables = self.charge(Metric::Metavariables, operation, self.metavariables)?;
        Ok(())
    }

    /// Charge one retry of a postponed constraint.
    ///
    /// # Errors
    ///
    /// [`CoreError::Exhausted`] when the charge would cross the limit.
    pub(crate) fn retry(&mut self, operation: &'static str) -> Result<(), CoreError> {
        self.retries = self.charge(Metric::Retries, operation, self.retries)?;
        Ok(())
    }

    /// Run `body` one evaluation level down, or refuse at the limit.
    ///
    /// A wrapper rather than a pair of enter/leave calls because the level has
    /// to be given back on *every* way out, and an evaluator whose arms are
    /// mostly `?` has more ways out than a reader can check.
    ///
    /// Generic in the error so that a caller with its own failure type — one
    /// that carries a [`CoreError`] alongside an answer of its own — can nest
    /// without wrapping its result in a second `Result` to get past this
    /// signature.
    ///
    /// # Errors
    ///
    /// [`CoreError::Exhausted`] at the nesting limit, or whatever `body`
    /// returns.
    pub(crate) fn nested<T, E: From<CoreError>>(
        &mut self,
        operation: &'static str,
        body: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        self.nesting = self.charge(Metric::Nesting, operation, self.nesting)?;
        let value = body(self);
        self.nesting = self.nesting.saturating_sub(1);
        value
    }

    /// What this meter has charged so far.
    pub(crate) const fn spent(&self) -> Spend {
        Spend {
            steps: self.steps,
            quoted_nodes: self.quoted_nodes,
            metavariables: self.metavariables,
            retries: self.retries,
        }
    }

    fn charge(&self, metric: Metric, operation: &'static str, spent: u64) -> Result<u64, CoreError> {
        let attempted = spent.saturating_add(1);
        if attempted > self.budget.limit(metric) {
            return Err(CoreError::Exhausted(ResourceError {
                operation,
                metric,
                limit: self.budget.limit(metric),
                attempted,
            }));
        }
        Ok(attempted)
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::{Budget, Meter, Metric};
    use crate::error::CoreError;

    #[test]
    fn a_narrowed_budget_refuses_where_the_language_budget_does_not() {
        let mut generous = Meter::new(Budget::LANGUAGE);
        for _ in 0..1_000 {
            assert!(generous.step("test").is_ok());
        }

        let mut narrow = Meter::new(Budget::LANGUAGE.scaled(1_000_000));
        let refusal = (0..1_000).map(|_| narrow.step("test")).find(Result::is_err);
        match refusal {
            Some(Err(CoreError::Exhausted(error))) => assert_eq!(error.metric, Metric::Steps),
            Some(Err(other)) => panic!("expected exhaustion, got {other}"),
            Some(Ok(())) | None => panic!("a budget of zero steps must refuse"),
        }
    }

    #[test]
    fn nesting_is_the_one_metric_that_is_given_back() {
        let mut meter = Meter::new(Budget::LANGUAGE);
        for _ in 0..1_000 {
            let entered = meter.nested::<(), CoreError>("test", |_| Ok(()));
            assert!(entered.is_ok(), "a thousand siblings are one level, not a thousand");
        }
    }

    #[test]
    fn nesting_refuses_when_levels_stand_inside_one_another() {
        fn descend(meter: &mut Meter, remaining: u32) -> Result<(), CoreError> {
            meter.nested("test", |meter| match remaining.checked_sub(1) {
                Some(next) => descend(meter, next),
                None => Ok(()),
            })
        }

        let mut meter = Meter::new(Budget::LANGUAGE);
        let deep = u32::try_from(Budget::NESTING).unwrap_or(u32::MAX).saturating_add(1);
        match descend(&mut meter, deep) {
            Err(CoreError::Exhausted(error)) => assert_eq!(error.metric, Metric::Nesting),
            Err(other) => panic!("expected exhaustion, got {other}"),
            Ok(()) => panic!("descending past the nesting limit must refuse"),
        }
    }

    #[test]
    fn quoted_nodes_are_charged_and_the_language_budget_does_not_limit_them() {
        let mut language = Meter::new(Budget::LANGUAGE);
        for _ in 0..100_000 {
            assert!(language.quoted_node("test").is_ok());
        }

        let mut narrow = Meter::new(Budget::LANGUAGE.scaled(u64::MAX));
        assert!(narrow.quoted_node("test").is_ok(), "one node fits");
        let refusal = (0..8).map(|_| narrow.quoted_node("test")).find(Result::is_err);
        assert!(
            refusal.is_some(),
            "the charge path must be live before 144 sets a limit"
        );
    }
}

// ---- scratch instrumentation, removed before 142 closes ----
