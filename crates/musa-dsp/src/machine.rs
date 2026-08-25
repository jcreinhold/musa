//! The reference interpreter for the governing machine calculus.
//!
//! Preparation is the only constructor. It checks the compiler's finite,
//! bottom-up projection against the build-local primitive table, solves the
//! structural port constraints, decodes stored data once, and builds a private
//! tree. Starting then creates the private state prescribed by §3; stepping is
//! a direct transcription of the equations and has no scheduler.

use std::collections::HashSet;

use musa_score::machine::{MachineSpec, PortSchema, SpecForm};
use num_bigint::BigInt;
use num_rational::BigRational;

const MAX_NODES: usize = 4_096;
const MAX_DEPTH: usize = 256;
const MAX_STATE_BYTES: usize = 1 << 20;
const MAX_STEP_WORK: usize = 1 << 20;

/// Storable data crossing a machine port.
///
/// Products are binary because that is the product in the governing calculus.
/// Rationals use arbitrary-precision arithmetic so registered exact operations
/// remain total rather than becoming host-integer overflow paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MachineValue {
    Unit,
    Bool(bool),
    Nat(u64),
    Ratio(BigRational),
    Pair(Box<(Self, Self)>),
}

impl MachineValue {
    /// Construct a small exact rational conveniently.
    #[must_use]
    pub fn ratio(numerator: i64, denominator: i64) -> Option<Self> {
        (denominator != 0).then(|| Self::Ratio(BigRational::new(BigInt::from(numerator), BigInt::from(denominator))))
    }

    /// Construct a product value.
    #[must_use]
    pub fn pair(first: Self, second: Self) -> Self {
        Self::Pair(Box::new((first, second)))
    }

    fn schema(&self) -> PortSchema {
        match self {
            Self::Unit => PortSchema::Unit,
            Self::Bool(_) => PortSchema::Bool,
            Self::Nat(_) => PortSchema::Nat,
            Self::Ratio(_) => PortSchema::Ratio,
            Self::Pair(pair) => PortSchema::Pair(Box::new(pair.0.schema()), Box::new(pair.1.schema())),
        }
    }

    fn matches(&self, schema: &PortSchema) -> bool {
        match (self, schema) {
            (Self::Unit, PortSchema::Unit)
            | (Self::Bool(_), PortSchema::Bool)
            | (Self::Nat(_), PortSchema::Nat)
            | (Self::Ratio(_), PortSchema::Ratio) => true,
            (Self::Pair(pair), PortSchema::Pair(first, second)) => pair.0.matches(first) && pair.1.matches(second),
            _ => false,
        }
    }

    fn into_pair(self) -> Option<(Self, Self)> {
        match self {
            Self::Pair(pair) => Some(*pair),
            Self::Unit | Self::Bool(_) | Self::Nat(_) | Self::Ratio(_) => None,
        }
    }

    fn stored_bytes(&self) -> usize {
        match self {
            Self::Unit => 1,
            Self::Bool(_) => 2,
            Self::Nat(_) => 9,
            Self::Ratio(value) => value
                .numer()
                .to_signed_bytes_be()
                .len()
                .saturating_add(value.denom().to_signed_bytes_be().len()),
            Self::Pair(pair) => 1usize
                .saturating_add(pair.0.stored_bytes())
                .saturating_add(pair.1.stored_bytes()),
        }
    }
}

/// Why a finite checked machine description could not be prepared.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PrepareError {
    #[error("the machine projection has no root")]
    Empty,
    #[error("the machine has {actual} nodes; this build admits at most {limit}")]
    NodeLimit { actual: usize, limit: usize },
    #[error("node {node} ({form:?}) has {actual} children; it requires {expected}")]
    ChildCount {
        node: usize,
        form: SpecForm,
        actual: usize,
        expected: usize,
    },
    #[error("node {node} refers to child {child}, which does not precede it")]
    ChildOrder { node: usize, child: usize },
    #[error("node {node} is reused or is not beneath the root; a machine projection must be a tree")]
    NotTree { node: usize },
    #[error("primitive `{id}` version {version} is not registered in this build")]
    Unregistered { id: String, version: u32 },
    #[error("runtime registration for `{id}` version {version} disagrees with the compiler descriptor")]
    DescriptorConflict { id: String, version: u32 },
    #[error("stored data at node {node} is not a complete {expected} value")]
    StoredValue { node: usize, expected: String },
    #[error("machine port constraints disagree: {left} cannot equal {right}")]
    PortMismatch { left: String, right: String },
    #[error("machine port constraints contain an infinite schema")]
    InfinitePort,
    #[error("node {node}'s port remained undetermined after checking the root")]
    UndeterminedPort { node: usize },
    #[error("machine nesting depth {actual} exceeds this build's limit {limit}")]
    DepthLimit { actual: usize, limit: usize },
    #[error("machine private state requires {actual} bytes; this build admits at most {limit}")]
    StateLimit { actual: usize, limit: usize },
    #[error("one machine step costs {actual} work units; this build admits at most {limit}")]
    WorkLimit { actual: usize, limit: usize },
    #[error("primitive registry contains conflicting entries for `{id}` version {version}")]
    RegistryConflict { id: String, version: u32 },
}

/// A prepared description with decoded primitives and checked wiring.
///
/// Node order, primitive instances, buffers, and state layout are private. The
/// value is inert until [`Self::start`] is called explicitly.
pub struct PreparedMachine {
    root: PreparedNode,
    input: PortSchema,
    output: PortSchema,
}

impl PreparedMachine {
    /// Allocate the unique initial private state prescribed by the machine.
    #[must_use]
    pub fn start(&self) -> StartedMachine {
        StartedMachine {
            root: self.root.start(),
            input: self.input.clone(),
            output: self.output.clone(),
        }
    }

    /// The checked input schema.
    #[must_use]
    pub const fn input(&self) -> &PortSchema {
        &self.input
    }

    /// The checked output schema.
    #[must_use]
    pub const fn output(&self) -> &PortSchema {
        &self.output
    }
}

/// A running machine. Its combined state is deliberately inaccessible.
pub struct StartedMachine {
    root: RunningNode,
    input: PortSchema,
    output: PortSchema,
}

impl StartedMachine {
    /// Take exactly one semantic step.
    ///
    /// The interpreter is total on a value of the prepared input schema. A
    /// mismatched dynamic value is refused before private state is touched.
    ///
    /// # Errors
    ///
    /// [`StepError::Input`] when `input` has a different storable schema.
    pub fn step(&mut self, input: MachineValue) -> Result<MachineValue, StepError> {
        if !input.matches(&self.input) {
            return Err(StepError::Input {
                expected: self.input.spelling(),
                actual: input.schema().spelling(),
            });
        }
        let output = self.root.step(input)?;
        if !output.matches(&self.output) {
            return Err(StepError::Registration {
                expected: self.output.spelling(),
                actual: output.schema().spelling(),
            });
        }
        Ok(output)
    }
}

/// A dynamic caller supplied a value at the wrong checked port.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StepError {
    #[error("machine input requires {expected}, but received {actual}")]
    Input { expected: String, actual: String },
    #[error("a checked primitive returned {actual}; its registration promises {expected}")]
    Registration { expected: String, actual: String },
}

/// Validate and decode one compiler-produced machine description.
///
/// This does not evaluate source or admit payloads. It consumes only the
/// rechecked prompt-149 projection and the build-local primitive table.
///
/// # Errors
///
/// [`PrepareError`] identifies an invalid projection, conflicting build-local
/// registration, malformed stored value, or exceeded resource bound.
pub fn prepare_machine(spec: &MachineSpec) -> Result<PreparedMachine, PrepareError> {
    Registry::build(REGISTRATIONS)?.prepare(spec)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PrimitiveKind {
    Count,
    Scale,
    ScaleOffset,
    Mix,
    Reached,
    DelayNot,
}

#[derive(Clone, Copy)]
struct Registration {
    id: &'static str,
    version: u32,
    kind: PrimitiveKind,
    state_bytes: usize,
    step_work: usize,
}

const REGISTRATIONS: &[Registration] = &[
    Registration {
        id: "count",
        version: 1,
        kind: PrimitiveKind::Count,
        state_bytes: 8,
        step_work: 1,
    },
    Registration {
        id: "scale",
        version: 1,
        kind: PrimitiveKind::Scale,
        state_bytes: 0,
        step_work: 2,
    },
    Registration {
        id: "scale",
        version: 2,
        kind: PrimitiveKind::ScaleOffset,
        state_bytes: 0,
        step_work: 3,
    },
    Registration {
        id: "mix",
        version: 1,
        kind: PrimitiveKind::Mix,
        state_bytes: 0,
        step_work: 5,
    },
    Registration {
        id: "reached",
        version: 1,
        kind: PrimitiveKind::Reached,
        state_bytes: 0,
        step_work: 1,
    },
    Registration {
        id: "delay_not",
        version: 1,
        kind: PrimitiveKind::DelayNot,
        state_bytes: 0,
        step_work: 1,
    },
];

struct Registry {
    entries: &'static [Registration],
}

impl Registry {
    fn build(entries: &'static [Registration]) -> Result<Self, PrepareError> {
        let mut seen = HashSet::with_capacity(entries.len());
        for entry in entries {
            if !seen.insert((entry.id, entry.version)) {
                return Err(PrepareError::RegistryConflict {
                    id: entry.id.to_owned(),
                    version: entry.version,
                });
            }
        }
        Ok(Self { entries })
    }

    fn get(&self, id: &str, version: u32) -> Option<Registration> {
        self.entries
            .iter()
            .copied()
            .find(|entry| entry.id == id && entry.version == version)
    }

    fn prepare(&self, spec: &MachineSpec) -> Result<PreparedMachine, PrepareError> {
        let nodes = spec.nodes();
        if nodes.is_empty() {
            return Err(PrepareError::Empty);
        }
        if nodes.len() > MAX_NODES {
            return Err(PrepareError::NodeLimit {
                actual: nodes.len(),
                limit: MAX_NODES,
            });
        }

        let mut parents = vec![0usize; nodes.len()];
        let mut depths = vec![1usize; nodes.len()];
        for (index, node) in nodes.iter().enumerate() {
            let expected = arity(node.form());
            if node.children().len() != expected {
                return Err(PrepareError::ChildCount {
                    node: index,
                    form: node.form(),
                    actual: node.children().len(),
                    expected,
                });
            }
            for &child in node.children() {
                if child >= index {
                    return Err(PrepareError::ChildOrder { node: index, child });
                }
                let parent_count = parents
                    .get_mut(child)
                    .ok_or(PrepareError::ChildOrder { node: index, child })?;
                *parent_count = parent_count.saturating_add(1);
                let child_depth = depths
                    .get(child)
                    .copied()
                    .ok_or(PrepareError::ChildOrder { node: index, child })?;
                let depth = depths.get_mut(index).ok_or(PrepareError::NotTree { node: index })?;
                *depth = (*depth).max(child_depth.saturating_add(1));
            }
        }
        let root = nodes.len().checked_sub(1).ok_or(PrepareError::Empty)?;
        for (index, count) in parents.into_iter().enumerate() {
            let expected = usize::from(index != root);
            if count != expected {
                return Err(PrepareError::NotTree { node: index });
            }
        }
        let root_depth = depths.get(root).copied().ok_or(PrepareError::Empty)?;
        if root_depth > MAX_DEPTH {
            return Err(PrepareError::DepthLimit {
                actual: root_depth,
                limit: MAX_DEPTH,
            });
        }

        let mut unifier = Unifier::new(nodes.len().saturating_mul(2));
        let mut feedback = vec![None; nodes.len()];
        for (index, node) in nodes.iter().enumerate() {
            let input = input_var(index);
            let output = output_var(index);
            let children = node.children();
            match node.form() {
                SpecForm::Primitive => {
                    let id = node.id().ok_or_else(|| PrepareError::Unregistered {
                        id: "<missing>".to_owned(),
                        version: node.version(),
                    })?;
                    let registration = self.get(id, node.version()).ok_or_else(|| PrepareError::Unregistered {
                        id: id.to_owned(),
                        version: node.version(),
                    })?;
                    let descriptor = musa_score::machine::descriptor(id, node.version()).ok_or_else(|| {
                        PrepareError::DescriptorConflict {
                            id: id.to_owned(),
                            version: node.version(),
                        }
                    })?;
                    if !registration_matches(registration, descriptor) {
                        return Err(PrepareError::DescriptorConflict {
                            id: id.to_owned(),
                            version: node.version(),
                        });
                    }
                    unifier.unify(input, Expr::from_schema(&PortSchema::from(descriptor.input())))?;
                    unifier.unify(output, Expr::from_schema(&PortSchema::from(descriptor.output())))?;
                }
                SpecForm::Identity => unifier.unify(input, output)?,
                SpecForm::Connect => {
                    let (left, right) = two_children(index, node.form(), children)?;
                    unifier.unify(input, input_var(left))?;
                    unifier.unify(output_var(left), input_var(right))?;
                    unifier.unify(output, output_var(right))?;
                }
                SpecForm::Beside => {
                    let (left, right) = two_children(index, node.form(), children)?;
                    unifier.unify(input, Expr::pair(input_var(left), input_var(right)))?;
                    unifier.unify(output, Expr::pair(output_var(left), output_var(right)))?;
                }
                SpecForm::Feedback => {
                    let stored = unifier.fresh();
                    let child = one_child(index, node.form(), children)?;
                    unifier.unify(input_var(child), Expr::pair(input.clone(), stored.clone()))?;
                    unifier.unify(output_var(child), Expr::pair(output.clone(), stored.clone()))?;
                    let slot = feedback.get_mut(index).ok_or(PrepareError::NotTree { node: index })?;
                    *slot = Some(stored);
                }
                SpecForm::Copy => unifier.unify(output, Expr::pair(input.clone(), input))?,
                SpecForm::Drop => unifier.unify(output, Expr::Known(PortSchema::Unit))?,
                SpecForm::Swap => {
                    let first = unifier.fresh();
                    let second = unifier.fresh();
                    unifier.unify(input, Expr::pair(first.clone(), second.clone()))?;
                    unifier.unify(output, Expr::pair(second, first))?;
                }
            }
        }
        unifier.unify(input_var(root), Expr::from_schema(spec.input_schema()))?;
        unifier.unify(output_var(root), Expr::from_schema(spec.output_schema()))?;

        for index in 0..nodes.len() {
            unifier
                .schema(input_var(index))
                .ok_or(PrepareError::UndeterminedPort { node: index })?;
            unifier
                .schema(output_var(index))
                .ok_or(PrepareError::UndeterminedPort { node: index })?;
        }

        let mut prepared: Vec<Option<PreparedNode>> = (0..nodes.len()).map(|_| None).collect();
        let mut state_bytes = 0usize;
        let mut step_work = 0usize;
        for (index, node) in nodes.iter().enumerate() {
            let built = match node.form() {
                SpecForm::Primitive => {
                    let id = node.id().ok_or_else(|| PrepareError::Unregistered {
                        id: "<missing>".to_owned(),
                        version: node.version(),
                    })?;
                    let registration = self.get(id, node.version()).ok_or_else(|| PrepareError::Unregistered {
                        id: id.to_owned(),
                        version: node.version(),
                    })?;
                    let descriptor = musa_score::machine::descriptor(id, node.version()).ok_or_else(|| {
                        PrepareError::DescriptorConflict {
                            id: id.to_owned(),
                            version: node.version(),
                        }
                    })?;
                    let configuration_schema = PortSchema::from(descriptor.configuration());
                    let configuration =
                        decode(node.stored(), &configuration_schema).ok_or_else(|| PrepareError::StoredValue {
                            node: index,
                            expected: configuration_schema.spelling(),
                        })?;
                    state_bytes = state_bytes.saturating_add(registration.state_bytes);
                    step_work = step_work.saturating_add(registration.step_work);
                    PreparedNode::Primitive(PreparedPrimitive::decode(registration.kind, configuration).ok_or_else(
                        || PrepareError::DescriptorConflict {
                            id: id.to_owned(),
                            version: node.version(),
                        },
                    )?)
                }
                SpecForm::Identity => PreparedNode::Identity,
                SpecForm::Connect => {
                    let (left, right) = two_children(index, node.form(), node.children())?;
                    PreparedNode::Connect(
                        Box::new(take(&mut prepared, left)?),
                        Box::new(take(&mut prepared, right)?),
                    )
                }
                SpecForm::Beside => {
                    let (left, right) = two_children(index, node.form(), node.children())?;
                    PreparedNode::Beside(
                        Box::new(take(&mut prepared, left)?),
                        Box::new(take(&mut prepared, right)?),
                    )
                }
                SpecForm::Feedback => {
                    let stored_expression = feedback
                        .get(index)
                        .and_then(Clone::clone)
                        .ok_or(PrepareError::UndeterminedPort { node: index })?;
                    let stored_schema = unifier
                        .schema(stored_expression)
                        .ok_or(PrepareError::UndeterminedPort { node: index })?;
                    let initial = decode(node.stored(), &stored_schema).ok_or_else(|| PrepareError::StoredValue {
                        node: index,
                        expected: stored_schema.spelling(),
                    })?;
                    state_bytes = state_bytes.saturating_add(initial.stored_bytes());
                    let child = one_child(index, node.form(), node.children())?;
                    PreparedNode::Feedback(Box::new(take(&mut prepared, child)?), initial)
                }
                SpecForm::Copy => PreparedNode::Copy,
                SpecForm::Drop => PreparedNode::Drop,
                SpecForm::Swap => PreparedNode::Swap,
            };
            step_work = step_work.saturating_add(1);
            let slot = prepared.get_mut(index).ok_or(PrepareError::NotTree { node: index })?;
            *slot = Some(built);
        }
        if state_bytes > MAX_STATE_BYTES {
            return Err(PrepareError::StateLimit {
                actual: state_bytes,
                limit: MAX_STATE_BYTES,
            });
        }
        if step_work > MAX_STEP_WORK {
            return Err(PrepareError::WorkLimit {
                actual: step_work,
                limit: MAX_STEP_WORK,
            });
        }
        Ok(PreparedMachine {
            root: take(&mut prepared, root)?,
            input: spec.input_schema().clone(),
            output: spec.output_schema().clone(),
        })
    }
}

fn registration_matches(registration: Registration, descriptor: &musa_score::machine::PrimitiveDescriptor) -> bool {
    let declared = (
        PortSchema::from(descriptor.input()),
        PortSchema::from(descriptor.output()),
        PortSchema::from(descriptor.configuration()),
    );
    let expected = match registration.kind {
        PrimitiveKind::Count => (PortSchema::Unit, PortSchema::Nat, PortSchema::Nat),
        PrimitiveKind::Scale => (PortSchema::Ratio, PortSchema::Ratio, PortSchema::Ratio),
        PrimitiveKind::ScaleOffset => (
            PortSchema::Ratio,
            PortSchema::Ratio,
            PortSchema::Pair(Box::new(PortSchema::Ratio), Box::new(PortSchema::Ratio)),
        ),
        PrimitiveKind::Mix => (
            PortSchema::Pair(Box::new(PortSchema::Ratio), Box::new(PortSchema::Ratio)),
            PortSchema::Ratio,
            PortSchema::Pair(Box::new(PortSchema::Ratio), Box::new(PortSchema::Ratio)),
        ),
        PrimitiveKind::Reached => (PortSchema::Nat, PortSchema::Bool, PortSchema::Nat),
        PrimitiveKind::DelayNot => (
            PortSchema::Pair(Box::new(PortSchema::Unit), Box::new(PortSchema::Bool)),
            PortSchema::Pair(Box::new(PortSchema::Bool), Box::new(PortSchema::Bool)),
            PortSchema::Bool,
        ),
    };
    registration.id == descriptor.id()
        && registration.version == descriptor.version()
        && descriptor.step().spelling() == "AudioFrameStep"
        && declared == expected
}

fn arity(form: SpecForm) -> usize {
    match form {
        SpecForm::Primitive | SpecForm::Identity | SpecForm::Copy | SpecForm::Drop | SpecForm::Swap => 0,
        SpecForm::Feedback => 1,
        SpecForm::Connect | SpecForm::Beside => 2,
    }
}

fn input_var(node: usize) -> Expr {
    Expr::Var(node.saturating_mul(2))
}

fn output_var(node: usize) -> Expr {
    Expr::Var(node.saturating_mul(2).saturating_add(1))
}

fn one_child(node: usize, form: SpecForm, children: &[usize]) -> Result<usize, PrepareError> {
    children.first().copied().ok_or(PrepareError::ChildCount {
        node,
        form,
        actual: children.len(),
        expected: 1,
    })
}

fn two_children(node: usize, form: SpecForm, children: &[usize]) -> Result<(usize, usize), PrepareError> {
    children
        .first()
        .copied()
        .zip(children.get(1).copied())
        .ok_or(PrepareError::ChildCount {
            node,
            form,
            actual: children.len(),
            expected: 2,
        })
}

fn take(nodes: &mut [Option<PreparedNode>], index: usize) -> Result<PreparedNode, PrepareError> {
    nodes
        .get_mut(index)
        .and_then(Option::take)
        .ok_or(PrepareError::NotTree { node: index })
}

enum PreparedNode {
    Primitive(PreparedPrimitive),
    Identity,
    Connect(Box<Self>, Box<Self>),
    Beside(Box<Self>, Box<Self>),
    Feedback(Box<Self>, MachineValue),
    Copy,
    Drop,
    Swap,
}

impl PreparedNode {
    fn start(&self) -> RunningNode {
        match self {
            Self::Primitive(primitive) => RunningNode::Primitive(primitive.start()),
            Self::Identity => RunningNode::Identity,
            Self::Connect(left, right) => RunningNode::Connect(Box::new(left.start()), Box::new(right.start())),
            Self::Beside(left, right) => RunningNode::Beside(Box::new(left.start()), Box::new(right.start())),
            Self::Feedback(child, initial) => RunningNode::Feedback(Box::new(child.start()), initial.clone()),
            Self::Copy => RunningNode::Copy,
            Self::Drop => RunningNode::Drop,
            Self::Swap => RunningNode::Swap,
        }
    }
}

enum RunningNode {
    Primitive(RunningPrimitive),
    Identity,
    Connect(Box<Self>, Box<Self>),
    Beside(Box<Self>, Box<Self>),
    Feedback(Box<Self>, MachineValue),
    Copy,
    Drop,
    Swap,
}

impl RunningNode {
    fn step(&mut self, input: MachineValue) -> Result<MachineValue, StepError> {
        match self {
            Self::Primitive(primitive) => primitive.step(input),
            Self::Identity => Ok(input),
            Self::Connect(left, right) => {
                let middle = left.step(input)?;
                right.step(middle)
            }
            Self::Beside(left, right) => {
                let (first, second) = pair_or_registration(input)?;
                Ok(MachineValue::pair(left.step(first)?, right.step(second)?))
            }
            Self::Feedback(child, stored) => {
                let joined = MachineValue::pair(input, stored.clone());
                let (output, next) = pair_or_registration(child.step(joined)?)?;
                *stored = next;
                Ok(output)
            }
            Self::Copy => Ok(MachineValue::pair(input.clone(), input)),
            Self::Drop => Ok(MachineValue::Unit),
            Self::Swap => {
                let (first, second) = pair_or_registration(input)?;
                Ok(MachineValue::pair(second, first))
            }
        }
    }
}

fn pair_or_registration(value: MachineValue) -> Result<(MachineValue, MachineValue), StepError> {
    let actual = value.schema().spelling();
    value.into_pair().ok_or_else(|| StepError::Registration {
        expected: "a product".to_owned(),
        actual,
    })
}

enum PreparedPrimitive {
    Count(u64),
    Scale(BigRational),
    ScaleOffset(BigRational, BigRational),
    Mix(BigRational, BigRational),
    Reached(u64),
    DelayNot(bool),
}

impl PreparedPrimitive {
    fn decode(kind: PrimitiveKind, configuration: MachineValue) -> Option<Self> {
        match (kind, configuration) {
            (PrimitiveKind::Count, MachineValue::Nat(initial)) => Some(Self::Count(initial)),
            (PrimitiveKind::Scale, MachineValue::Ratio(factor)) => Some(Self::Scale(factor)),
            (PrimitiveKind::ScaleOffset, MachineValue::Pair(configuration)) => match *configuration {
                (MachineValue::Ratio(factor), MachineValue::Ratio(offset)) => Some(Self::ScaleOffset(factor, offset)),
                _ => None,
            },
            (PrimitiveKind::Mix, MachineValue::Pair(configuration)) => match *configuration {
                (MachineValue::Ratio(left), MachineValue::Ratio(right)) => Some(Self::Mix(left, right)),
                _ => None,
            },
            (PrimitiveKind::Reached, MachineValue::Nat(limit)) => Some(Self::Reached(limit)),
            (PrimitiveKind::DelayNot, MachineValue::Bool(negate)) => Some(Self::DelayNot(negate)),
            (
                PrimitiveKind::Count
                | PrimitiveKind::Scale
                | PrimitiveKind::ScaleOffset
                | PrimitiveKind::Mix
                | PrimitiveKind::Reached
                | PrimitiveKind::DelayNot,
                _,
            ) => None,
        }
    }

    fn start(&self) -> RunningPrimitive {
        match self {
            Self::Count(initial) => RunningPrimitive::Count(*initial),
            Self::Scale(factor) => RunningPrimitive::Scale(factor.clone()),
            Self::ScaleOffset(factor, offset) => RunningPrimitive::ScaleOffset(factor.clone(), offset.clone()),
            Self::Mix(left, right) => RunningPrimitive::Mix(left.clone(), right.clone()),
            Self::Reached(limit) => RunningPrimitive::Reached(*limit),
            Self::DelayNot(negate) => RunningPrimitive::DelayNot(*negate),
        }
    }
}

enum RunningPrimitive {
    Count(u64),
    Scale(BigRational),
    ScaleOffset(BigRational, BigRational),
    Mix(BigRational, BigRational),
    Reached(u64),
    DelayNot(bool),
}

impl RunningPrimitive {
    fn step(&mut self, input: MachineValue) -> Result<MachineValue, StepError> {
        match self {
            Self::Count(count) => match input {
                MachineValue::Unit => {
                    let output = *count;
                    *count = count.saturating_add(1);
                    Ok(MachineValue::Nat(output))
                }
                ref other @ (MachineValue::Bool(_)
                | MachineValue::Nat(_)
                | MachineValue::Ratio(_)
                | MachineValue::Pair(_)) => wrong_primitive_input("Unit", other),
            },
            Self::Scale(factor) => match input {
                MachineValue::Ratio(input) => Ok(MachineValue::Ratio(std::ops::Mul::mul(input, &*factor))),
                ref other @ (MachineValue::Unit
                | MachineValue::Bool(_)
                | MachineValue::Nat(_)
                | MachineValue::Pair(_)) => wrong_primitive_input("Ratio", other),
            },
            Self::ScaleOffset(factor, offset) => match input {
                MachineValue::Ratio(input) => Ok(MachineValue::Ratio(std::ops::Add::add(
                    std::ops::Mul::mul(input, &*factor),
                    &*offset,
                ))),
                ref other @ (MachineValue::Unit
                | MachineValue::Bool(_)
                | MachineValue::Nat(_)
                | MachineValue::Pair(_)) => wrong_primitive_input("Ratio", other),
            },
            Self::Mix(left_gain, right_gain) => {
                let (left, right) = pair_or_registration(input)?;
                match (left, right) {
                    (MachineValue::Ratio(left), MachineValue::Ratio(right)) => {
                        Ok(MachineValue::Ratio(std::ops::Add::add(
                            std::ops::Mul::mul(left, &*left_gain),
                            std::ops::Mul::mul(right, &*right_gain),
                        )))
                    }
                    (left, right) => wrong_primitive_input("(Ratio, Ratio)", &MachineValue::pair(left, right)),
                }
            }
            Self::Reached(limit) => match input {
                MachineValue::Nat(input) => Ok(MachineValue::Bool(input >= *limit)),
                ref other @ (MachineValue::Unit
                | MachineValue::Bool(_)
                | MachineValue::Ratio(_)
                | MachineValue::Pair(_)) => wrong_primitive_input("Nat", other),
            },
            Self::DelayNot(negate) => {
                let (unit, old) = pair_or_registration(input)?;
                match (unit, old) {
                    (MachineValue::Unit, MachineValue::Bool(old)) => {
                        let next = if *negate { !old } else { old };
                        Ok(MachineValue::pair(MachineValue::Bool(old), MachineValue::Bool(next)))
                    }
                    (unit, old) => wrong_primitive_input("(Unit, Bool)", &MachineValue::pair(unit, old)),
                }
            }
        }
    }
}

fn wrong_primitive_input(expected: &str, actual: &MachineValue) -> Result<MachineValue, StepError> {
    Err(StepError::Registration {
        expected: expected.to_owned(),
        actual: actual.schema().spelling(),
    })
}

#[derive(Clone, Debug)]
enum Expr {
    Var(usize),
    Known(PortSchema),
    Pair(Box<(Self, Self)>),
}

impl Expr {
    fn pair(first: Self, second: Self) -> Self {
        Self::Pair(Box::new((first, second)))
    }

    fn from_schema(schema: &PortSchema) -> Self {
        match schema {
            PortSchema::Pair(first, second) => Self::pair(Self::from_schema(first), Self::from_schema(second)),
            other @ (PortSchema::Unit | PortSchema::Bool | PortSchema::Nat | PortSchema::Ratio) => {
                Self::Known(other.clone())
            }
        }
    }
}

struct Unifier {
    solutions: Vec<Option<Expr>>,
}

impl Unifier {
    fn new(variables: usize) -> Self {
        Self {
            solutions: vec![None; variables],
        }
    }

    fn fresh(&mut self) -> Expr {
        let variable = self.solutions.len();
        self.solutions.push(None);
        Expr::Var(variable)
    }

    fn resolve(&self, expression: Expr) -> Expr {
        match expression {
            Expr::Var(variable) => self
                .solutions
                .get(variable)
                .and_then(Clone::clone)
                .map_or(Expr::Var(variable), |solution| self.resolve(solution)),
            Expr::Pair(pair) => Expr::pair(self.resolve(pair.0), self.resolve(pair.1)),
            known @ Expr::Known(_) => known,
        }
    }

    fn unify(&mut self, left: Expr, right: Expr) -> Result<(), PrepareError> {
        let left = self.resolve(left);
        let right = self.resolve(right);
        match (left, right) {
            (Expr::Var(left), Expr::Var(right)) if left == right => Ok(()),
            (Expr::Var(variable), expression) | (expression, Expr::Var(variable)) => {
                if occurs(variable, &expression, self) {
                    return Err(PrepareError::InfinitePort);
                }
                let slot = self.solutions.get_mut(variable).ok_or(PrepareError::InfinitePort)?;
                *slot = Some(expression);
                Ok(())
            }
            (Expr::Known(left), Expr::Known(right)) if left == right => Ok(()),
            (Expr::Pair(left), Expr::Pair(right)) => {
                self.unify(left.0, right.0)?;
                self.unify(left.1, right.1)
            }
            (left, right) => Err(PrepareError::PortMismatch {
                left: expression_name(&left, self),
                right: expression_name(&right, self),
            }),
        }
    }

    fn schema(&self, expression: Expr) -> Option<PortSchema> {
        match self.resolve(expression) {
            Expr::Var(_) => None,
            Expr::Known(schema) => Some(schema),
            Expr::Pair(pair) => Some(PortSchema::Pair(
                Box::new(self.schema(pair.0)?),
                Box::new(self.schema(pair.1)?),
            )),
        }
    }
}

fn occurs(variable: usize, expression: &Expr, unifier: &Unifier) -> bool {
    match unifier.resolve(expression.clone()) {
        Expr::Var(found) => found == variable,
        Expr::Pair(pair) => occurs(variable, &pair.0, unifier) || occurs(variable, &pair.1, unifier),
        Expr::Known(_) => false,
    }
}

fn expression_name(expression: &Expr, unifier: &Unifier) -> String {
    unifier
        .schema(expression.clone())
        .map_or_else(|| "an undetermined port".to_owned(), |schema| schema.spelling())
}

fn decode(bytes: &[u8], schema: &PortSchema) -> Option<MachineValue> {
    let mut cursor = bytes;
    let value = decode_one(&mut cursor, schema)?;
    cursor.is_empty().then_some(value)
}

fn decode_one(bytes: &mut &[u8], schema: &PortSchema) -> Option<MachineValue> {
    let (&tag, rest) = bytes.split_first()?;
    *bytes = rest;
    match (tag, schema) {
        (0, PortSchema::Ratio) => {
            let numerator = take_i64(bytes)?;
            let denominator = take_i64(bytes)?;
            (denominator != 0)
                .then(|| MachineValue::Ratio(BigRational::new(BigInt::from(numerator), BigInt::from(denominator))))
        }
        (1, PortSchema::Unit) => Some(MachineValue::Unit),
        (2, PortSchema::Bool) => {
            let (&value, rest) = bytes.split_first()?;
            *bytes = rest;
            match value {
                0 => Some(MachineValue::Bool(false)),
                1 => Some(MachineValue::Bool(true)),
                _ => None,
            }
        }
        (3, PortSchema::Nat) => Some(MachineValue::Nat(take_u64(bytes)?)),
        (4, PortSchema::Pair(first, second)) => Some(MachineValue::pair(
            decode_one(bytes, first)?,
            decode_one(bytes, second)?,
        )),
        _ => None,
    }
}

fn take_i64(bytes: &mut &[u8]) -> Option<i64> {
    let (value, rest) = bytes.split_at_checked(8)?;
    *bytes = rest;
    Some(i64::from_be_bytes(value.try_into().ok()?))
}

fn take_u64(bytes: &mut &[u8]) -> Option<u64> {
    let (value, rest) = bytes.split_at_checked(8)?;
    *bytes = rest;
    Some(u64::from_be_bytes(value.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::{PrepareError, PrimitiveKind, Registration, Registry};

    #[test]
    fn conflicting_runtime_registrations_are_refused() {
        const CONFLICT: &[Registration] = &[
            Registration {
                id: "same",
                version: 1,
                kind: PrimitiveKind::Scale,
                state_bytes: 0,
                step_work: 1,
            },
            Registration {
                id: "same",
                version: 1,
                kind: PrimitiveKind::Mix,
                state_bytes: 0,
                step_work: 1,
            },
        ];
        assert_eq!(
            Registry::build(CONFLICT).err(),
            Some(PrepareError::RegistryConflict {
                id: "same".to_owned(),
                version: 1
            })
        );
    }
}
