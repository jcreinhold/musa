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

/// The versioned assignment of nonnegative integer costs to reductions and
/// constructions (`02-core-calculus.md` §3).
///
/// Costs are data rather than scattered constants because changing one changes
/// which projects the language accepts. A change is a version bump with a
/// stated reason, and [`CostTable::version`] is what a rejection cites and what
/// a later result cache will have to record: two compilers that agree on the
/// source and disagree on this table do not agree on acceptance.
#[derive(Clone, Copy)]
pub(crate) struct CostTable {
    version: u32,
    reduction: u64,
    node: u64,
    byte: u64,
    instance: u64,
    occurrence: u64,
}

impl CostTable {
    /// Version 1: one unit per reduction, per constructed node, per logical
    /// value byte, per instantiated prelude entry, and per estimated
    /// occurrence.
    ///
    /// Uniform on purpose. A weight that differed between reductions would be a
    /// claim about their relative expense, and that claim needs measurement
    /// (prompts 124 and 142) rather than an author's intuition. What version 1
    /// fixes is that the weights exist, are named, and move together.
    pub(crate) const V1: Self = Self {
        version: 1,
        reduction: 1,
        node: 1,
        byte: 1,
        instance: 1,
        occurrence: 1,
    };

    pub(crate) const fn version(self) -> u32 {
        self.version
    }
}

/// The limits the cost table is spent against (`02-core-calculus.md` §4).
#[derive(Clone, Copy)]
pub(crate) struct Budget {
    steps: u64,
    nodes: u64,
    bytes: u64,
    instances: u64,
    output: u64,
}

impl Budget {
    /// The prompt-96 language defaults: 200,000 reduction steps, 100,000
    /// constructed value nodes, 1,048,576 logical value bytes, 2,048
    /// instantiated prelude entries, and 1,000,000 estimated occurrences.
    ///
    /// These are language-version constants, not timeouts or machine-memory
    /// observations.
    pub(crate) const LANGUAGE: Self = Self {
        steps: 200_000,
        nodes: 100_000,
        bytes: 1024 * 1024,
        instances: 2_048,
        output: 1_000_000,
    };

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
        }
    }
}

/// Which reduction or construction is being charged.
///
/// The name is the one the diagnostic prints, so a rejection says what the
/// project asked for rather than which counter overflowed.
#[derive(Clone, Copy)]
pub(crate) enum Reduction {
    Expression,
    MatchArm,
    ScaleStep,
    Application,
    Range,
    Repeat,
    Map,
    Filter,
    NatFold,
    ListFold,
    OptionFold,
    DataFold,
}

impl Reduction {
    pub(crate) const fn operation(self) -> &'static str {
        match self {
            Self::Expression => "expression evaluation",
            Self::MatchArm => "match arm",
            Self::ScaleStep => "scale step",
            Self::Application => "function application",
            Self::Range => "range",
            Self::Repeat => "repeat",
            Self::Map => "map",
            Self::Filter => "filter",
            Self::NatFold => "nat_fold",
            Self::ListFold => "list_fold",
            Self::OptionFold => "option_fold",
            Self::DataFold => "fold",
        }
    }
}

/// The one resource outcome: which operation crossed which limit, and where.
#[derive(Clone, Copy)]
pub(crate) struct ResourceError {
    pub(crate) operation: &'static str,
    pub(crate) metric: &'static str,
    pub(crate) limit: u64,
    pub(crate) attempted: u64,
    pub(crate) span: SourceSpan,
    pub(crate) cost_version: u32,
}

/// A typed evaluation configuration for a result of type `T`.
///
/// The language has exactly two outcomes, `done` and `failed`; [`Self::Broken`]
/// is not a third. A closed well-typed term can always take a step (research
/// `core-calculus/06-proof-outline.md` Theorem 2.3), so reaching `Broken` means
/// the checker admitted something the evaluator cannot run — a compiler
/// invariant failure, reported as one rather than as a language effect.
pub(crate) enum Evaluation<T> {
    Done(T),
    Failed(ResourceError),
    Broken,
}

pub(crate) struct WorkMeter {
    budget: Budget,
    costs: CostTable,
    steps: u64,
    nodes: u64,
    bytes: u64,
    instances: u64,
    output: u64,
    failure: Option<ResourceError>,
}

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
            costs: CostTable::V1,
            steps: 0,
            nodes: 0,
            bytes: 0,
            instances: 0,
            output: 0,
            failure: None,
        }
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

    /// Check an eventual timeline before any occurrence-sized allocation is
    /// performed. This is deliberately separate from [`Self::output`]: a
    /// transform may need a temporary value, while the published score is
    /// charged exactly once at the compilation boundary.
    pub(crate) fn preflight_output(&mut self, operation: &'static str, amount: u64, span: SourceSpan) -> Option<()> {
        self.preview(
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
        }
    }

    fn current(&mut self, counter: Counter) -> &mut u64 {
        match counter {
            Counter::Steps => &mut self.steps,
            Counter::Nodes => &mut self.nodes,
            Counter::Bytes => &mut self.bytes,
            Counter::Instances => &mut self.instances,
            Counter::Output => &mut self.output,
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

#[derive(Clone, Copy)]
enum Counter {
    Steps,
    Nodes,
    Bytes,
    Instances,
    Output,
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
    fn corpus() -> Vec<(String, String)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&root)
            .expect("read examples/")
            .map(|entry| entry.expect("directory entry").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "musa"))
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

    /// The law over every committed example, at every divisor.
    ///
    /// The whole corpus rather than a sample because the law is about the
    /// evaluator, not about a file: one declaration anywhere that read the
    /// budget as data would be a piece that compiles to two different scores.
    #[test]
    fn a_narrowed_budget_stops_a_run_without_changing_an_accepted_one() {
        let mut stops: Stops = 0;
        for (name, text) in corpus() {
            stops = stops.saturating_add(assert_budget_independent(&name, &text));
        }
        assert!(stops > 0, "no narrowed budget stopped a run: the law was never asked");
    }
}
