//! Adapter expansion: the compiler's one place where a module (an adapter
//! package) runs against a piece of source and produces either edits to the
//! source or a printed answer, charged to the phase budget.
//!
//! The four submodules split the one expansion concern by role: `records`
//! holds the data types the whole pass shares (charges, expansion records,
//! the source map, the expansion header), `levels` the declared-adapter-level
//! machinery (which regions an adapter may touch), `machine` the mechanics of
//! expanding one region and digesting the result, and `adapter` the public
//! API (`expand`, `adapter_edits`, `adapter_print`) plus the error types the
//! library surface exposes.

mod adapter;
mod levels;
mod machine;
mod records;
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests;

pub use adapter::{AdapterEdit, AdapterEditError, AdapterPrintError, adapter_edits, adapter_print};
pub(crate) use adapter::{expand, refusal};
pub(crate) use levels::{Level, SyntaxImport, level_of};
pub(crate) use machine::{Cached, expand_one, names_written, region_body, region_name, syntax_imports};
#[cfg(test)]
pub(crate) use machine::{ordinary_expression, stopped_or_refused};
pub(crate) use records::{Charges, Expansion, ExpansionRecord, Replacement, SourceMap};
