//! The trusted half: what a finished term is, and how it computes.
//!
//! Everything here works on [`Term`](term::Term) and [`Value`](value::Value)
//! and raises [`CoreError`](error::CoreError). Nothing here may name an item
//! defined under [`crate::elaboration`] — the direction is stated in
//! [`crate`]'s invariants and checked by `boundary_laws.rs`, because Rust
//! cannot say "a module may not see its sibling".

pub(crate) mod base;
pub(crate) mod budget;
pub(crate) mod case_tree;
pub(crate) mod checked;
pub(crate) mod context;
pub(crate) mod error;
pub(crate) mod eval;
pub(crate) mod family;
pub(crate) mod list;
pub(crate) mod meta;
pub(crate) mod origin;
pub(crate) mod program;
pub(crate) mod quote;
pub(crate) mod recheck;
pub(crate) mod room;
pub(crate) mod scope;
pub(crate) mod sort;
pub(crate) mod term;
pub(crate) mod terminate;
pub(crate) mod unify;
pub(crate) mod value;
pub(crate) mod visibility;
