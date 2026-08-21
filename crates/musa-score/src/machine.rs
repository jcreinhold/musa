//! The build-local primitive registry and the exact machine projection
//! (`docs/rules/across-stages/03-machine-calculus.md` §§1–2).
//!
//! A machine is a *description* of a stepping process, never the history it
//! produces. This module owns the two things about that description which are
//! not the evaluator's business: which primitive units this build knows about,
//! and how a finished machine is handed to the crate that will one day run it.
//!
//! What runs a machine is not here and is not anywhere yet. §1's `State(p)`,
//! `start_p`, and `step_p` belong to a primitive's owner, and prompt 150 is
//! where they arrive. A descriptor states what a unit *is* — its name, its
//! version, the step it counts in, its ports, and the shape of its
//! configuration — which is exactly what the type checker needs and no more.

/// One kind of machine step — `docs/rules/language/02-core-calculus.md` §1's
/// `K`.
///
/// A tag says what *one step means*, which is what stops a host block size
/// from becoming the unit of meaning: two machines whose steps count different
/// things do not connect, and the type checker refuses the wiring rather than
/// letting a resampling appear at run time.
///
/// The governing grammar names exactly one tag. The second exists only under
/// `cfg(test)`, because a discipline with one inhabitant cannot be shown to
/// refuse anything: without a second tag, every "unlike steps do not connect"
/// test would pass vacuously.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StepTag {
    /// One step is one sample frame at the prepared rate
    /// (`docs/rules/constitution.md` §4).
    AudioFrameStep,
    /// A step that counts nothing in particular, for the law suite.
    #[cfg(test)]
    TestStep,
}

impl StepTag {
    /// The tag this type name spells, if it spells one.
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "AudioFrameStep" => Some(Self::AudioFrameStep),
            #[cfg(test)]
            "TestStep" => Some(Self::TestStep),
            _ => None,
        }
    }

    /// Whether two tags count the same thing.
    ///
    /// Written out rather than derived because the registry's conflict check
    /// runs in a `const` context, where `PartialEq` is not available.
    const fn same(self, other: Self) -> bool {
        match (self, other) {
            (Self::AudioFrameStep, Self::AudioFrameStep) => true,
            #[cfg(test)]
            (Self::TestStep, Self::TestStep) => true,
            #[cfg(test)]
            (Self::AudioFrameStep | Self::TestStep, _) => false,
        }
    }

    /// How this tag is written, in source and in a projection.
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::AudioFrameStep => "AudioFrameStep",
            #[cfg(test)]
            Self::TestStep => "TestStep",
        }
    }
}

impl std::fmt::Display for StepTag {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.spelling())
    }
}

/// A port or configuration type, as a registry entry declares it.
///
/// There is deliberately no arrow constructor, so §1.1's rule that a machine's
/// ports, its feedback value, and a registered primitive's configuration are
/// storable data holds of every entry by construction rather than by
/// inspection. A unit that wanted a callback could not be spelled here at all.
///
/// It spans what the registered units declare and no more. A shape nothing
/// declares would be a vocabulary this build cannot check anything against, so
/// the enum grows with the units that need it rather than ahead of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortShape {
    Unit,
    Bool,
    Nat,
    Ratio,
    Product(&'static [Self]),
}

impl PortShape {
    /// Whether a value of this shape is one this language can write.
    ///
    /// `Unit` is not, and deliberately so: no source expression produces one
    /// (`core.rs`'s `unit_is_the_one_offered_type_no_written_expression_produces`,
    /// argued in `docs/notes/research/core-calculus/19-unit-has-no-surface-value.md`).
    /// A *port* typed `Unit` says nothing flows there, which is something a
    /// machine can mean and `count` and `drop` do mean. A *configuration*
    /// typed `Unit` would say nothing is written there, and nothing written is
    /// not a thing a composer can write.
    ///
    /// A product is writable at two members or more, because `(e)` is a
    /// parenthesized expression and `()` is not an expression at all: the
    /// surface's product literal starts at the comma.
    const fn is_writable(self) -> bool {
        match self {
            Self::Unit => false,
            Self::Bool | Self::Nat | Self::Ratio => true,
            Self::Product(members) => match members {
                [] | [_] => false,
                _ => all_writable(members),
            },
        }
    }

    /// Whether two declared shapes are the same shape.
    ///
    /// Written out rather than derived because the conflict check below runs
    /// in a `const` context, where `PartialEq` is not available.
    const fn same(self, other: Self) -> bool {
        match (self, other) {
            (Self::Unit, Self::Unit)
            | (Self::Bool, Self::Bool)
            | (Self::Nat, Self::Nat)
            | (Self::Ratio, Self::Ratio) => true,
            (Self::Product(ours), Self::Product(theirs)) => all_same(ours, theirs),
            _ => false,
        }
    }
}

/// Whether every shape in a list is one this language can write.
const fn all_writable(members: &[PortShape]) -> bool {
    match members {
        [] => true,
        [first, rest @ ..] => first.is_writable() && all_writable(rest),
    }
}

/// Whether two declared shape lists are the same list, position by position.
const fn all_same(ours: &[PortShape], theirs: &[PortShape]) -> bool {
    match (ours, theirs) {
        ([], []) => true,
        ([ours, our_rest @ ..], [theirs, their_rest @ ..]) => ours.same(*theirs) && all_same(our_rest, their_rest),
        _ => false,
    }
}

/// One registered stepping unit, as this build knows it.
///
/// §1's registry rule: a pair `(name, version)` selects exactly one state
/// layout, configuration codec, start function, step function, and resource
/// contract. What a descriptor holds is the part of that the *compiler* needs
/// — the ports it must type and the configuration it must check — and the
/// pair is what will select the rest when prompt 150 supplies it.
///
/// This is a build-local execution rule, not a promise of persistent compiled
/// identity: the same name at the same version is one unit within one build,
/// and says nothing about another build's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimitiveDescriptor {
    id: &'static str,
    version: u32,
    step: StepTag,
    input: PortShape,
    output: PortShape,
    configuration: PortShape,
}

impl PrimitiveDescriptor {
    /// The registered name, which with [`Self::version`] selects this entry.
    pub const fn id(&self) -> &'static str {
        self.id
    }

    pub const fn version(&self) -> u32 {
        self.version
    }

    pub const fn step(&self) -> StepTag {
        self.step
    }

    pub const fn input(&self) -> PortShape {
        self.input
    }

    pub const fn output(&self) -> PortShape {
        self.output
    }

    pub const fn configuration(&self) -> PortShape {
        self.configuration
    }
}

/// The small deterministic reference family this build registers.
///
/// A descriptor says what a unit *is*, which is what the compiler needs; §1's
/// `State(p)`, `start_p`, and `step_p` are the unit owner's and arrive with
/// prompt 150's runtime. That split is why a name is registered here before
/// anything can step it: the ports a machine is wired at are the compiler's to
/// decide, and what one step does is not.
///
/// The family is deliberately small and deliberately plain. Between them these
/// five declare every shape the port vocabulary spells, a unit whose input is a
/// pair, a unit whose ports differ, and one name at two versions differing in
/// configuration — which is what makes §1's registry rule refutable rather than
/// merely stated. Every configuration is a value this language can write, since
/// a unit whose configuration could not be spelled could not be instantiated.
const COUNT: PrimitiveDescriptor = PrimitiveDescriptor {
    id: "count",
    version: 1,
    step: StepTag::AudioFrameStep,
    input: PortShape::Unit,
    output: PortShape::Nat,
    configuration: PortShape::Nat,
};

const SCALE_BY_ONE: PrimitiveDescriptor = PrimitiveDescriptor {
    id: "scale",
    version: 1,
    step: StepTag::AudioFrameStep,
    input: PortShape::Ratio,
    output: PortShape::Ratio,
    configuration: PortShape::Ratio,
};

/// The same name at a second version: a version is part of which unit this is,
/// so the two may differ in configuration and both be registered.
const SCALE_AND_OFFSET: PrimitiveDescriptor = PrimitiveDescriptor {
    id: "scale",
    version: 2,
    step: StepTag::AudioFrameStep,
    input: PortShape::Ratio,
    output: PortShape::Ratio,
    configuration: PortShape::Product(&[PortShape::Ratio, PortShape::Ratio]),
};

/// §2's own example of what belongs in the registry rather than in the wiring:
/// "a mixer is a primitive from a pair of frames to one frame, never an implied
/// meaning of parallel placement".
const MIX: PrimitiveDescriptor = PrimitiveDescriptor {
    id: "mix",
    version: 1,
    step: StepTag::AudioFrameStep,
    input: PortShape::Product(&[PortShape::Ratio, PortShape::Ratio]),
    output: PortShape::Ratio,
    configuration: PortShape::Product(&[PortShape::Ratio, PortShape::Ratio]),
};

/// A unit whose two ports are different types, so that a chain has something to
/// get wrong.
const REACHED: PrimitiveDescriptor = PrimitiveDescriptor {
    id: "reached",
    version: 1,
    step: StepTag::AudioFrameStep,
    input: PortShape::Nat,
    output: PortShape::Bool,
    configuration: PortShape::Nat,
};

/// A unit whose step counts something else, registered by the law suite and by
/// no real build.
///
/// A discipline with one step tag cannot be shown to refuse anything: without
/// a second tag, every "unlike steps do not connect" test would pass vacuously.
#[cfg(test)]
const OTHER_STEP: PrimitiveDescriptor = PrimitiveDescriptor {
    id: "other_step",
    version: 1,
    step: StepTag::TestStep,
    input: PortShape::Ratio,
    output: PortShape::Ratio,
    configuration: PortShape::Ratio,
};

/// Every primitive this build registers.
#[cfg(not(test))]
const REGISTERED: &[PrimitiveDescriptor] = &[COUNT, SCALE_BY_ONE, SCALE_AND_OFFSET, MIX, REACHED];

/// Every primitive the law suite's build registers.
#[cfg(test)]
const REGISTERED: &[PrimitiveDescriptor] = &[COUNT, SCALE_BY_ONE, SCALE_AND_OFFSET, MIX, REACHED, OTHER_STEP];

/// Registration rejects a conflict, at build time.
///
/// §1 says a pair `(name, version)` selects *exactly one* of everything. Two
/// entries sharing a pair and disagreeing about ports or configuration would
/// make "the unit called `gain` version 2" an ambiguous phrase, and every
/// machine mentioning it ambiguous with it. Checking it here means the
/// contradiction is a build error rather than a lookup that silently takes the
/// first entry.
const _: () = assert!(
    registry_is_consistent(REGISTERED),
    "one primitive id and version must select one step tag, one port pair, and one configuration shape"
);

/// Registration rejects an unwritable configuration, at build time.
///
/// The paragraph above `COUNT` says every configuration is a value this
/// language can write, because a unit whose configuration could not be spelled
/// could not be instantiated. That was a promise; this makes it a rule. A port
/// may be `Unit` — `count` consumes nothing and `drop` produces nothing — and a
/// configuration may not, because a configuration is written at the call site
/// and `Unit` has no written form.
const _: () = assert!(
    every_configuration_is_writable(REGISTERED),
    "a registered unit's configuration is written where the unit is instantiated, so it must be a shape this \
     language can write; `Unit` and a product under two members are not"
);

/// Whether every entry declares a configuration a composer could write.
const fn every_configuration_is_writable(entries: &[PrimitiveDescriptor]) -> bool {
    match entries {
        [] => true,
        [first, rest @ ..] => first.configuration.is_writable() && every_configuration_is_writable(rest),
    }
}

/// Whether no two entries share an id and version while disagreeing.
const fn registry_is_consistent(entries: &[PrimitiveDescriptor]) -> bool {
    match entries {
        [] => true,
        [first, rest @ ..] => agrees_with_all(first, rest) && registry_is_consistent(rest),
    }
}

/// Whether `entry` contradicts no later entry sharing its id and version.
const fn agrees_with_all(entry: &PrimitiveDescriptor, rest: &[PrimitiveDescriptor]) -> bool {
    match rest {
        [] => true,
        [next, later @ ..] => {
            let same_unit = entry.version == next.version && equal_ids(entry.id, next.id);
            let agrees = !same_unit
                || (entry.step.same(next.step)
                    && entry.input.same(next.input)
                    && entry.output.same(next.output)
                    && entry.configuration.same(next.configuration));
            agrees && agrees_with_all(entry, later)
        }
    }
}

/// Whether two registered names are the same name.
const fn equal_ids(ours: &str, theirs: &str) -> bool {
    equal_bytes(ours.as_bytes(), theirs.as_bytes())
}

/// Whether two byte strings are the same string, taken a byte at a time
/// because `str` comparison is not available in a `const` context.
const fn equal_bytes(ours: &[u8], theirs: &[u8]) -> bool {
    match (ours, theirs) {
        ([], []) => true,
        ([ours, our_rest @ ..], [theirs, their_rest @ ..]) => *ours == *theirs && equal_bytes(our_rest, their_rest),
        _ => false,
    }
}

/// The unit this id and version select, if this build registers one.
pub fn descriptor(id: &str, version: u32) -> Option<&'static PrimitiveDescriptor> {
    REGISTERED
        .iter()
        .find(|entry| entry.id == id && entry.version == version)
}

/// Every version this build registers under `id`, in registration order.
///
/// Read only to say what a misspelled or mis-versioned name *could* have
/// meant, which is the difference between a diagnostic that helps and one that
/// only refuses.
pub fn versions_of(id: &str) -> Vec<u32> {
    REGISTERED
        .iter()
        .filter(|entry| entry.id == id)
        .map(|entry| entry.version)
        .collect()
}

/// Every registered name, without repetition, in registration order.
pub fn registered_ids() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    for entry in REGISTERED {
        if !names.contains(&entry.id) {
            names.push(entry.id);
        }
    }
    names
}

/// Which of §2's forms a projected node is.
///
/// A tag, not a tree: the shape is in [`MachineSpec`], and a consumer reads it
/// through that. This exists so a consumer can dispatch on what a node *is*
/// without the compiler publishing the value it projected from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecForm {
    /// A registered primitive instance — §2's `machine(p)`.
    Primitive,
    Identity,
    Connect,
    Beside,
    Feedback,
    Copy,
    Drop,
    Swap,
}

/// One node of a projected machine.
///
/// Immutable and accessor-only. Its children are indices into the owning
/// [`MachineSpec`]'s node list, so a projection is a flat array a consumer may
/// walk without recursion and may not rewire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecNode {
    form: SpecForm,
    children: Vec<usize>,
    id: Option<&'static str>,
    version: u32,
    configuration: Vec<u8>,
}

impl SpecNode {
    pub fn wiring(form: SpecForm, children: Vec<usize>) -> Self {
        Self {
            form,
            children,
            id: None,
            version: 0,
            configuration: Vec::new(),
        }
    }

    pub fn primitive(descriptor: &'static PrimitiveDescriptor, configuration: Vec<u8>) -> Self {
        Self {
            form: SpecForm::Primitive,
            children: Vec::new(),
            id: Some(descriptor.id),
            version: descriptor.version,
            configuration,
        }
    }

    pub fn initialized(form: SpecForm, children: Vec<usize>, initial: Vec<u8>) -> Self {
        Self {
            form,
            children,
            id: None,
            version: 0,
            configuration: initial,
        }
    }

    /// Which of §2's forms this node is.
    #[must_use]
    pub const fn form(&self) -> SpecForm {
        self.form
    }

    /// This node's children, as indices into the owning spec's nodes.
    #[must_use]
    pub fn children(&self) -> &[usize] {
        &self.children
    }

    /// The registered name this node instantiates, for a primitive node.
    #[must_use]
    pub const fn id(&self) -> Option<&'static str> {
        self.id
    }

    /// The registered version this node instantiates, for a primitive node.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// The exact bytes of this node's stored value: a primitive's
    /// configuration, or a feedback node's initial value. Empty for wiring.
    #[must_use]
    pub fn stored(&self) -> &[u8] {
        &self.configuration
    }
}

/// A finished machine, as the crate that will prepare it receives one.
///
/// This is the *only* way a machine leaves the compiler. The evaluator's value
/// stays private (`docs/rules/language/02-core-calculus.md` §6), because a
/// consumer that could see it could also see the source types, environments,
/// and provenance that built it — none of which is part of what a machine
/// means.
///
/// It is immutable, flat, and exact. Exact because the whole point of a
/// description is that two equal descriptions are one machine: [`digest`]
/// covers the version, the step tag, the port types, and every node's stored
/// bytes, so a cache keyed on it cannot confuse two units that differ only in
/// a configuration value.
///
/// Its named consumer is `musa-audio`, at prompt 152's `prepare_audio`.
///
/// [`digest`]: MachineSpec::digest
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MachineSpec {
    step: &'static str,
    input: String,
    output: String,
    nodes: Vec<SpecNode>,
    digest: u128,
}

/// The version of the projection's exact encoding.
///
/// A consumer that cannot reproduce this number cannot reproduce the digest,
/// and says so rather than guessing.
pub const MACHINE_SPEC_VERSION: u32 = 1;

impl MachineSpec {
    /// Assemble a projection and stamp its digest.
    ///
    /// The root is the last node, which is what building bottom-up gives and
    /// what a consumer walking children needs.
    pub fn new(step: StepTag, input: String, output: String, nodes: Vec<SpecNode>) -> Self {
        let digest = digest_of(step, &input, &output, &nodes);
        Self {
            step: step.spelling(),
            input,
            output,
            nodes,
            digest,
        }
    }

    /// What one step of this machine counts.
    #[must_use]
    pub const fn step(&self) -> &'static str {
        self.step
    }

    /// The machine's input port type, as source writes it.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }

    /// The machine's output port type, as source writes it.
    #[must_use]
    pub fn output(&self) -> &str {
        &self.output
    }

    /// Every node, in an order where a node's children precede it.
    #[must_use]
    pub fn nodes(&self) -> &[SpecNode] {
        &self.nodes
    }

    /// The index of the root node — the machine itself.
    ///
    /// `None` only for an empty projection, which no construction produces.
    #[must_use]
    pub fn root(&self) -> Option<usize> {
        self.nodes.len().checked_sub(1)
    }

    /// This machine's exact identity, as a stable digest.
    ///
    /// Equal descriptions digest equal, in every process and on every
    /// platform. Unequal digests prove the descriptions differ; equal digests
    /// select candidates, and a correctness-sensitive lookup confirms the
    /// nodes. It is **not** behavioural equality: two machines that sound
    /// alike may digest differently, and nothing here decides otherwise
    /// (`docs/rules/across-stages/03-machine-calculus.md` §7).
    #[must_use]
    pub const fn digest(&self) -> u128 {
        self.digest
    }
}

/// The framed bytes a projection's identity is taken over.
///
/// Every variable-length part is preceded by its length, so no two different
/// projections can produce one byte string by running together.
pub(crate) fn digest_of(step: StepTag, input: &str, output: &str, nodes: &[SpecNode]) -> u128 {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"musa-machine");
    bytes.extend_from_slice(&MACHINE_SPEC_VERSION.to_be_bytes());
    framed(&mut bytes, step.spelling().as_bytes());
    framed(&mut bytes, input.as_bytes());
    framed(&mut bytes, output.as_bytes());
    bytes.extend_from_slice(&(nodes.len() as u64).to_be_bytes());
    for node in nodes {
        bytes.push(match node.form {
            SpecForm::Primitive => 0,
            SpecForm::Identity => 1,
            SpecForm::Connect => 2,
            SpecForm::Beside => 3,
            SpecForm::Feedback => 4,
            SpecForm::Copy => 5,
            SpecForm::Drop => 6,
            SpecForm::Swap => 7,
        });
        framed(&mut bytes, node.id.unwrap_or("").as_bytes());
        bytes.extend_from_slice(&node.version.to_be_bytes());
        bytes.extend_from_slice(&(node.children.len() as u64).to_be_bytes());
        for child in &node.children {
            bytes.extend_from_slice(&(*child as u64).to_be_bytes());
        }
        framed(&mut bytes, &node.configuration);
    }
    musa_kernel::stable_digest(&bytes)
}

/// Write `part` preceded by its length.
pub fn framed(bytes: &mut Vec<u8>, part: &[u8]) {
    bytes.extend_from_slice(&(part.len() as u64).to_be_bytes());
    bytes.extend_from_slice(part);
}

#[cfg(test)]
// A law suite reports a violated law by failing, which is what `expect` is for here; the crate's
// other law suites carry the same allowance for the same reason.
#[allow(clippy::expect_used)]
mod tests {
    use super::{MachineSpec, PortShape, SpecForm, SpecNode, StepTag, descriptor, registered_ids, versions_of};

    #[test]
    fn one_id_and_version_select_one_unit() {
        let first = descriptor("scale", 1).expect("registered");
        let second = descriptor("scale", 2).expect("registered");
        assert_eq!(first.configuration(), PortShape::Ratio);
        assert_eq!(
            second.configuration(),
            PortShape::Product(&[PortShape::Ratio, PortShape::Ratio]),
            "a version is part of which unit this is, so two versions may configure differently"
        );
        assert_eq!(versions_of("scale"), vec![1, 2]);
        assert!(descriptor("scale", 3).is_none());
        assert!(descriptor("no_such_unit", 1).is_none());
        assert!(registered_ids().contains(&"count"));
        assert_eq!(
            registered_ids().iter().filter(|name| **name == "scale").count(),
            1,
            "one name, however many versions it has"
        );
    }

    #[test]
    fn a_projection_digests_its_configuration_and_not_only_its_shape() {
        let unit = descriptor("scale", 1).expect("registered");
        let one = MachineSpec::new(
            StepTag::AudioFrameStep,
            "Ratio".to_owned(),
            "Ratio".to_owned(),
            vec![SpecNode::primitive(unit, vec![2])],
        );
        let other = MachineSpec::new(
            StepTag::AudioFrameStep,
            "Ratio".to_owned(),
            "Ratio".to_owned(),
            vec![SpecNode::primitive(unit, vec![3])],
        );
        assert_ne!(
            one.digest(),
            other.digest(),
            "a configuration is part of which machine this is"
        );
        let again = MachineSpec::new(
            StepTag::AudioFrameStep,
            "Ratio".to_owned(),
            "Ratio".to_owned(),
            vec![SpecNode::primitive(unit, vec![2])],
        );
        assert_eq!(
            one.digest(),
            again.digest(),
            "the digest is a function of the description and nothing else"
        );
        assert_eq!(one.root(), Some(0));
        assert_eq!(one.nodes().first().map(SpecNode::form), Some(SpecForm::Primitive));
    }

    #[test]
    fn the_step_tag_is_part_of_exact_identity() {
        let nodes = vec![SpecNode::wiring(SpecForm::Identity, Vec::new())];
        let audio = MachineSpec::new(
            StepTag::AudioFrameStep,
            "Nat".to_owned(),
            "Nat".to_owned(),
            nodes.clone(),
        );
        let other = MachineSpec::new(StepTag::TestStep, "Nat".to_owned(), "Nat".to_owned(), nodes);
        assert_ne!(
            audio.digest(),
            other.digest(),
            "two machines whose steps count different things are two machines"
        );
    }

    #[test]
    fn the_ports_are_part_of_exact_identity() {
        let nodes = vec![SpecNode::wiring(SpecForm::Identity, Vec::new())];
        let over_nats = MachineSpec::new(
            StepTag::AudioFrameStep,
            "Nat".to_owned(),
            "Nat".to_owned(),
            nodes.clone(),
        );
        let over_ratios = MachineSpec::new(StepTag::AudioFrameStep, "Ratio".to_owned(), "Ratio".to_owned(), nodes);
        assert_ne!(over_nats.digest(), over_ratios.digest());
    }
}
