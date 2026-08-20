//! The semantic core the checker lowers into: shapes, builtin ownership
//! tables, adapter-phase reading, and the expand/edit/print entry points.

mod adapter;
mod edit;
mod expand;
mod nodes;
mod ownership;
mod print;
mod printer;
mod transform;
mod types;

pub(crate) use adapter::{ModuleFault, PhaseImports, Unrun, read_adapter_module};
pub(crate) use edit::{EditFailure, PhaseWork, edit_syntax, region, said};
#[cfg(test)]
pub(crate) use expand::expand_region;
pub(crate) use expand::{ExpansionFailure, refusal_of};
pub(crate) use nodes::{expr_node_of, phase_type, type_node_of};
#[cfg(test)]
pub(crate) use ownership::PhaseFamily;
pub(crate) use ownership::{BUILTIN_OWNERSHIP, Eliminator, MachineOp, SYNTAX_OWNERSHIP, SyntaxOp};
pub(crate) use print::{PrintFailure, print_value};
pub(crate) use printer::{Printer, printer_source};
#[cfg(test)]
pub(crate) use transform::evaluate_text;
pub(crate) use transform::{expand_syntax, not_the_operation};
pub(crate) use types::{
    BOOL, Base, Builtin, CHORD, CLASS, Coordinate, DEGREE, DURATION, FRAME, Family, INTERVAL, INTERVALS, KEY,
    MAYBE_CHORD, MAYBE_CLASS, MAYBE_DEGREE, MAYBE_FRAME, MAYBE_NAT, MAYBE_ROMAN, MAYBE_TEXT, MAYBE_TRIAD,
    MAYBE_VOICING, NAT, NATS, PC12, PC12S, PCSET12, PITCH, PITCHES, POSITION, RATIO, ROMAN, ROW12, ROW12_OR_FAULT,
    ROW12S, SCALE, Shape, TEXT, TEXTS, TRIAD, Type, VOICING, delta,
};
