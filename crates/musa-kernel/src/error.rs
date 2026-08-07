//! Static violations of the kernel rules (docs/kernel/02). The kernel has no
//! warnings: a construct is well-formed or rejected.

use crate::time::{Beat, Span};

/// A kernel construction error, naming the violated rule.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum KernelError {
    /// A span's start exceeds its end, or either is negative (K1).
    #[error("invalid span [{start}, {end}): spans satisfy 0 <= start <= end")]
    InvalidSpan {
        /// The offending start.
        start: Beat,
        /// The offending end.
        end: Beat,
    },
    /// An occurrence lies outside its timeline's extent (K1).
    #[error("occurrence span {span} outside extent [0, {extent})")]
    OccurrenceOutOfBounds {
        /// The offending span.
        span: Span,
        /// The timeline extent.
        extent: Beat,
    },
    /// Time scaling by a non-positive factor (K2/D5).
    #[error("scale factor must be a positive rational, got {factor}")]
    NonPositiveScale {
        /// The offending factor, in reduced form.
        factor: String,
    },
}
