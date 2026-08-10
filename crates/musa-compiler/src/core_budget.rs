//! Deterministic resource accounting for the private elaboration evaluator.
//!
//! These are language acceptance limits, not wall-clock timeouts. The same
//! source and compiler version therefore fail at the same operation on every
//! machine. Prompt 120 may tune the constants from a wider corpus, but may not
//! change this accounting into an interrupt or a partial-result mechanism.

use crate::origin::SourceSpan;

pub(crate) const STEP_LIMIT: u64 = 200_000;
pub(crate) const NODE_LIMIT: u64 = 100_000;
pub(crate) const BYTE_LIMIT: u64 = 1024 * 1024;
pub(crate) const INSTANCE_LIMIT: u64 = 2_048;
pub(crate) const OUTPUT_LIMIT: u64 = 1_000_000;

#[derive(Clone, Copy)]
pub(crate) struct Exhaustion {
    pub(crate) operation: &'static str,
    pub(crate) metric: &'static str,
    pub(crate) limit: u64,
    pub(crate) attempted: u64,
    pub(crate) span: SourceSpan,
}

#[derive(Default)]
pub(crate) struct WorkMeter {
    steps: u64,
    nodes: u64,
    bytes: u64,
    instances: u64,
    output: u64,
    exhaustion: Option<Exhaustion>,
}

impl WorkMeter {
    pub(crate) fn step(&mut self, operation: &'static str, amount: u64, span: SourceSpan) -> bool {
        Self::charge(
            &mut self.steps,
            STEP_LIMIT,
            operation,
            "reduction steps",
            amount,
            span,
            &mut self.exhaustion,
        )
    }

    pub(crate) fn construct(&mut self, operation: &'static str, nodes: u64, bytes: u64, span: SourceSpan) -> bool {
        Self::charge(
            &mut self.nodes,
            NODE_LIMIT,
            operation,
            "constructed value nodes",
            nodes,
            span,
            &mut self.exhaustion,
        ) && Self::charge(
            &mut self.bytes,
            BYTE_LIMIT,
            operation,
            "constructed value bytes",
            bytes,
            span,
            &mut self.exhaustion,
        )
    }

    pub(crate) fn instantiate(&mut self, operation: &'static str, span: SourceSpan) -> bool {
        Self::charge(
            &mut self.instances,
            INSTANCE_LIMIT,
            operation,
            "monomorphized prelude instances",
            1,
            span,
            &mut self.exhaustion,
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
    ) -> bool {
        Self::preview(
            self.nodes,
            NODE_LIMIT,
            operation,
            "constructed value nodes",
            nodes,
            span,
            &mut self.exhaustion,
        ) && Self::preview(
            self.bytes,
            BYTE_LIMIT,
            operation,
            "constructed value bytes",
            bytes,
            span,
            &mut self.exhaustion,
        )
    }

    /// Reserve the number of eventual music occurrences. The finite scalar
    /// fragment reserves zero; prompt 97 charges this same meter when `music`
    /// constructors arrive.
    pub(crate) fn output(&mut self, operation: &'static str, amount: u64, span: SourceSpan) -> bool {
        Self::charge(
            &mut self.output,
            OUTPUT_LIMIT,
            operation,
            "estimated music occurrences",
            amount,
            span,
            &mut self.exhaustion,
        )
    }

    /// Check an eventual timeline before any occurrence-sized allocation is
    /// performed. This is deliberately separate from [`Self::output`]: a
    /// transform may need a temporary value, while the published score is
    /// charged exactly once at the compilation boundary.
    pub(crate) fn preflight_output(&mut self, operation: &'static str, amount: u64, span: SourceSpan) -> bool {
        Self::preview(
            self.output,
            OUTPUT_LIMIT,
            operation,
            "estimated music occurrences",
            amount,
            span,
            &mut self.exhaustion,
        )
    }

    pub(crate) fn exhaustion(&self) -> Option<Exhaustion> {
        self.exhaustion
    }

    #[allow(clippy::too_many_arguments)]
    fn charge(
        current: &mut u64,
        limit: u64,
        operation: &'static str,
        metric: &'static str,
        amount: u64,
        span: SourceSpan,
        exhaustion: &mut Option<Exhaustion>,
    ) -> bool {
        if exhaustion.is_some() {
            return false;
        }
        let attempted = current.saturating_add(amount);
        if attempted > limit {
            *exhaustion = Some(Exhaustion {
                operation,
                metric,
                limit,
                attempted,
                span,
            });
            return false;
        }
        *current = attempted;
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn preview(
        current: u64,
        limit: u64,
        operation: &'static str,
        metric: &'static str,
        amount: u64,
        span: SourceSpan,
        exhaustion: &mut Option<Exhaustion>,
    ) -> bool {
        if exhaustion.is_some() {
            return false;
        }
        let attempted = current.saturating_add(amount);
        if attempted > limit {
            *exhaustion = Some(Exhaustion {
                operation,
                metric,
                limit,
                attempted,
                span,
            });
            return false;
        }
        true
    }
}
