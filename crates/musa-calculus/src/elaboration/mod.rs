//! The untrusted half: what an author wrote, turned into a finished term.
//!
//! Everything here reads a [`Raw`](raw::Raw) or raises a
//! [`Refusal`](refuse::Refusal), and may name anything under
//! [`crate::kernel`]. The kernel may name nothing here.

pub(crate) mod admit;
pub(crate) mod case;
pub(crate) mod convert;
pub(crate) mod declare;
pub(crate) mod declare_program;
pub(crate) mod elab;
pub(crate) mod namespace;
pub(crate) mod raw;
pub(crate) mod rec;
pub(crate) mod refuse;
pub(crate) mod show;
pub(crate) mod storable;
