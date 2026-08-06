//! Graph compilation errors: explicit, never silent (roadmap §7.2).

use crate::spec::NodeId;

/// A failure to compile a `StudioGraphSpec` into a `RenderPlan`.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GraphError {
    /// A connection names a node that does not exist.
    #[error("unknown node {0:?} in connection")]
    UnknownNode(NodeId),
    /// A connection names a port outside the processor's port list.
    #[error("node {node:?}: port index {port} is out of range")]
    PortOutOfRange {
        /// The node.
        node: NodeId,
        /// The offending port index.
        port: usize,
    },
    /// Connected ports have incompatible kinds or channel counts (§13.4).
    #[error("port mismatch: {from} into {to}")]
    PortMismatch {
        /// Source port description.
        from: String,
        /// Destination port description.
        to: String,
    },
    /// An input port is fed by more than one connection.
    #[error("node {node:?}: input port {port} has multiple sources")]
    DuplicateInput {
        /// The node.
        node: NodeId,
        /// The input port.
        port: usize,
    },
    /// The graph has a cycle. Cycles are legal only through an explicit
    /// delay node (§5.6); none exists yet, so every cycle is an error —
    /// the rule is encoded, not just the current case.
    #[error("the graph has a cycle (legal only through an explicit delay node)")]
    Cycle,
    /// No output node was designated.
    #[error("no output node designated; call `set_output`")]
    OutputMissing,
    /// A parameter name is unknown for the node's processor, or its value is
    /// not finite.
    #[error("node {node:?}: invalid parameter `{name}`")]
    InvalidParameter {
        /// The node.
        node: NodeId,
        /// The parameter name.
        name: String,
    },
}
