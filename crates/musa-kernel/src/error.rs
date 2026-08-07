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
    /// A `seq` or `over` term with no arguments (K7).
    #[error("`{form}` needs at least one argument; the empty case is written as a literal timeline")]
    EmptyComposition {
        /// The offending form, `"seq"` or `"over"`.
        form: &'static str,
    },
    /// A term names something no enclosing `let` binds (K7).
    #[error("`{name}` is not bound by any enclosing `let`")]
    FreeName {
        /// The offending name.
        name: String,
    },
    /// A `let` rebinds a name already in scope (K7).
    #[error("`{name}` is already bound; shadowing is rejected so substitution stays textual")]
    ShadowedName {
        /// The offending name.
        name: String,
    },
    /// Kernel text that is not a term (docs/kernel/01).
    #[error("kernel text at byte {offset}: {message}")]
    Parse {
        /// The byte offset the reader stopped at.
        offset: usize,
        /// What was expected there.
        message: String,
    },
    /// Time scaling by a non-positive factor (K2/D5).
    #[error("scale factor must be a positive rational, got {factor}")]
    NonPositiveScale {
        /// The offending factor, in reduced form.
        factor: String,
    },
}
