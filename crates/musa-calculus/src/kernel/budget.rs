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

use crate::kernel::error::CoreError;

/// The metric a charge is spent against.
///
/// Five, and each is here because §4 names it. Instantiated prelude entries and
/// estimated occurrences remain compiler-boundary counters: this crate neither
/// instantiates the prelude nor knows what an occurrence is. Logical value
/// shape is core work, however, and is charged without inspecting a host
/// payload: a literal contributes one node and the payload bytes its owner
/// reports, while a closure or constructor contributes its own cell and one
/// logical pointer per field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    /// Evaluation steps: β, δ, ι, and projection.
    Steps,
    /// Nodes written by quotation, which §4 calls "quoted nodes charged during
    /// a `Switch`".
    QuotedNodes,
    /// Semantic value cells constructed by checking or evaluation.
    ConstructedNodes,
    /// The deterministic logical size of newly constructed values.
    LogicalBytes,
    /// How far inside itself an evaluation currently is. The one metric that
    /// goes back down.
    ///
    /// **Two descents, and only two.** §4.1 derives the metric from how deeply
    /// a *term* is written and from `quote`'s walk back over a *value*: both
    /// are the depth of the thing being walked, both spend a host frame a
    /// level, and the limit is what turns a stack overflow into a refusal. A
    /// third thing used to be charged here and is not: reading a δ-builtin's
    /// argument into a [`Datum`](crate::Datum) and building its answer back
    /// out walk *data*, whose depth is the length of one argument rather than
    /// anything a composer wrote, and a six-hundred-element list was six
    /// hundred levels. Those two walks now carry their own explicit stacks and
    /// charge [`Self::Steps`], which is the metric that already prices how
    /// much work a long argument is. See
    /// `docs/notes/research/language-design-closure/59-the-staff-wall-is-the-evaluators.md`
    /// and prompt 165b.
    ///
    /// A third quantity was charged here until prompt 165a and is not: a
    /// recursive call used to be evaluated inside the enclosing evaluation and
    /// to hold its level until the call beneath it finished, so one counter
    /// bounded how many times a definition may call itself as well as how
    /// deeply a term is written. The evaluator carries its own control stack
    /// now, so a call is [`Self::Steps`] and nothing else — measured, `add`
    /// three thousand deep runs inside three nesting levels. See
    /// [`Budget::NESTING`].
    Nesting,
}

impl Metric {
    /// What a refusal prints for this metric.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Steps => "reduction steps",
            Self::QuotedNodes => "quoted nodes",
            Self::ConstructedNodes => "constructed value nodes",
            Self::LogicalBytes => "logical value bytes",
            Self::Nesting => "nested evaluation levels",
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
    constructed_nodes: u64,
    logical_bytes: u64,
    nesting: u64,
}

impl Budget {
    /// How many evaluator frames may stand inside one another (§4.1).
    ///
    /// Past anything a person writes and short of anything a host cannot hold.
    /// Normalization by evaluation adds a *second* descent to the one §4.1 was
    /// written for — `quote` walks a value the way `eval` walks a term — so
    /// this limit bounds both.
    ///
    /// **320 since prompt 155a, and it now measures one quantity rather than
    /// two.** Until prompt 165a a recursive call was evaluated inside the
    /// enclosing `eval` and held its level until the steps beneath it
    /// finished, so this number decided how many times a definition may call
    /// itself as well as how deeply a term is written — bisected, about three
    /// levels a call, which is why 155a had to raise it from 256 and why note
    /// 54 could show there was no third raise available. The evaluator carries
    /// its own control stack now. A recursion is charged [`Metric::Steps`],
    /// which is the metric that prices work, and this one is back to the
    /// structural descent §4.1 derives it from.
    ///
    /// **Re-earned on the measurement rather than inherited.** `add` recursing
    /// three thousand times peaks at three nesting levels and answers under a
    /// budget of eight; `stdlib/src/adapters/staff.musa` peaks at 62, the same
    /// 62 for a region of nothing, of one item, of three, of four, and for
    /// `examples/staff-page.musa`'s 77 lines — a number that is the depth of
    /// the adapter's own source and does not move with what it reads. Against
    /// 62 the limit is five times what the corpus needs.
    ///
    /// **So why not lower it.** Because prompt 165 charged `check` and `infer`
    /// and thereby made this the acceptance boundary for deeply written terms.
    /// Lowering it would now refuse programs for structure the finished corpus
    /// does not require, while also shrinking §4.1's derived room. The direct
    /// nesting laws and the size-bounded adapter law re-earn 320 on the smallest
    /// host; `kernel::room` records the finished frame-ceiling bisection.
    pub const NESTING: u64 = 320;

    /// The language budget.
    ///
    /// Prompt 165 re-derived all three aggregate core limits together. The
    /// corpus high-water marks, measured with `Spend`, are:
    ///
    /// | workload | steps | nodes | bytes |
    /// | --- | ---: | ---: | ---: |
    /// | `examples/in-c.musa` | 29,113 | 54,258 | 1,280,938 |
    /// | `examples/staff-page.musa` | 180,873 | 99,672 | 167,222 |
    /// | `tests/fixtures/large-score.musa` | 355,992 | 239,394 | 1,319,043 |
    /// | a generic row's 48 forms at twelve | 1,081,475 | 574,098 | 596,344 |
    ///
    /// Two million steps and one million nodes leave 85% and 74% headroom over
    /// the respective maxima. Sixteen MiB leaves more than twelve times the
    /// logical bytes of the worst event-track workload. The old
    /// 200,000/100,000/1 MiB
    /// table could not admit the desktop fixture or the full post-tonal laws;
    /// worse, the replacement checker did not enforce its size entries at all.
    /// A 1,500-level generated region now refuses at 1,001,185 of 1,000,000
    /// constructed nodes, where raising steps alone previously let a deeper
    /// region reach an operating-system kill. Section 4 records the acceptance
    /// bump and its boundary laws.
    pub const LANGUAGE: Self = Self {
        steps: 2_000_000,
        quoted_nodes: u64::MAX,
        constructed_nodes: 1_000_000,
        logical_bytes: 16 * 1024 * 1024,
        nesting: Self::NESTING,
    };

    /// This budget with quotation forbidden outright.
    ///
    /// §3's conversion decides by walking two *values*, and two things read a
    /// value back: the path that builds a mismatch's message, and the assembly
    /// of a declared constant's type, which `family.rs` computes on demand
    /// rather than storing. This exists so that can be stated as a law rather
    /// than left as a comment — a conversion that answers `true` under this
    /// budget read nothing back — with [`Self::quoting`] carrying the second
    /// half for the conversions that must read a declaration back to decide at
    /// all. `conversion_laws.rs` is the caller, and prompt 165 is the one that
    /// turns the measurement into a real limit.
    #[must_use]
    pub const fn without_quotation(self) -> Self {
        Self {
            quoted_nodes: 0,
            ..self
        }
    }

    /// This budget with quotation allowed up to `nodes`.
    ///
    /// The other half of [`Self::without_quotation`]'s law. A conversion over a
    /// declared family assembles that family's constants' types, so it cannot
    /// read *nothing* back; what it must not do is read back an amount that
    /// grows with the terms it was handed. A fixed allowance is how that is
    /// said with the accounting the budget already has.
    #[must_use]
    pub const fn quoting(self, nodes: u64) -> Self {
        Self {
            quoted_nodes: nodes,
            ..self
        }
    }

    /// This budget with at most `levels` of nesting allowed.
    ///
    /// The one metric that is a *depth* rather than a total, narrowed on its
    /// own so that a law can say what a run's peak depth was without the meter
    /// exposing a high-water mark it otherwise has no reason to keep. A run
    /// that answers under this budget never stood more than `levels` deep, and
    /// that is the whole of what the accessor is for: prompt 165a took
    /// recursion off this metric, and the statement it owes is that a
    /// definition calling itself three thousand times still peaks at the depth
    /// of the *term*, which is a small number.
    ///
    /// Not a knob the compiler turns, for [`Self::scaled`]'s reason: the
    /// pipeline runs at [`Self::LANGUAGE`] and a budget the caller could lower
    /// would make acceptance a property of the invocation.
    #[must_use]
    pub const fn nesting(self, levels: u64) -> Self {
        Self {
            nesting: levels,
            ..self
        }
    }

    /// This budget with at most `steps` reductions.
    ///
    /// Test laws narrow this counter independently to identify work exhaustion
    /// without also narrowing the size and nesting counters. The compiler
    /// always uses [`Self::LANGUAGE`].
    #[must_use]
    pub const fn reduction_steps(self, steps: u64) -> Self {
        Self { steps, ..self }
    }

    /// This budget with at most `nodes` newly constructed value nodes.
    ///
    /// Test laws narrow this counter independently to prove its exact boundary.
    /// The compiler always uses [`Self::LANGUAGE`].
    #[must_use]
    pub const fn constructed_nodes(self, nodes: u64) -> Self {
        Self {
            constructed_nodes: nodes,
            ..self
        }
    }

    /// This budget with at most `bytes` of newly constructed logical values.
    ///
    /// Logical bytes are deterministic language costs, not host allocation
    /// sizes. Test laws narrow this counter independently; the compiler always
    /// uses [`Self::LANGUAGE`].
    #[must_use]
    pub const fn logical_bytes(self, bytes: u64) -> Self {
        Self {
            logical_bytes: bytes,
            ..self
        }
    }

    /// How many reduction steps this budget allows (§4's cost table).
    ///
    /// Read by the laws that state a refusal names its limit: §4 requires a
    /// diagnostic to name "the operation, metric, attempted amount, and limit",
    /// and a law checking that the message says so has to know the number
    /// without writing it down a second time.
    #[must_use]
    pub const fn steps(self) -> u64 {
        self.steps
    }

    /// How many newly constructed value nodes this budget allows.
    #[must_use]
    pub const fn constructed_node_limit(self) -> u64 {
        self.constructed_nodes
    }

    /// How many deterministic logical value bytes this budget allows.
    #[must_use]
    pub const fn logical_byte_limit(self) -> u64 {
        self.logical_bytes
    }

    /// The language budget with every limit divided by `divisor`.
    ///
    /// This exists for one caller and it is a real one: §4's independence law
    /// is normative, and the only way to state it is to run the same terms
    /// under two budgets and check that a narrower one either exhausted or
    /// agreed. Prompt 169's audit consumes those tests.
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
            constructed_nodes: share(self.constructed_nodes, divisor),
            logical_bytes: share(self.logical_bytes, divisor),
            nesting: share(self.nesting, divisor),
        }
    }

    const fn limit(self, metric: Metric) -> u64 {
        match metric {
            Metric::Steps => self.steps,
            Metric::QuotedNodes => self.quoted_nodes,
            Metric::ConstructedNodes => self.constructed_nodes,
            Metric::LogicalBytes => self.logical_bytes,
            Metric::Nesting => self.nesting,
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
    /// [`Metric::ConstructedNodes`].
    pub constructed_nodes: u64,
    /// [`Metric::LogicalBytes`].
    pub logical_bytes: u64,
}

impl Spend {
    /// The two spends of one operation done in two calls, added.
    #[must_use]
    pub const fn and(self, later: Self) -> Self {
        Self {
            steps: self.steps.saturating_add(later.steps),
            quoted_nodes: self.quoted_nodes.saturating_add(later.quoted_nodes),
            constructed_nodes: self.constructed_nodes.saturating_add(later.constructed_nodes),
            logical_bytes: self.logical_bytes.saturating_add(later.logical_bytes),
        }
    }
}

/// Which run a δ-unfolding memo was filled by, and how much of that run's
/// solving had happened when it was.
///
/// [`crate::kernel::eval`]'s memo is sound only while no metavariable
/// *reachable from the neutral it is on* has been solved since the cell was
/// filled, and this pair is how that premise is decided in one comparison.
/// Equal stamps mean the same run filled and read the cell and solved nothing
/// in between, which implies the premise and is what the memo needs.
///
/// **Both halves, and the run half is the one that was missing.** Until this
/// type existed the stamp was a single process-wide count of every solution
/// anywhere, which decides the premise correctly and decides much more besides:
/// a metavariable solved by an *unrelated* run — a second document compiled on
/// another thread of the same process — invalidated memos it could not reach,
/// so how much work a program cost depended on what else the process happened
/// to be doing. That is the one thing this module's own header forbids: "the
/// same terms exhaust at the same operation on every host — which is what makes
/// acceptance a property of the language rather than of the machine that ran
/// it." Measured, it was not a nuisance but a wall. Compiling
/// `examples/staff-page.musa` alone took 4,434 memo hits and no stale reads,
/// and peaked at 191,781 of [`Budget::LANGUAGE`]'s 200,000 steps; the same
/// compile run beside `musa-compiler`'s other library tests took 2,745 hits
/// and 2,043 stale reads, and the re-unfolding those cost is far more than the
/// four percent of margin the file has — so `cargo test -p musa-compiler
/// --lib` refused a file that `cargo nextest`, which gives every law its own
/// process, compiles.
///
/// **Scoping it to the run costs nothing.** Of those 4,434 hits, the number
/// that read a cell some *other* meter had filled is zero: a memo pays for
/// itself inside the run that fills it, so a stamp that only sees its own run's
/// solutions keeps every hit the process-wide one kept.
///
/// **What it rests on.** A cell is only ever read by the run that filled it, so
/// the only way a solution could go unnoticed is for a second run to be live
/// over the same values at the same time and to solve a metavariable one of
/// those cells depends on. It cannot: a cell shared between two runs is on a
/// neutral built entirely by the elaboration that produced the shared values —
/// `Neutral::eliminated` gives a neutral a fresh cell for every spine, so a
/// cell a later run could reach is never one that run put its own arguments
/// into — and that elaboration's metavariables were all solved or reported when
/// it ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Stamp {
    run: u64,
    solved: u64,
}

/// How many meters this process has built.
///
/// Only ever read to name one, so wrapping would need 2⁶⁴ facade calls in one
/// process before two live runs could share a number.
static RUNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

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
    constructed_nodes: u64,
    logical_bytes: u64,
    nesting: u64,
    /// Which run this is, for [`Stamp`].
    run: u64,
    /// How many metavariables this run has solved, for [`Stamp`].
    ///
    /// Here rather than in a process-wide counter for the reason this struct's
    /// own doc comment gives about spends, one level down: a run's memos must
    /// be invalidated by that run's solutions and by nothing else.
    solved: u64,
}

impl Meter {
    pub(crate) fn new(budget: Budget) -> Self {
        Self {
            budget,
            steps: 0,
            quoted_nodes: 0,
            constructed_nodes: 0,
            logical_bytes: 0,
            nesting: 0,
            run: RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            solved: 0,
        }
    }

    /// Where this run stands in its own solving history.
    ///
    /// `kernel::eval` fills a memo with this and reads one back only at the
    /// same value; see [`Stamp`] for what that decides.
    pub(crate) const fn stamp(&self) -> Stamp {
        Stamp {
            run: self.run,
            solved: self.solved,
        }
    }

    /// Record that this run solved a metavariable.
    ///
    /// [`Meta::solve`](crate::kernel::meta::Meta::solve) is the only caller and
    /// it takes a meter for that reason: a solution that did not move the stamp
    /// would leave every memo standing over a value that has since reduced
    /// further, and the two halves must not be able to drift apart.
    pub(crate) const fn solved(&mut self) {
        self.solved = self.solved.saturating_add(1);
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

    /// Charge one newly constructed value shape.
    ///
    /// `nodes` and `bytes` describe only the cell being built and its wiring;
    /// values already held by its fields were charged where they were built.
    pub(crate) fn construct(&mut self, operation: &'static str, nodes: u64, bytes: u64) -> Result<(), CoreError> {
        self.constructed_nodes = self.charge_by(Metric::ConstructedNodes, operation, self.constructed_nodes, nodes)?;
        self.logical_bytes = self.charge_by(Metric::LogicalBytes, operation, self.logical_bytes, bytes)?;
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

    /// Charge one evaluation level, with nothing to give it back.
    ///
    /// [`Self::nested`]'s pair, for a caller that keeps its pending work as
    /// data rather than as host frames and so has no scope to hang the release
    /// on: `kernel::eval`'s control stack releases the level when the frame
    /// that took it is popped. A caller with a scope should use
    /// [`Self::nested`], which cannot forget.
    ///
    /// # Errors
    ///
    /// [`CoreError::Exhausted`] at the nesting limit.
    pub(crate) fn enter(&mut self, operation: &'static str) -> Result<(), CoreError> {
        self.nesting = self.charge(Metric::Nesting, operation, self.nesting)?;
        Ok(())
    }

    /// Give one evaluation level back.
    pub(crate) fn leave(&mut self) {
        self.nesting = self.nesting.saturating_sub(1);
    }

    /// How far in the current evaluation stands.
    pub(crate) const fn level(&self) -> u64 {
        self.nesting
    }

    /// Stand at `level` again.
    ///
    /// The *dump* of Peyton Jones ch. 18 §18.8, as a number rather than as a
    /// second stack: a nested evaluation — a λ body opened, a case arm chosen —
    /// is a new term, and §4.1 measures how deeply a term is written. So the
    /// enclosing depth is saved when one begins and stood at again when it
    /// ends, which is what keeps a definition's own recursion off a metric
    /// derived from structure. `kernel::eval`'s `Frame::Dump` is the only
    /// caller and it saves with [`Self::level`].
    pub(crate) const fn at(&mut self, level: u64) {
        self.nesting = level;
    }

    /// What this meter has charged so far.
    pub(crate) const fn spent(&self) -> Spend {
        Spend {
            steps: self.steps,
            quoted_nodes: self.quoted_nodes,
            constructed_nodes: self.constructed_nodes,
            logical_bytes: self.logical_bytes,
        }
    }

    fn charge(&self, metric: Metric, operation: &'static str, spent: u64) -> Result<u64, CoreError> {
        self.charge_by(metric, operation, spent, 1)
    }

    fn charge_by(&self, metric: Metric, operation: &'static str, spent: u64, amount: u64) -> Result<u64, CoreError> {
        let attempted = spent.saturating_add(amount);
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
    use crate::kernel::error::CoreError;

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

    #[test]
    fn constructed_nodes_accept_the_limit_and_refuse_the_next_node() {
        let mut meter = Meter::new(Budget::LANGUAGE.constructed_nodes(3));
        assert!(meter.construct("test value", 3, 0).is_ok());
        match meter.construct("test value", 1, 0) {
            Err(CoreError::Exhausted(error)) => {
                assert_eq!(error.metric, Metric::ConstructedNodes);
                assert_eq!(error.attempted, 4);
                assert_eq!(error.limit, 3);
            }
            other => panic!("the fourth node must exhaust a three-node budget: {other:?}"),
        }
    }

    #[test]
    fn logical_bytes_accept_the_limit_and_refuse_the_next_byte() {
        let mut meter = Meter::new(Budget::LANGUAGE.logical_bytes(3));
        assert!(meter.construct("test value", 0, 3).is_ok());
        match meter.construct("test value", 0, 1) {
            Err(CoreError::Exhausted(error)) => {
                assert_eq!(error.metric, Metric::LogicalBytes);
                assert_eq!(error.attempted, 4);
                assert_eq!(error.limit, 3);
            }
            other => panic!("the fourth byte must exhaust a three-byte budget: {other:?}"),
        }
    }
}

// ---- scratch instrumentation, removed before 142 closes ----
