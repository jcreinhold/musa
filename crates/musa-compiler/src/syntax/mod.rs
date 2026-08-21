//! The phase-local syntax calculus: finite syntax values, derived paths, and
//! the gate a transformer's output passes through.
//!
//! `docs/rules/language/02-core-calculus.md` §5 closes the source type grammar
//! and says the source language has no syntax value. That stays true. Nothing
//! here is nameable from ordinary source: the types have no written spelling at
//! all, and the operations over them are offered only where a transformer is
//! checked ([`crate::core`]'s expansion phase). What §5's sentence forbids is a
//! program that can inspect itself, and a program still cannot.
//!
//! The defect this module exists to repair is
//! `docs/notes/research/language-design-closure/37-final-blocker.md` §1: the
//! previous design's `NodePath` and `BindingPath` were abstract with no
//! constructors, so a transformer could not obtain the path of an input node
//! nor derive a child path, and the displayed staff and studio expansions were
//! desired output rather than programs. Here **a path is derived, never
//! invented**. There are exactly two sources of one:
//!
//! - the path-aware fold hands each input node its own structural path; and
//! - [`NodePath::built`] and [`NodePath::binding`] derive an output path from a
//!   path already held, a builder role, and a child number.
//!
//! There is no constructor from a number, a name, or a counter, and nothing
//! mints a fresh id — which is the other half of the repair
//! `34-proof-review.md` asked for, where a `fresh_name` operation could not be
//! both deterministic and fresh. Hygiene here is a *coordinate*: a binder and
//! a reference written at one [`BindingPath`] carry one [`Scope`], and two
//! binding paths are two names. Nothing is allocated, so two runs agree.
//!
//! A transformer also never reads a source range. [`SourceInfo`] lives inside
//! the value and has no eliminator: a transformer carries a node in order to
//! point at it, and can neither read nor forge where it came from.

mod build;
mod category;
mod gate;
mod instantiate;
mod matching;
mod path;
mod print;
mod read;
#[cfg(test)]
mod tests;
mod tree;

pub(crate) use build::{binder, delimited, group, identifier, reference, token};
pub(crate) use category::{Cat, Delimiter, token_kind_named, token_kind_spelling};
pub(crate) use gate::{NotAnExpression, check_expression};
pub(crate) use instantiate::{instantiate, template_root};
pub(crate) use matching::{Hygiene, Spliced, Template, matched};
pub(crate) use path::{BindingPath, Derived, ExpansionPath, NodePath, anchor_place};
#[cfg(test)]
pub(crate) use print::Printed;
pub(crate) use print::print;
pub(crate) use read::{parses_as_expression, read_expression, read_region};
pub(crate) use tree::{SourceInfo, Syntax};
