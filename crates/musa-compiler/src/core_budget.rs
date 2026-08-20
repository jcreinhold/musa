//! Deterministic resource accounting for the private elaboration evaluator.
//!
//! Evaluation is stated on typed configurations, so that a resource limit is
//! part of the semantics rather than an implementation escape
//! (`docs/rules/language/02-core-calculus.md` §3):
//!
//! ```text
//! run(budget, e)   ⇓   done(v)   |   failed(ResourceError)
//! ```
//!
//! These are language acceptance limits, not wall-clock timeouts. A charge
//! never reads wall time, allocator behaviour, or machine load, so the same
//! source and compiler version fail at the same operation on every machine —
//! which is what makes acceptance a property of the language rather than of the
//! host. This accounting must not become an interrupt or a partial-result
//! mechanism.
//!
//! The budget can stop an evaluation; it cannot change an accepted one. That
//! law is this module's `a_narrowed_budget_*` tests, and it is what lets a
//! caller lower a budget to probe a piece without wondering whether the answer
//! it did get was the answer it would have got.

use crate::origin::SourceSpan;

impl CostTable {
    pub(crate) const fn version(self) -> u32 {
        self.version
    }
}

/// How many evaluator frames may stand inside one another
/// (`../rules/language/02-core-calculus.md` §4).
///
/// The limit that turns a stack overflow into a refusal. The inherited-context
/// recursor descends *through* the transformer's own branches, so one level of
/// source nesting costs a whole chain of `eval`/`apply_closure` frames, and a
/// region deep enough to exhaust the machine's stack would abort the process
/// rather than earn a diagnostic — which a total language whose budget system
/// exists precisely to refuse rather than crash cannot have.
///
/// 256 because it is past anything a person writes and short of anything a
/// host cannot hold. The reference recursor of `crate::expand`'s law suite
/// spends four levels per level of source nesting, so 256 admits regions
/// nested some sixty deep; a transformer that does more per node admits
/// proportionally less, and still more than a real region needs.
pub(crate) const NESTING: u64 = 256;

/// The stack one nested evaluation level may spend, in bytes.
///
/// Not a language constant: [`NESTING`] decides acceptance, and this decides
/// what the implementation must arrange so that the decision can be *reached*.
/// A limit no host can afford to run up to would be a limit that still aborts.
///
/// Measured at 62,876 bytes per level in a debug build on arm64 — `eval` and
/// `eval_builtin` are the two fat frames, at 22,608 and 50,528 bytes, because a
/// debug build gives every arm of a large match its own slots — and 5,968 in a
/// release build. The ceiling doubles the debug measurement, since it has to
/// hold on targets and future arms nobody has measured. Shrinking those two
/// matches is how this number comes down; it is not how the refusal happens.
pub(crate) const FRAME_CEILING: u64 = 128 * 1024;

impl Budget {
    /// The language budget with every limit divided by `divisor`.
    ///
    /// A smaller budget may only turn `done` into `failed`; it may never turn
    /// one accepted value into another. That is the law the budget-independence
    /// tests use this to state, and it is the only reason a budget other than
    /// [`Self::LANGUAGE`] exists — the compiler proper never narrows.
    #[cfg(test)]
    pub(crate) const fn narrowed(self, divisor: u64) -> Self {
        /// One limit's share, leaving it alone when the divisor is zero.
        const fn share(limit: u64, divisor: u64) -> u64 {
            match limit.checked_div(divisor) {
                Some(share) => share,
                None => limit,
            }
        }
        Self {
            steps: share(self.steps, divisor),
            nodes: share(self.nodes, divisor),
            bytes: share(self.bytes, divisor),
            instances: share(self.instances, divisor),
            output: share(self.output, divisor),
            nesting: share(self.nesting, divisor),
        }
    }
}

impl Reduction {}

// The budget the next meter is built under.
//
// Test-only, and scoped: the compiler proper has exactly one budget, because a
// budget the caller could turn down would make acceptance a property of the
// invocation rather than of the language. What a test needs is not a knob but a
// way to state the independence law — run the same source under a smaller
// budget and check that it either failed or agreed.
#[cfg(test)]
thread_local! {
    static ACTIVE: std::cell::Cell<Budget> = const { std::cell::Cell::new(Budget::LANGUAGE) };
}

/// Run `body` with every meter it builds metered by `budget`.
#[cfg(test)]
pub(crate) fn under_budget<T>(budget: Budget, body: impl FnOnce() -> T) -> T {
    let restored = ACTIVE.with(|active| active.replace(budget));
    let value = body();
    ACTIVE.with(|active| active.set(restored));
    value
}

impl Default for WorkMeter {
    fn default() -> Self {
        #[cfg(test)]
        {
            Self::new(ACTIVE.with(std::cell::Cell::get))
        }
        #[cfg(not(test))]
        {
            Self::new(Budget::LANGUAGE)
        }
    }
}

impl WorkMeter {
    pub(crate) fn new(budget: Budget) -> Self {
        Self {
            budget,
            costs: CostTable::V3,
            steps: 0,
            nodes: 0,
            bytes: 0,
            instances: 0,
            output: 0,
            nesting: 0,
            failure: None,
        }
    }

    /// How many constructed value nodes this meter has charged.
    ///
    /// Test-only, and it is the charging *locus* the tests read it for: a law
    /// about where a value is charged is a law about this number's growth, and
    /// stating it through the accept/reject boundary instead would only say
    /// that one hand-picked program fits.
    #[cfg(test)]
    pub(crate) const fn nodes(&self) -> u64 {
        self.nodes
    }

    /// Drive one evaluation to its typed configuration.
    ///
    /// Charging returns `Option` so that `?` is the only way to spend it: a
    /// charge that is not checked cannot be written, which is what the boolean
    /// protocol this replaced could not promise. Everything that stops mid-way
    /// arrives here as `None`, and this is the one place that decides whether
    /// that was the budget talking or a broken invariant.
    ///
    /// The failure moves out of the meter into the configuration: this run is
    /// over and owns its refusal, so the enclosing boundary reports only the
    /// failures no configuration classified.
    pub(crate) fn run<T>(&mut self, evaluation: impl FnOnce(&mut Self) -> Option<T>) -> Evaluation<T> {
        let value = evaluation(self);
        match (value, self.failure.take()) {
            // A recorded failure wins over a value: exhaustion publishes
            // neither a partial value nor a partial score (§4).
            (_, Some(failure)) => Evaluation::Failed(failure),
            (Some(value), None) => Evaluation::Done(value),
            (None, None) => Evaluation::Broken,
        }
    }

    /// Run `body` one evaluation level down, or refuse at the limit.
    ///
    /// A wrapper rather than a pair of `enter`/`leave` calls because the level
    /// has to be given back on *every* way out, and an evaluator whose arms are
    /// mostly `?` has more ways out than a reader can check. The level is
    /// released even when `body` answers `None`: a meter that has already
    /// failed refuses every later charge anyway, so what is being protected
    /// here is the run that has not failed.
    ///
    /// `reduction` is the operation the diagnostic names, so a region that
    /// nests too far is reported against the descent that went one level too
    /// deep rather than against whatever happened to be underneath it.
    pub(crate) fn nested<T>(
        &mut self,
        reduction: Reduction,
        span: SourceSpan,
        body: impl FnOnce(&mut Self) -> Option<T>,
    ) -> Option<T> {
        let level = self.costs.level;
        self.charge(Counter::Nesting, reduction.operation(), NESTING_METRIC, level, span)?;
        let value = body(self);
        self.nesting = self.nesting.saturating_sub(level);
        value
    }

    /// Charge `amount` reductions of `reduction`.
    pub(crate) fn step(&mut self, reduction: Reduction, amount: u64, span: SourceSpan) -> Option<()> {
        self.charge(
            Counter::Steps,
            reduction.operation(),
            "reduction steps",
            amount.saturating_mul(self.costs.reduction),
            span,
        )
    }

    /// Charge a constructed value: its node count and its logical byte size.
    pub(crate) fn construct(
        &mut self,
        operation: &'static str,
        nodes: u64,
        bytes: u64,
        span: SourceSpan,
    ) -> Option<()> {
        self.charge(
            Counter::Nodes,
            operation,
            "constructed value nodes",
            nodes.saturating_mul(self.costs.node),
            span,
        )?;
        self.charge(
            Counter::Bytes,
            operation,
            "constructed value bytes",
            bytes.saturating_mul(self.costs.byte),
            span,
        )
    }

    pub(crate) fn instantiate(&mut self, operation: &'static str, span: SourceSpan) -> Option<()> {
        self.charge(
            Counter::Instances,
            operation,
            "monomorphized prelude instances",
            self.costs.instance,
            span,
        )
    }

    /// Check a forthcoming aggregate allocation before reserving it. The
    /// actual constructed value is charged once when it exists.
    pub(crate) fn preflight_construct(
        &mut self,
        operation: &'static str,
        nodes: u64,
        bytes: u64,
        span: SourceSpan,
    ) -> Option<()> {
        self.preview(
            Counter::Nodes,
            operation,
            "constructed value nodes",
            nodes.saturating_mul(self.costs.node),
            span,
        )?;
        self.preview(
            Counter::Bytes,
            operation,
            "constructed value bytes",
            bytes.saturating_mul(self.costs.byte),
            span,
        )
    }

    /// Reserve the number of eventual music occurrences. The finite scalar
    /// fragment reserves zero.
    pub(crate) fn output(&mut self, operation: &'static str, amount: u64, span: SourceSpan) -> Option<()> {
        self.charge(
            Counter::Output,
            operation,
            "estimated music occurrences",
            amount.saturating_mul(self.costs.occurrence),
            span,
        )
    }

    pub(crate) fn failure(&self) -> Option<ResourceError> {
        self.failure
    }

    fn limit(&self, counter: Counter) -> u64 {
        match counter {
            Counter::Steps => self.budget.steps,
            Counter::Nodes => self.budget.nodes,
            Counter::Bytes => self.budget.bytes,
            Counter::Instances => self.budget.instances,
            Counter::Output => self.budget.output,
            Counter::Nesting => self.budget.nesting,
        }
    }

    fn current(&mut self, counter: Counter) -> &mut u64 {
        match counter {
            Counter::Steps => &mut self.steps,
            Counter::Nodes => &mut self.nodes,
            Counter::Bytes => &mut self.bytes,
            Counter::Instances => &mut self.instances,
            Counter::Output => &mut self.output,
            Counter::Nesting => &mut self.nesting,
        }
    }

    fn charge(
        &mut self,
        counter: Counter,
        operation: &'static str,
        metric: &'static str,
        amount: u64,
        span: SourceSpan,
    ) -> Option<()> {
        let attempted = self.attempt(counter, operation, metric, amount, span)?;
        *self.current(counter) = attempted;
        Some(())
    }

    fn preview(
        &mut self,
        counter: Counter,
        operation: &'static str,
        metric: &'static str,
        amount: u64,
        span: SourceSpan,
    ) -> Option<()> {
        self.attempt(counter, operation, metric, amount, span).map(|_| ())
    }

    /// What this charge would bring the counter to, or `None` with the failure
    /// recorded. A meter that has already failed refuses everything after, so
    /// the reported operation is the one that actually crossed the limit.
    fn attempt(
        &mut self,
        counter: Counter,
        operation: &'static str,
        metric: &'static str,
        amount: u64,
        span: SourceSpan,
    ) -> Option<u64> {
        if self.failure.is_some() {
            return None;
        }
        let limit = self.limit(counter);
        let attempted = self.current(counter).saturating_add(amount);
        if attempted > limit {
            self.failure = Some(ResourceError {
                operation,
                metric,
                limit,
                attempted,
                span,
                cost_version: self.costs.version(),
            });
            return None;
        }
        Some(attempted)
    }
}

#[cfg(test)]
// A law suite reports a violated law by failing, which is what `panic!` and
// `expect` are for here.
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::{CompileOptions, SourceDocument, compile};

    /// The divisors the independence law is stated across. Each one is small
    /// enough to make some file fail and large enough to let others through,
    /// which is the interesting range: a budget that stopped everything or
    /// nothing would test the law against no evidence.
    const DIVISORS: [u64; 4] = [2, 8, 64, 512];

    /// Compile `text` under `budget` and return its identity, or `None` if the
    /// compilation was refused.
    ///
    /// Identity is the right thing to compare because it is the versioned
    /// exact identity of the semantic result (`docs/rules/constitution.md`
    /// §8): two compilations that agree here agree on the published score.
    fn accepted_identity(name: &str, text: &str, budget: Budget) -> Option<musa_kernel::SemanticHash> {
        let compilation = under_budget(budget, || {
            compile(&SourceDocument::new(text, name), &CompileOptions::default())
        });
        if compilation.has_errors() {
            let refusal = compilation
                .diagnostics()
                .iter()
                .find(|diagnostic| diagnostic.severity == crate::diagnose::Severity::Error)
                .map(|diagnostic| format!("{:?}: {}", diagnostic.code, diagnostic.message))
                .unwrap_or_default();
            // `Import` as well as `ResourceLimit`, because a narrowed budget
            // may stop a bundled module before the document that imports it
            // ever runs, and the document says what it saw: its import did not
            // compile. Any other code would mean the budget changed which
            // programs are *well-typed*, which is the failure this guards.
            assert!(
                refusal.contains("ResourceLimit") || refusal.contains("Import"),
                "`{name}` was refused under a narrowed budget for a reason that is not the budget: {refusal}"
            );
            return None;
        }
        Some(compilation.identity())
    }

    /// The budget-independence law (`docs/rules/language/00-semantics.md` §2):
    /// a budget can stop an evaluation; it cannot change an accepted one.
    ///
    /// Formally, if `run(b₁, e) ⇓ done(v₁)` and `run(b₂, e) ⇓ done(v₂)` then
    /// `v₁ = v₂`. This is what lets a caller lower a budget to probe a piece
    /// without wondering whether the answer it did get is the answer it would
    /// have got, and it is the reason resource accounting can be an acceptance
    /// rule rather than a scheduler: an accepted value never depends on how
    /// much room the run had left.
    fn assert_budget_independent(name: &str, text: &str) -> Stops {
        let Some(full) = accepted_identity(name, text, Budget::LANGUAGE) else {
            panic!("`{name}` does not compile at the language budget");
        };
        let mut stops: Stops = 0;
        for divisor in DIVISORS {
            let Some(narrowed) = accepted_identity(name, text, Budget::LANGUAGE.narrowed(divisor)) else {
                stops = stops.saturating_add(1); // Stopped, which the law permits.
                continue;
            };
            assert_eq!(
                narrowed, full,
                "`{name}` compiled to a different score at a budget {divisor} times smaller"
            );
        }
        stops
    }

    /// How many of the narrowed budgets stopped the run. Counted because a law
    /// that only ever saw budgets nothing could exhaust would pass without
    /// having been asked anything.
    type Stops = usize;

    /// Every committed example, as (name, text), excluding the fixtures that
    /// exist in order to be refused.
    ///
    /// Two examples are excluded for cost rather than content, and the law
    /// cannot ask its question of a run that never answers: both exhaust the
    /// language budget on both checkers — measured again after the course
    /// correction's engine landed, against `10b7432` before it. One is
    /// `diatonic-sequences.musa`, whose scale-degree arithmetic costs more
    /// reduction steps than `Budget::LANGUAGE` allows; the other is
    /// `staff-page.musa`, whose adapter expansion crosses the nesting limit
    /// — the residual `musa_core::Budget::LANGUAGE`'s comment records for the
    /// staff adapter's rewrite. Each returns to the corpus with the migration
    /// that makes it cheap, not with a higher budget: §4's independence law is
    /// stated on the ones that fit.
    fn corpus() -> Vec<(String, String)> {
        const OVER_BUDGET: [&str; 2] = ["diatonic-sequences.musa", "staff-page.musa"];
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&root)
            .expect("read examples/")
            .map(|entry| entry.expect("directory entry").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "musa"))
            .filter(|path| !OVER_BUDGET.contains(&path.file_name().expect("file name").to_string_lossy().as_ref()))
            .collect();
        files.sort();
        assert!(!files.is_empty(), "the example corpus is empty");
        files
            .into_iter()
            .map(|path| {
                let name = path.file_name().expect("file name").to_string_lossy().into_owned();
                (name, std::fs::read_to_string(&path).expect("example text"))
            })
            .collect()
    }

    /// Nesting is refused at the limit, and the refusal says what crossed it.
    ///
    /// Stated on the meter rather than through a deep region because the
    /// property is the counter's, not any one traversal's: whatever recurses,
    /// it stops here, and it stops with an operation, a metric, a limit, and a
    /// place rather than with a dead process.
    #[test]
    fn nesting_stops_at_the_limit_and_names_where_it_stopped() {
        /// Descend forever, or as far as the meter allows.
        fn descend(meter: &mut WorkMeter, span: SourceSpan) -> Option<()> {
            meter.nested(Reduction::SyntaxRecurse, span, |meter| descend(meter, span))
        }
        let mut meter = WorkMeter::new(Budget::LANGUAGE);
        let span = SourceSpan::new(7, 11);
        assert!(descend(&mut meter, span).is_none(), "unbounded descent was not stopped");
        let failure = meter.failure().expect("the stop was recorded as a resource failure");
        assert_eq!(failure.metric, NESTING_METRIC);
        assert_eq!(failure.operation, Reduction::SyntaxRecurse.operation());
        assert_eq!(failure.limit, NESTING);
        assert_eq!(failure.attempted, NESTING.saturating_add(1));
        assert_eq!(
            failure.span, span,
            "the refusal points at the operation that went too deep"
        );
    }

    /// A level is given back when its frame returns.
    ///
    /// The one counter that goes down, and the one that would silently turn
    /// every long run into a refusal if it did not: an evaluation that enters
    /// and leaves a million times is a level deep, not a million.
    #[test]
    fn a_level_is_released_when_the_work_at_it_finishes() {
        let mut meter = WorkMeter::new(Budget::LANGUAGE);
        let span = SourceSpan::default();
        for _ in 0..NESTING.saturating_mul(4) {
            assert!(
                meter.nested(Reduction::Expression, span, |_| Some(())).is_some(),
                "a sibling was charged as if it were a descendant"
            );
        }
        assert!(meter.failure().is_none());
    }

    /// The law over every committed example, at every divisor.
    ///
    /// The whole corpus rather than a sample because the law is about the
    /// evaluator, not about a file: one declaration anywhere that read the
    /// budget as data would be a piece that compiles to two different scores.
    ///
    /// The non-vacuity half — some run that a narrowed budget stops — is not
    /// stated here, because it cannot be: the language's own meters run at
    /// `Budget::LANGUAGE` on purpose (a budget the caller could lower would
    /// make acceptance a property of the invocation), so a narrowed
    /// `WorkMeter` can only stop the phases this crate still meters, and no
    /// committed example spends enough there. That narrowing *can* stop a run
    /// is the meter-level laws above and `musa_core`'s `scaled` tests, and a
    /// corpus law asserting it here would be asserting it of nothing.
    #[test]
    fn a_narrowed_budget_stops_a_run_without_changing_an_accepted_one() {
        for (name, text) in corpus() {
            assert_budget_independent(&name, &text);
        }
    }
}
