//! The semantic hash (docs/kernel/05 N6): a digest of a timeline's canonical
//! serialization, so "did the meaning change" is one comparison instead of a
//! walk.
//!
//! The digest is **FNV-1a, 128-bit**, over exactly the bytes N5 defines. The
//! algorithm is named in the documentation and fixed here because a hash that
//! varies between runs or between processes is not an identity: it would make
//! a cache stale on restart and a golden file unreproducible. Rust's
//! `DefaultHasher` is explicitly unsuitable — its output is not guaranteed
//! stable across releases, and `HashMap`'s is randomly seeded per process.
//!
//! FNV-1a is not cryptographic. Nothing here defends against an adversary
//! constructing a collision; the claim is the weaker, sufficient one that two
//! different pieces a person writes will not collide (128 bits, so a corpus
//! would need ~2⁶⁴ distinct pieces before a chance collision is likely). If a
//! consumer ever needs collision resistance against an attacker — signing a
//! published score, say — that is a different function with a different
//! algorithm, chosen then, not a stronger reading of this one.

/// A stable digest of a timeline's canonical form (N6).
///
/// Equal canonical forms hash equal, always and everywhere. Unequal hashes
/// therefore prove the semantics differ; equal hashes mean they agree, up to
/// the collision probability of a 128-bit digest.
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
/// site's randomness from a seed and a path (`docs/kernel/11-realization.md`)
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

/// An FNV-1a accumulator, fed the canonical bytes as they are produced so
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

impl std::fmt::Write for Digest {
    /// Absorbing through `Write` is what makes the digest and the canonical
    /// text the same bytes *by construction*: one writer produces N5, and
    /// hashing is that writer pointed at an accumulator instead of a
    /// `String`. Two serializations that could drift apart is exactly the bug
    /// a semantic hash cannot survive.
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.write(text.as_bytes());
        Ok(())
    }
}
