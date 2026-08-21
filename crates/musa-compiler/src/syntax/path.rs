//! Where a node sits: expansion coordinates, structural and derived
//! paths, the binding paths that carry hygiene, and their byte encoding.
//!
//! A path is derived, never invented — see the module docs for why.
//!
//! One concern of the `syntax` module; see its docs for the calculus.

/// Which adapter call a generated node belongs to.
///
/// A structural coordinate rather than an allocated id — the build node, the
/// file, the region's ordinal within it, and so on down through parent
/// expansions — so that two runs of one compiler over one input agree on it
/// exactly. The expansion phase derives it by a fixed traversal; here the
/// driver supplies a root.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ExpansionPath(Vec<u32>);

impl ExpansionPath {
    /// The expansion at `ordinals`, counted from the outermost coordinate in.
    pub(crate) fn at(ordinals: Vec<u32>) -> Self {
        Self(ordinals)
    }

    fn write_into(&self, out: &mut Vec<u8>) {
        push_len(out, self.0.len());
        for ordinal in &self.0 {
            out.extend_from_slice(&ordinal.to_be_bytes());
        }
    }
}

/// One step along a path.
///
/// The two constructors are disjoint by construction, and that disjointness is
/// what keeps a derived output path from ever colliding with the structural
/// path of an input node: reading descends by [`Self::Child`] only, and
/// building extends by [`Self::Built`] only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum PathStep {
    /// The nth child of a group, as the region was read.
    Child(u32),
    /// What a builder made at `role`, `child` steps along.
    Built { role: u32, child: u32 },
}

/// Where one node sits, as a path from an expansion's root.
///
/// Two nodes have the same path exactly when they were derived the same way,
/// which is what makes "every generated node has a unique path" a property a
/// transformer can violate and [`check_expression`] can report.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NodePath {
    pub(super) expansion: ExpansionPath,
    pub(super) steps: Vec<PathStep>,
}

impl NodePath {
    /// The root of one expansion — the only path not derived from another, and
    /// the compiler's to make.
    pub(crate) const fn root(expansion: ExpansionPath) -> Self {
        Self {
            expansion,
            steps: Vec::new(),
        }
    }

    /// The path of this node's `index`th child, as the region was read.
    ///
    /// Private, because descending is the fold's job: a transformer that could
    /// walk to a child itself could walk to one that is not there.
    pub(super) fn child(&self, index: u32) -> Self {
        let mut steps = self.steps.clone();
        steps.push(PathStep::Child(index));
        Self {
            expansion: self.expansion.clone(),
            steps,
        }
    }

    /// A path for something built here: at builder `role`, `child` steps along.
    ///
    /// This is the derivation blocker 1 asked for. A transformer holding one
    /// input path can address as much output as it needs — the role separates
    /// two builders that read the same input node, and the child number
    /// separates a run of siblings — and it can address nothing else.
    pub(crate) fn built(&self, role: u32, child: u32) -> Self {
        let mut steps = self.steps.clone();
        steps.push(PathStep::Built { role, child });
        Self {
            expansion: self.expansion.clone(),
            steps,
        }
    }

    /// The binding this path declares at builder `role`.
    ///
    /// Deriving a name's identity instead of allocating it is what lets an
    /// operation be both hygienic and a function of its arguments. Asking
    /// twice gives the same binding, which is exactly what a reference to a
    /// binder needs.
    pub(crate) fn binding(&self, role: u32) -> BindingPath {
        BindingPath(self.built(role, 0))
    }

    pub(crate) fn write_into(&self, out: &mut Vec<u8>) {
        self.expansion.write_into(out);
        push_len(out, self.steps.len());
        for step in &self.steps {
            match *step {
                PathStep::Child(index) => {
                    out.push(0);
                    out.extend_from_slice(&index.to_be_bytes());
                }
                PathStep::Built { role, child } => {
                    out.push(1);
                    out.extend_from_slice(&role.to_be_bytes());
                    out.extend_from_slice(&child.to_be_bytes());
                }
            }
        }
    }
}

/// A path as a diagnostic shows it: the expansion's ordinals, then one step per
/// derivation.
///
/// Written out rather than left to `Debug` because a path is the *only* thing a
/// diagnostic about a generated node has to point at — there is no line for it —
/// so two paths that differ must read differently at a glance. Reading steps and
/// building steps are spelled differently for the reason they are separate
/// constructors: a derived path can never be an input node's, and the text says
/// so.
impl std::fmt::Display for NodePath {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, ordinal) in self.expansion.0.iter().enumerate() {
            if index > 0 {
                out.write_str(":")?;
            }
            write!(out, "{ordinal}")?;
        }
        for step in &self.steps {
            match *step {
                PathStep::Child(index) => write!(out, "/{index}")?,
                PathStep::Built { role, child } => write!(out, "/+{role}.{child}")?,
            }
        }
        Ok(())
    }
}

/// Which name a binder declares and a reference means.
///
/// One binding path is one name: a binder and every reference derived from the
/// same path resolve together, and two paths are two names however they are
/// spelled. Nothing else about a [`BindingPath`] is observable, which is what
/// makes hygiene a property of the API rather than a convention a transformer
/// is asked to keep.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct BindingPath(pub(super) NodePath);

impl BindingPath {
    /// The scope this binding carries, which a binder and its references share.
    pub(super) fn scope(&self) -> Scope {
        Scope(self.0.clone())
    }
}

/// A binding path, shown as the path it derives from.
///
/// One binding is one name, and the path is the name, so there is nothing to add
/// to it.
impl std::fmt::Display for BindingPath {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(out)
    }
}

/// An opaque hygiene mark on an identifier.
///
/// A transformer may preserve the scopes it received and may compare two
/// identifiers, and has no operation that constructs one: a [`Scope`] enters a
/// value only by being copied from an input node or derived from a
/// [`BindingPath`]. That is the whole of what "package code cannot invent a
/// scope id" means here.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Scope(pub(super) NodePath);

impl Scope {
    /// This scope's exact bytes, which the printer interns to a mark.
    pub(super) fn key(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.0.write_into(&mut out);
        out
    }
}

/// How a node was derived, when it was derived rather than read.
///
/// `../rules/language/11-quotation.md` §3's triple, and the name for what
/// [`NodePath::built`] has computed all along: the `origin` is the path a
/// builder was pointed at, the `quotation` separates two construction sites
/// that read one input node, and the `path` is the position within what that
/// site built. It is not a third case of [`SourceInfo`] and not a second
/// derivation graph — a derived node is [`SourceInfo::Generated`], and this is
/// *how* its path is computed.
///
/// **The identity law**: two derived nodes are one node exactly when their
/// origin, quotation, and path are all equal, and nothing else makes two
/// equal. Uniqueness is then structural rather than a convention an author
/// keeps, which is what `check_expression`'s duplicate-path gate turns from a
/// check on an author into evidence about the compiler.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Derived {
    /// The node this was built from.
    pub(crate) origin: NodePath,
    /// Which construction site built it.
    pub(crate) quotation: u32,
    /// Where inside that site's own tree it sits.
    ///
    /// Never empty. A derivation names a place *inside* what a site built, and
    /// a site that built nothing named no place — an empty path would restate
    /// the origin, which is neither derived nor distinct from the input node
    /// and would put every site's outermost node at one address. So the
    /// outermost node a site builds is `[0]` rather than `[]`, and every
    /// builder here maintains that: `syntax_built(here, role, child)` names
    /// `[child]`, and [`instantiate`] starts a template's walk one step in.
    pub(crate) path: Vec<u32>,
}

impl Derived {
    /// The path this derivation names.
    ///
    /// Total, and injective in all three components *given a non-empty path*:
    /// the steps a [`NodePath`] holds are [`PathStep::Built`] only, which is
    /// disjoint from the [`PathStep::Child`] steps reading produces, so a
    /// derived path can never collide with the structural path of an input
    /// node either. See [`Self::path`](Derived::path) — the field — for why the
    /// empty path is not a place.
    pub(crate) fn path(&self) -> NodePath {
        let mut built = self.origin.clone();
        for step in &self.path {
            built = built.built(self.quotation, *step);
        }
        built
    }
}

/// The `quotation` component a construction site that runs at *evaluation*
/// carries: `u32::MAX`, a reservation and not an allocation.
///
/// Quote sites draw their indices upward from zero at lowering time, in both
/// checkers, so the top of the `u32` range is a site no quote can reach before
/// the counter itself gives out — disjointness by construction rather than by
/// a discipline anyone maintains. A δ rule needs this because it is a
/// function of its argument values alone (`02-core-calculus.md` §5.8's D3 is a
/// property of the `musa_calculus::Rule` type, not a promise): it has no counter
/// to draw from, and its answer's identity must therefore come from structure
/// the arguments already carry. `48-the-anchors-place-without-a-name-supply.md`
/// is the argument, from Peyton Jones ch. 9's name supply and Idris2's
/// `UST.nextName` living in the elaboration monad while `Normalise` mints
/// nothing.
pub(crate) const DELTA_QUOTATION: u32 = u32::MAX;

/// The place a δ-built node stands at: derived from the node the operation is
/// *about*, at the reserved site, one step in.
///
/// The node's own path is already a unique, elaboration-time identity — no two
/// nodes of a region share a reading-order path — so the place needs no fresh
/// name at all. Two calls about one node produce one place and meet
/// `check_expression`'s duplicate-path gate, which is the one-call-per-anchor
/// obligation `11-quotation.md` §5 keeps, stated on the anchored node rather
/// than on a call site.
pub(crate) fn anchor_place(anchor_of: &NodePath) -> NodePath {
    Derived {
        origin: anchor_of.clone(),
        quotation: DELTA_QUOTATION,
        path: vec![0],
    }
    .path()
}

pub(super) fn push_len(out: &mut Vec<u8>, len: usize) {
    out.extend_from_slice(&u64::try_from(len).unwrap_or(u64::MAX).to_be_bytes());
}
