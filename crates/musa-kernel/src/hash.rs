//! The semantic hash (docs/rules/kernel/05 N6): a digest of a timeline's exact,
//! versioned, uniquely framed semantic bytes.
//!
//! The digest is **FNV-1a, 128-bit**, over exactly the private bytes N6
//! defines. Human N5 display is deliberately separate. The
//! algorithm is named in the documentation and fixed here because a hash that
//! varies between runs or between processes is not an identity: it would make
//! a cache stale on restart and a golden file unreproducible. Rust's
//! `DefaultHasher` is explicitly unsuitable — its output is not guaranteed
//! stable across releases, and `HashMap`'s is randomly seeded per process.
//!
//! FNV-1a is not cryptographic. A digest is only an index: correctness-sensitive
//! lookup confirms the complete framed arguments after finding candidates.

/// A stable digest of a timeline's canonical form (N6).
///
/// Equal canonical forms hash equal, always and everywhere. Unequal hashes
/// therefore prove the framed semantic bytes differ. Equal hashes select
/// candidates; they do not prove equality.
///
/// The hash covers the payload's canonical key, which for musa's score facts
/// includes provenance: re-indenting a file moves source spans and *does*
/// change the hash. That is correct — the hash answers "is this the same
/// compiled piece", not "does it sound the same" — and a sounds-the-same
/// digest, if one is ever wanted, is a second function over a provenance-free
/// projection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticHash(u128);

impl SemanticHash {
    /// The digest as an integer, for callers that key a map on it.
    #[must_use]
    pub fn to_u128(self) -> u128 {
        self.0
    }
}

impl std::fmt::Display for SemanticHash {
    /// Thirty-two lowercase hex digits, zero-padded — a stable spelling for
    /// logs, golden files, and cache keys.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

/// The workspace's one stable digest, over arbitrary bytes.
///
/// Same algorithm, same constants, same guarantee as [`SemanticHash`]: equal
/// input, equal output, in every process and on every platform. It is exposed
/// because a second consumer arrived — a realization derives each decision
/// site's randomness from a seed and a path (`docs/rules/kernel/11-realization.md`)
/// — and a second digest function would be a second answer to "are these the
/// same bytes".
///
/// It returns a plain `u128` rather than a [`SemanticHash`], because a
/// `SemanticHash` is a claim about a *timeline*: handing one out for a byte
/// string would let two unrelated identities be compared.
#[must_use]
pub fn stable_digest(bytes: &[u8]) -> u128 {
    let mut digest = Digest::new();
    digest.write(bytes);
    digest.state
}

/// The FNV-1a offset basis and prime for 128 bits, as published.
const OFFSET_BASIS: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

/// An FNV-1a accumulator, fed the framed semantic bytes as they are produced so
/// that no caller ever materializes a whole serialization to hash it.
pub(crate) struct Digest {
    state: u128,
}

impl Digest {
    pub(crate) fn new() -> Self {
        Self { state: OFFSET_BASIS }
    }

    /// Absorb bytes. Wrapping arithmetic is the algorithm, not an overflow
    /// hazard, so the workspace lint is answered explicitly here.
    pub(crate) fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= u128::from(*byte);
            self.state = self.state.wrapping_mul(PRIME);
        }
    }

    pub(crate) fn finish(self) -> SemanticHash {
        SemanticHash(self.state)
    }
}
