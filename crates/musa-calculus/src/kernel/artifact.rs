//! Opaque checked source values for consumers below the compiler.
//!
//! [`crate::Datum`] is the canonical-data vocabulary used by δ-rules. This
//! module does not add to that vocabulary: it freezes one datum that was
//! obtained by checking and normalizing source, replaces host-owned literals
//! with their owner's exact bytes, and stores the result in a flat arena. A
//! consumer can inspect the value without seeing the evaluator's `Value`, a
//! closure, or the declaration context that produced it.

use std::ops::Range;
use std::sync::Arc;

use crate::{Datum, Literal, Name};

const ENCODING_VERSION: u64 = 1;

/// The source declaration schema a checked artifact is expected to inhabit.
///
/// The root type is checked against the inferred source type. `name` identifies
/// the consumer contract independently of a Rust type name; changing the
/// source fields or their meanings changes `version`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSchema {
    name: Name,
    root_type: Name,
    version: u64,
}

impl SourceSchema {
    /// Declare an exact source-value schema.
    #[must_use]
    pub fn new(name: impl Into<Name>, root_type: impl Into<Name>, version: u64) -> Self {
        Self {
            name: name.into(),
            root_type: root_type.into(),
            version,
        }
    }

    /// The stable schema name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The qualified source type the value must inhabit.
    #[must_use]
    pub fn root_type(&self) -> &str {
        &self.root_type
    }

    /// The source schema version.
    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }
}

/// One exact host-owned literal supplied while a checked source value is frozen.
///
/// The core cannot invent these bytes: the owner that registered the base type
/// supplies them. `type_name` distinguishes equal byte strings at different
/// base types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLiteral {
    type_name: Name,
    bytes: Arc<[u8]>,
}

impl SourceLiteral {
    /// Supply one exact literal encoding.
    #[must_use]
    pub fn new(type_name: impl Into<Name>, bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            type_name: type_name.into(),
            bytes: bytes.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum NodeKind {
    Literal(SourceLiteral),
    Count { family: Name, count: u64 },
    Case { constructor: Name },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Node {
    kind: NodeKind,
    children: Range<usize>,
}

/// A checked, normalized, exact source value.
///
/// There is deliberately no public constructor. [`crate::checked_source`]
/// creates one only after ordinary inference and normalization have succeeded.
/// The flat arena makes traversal and destruction independent of source-data
/// nesting depth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedSource {
    schema: SourceSchema,
    nodes: Arc<[Node]>,
    children: Arc<[usize]>,
    root: usize,
    exact: Arc<[u8]>,
}

impl CheckedSource {
    pub(crate) fn from_datum(
        schema: SourceSchema,
        datum: &Datum,
        encode: &impl Fn(&Literal) -> Option<SourceLiteral>,
    ) -> Result<Self, Literal> {
        let mut nodes = Vec::new();
        let mut children = Vec::new();
        let root = flatten(datum, encode, &mut nodes, &mut children)?;
        let exact = encoded(&schema, &nodes, &children, root);
        Ok(Self {
            schema,
            nodes: nodes.into(),
            children: children.into(),
            root,
            exact: exact.into(),
        })
    }

    /// The exact source schema this value was checked against.
    #[must_use]
    pub const fn schema(&self) -> &SourceSchema {
        &self.schema
    }

    /// The value's root datum.
    #[must_use]
    pub fn root(&self) -> SourceDatum<'_> {
        SourceDatum {
            source: self,
            node: self.root,
        }
    }

    /// The complete framed encoding used for exact equality and cache checks.
    #[must_use]
    pub fn exact_bytes(&self) -> &[u8] {
        &self.exact
    }

    /// Whether the arena and its complete exact framing still agree.
    ///
    /// Safe code can only obtain a valid artifact from [`crate::checked_source`].
    /// Consumers check this invariant at trust boundaries so a future storage
    /// decoder cannot accidentally turn malformed indices into a traversal.
    #[must_use]
    pub fn has_valid_framing(&self) -> bool {
        self.root < self.nodes.len()
            && self.nodes.iter().enumerate().all(|(index, node)| {
                node.children.start <= node.children.end
                    && node.children.end <= self.children.len()
                    && self
                        .children
                        .get(node.children.clone())
                        .is_some_and(|direct| direct.iter().all(|child| *child < index))
            })
            && self.exact.as_ref() == encoded(&self.schema, &self.nodes, &self.children, self.root)
    }
}

/// A borrowed node in a [`CheckedSource`].
#[derive(Clone, Copy, Debug)]
pub struct SourceDatum<'a> {
    source: &'a CheckedSource,
    node: usize,
}

impl<'a> SourceDatum<'a> {
    /// Which canonical-data form this node has.
    #[must_use]
    pub fn kind(self) -> Option<SourceDatumKind<'a>> {
        Some(match &self.source.nodes.get(self.node)?.kind {
            NodeKind::Literal(literal) => SourceDatumKind::Literal {
                type_name: &literal.type_name,
                bytes: &literal.bytes,
            },
            NodeKind::Count { family, count } => SourceDatumKind::Count { family, count: *count },
            NodeKind::Case { constructor } => SourceDatumKind::Case { constructor },
        })
    }

    /// Direct fields, in source declaration order.
    pub fn fields(self) -> Option<impl ExactSizeIterator<Item = Option<Self>> + 'a> {
        let range = self.source.nodes.get(self.node)?.children.clone();
        Some(self.source.children.get(range)?.iter().map(move |node| {
            self.source.nodes.get(*node).map(|_| Self {
                source: self.source,
                node: *node,
            })
        }))
    }

    /// The exact, self-framed encoding of this datum and its descendants.
    ///
    /// Unlike [`CheckedSource::exact_bytes`], this deliberately omits the
    /// artifact root schema. It is the representation-independent key for a
    /// checked source subvalue: constructor names, owner-supplied literal
    /// bytes, counting-family names, field order, and field boundaries all
    /// participate. Consumers use it when one member of a checked aggregate
    /// becomes an admitted opaque payload.
    #[must_use]
    pub fn exact_bytes(self) -> Vec<u8> {
        let mut bytes = b"musa-source-datum".to_vec();
        word(&mut bytes, ENCODING_VERSION);
        encode_datum(self, &mut bytes);
        bytes
    }
}

fn encode_datum(datum: SourceDatum<'_>, bytes: &mut Vec<u8>) {
    let mut work = vec![datum];
    while let Some(next) = work.pop() {
        match next.kind() {
            Some(SourceDatumKind::Literal { type_name, bytes: literal }) => {
                bytes.push(0);
                framed(bytes, type_name.as_bytes());
                framed(bytes, literal);
            }
            Some(SourceDatumKind::Count { family, count }) => {
                bytes.push(1);
                framed(bytes, family.as_bytes());
                word(bytes, count);
            }
            Some(SourceDatumKind::Case { constructor }) => {
                bytes.push(2);
                framed(bytes, constructor.as_bytes());
                let fields: Vec<_> = next.fields().into_iter().flatten().flatten().collect();
                word(bytes, u64::try_from(fields.len()).unwrap_or(u64::MAX));
                work.extend(fields.into_iter().rev());
            }
            None => bytes.push(u8::MAX),
        }
    }
}

/// The three canonical source-data forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceDatumKind<'a> {
    /// A registered base literal, in its owner's exact encoding.
    Literal {
        /// The registered base type.
        type_name: &'a str,
        /// Exact owner-supplied bytes.
        bytes: &'a [u8],
    },
    /// A counting family, stored without constructing a unary tower.
    Count {
        /// The counting family.
        family: &'a str,
        /// Distance above its floor.
        count: u64,
    },
    /// A saturated constructor application. Records use their one constructor.
    Case {
        /// The qualified constructor name.
        constructor: &'a str,
    },
}

fn flatten(
    datum: &Datum,
    encode: &impl Fn(&Literal) -> Option<SourceLiteral>,
    nodes: &mut Vec<Node>,
    children: &mut Vec<usize>,
) -> Result<usize, Literal> {
    enum Work<'a> {
        Visit(&'a Datum),
        Finish { kind: NodeKind, fields: usize },
    }

    let mut work = vec![Work::Visit(datum)];
    let mut completed = Vec::new();
    while let Some(next) = work.pop() {
        match next {
            Work::Visit(Datum::Lit(literal)) => {
                let kind = NodeKind::Literal(encode(literal).ok_or_else(|| literal.clone())?);
                let node = nodes.len();
                nodes.push(Node {
                    kind,
                    children: children.len()..children.len(),
                });
                completed.push(node);
            }
            Work::Visit(Datum::Count { family, count }) => {
                let node = nodes.len();
                nodes.push(Node {
                    kind: NodeKind::Count {
                        family: Arc::clone(family),
                        count: *count,
                    },
                    children: children.len()..children.len(),
                });
                completed.push(node);
            }
            Work::Visit(Datum::Case { constructor, fields }) => {
                work.push(Work::Finish {
                    kind: NodeKind::Case {
                        constructor: Arc::clone(constructor),
                    },
                    fields: fields.len(),
                });
                work.extend(fields.iter().rev().map(Work::Visit));
            }
            Work::Finish { kind, fields } => {
                let first = completed.len().saturating_sub(fields);
                let start = children.len();
                children.extend_from_slice(completed.get(first..).unwrap_or_default());
                completed.truncate(first);
                let node = nodes.len();
                nodes.push(Node {
                    kind,
                    children: start..children.len(),
                });
                completed.push(node);
            }
        }
    }
    Ok(completed.into_iter().next().unwrap_or_default())
}

fn encoded(schema: &SourceSchema, nodes: &[Node], children: &[usize], root: usize) -> Vec<u8> {
    let mut bytes = b"musa-checked-source".to_vec();
    word(&mut bytes, ENCODING_VERSION);
    framed(&mut bytes, schema.name.as_bytes());
    framed(&mut bytes, schema.root_type.as_bytes());
    word(&mut bytes, schema.version);
    word(&mut bytes, u64::try_from(nodes.len()).unwrap_or(u64::MAX));
    for node in nodes {
        match &node.kind {
            NodeKind::Literal(literal) => {
                bytes.push(0);
                framed(&mut bytes, literal.type_name.as_bytes());
                framed(&mut bytes, &literal.bytes);
            }
            NodeKind::Count { family, count } => {
                bytes.push(1);
                framed(&mut bytes, family.as_bytes());
                word(&mut bytes, *count);
            }
            NodeKind::Case { constructor } => {
                bytes.push(2);
                framed(&mut bytes, constructor.as_bytes());
                word(&mut bytes, u64::try_from(node.children.len()).unwrap_or(u64::MAX));
                for child in children.get(node.children.clone()).unwrap_or_default() {
                    word(&mut bytes, u64::try_from(*child).unwrap_or(u64::MAX));
                }
            }
        }
    }
    word(&mut bytes, u64::try_from(root).unwrap_or(u64::MAX));
    bytes
}

fn framed(bytes: &mut Vec<u8>, value: &[u8]) {
    word(bytes, u64::try_from(value.len()).unwrap_or(u64::MAX));
    bytes.extend_from_slice(value);
}

fn word(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

#[cfg(test)]
mod laws {
    use super::{CheckedSource, Node, NodeKind, SourceSchema};

    #[test]
    fn malformed_framing_and_forward_edges_are_refused() {
        let schema = SourceSchema::new("test.unit", "Unit", 1);
        let node = Node {
            kind: NodeKind::Case {
                constructor: "Unit.Unit".into(),
            },
            children: 0..0,
        };
        let mut source = CheckedSource {
            schema,
            nodes: vec![node].into(),
            children: Vec::new().into(),
            root: 0,
            exact: Vec::new().into(),
        };
        source.exact = super::encoded(&source.schema, &source.nodes, &source.children, source.root).into();
        assert!(source.has_valid_framing());

        source.exact = b"unframed".to_vec().into();
        assert!(!source.has_valid_framing());

        source.children = vec![0].into();
        source.nodes = vec![Node {
            kind: NodeKind::Case {
                constructor: "Unit.Unit".into(),
            },
            children: 0..1,
        }]
        .into();
        source.exact = super::encoded(&source.schema, &source.nodes, &source.children, source.root).into();
        assert!(!source.has_valid_framing());
    }
}
