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
    level: u64,
}

impl CostTable {
    /// Version 3: one unit per reduction, per constructed node, per logical
    /// value byte, per instantiated prelude entry, per estimated occurrence,
    /// and per nested evaluation level.
    ///
    /// Uniform on purpose. A weight that differed between reductions would be a
    /// claim about their relative expense, and that claim needs measurement
    /// (prompts 124 and 168) rather than an author's intuition. What the
    /// weights fix is that they exist, are named, and move together.
    ///
    /// A new reduction *kind* is therefore not a table change. Prompt 127dcfaa
    /// added `list_fold_from_start` and `list_fold_from_end` and left this
    /// version alone: the weights are per metric, not per kind, so the new
    /// names change what a rejection prints and nothing about what it costs. A
    /// program naming `list_fold_from_end` is refused by a version-2 compiler
    /// at resolution, because the builtin is not there to resolve, so there is
    /// no version at which two compilers disagree about its cost.
    ///
    /// Version 2 changed no weight. It changed where a value is charged:
    /// version 1 charged a value's whole shape at every expression that named
    /// it and at every closure that captured it, which made a project's cost
    /// the product of its data size and its program size rather than the count
    /// of what it built. A value is now charged once, where it is constructed
    /// (`crate::core::charged_shape`). Every charge is pointwise no larger than
    /// version 1's, so no project that compiled under version 1 stops
    /// compiling; a rejection cites the version because two compilers that
    /// disagree here do not agree on acceptance.
    ///
    /// Version 3 added the sixth metric, nested evaluation levels
    /// (`../rules/language/02-core-calculus.md` §4). That *is* a table change
    /// rather than a new reduction kind: a program the evaluator would have
    /// entered 300 levels deep is refused now and was not before, so two
    /// compilers that disagree about this metric disagree about acceptance and
    /// must not share a version. Nothing else moved — every version-2 weight is
    /// still 1, and every charge a version-2 compiler made a version-3 compiler
    /// makes identically.
    pub(crate) const V3: Self = Self {
        version: 3,
        reduction: 1,
        node: 1,
        byte: 1,
        instance: 1,
        occurrence: 1,
        level: 1,
    };

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

/// What a nesting refusal prints for its metric.
///
/// Named because two places read it: the refusal that records it and the
/// diagnostic that decides what advice to offer, which for this one metric is
/// not "make it smaller".
pub(crate) const NESTING_METRIC: &str = "nested evaluation levels";

/// The limits the cost table is spent against (`02-core-calculus.md` §4).
#[derive(Clone, Copy)]
pub(crate) struct Budget {
    steps: u64,
    nodes: u64,
    bytes: u64,
    instances: u64,
    output: u64,
    nesting: u64,
}

impl Budget {
    /// The prompt-96 language defaults: 200,000 reduction steps, 100,000
    /// constructed value nodes, 1,048,576 logical value bytes, 2,048
    /// instantiated prelude entries, and 1,000,000 estimated occurrences —
    /// with 256 nested evaluation levels, added to guard the descent prompt
    /// 127dcfaf's recursor introduced.
    ///
    /// These are language-version constants, not timeouts or machine-memory
    /// observations.
    pub(crate) const LANGUAGE: Self = Self {
        steps: 200_000,
        nodes: 100_000,
        bytes: 1024 * 1024,
        instances: 2_048,
        output: 1_000_000,
        nesting: NESTING,
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
            nesting: share(self.nesting, divisor),
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
    ListFoldFromStart,
    ListFoldFromEnd,
    OptionFold,
    DataFold,
    SyntaxFold,
    SyntaxRecurse,
    SyntaxStepMint,
    SyntaxStepRun,
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
            Self::ListFoldFromStart => "list_fold_from_start",
            Self::ListFoldFromEnd => "list_fold_from_end",
            Self::OptionFold => "option_fold",
            Self::DataFold => "fold",
            Self::SyntaxFold => "syntax_fold_from_leaves",
            Self::SyntaxRecurse => "recurse_syntax",
            // Minting and running are charged apart from entering a node so
            // that capture and repetition cost what they cost: a step kept and
            // never run is charged its mint alone, and a step run twice is
            // charged twice (`../rules/language/02-core-calculus.md` §5.9,
            // law 10).
            Self::SyntaxStepMint => "syntax step",
            Self::SyntaxStepRun => "run_syntax_step",
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
    /// How many evaluator frames are open right now.
    ///
    /// The one counter that goes back down. Every other metric measures what a
    /// run has spent and never returns; this one measures how far in it
    /// currently is, because what it stands for — machine stack — is given back
    /// when a frame returns.
    nesting: u64,
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

    /// How many reduction steps this meter has charged.
    ///
    /// The `evaluation_steps` charge of `26-language-design-decision.md` §3.5.
    pub(crate) const fn steps(&self) -> u64 {
        self.steps
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

#[derive(Clone, Copy)]
enum Counter {
    Steps,
    Nodes,
    Bytes,
    Instances,
    Output,
    Nesting,
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
    #[test]
    fn a_narrowed_budget_stops_a_run_without_changing_an_accepted_one() {
        let mut stops: Stops = 0;
        for (name, text) in corpus() {
            stops = stops.saturating_add(assert_budget_independent(&name, &text));
        }
        assert!(stops > 0, "no narrowed budget stopped a run: the law was never asked");
    }
}
