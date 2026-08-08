//! Which performance to compile.
//!
//! A piece may leave decisions to the performance: how many times to repeat a
//! figure, in what order to play a set of fragments, how long to hold a free
//! duration. `docs/kernel/11-realization.md` puts those decisions **here** —
//! above the kernel, before a term exists — rather than inside the kernel as a
//! `choose` form. The four reasons that form was refused are in that document;
//! the consequence for this module is the whole of its design:
//!
//! > By the time elaboration produces a `Term`, every choice is made. So
//! > evaluation is still total, still deterministic, still confluent, and the
//! > normal form still has a semantic hash.
//!
//! Which gives the law this module exists to keep (R1):
//!
//! > Same source **and same realization** ⇒ same term, same normal form, same
//! > semantic hash, byte-identical exports.
//!
//! Randomness is derived **per path**, never from a stream. A stream would
//! re-roll every later decision the moment a site is inserted anywhere above
//! it — the same failure a span-based identity has, arriving later and much
//! less visibly, because the first few decisions would still look right.

use std::collections::BTreeMap;

use num_rational::Ratio;

use crate::origin::ChoicePath;

/// What was decided at one site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// How many passes a ranged repeat takes.
    Count(u32),
    /// The order a set of fragments is played in, by index.
    Order(Vec<u32>),
    /// How long a free duration lasts, in whole notes.
    Duration(Ratio<i64>),
}

impl std::fmt::Display for Decision {
    /// How a decision is written in a `.kernel` header and shown to a person.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Count(count) => write!(formatter, "count={count}"),
            Self::Order(order) => {
                formatter.write_str("order=")?;
                let mut first = true;
                for index in order {
                    if !first {
                        formatter.write_str(",")?;
                    }
                    first = false;
                    write!(formatter, "{index}")?;
                }
                Ok(())
            }
            Self::Duration(value) => write!(formatter, "duration={}/{}", value.numer(), value.denom()),
        }
    }
}

/// Which performance to compile: a seed, and the sites the composer pinned.
///
/// Not "a default performance" and not a random number generator. A piece that
/// asks no questions never consults it, which is why every determinate piece
/// in the corpus is bit-identical under every seed — a property the tests
/// check rather than assume.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Realization {
    seed: u64,
    overrides: BTreeMap<ChoicePath, Decision>,
}

impl Realization {
    /// The realization every existing piece gets: seed zero, no overrides.
    ///
    /// A determinate piece never consults it, so this is not "a default
    /// performance" — it is the absence of a question.
    #[must_use]
    pub fn deterministic() -> Self {
        Self::default()
    }

    /// A named performance. Two compiles of one source under one seed agree
    /// down to the byte.
    #[must_use]
    pub fn seeded(seed: u64) -> Self {
        Self {
            seed,
            overrides: BTreeMap::new(),
        }
    }

    /// Pin one site, leaving the rest to the seed.
    ///
    /// What makes a realization editable rather than a lottery ticket: a
    /// composer who likes the fourth pass but not the sixth pins the sixth.
    #[must_use]
    pub fn pinned(mut self, path: ChoicePath, decision: Decision) -> Self {
        self.overrides.insert(path, decision);
        self
    }

    /// The seed, for a header line or a status readout.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Every decision pinned by hand, in path order.
    ///
    /// The decisions a *compile* took are reported by the compilation
    /// (`Compilation::decisions`), because only elaboration knows which sites
    /// the piece actually has; this is the subset the composer fixed.
    pub fn taken(&self) -> impl Iterator<Item = (&ChoicePath, &Decision)> {
        self.overrides.iter()
    }

    /// Choose an order for `count` fragments at the site at `path`.
    ///
    /// Fisher–Yates over a stream derived from the path, so the result is a
    /// permutation — Klavierstück XI's nineteen fragments give 19! orderings,
    /// the number that killed `choose`, and here they cost nineteen `u32`s.
    /// A pin of the wrong length is ignored rather than trusted: the piece
    /// says how many fragments there are.
    pub(crate) fn order(&self, path: &ChoicePath, count: u32) -> Vec<u32> {
        let mut order: Vec<u32> = (0..count).collect();
        if let Some(Decision::Order(pinned)) = self.overrides.get(path)
            && is_permutation(pinned, count)
        {
            return pinned.clone();
        }
        let mut stream = Stream::from(draw(self.seed, path));
        // Downward Fisher–Yates: every permutation is reachable and each is
        // as likely as the digest is uniform.
        let mut index = order.len();
        while index > 1 {
            index = index.saturating_sub(1);
            let swap = usize::try_from(stream.next() % (index as u128).saturating_add(1)).unwrap_or(0);
            order.swap(index, swap);
        }
        order
    }

    /// Choose how long a free duration lasts, in whole notes.
    ///
    /// On a sixteenth-note grid offset from `least`, so what the performance
    /// picks is a duration the engraver can spell. A continuum is what the
    /// *instruction* means; a page is what musa has to draw.
    pub(crate) fn duration(&self, path: &ChoicePath, least: Ratio<i64>, most: Ratio<i64>) -> Ratio<i64> {
        if let Some(Decision::Duration(pinned)) = self.overrides.get(path) {
            return (*pinned).clamp(least, most);
        }
        let grid = Ratio::new(1, 16);
        let steps = ((most - least) / grid).to_integer().max(0);
        let Ok(steps) = u32::try_from(steps) else {
            return least;
        };
        let taken = self.count(path, 0, steps);
        least + grid * Ratio::from_integer(i64::from(taken))
    }

    /// Choose a repeat count in `least ..= most` for the site at `path`.
    ///
    /// A pin wins; otherwise the count is derived from the seed and the path
    /// and from nothing else — not from how many decisions came before, which
    /// is the property that makes an edit above a site leave that site alone.
    pub(crate) fn count(&self, path: &ChoicePath, least: u32, most: u32) -> u32 {
        if let Some(Decision::Count(pinned)) = self.overrides.get(path) {
            return (*pinned).clamp(least, most);
        }
        let span = u128::from(most.saturating_sub(least)).saturating_add(1);
        // `span` is at least one, so the remainder is defined; `checked_rem`
        // says so to the reader as well as to the lint.
        let offset = draw(self.seed, path)
            .checked_rem(span)
            .and_then(|offset| u32::try_from(offset).ok())
            .unwrap_or(0);
        least.saturating_add(offset)
    }
}

/// The randomness for one site: `fnv1a_128(seed ‖ path)`.
///
/// The workspace's one stable digest (`musa_kernel::stable_digest`), so the
/// same seed and the same path give the same answer in every process and on
/// every platform — which is what makes a realization something a composer can
/// write down and send to somebody else.
fn draw(seed: u64, path: &ChoicePath) -> u128 {
    let mut bytes = seed.to_be_bytes().to_vec();
    bytes.extend_from_slice(path.canonical().as_bytes());
    musa_kernel::stable_digest(&bytes)
}

/// A site's randomness, when one number is not enough.
///
/// A shuffle needs `n - 1` draws and they must not repeat. This is still *per
/// path* — the stream is seeded by the site and by nothing before it, so the
/// property that makes an edit elsewhere harmless is unaffected. SplitMix64's
/// mixing function, which is four lines and needs no dependency.
struct Stream(u64);

impl Stream {
    fn from(digest: u128) -> Self {
        Self((digest as u64) ^ ((digest >> 64) as u64))
    }

    fn next(&mut self) -> u128 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        u128::from(mixed ^ (mixed >> 31))
    }
}

/// Whether `pinned` is a permutation of `0 .. count`.
fn is_permutation(pinned: &[u32], count: u32) -> bool {
    let mut seen: Vec<bool> = vec![false; count as usize];
    if pinned.len() != seen.len() {
        return false;
    }
    for index in pinned {
        let Some(slot) = seen.get_mut(*index as usize) else {
            return false;
        };
        if *slot {
            return false;
        }
        *slot = true;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{Decision, Realization, draw};
    use crate::origin::{ChoicePath, ChoiceStep};

    fn path(steps: &[ChoiceStep]) -> ChoicePath {
        steps
            .iter()
            .fold(ChoicePath::default(), |path, step| path.then(step.clone()))
    }

    #[test]
    fn a_count_is_inside_the_range_it_was_asked_for() {
        for seed in 0..64u64 {
            let realization = Realization::seeded(seed);
            let count = realization.count(&path(&[ChoiceStep::Ordinal(0)]), 4, 16);
            assert!((4..=16).contains(&count), "{count} is outside 4..=16");
        }
    }

    #[test]
    fn one_number_in_the_range_is_still_that_number() {
        // A degenerate range is an exact count, and must not divide by zero.
        let realization = Realization::seeded(7);
        assert_eq!(realization.count(&path(&[ChoiceStep::Ordinal(0)]), 5, 5), 5);
    }

    #[test]
    fn a_pin_wins_over_the_seed() {
        let site = path(&[ChoiceStep::Motif("fill".into()), ChoiceStep::Ordinal(0)]);
        let realization = Realization::seeded(1).pinned(site.clone(), Decision::Count(9));
        assert_eq!(realization.count(&site, 4, 16), 9);
        // And a pin outside the range is brought into it rather than trusted:
        // the range is what the piece says, and a pin is a preference.
        let clamped = Realization::seeded(1).pinned(site.clone(), Decision::Count(99));
        assert_eq!(clamped.count(&site, 4, 16), 16);
    }

    #[test]
    fn a_site_is_drawn_from_its_own_path_and_nothing_else() {
        // The property per-path derivation exists for: two sites' decisions
        // are independent, so inserting one does not disturb the other.
        let first = path(&[ChoiceStep::Bar("fill".into()), ChoiceStep::Ordinal(0)]);
        let second = path(&[ChoiceStep::Bar("fill".into()), ChoiceStep::Ordinal(1)]);
        assert_ne!(draw(3, &first), draw(3, &second));
        assert_eq!(draw(3, &first), draw(3, &first));
        assert_ne!(draw(3, &first), draw(4, &first));
    }

    #[test]
    fn the_canonical_encoding_is_injective() {
        // The N3 requirement: no concatenation of names can imitate a
        // different splitting of them.
        let ab = path(&[ChoiceStep::Motif("ab".into())]);
        let a_b = path(&[ChoiceStep::Motif("a".into()), ChoiceStep::Motif("b".into())]);
        assert_ne!(ab.canonical(), a_b.canonical());
        // And the kind letter separates a bar called `x` from a motif called
        // `x`, which are different places in the piece.
        let motif = path(&[ChoiceStep::Motif("x".into())]);
        let bar = path(&[ChoiceStep::Bar("x".into())]);
        assert_ne!(motif.canonical(), bar.canonical());
    }
}
