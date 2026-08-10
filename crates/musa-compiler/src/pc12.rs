//! Unspelled pitch classes, pitch-class sets, and twelve-tone rows
//! (`docs/language/03-musical-domains.md` §4).
//!
//! This is the chromatic quotient and nothing else. `pc12` is `ℤ/12ℤ`, so
//! `b#` and `c` are the same element here and the spelled domain of
//! `pitch.rs` is where they stop being the same. Only one direction is
//! total — [`Pc12::forgetting`] — and coming back needs a policy the caller
//! states, which is why [`Pc12::spelled`] takes a scale and may answer
//! nothing. Nothing in this module ever produces a spelled value on its own.
//!
//! Three types, each of which is its own proof:
//!
//! - [`Pc12`] is canonical modulo twelve, so two equal pitch classes are the
//!   same value and equality is `==`;
//! - [`PcSet12`] is a twelve-bit membership word, so a set cannot contain a
//!   duplicate and its members always come out ascending;
//! - [`Row12`] is a checked permutation of all twelve, so every row
//!   operation below is total — the finite-closure lemma in the language
//!   documentation is exactly the reason these return a row rather than an
//!   `Option<Row12>`.
//!
//! The group accounting the surface documentation insists on lives here too.
//! There are **24** affine operations on `pc12`: twelve transpositions
//! `Tₙ(x) = x + n` and twelve inversions `Iₙ(x) = n − x`. Ordered rows admit
//! one further factor, reversal `R`, which is an involution commuting with
//! elementwise `T`/`I`; that gives `D12 × C2` and therefore **48** labelled
//! `P`/`I`/`R`/`RI` forms — not a 48-element group of pitch-class
//! operations. How many *distinct* rows those 48 labels produce is a
//! question about one row's stabilizer, so [`Row12::forms`] counts and
//! [`Row12::symmetries`] reports the stabilizer's order.

// Arithmetic here is arithmetic in `ℤ/12ℤ` and counting over twelve things:
// every operand is either reduced by `Pc12::from_number` or bounded by
// `CHROMA`, `INTERVAL_CLASSES`, or `LABELLED_FORMS`, and additions are widened
// before they are reduced. clippy::arithmetic_side_effects is about raw
// integer overflow, which none of these can reach.
#![allow(clippy::arithmetic_side_effects)]

use crate::pitch::{PitchClass, WrittenPitch};
use crate::scale::Scale;

/// The size of the chromatic quotient.
const CHROMA: u8 = 12;

/// The number of interval classes: `1` through `6`, since `ic 7` is `ic 5`.
const INTERVAL_CLASSES: usize = 6;

/// The number of labelled row forms: 24 affine operations times reversal.
const LABELLED_FORMS: u32 = 48;

/// One element of `ℤ/12ℤ`: a pitch class with its spelling forgotten.
///
/// Canonical by construction — the wrapped byte is always below twelve — so
/// derived equality is the equality of the quotient and no comparison has to
/// reduce first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Pc12(u8);

impl std::fmt::Display for Pc12 {
    /// The canonical representative, written as a number.
    ///
    /// A number and never a letter: this domain has no letters, and printing
    /// one would be the implicit spelling the whole module refuses.
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "{}", self.0)
    }
}

impl Pc12 {
    /// The pitch class a natural number names, reduced modulo twelve.
    ///
    /// Total: every natural number names one, which is what makes `pc12`
    /// arithmetic writable in a language whose `nat` has no subtraction.
    pub(crate) fn from_number(number: u64) -> Self {
        Self(u8::try_from(number % u64::from(CHROMA)).unwrap_or(0))
    }

    /// The canonical representative, `0` through `11`.
    pub(crate) const fn number(self) -> u8 {
        self.0
    }

    /// The forgetful map `χ`: a spelled pitch class loses its spelling.
    ///
    /// Well defined because respelling changes the chromatic coordinate by a
    /// multiple of twelve, and not injective because `c#` and `db` both land
    /// on `1`. This is the only total direction between the two domains.
    pub(crate) fn forgetting(spelled: PitchClass) -> Self {
        let chromatic = i64::from(spelled.letter.natural_semitone()).saturating_add(i64::from(spelled.accidental.0));
        Self(u8::try_from(chromatic.rem_euclid(i64::from(CHROMA))).unwrap_or(0))
    }

    /// `Tₙ`: transposition by `index` semitones.
    ///
    /// The index is reduced before it is added, so that an index near the
    /// top of `nat` transposes by its residue rather than saturating.
    pub(crate) fn transposed(self, index: u64) -> Self {
        Self::from_number(u64::from(self.0) + index % u64::from(CHROMA))
    }

    /// `Iₙ`: inversion about `index`, that is `index − x`.
    ///
    /// The addition of twelve keeps the subtraction inside the unsigned
    /// arithmetic the rest of the compiler uses; the result is the same
    /// element either way.
    pub(crate) fn inverted(self, index: u64) -> Self {
        Self::from_number(index % u64::from(CHROMA) + u64::from(CHROMA) - u64::from(self.0))
    }

    /// A spelling of this pitch class inside one scale, if the scale has one.
    ///
    /// The scale *is* the policy: `1` spells `c#` in D major and `db` in A
    /// flat major, and in a collection containing neither it spells nothing.
    /// A caller that wants a spelling must therefore say in which collection
    /// it wants it, which is the whole reason this direction is not a
    /// coercion.
    pub(crate) fn spelled(self, scale: Scale) -> Option<PitchClass> {
        let tonic = WrittenPitch {
            letter: scale.tonic().letter,
            accidental: scale.tonic().accidental,
            octave: 4,
        };
        scale
            .collection()
            .offsets()
            .iter()
            .filter_map(|offset| tonic.transpose(*offset))
            .map(WrittenPitch::pitch_class)
            .find(|spelled| Self::forgetting(*spelled) == self)
    }

    /// Every pitch class, ascending. The domain is finite and this is it.
    pub(crate) fn every() -> impl Iterator<Item = Self> {
        (0..CHROMA).map(Self)
    }
}

/// A finite set of unspelled pitch classes.
///
/// Represented as a twelve-bit membership word, so duplication is not a
/// state this type can be in and membership order is not a choice it can
/// make. Both are why the checked constructor below cannot fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PcSet12(u16);

impl std::fmt::Display for PcSet12 {
    /// Ascending from zero, comma-separated, in braces.
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str("{")?;
        for (index, member) in self.members().enumerate() {
            if index > 0 {
                out.write_str(",")?;
            }
            write!(out, "{member}")?;
        }
        out.write_str("}")
    }
}

impl PcSet12 {
    /// The set of everything listed, however often it was listed.
    ///
    /// Total: a repeated member is a set with that member in it, not an
    /// error, because a set is what was asked for. A *row* is where a
    /// repetition is a mistake, and [`Row12::checked`] is where it is caught.
    pub(crate) fn of(members: impl IntoIterator<Item = Pc12>) -> Self {
        Self(members.into_iter().fold(0, |word, member| word | 1 << member.0))
    }

    /// The members, ascending from zero.
    pub(crate) fn members(self) -> impl Iterator<Item = Pc12> {
        Pc12::every().filter(move |member| self.0 & 1 << member.0 != 0)
    }

    /// How many members the set has.
    ///
    /// Nothing the language can ask needs this — a set's size is the length
    /// of its member list, which the surface already has — so it exists for
    /// the orbit accounting the tests below check, and only there.
    #[cfg(test)]
    pub(crate) fn size(self) -> u32 {
        self.0.count_ones()
    }

    /// `Tₙ` applied to every member.
    ///
    /// Set transposition is written in `std::post_tonal::pcset` as `map_pc` over the
    /// members, which is where an author can see what it does; this is the
    /// same map, kept for the laws below that need it in Rust.
    #[cfg(test)]
    pub(crate) fn transposed(self, index: u64) -> Self {
        Self::of(self.members().map(|member| member.transposed(index)))
    }

    /// `Iₙ` applied to every member.
    pub(crate) fn inverted(self, index: u64) -> Self {
        Self::of(self.members().map(|member| member.inverted(index)))
    }

    /// Normal order: the rotation of the ascending members that packs them
    /// most tightly to the left (OMT `101-pitch-class-sets-normal-order-and-
    /// transformations.md`).
    ///
    /// Read the members ascending and consider each rotation as an ordering
    /// that wraps through zero. Prefer the rotation with the smallest span
    /// from first to last; on a tie, the smallest span from first to the one
    /// before last, and so on inward; on a tie throughout, the rotation
    /// beginning on the lowest-numbered pitch class. The empty set has no
    /// rotation, so it normalizes to nothing.
    pub(crate) fn normal_order(self) -> Vec<Pc12> {
        let ascending: Vec<Pc12> = self.members().collect();
        let rotations = (0..ascending.len()).map(|start| {
            let mut rotation: Vec<Pc12> = ascending.iter().copied().skip(start).collect();
            rotation.extend(ascending.iter().copied().take(start));
            rotation
        });
        rotations
            .min_by(|left, right| compactness(left).cmp(&compactness(right)))
            .unwrap_or_default()
    }

    /// Prime form: the set class this set belongs to, named by its
    /// most left-packed representative (OMT `102-set-class-and-prime-form.md`).
    ///
    /// Take the normal order of the set and of its inversion, transpose each
    /// so that it begins on zero, and keep whichever reads lower. The result
    /// is a set rather than an ordering because a set class is a set: the
    /// ordering was only ever the means of choosing it.
    pub(crate) fn prime_form(self) -> Self {
        let zeroed = |candidate: Self| {
            let normal = candidate.normal_order();
            let first = normal.first().map_or(0, |member| u64::from(member.number()));
            normal
                .iter()
                .map(|member| Pc12::from_number(u64::from(member.number()) + u64::from(CHROMA) - first))
                .collect::<Vec<_>>()
        };
        let upright = zeroed(self);
        let mirrored = zeroed(self.inverted(0));
        Self::of(if mirrored < upright { mirrored } else { upright })
    }

    /// Whether one pitch class belongs to this set.
    pub(crate) fn contains(self, member: Pc12) -> bool {
        self.0 & 1 << member.number() != 0
    }

    /// The interval-class vector: how many pairs realize each of the six
    /// interval classes (OMT `103-interval-class-vectors.md`).
    ///
    /// Interval class `7` is interval class `5` heard the other way round,
    /// so there are six entries and never twelve.
    pub(crate) fn interval_class_vector(self) -> [u32; INTERVAL_CLASSES] {
        let members: Vec<Pc12> = self.members().collect();
        let mut vector = [0; INTERVAL_CLASSES];
        for (index, lower) in members.iter().enumerate() {
            for upper in members.iter().skip(index + 1) {
                let distance = u32::from(upper.number().abs_diff(lower.number()));
                let class = distance.min(u32::from(CHROMA) - distance);
                if let Some(count) = vector.get_mut(class.saturating_sub(1) as usize) {
                    *count += 1;
                }
            }
        }
        vector
    }
}

/// How tightly one rotation packs, as a sortable key.
///
/// The spans from the first member outward to the last, the second-to-last,
/// and so on inward, followed by the first member itself as the final tie
/// break. Comparing these lexicographically is exactly the convention
/// documented on [`PcSet12::normal_order`].
fn compactness(rotation: &[Pc12]) -> (Vec<u8>, u8) {
    let Some(first) = rotation.first() else {
        return (Vec::new(), 0);
    };
    let spans = rotation
        .iter()
        .skip(1)
        .rev()
        .map(|member| (member.number() + CHROMA - first.number()) % CHROMA)
        .collect();
    (spans, first.number())
}

/// A twelve-tone row: a bijection from the twelve order positions onto the
/// twelve pitch classes.
///
/// The invariant is checked once, by [`Row12::checked`], and thereafter every
/// operation here returns a row rather than a maybe-row. That is not
/// optimism: composing a row with a permutation of order positions and a
/// bijection of pitch classes is again a bijection, so the operations below
/// *cannot* fail, and making them return `Option` would ask every caller to
/// handle a case that does not exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Row12([Pc12; CHROMA as usize]);

impl std::fmt::Display for Row12 {
    /// The twelve pitch classes in order, separated by spaces.
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, member) in self.0.iter().enumerate() {
            if index > 0 {
                out.write_str(" ")?;
            }
            write!(out, "{member}")?;
        }
        Ok(())
    }
}

impl Row12 {
    /// The row a sequence spells, or nothing when the sequence is not one.
    ///
    /// Nothing is returned when the length is wrong or a pitch class repeats.
    /// *Which* position repeated and *which* pitch class never arrived are
    /// separate questions with exact answers — [`repeated_positions`] and
    /// [`missing_classes`] — because a language whose failures are `option`
    /// cannot carry that detail inside the absence.
    pub(crate) fn checked(pcs: &[Pc12]) -> Option<Self> {
        if pcs.len() != CHROMA as usize || !repeated_positions(pcs).is_empty() {
            return None;
        }
        let mut row = [Pc12(0); CHROMA as usize];
        for (slot, pc) in row.iter_mut().zip(pcs) {
            *slot = *pc;
        }
        Some(Self(row))
    }

    /// The twelve pitch classes, in order position order.
    pub(crate) fn pcs(&self) -> impl Iterator<Item = Pc12> + '_ {
        self.0.iter().copied()
    }

    /// The pitch class this row begins on.
    pub(crate) fn head(&self) -> Pc12 {
        self.0.first().copied().unwrap_or(Pc12(0))
    }

    /// `Tₙ` applied to every pitch class, order positions untouched.
    pub(crate) fn transposed(&self, index: u64) -> Self {
        self.mapped(|member| member.transposed(index))
    }

    /// `Iₙ` applied to every pitch class, order positions untouched.
    pub(crate) fn inverted(&self, index: u64) -> Self {
        self.mapped(|member| member.inverted(index))
    }

    /// `R`: the order positions reversed, pitch classes untouched.
    ///
    /// An involution, and it commutes with elementwise `T` and `I` because it
    /// acts on the other side: one permutes positions, the others permute
    /// pitch classes.
    pub(crate) fn retrograde(&self) -> Self {
        let mut reversed = self.0;
        reversed.reverse();
        Self(reversed)
    }

    /// The transposition of this row that begins on `pc`.
    ///
    /// Exactly one transposition does, so this is total and is the operation
    /// the matrix is built out of.
    pub(crate) fn starting_on(&self, pc: Pc12) -> Self {
        self.transposed(u64::from(pc.number() + CHROMA - self.head().number()))
    }

    /// The twelve-tone matrix, as twelve rows.
    ///
    /// Row `i` is the transposition of this row beginning on the `i`th pitch
    /// class of its inversion about its own head, which is the classical
    /// construction: row `0` is the row as written, the leftmost column
    /// reads that inversion downward, and every column read downward is an
    /// inversion of the row.
    ///
    /// The construction fixes no naming convention, because it does not need
    /// one: the rows are rows, not labels. Which transposition is called
    /// `P0` is a separate question, and `std::post_tonal::serial` answers it with two
    /// differently named functions rather than one that quietly picks.
    pub(crate) fn matrix(&self) -> Vec<Self> {
        let leftmost = self.inverted(u64::from(self.head().number()).saturating_mul(2));
        leftmost.pcs().map(|start| self.starting_on(start)).collect()
    }

    /// Every labelled `P`/`I`/`R`/`RI` form of this row, with duplicates.
    ///
    /// Forty-eight of them, always: twenty-four affine operations on pitch
    /// classes, each read forward and backward. How many *rows* that is
    /// depends on the row.
    fn labelled_forms(&self) -> Vec<Self> {
        (0..u64::from(CHROMA))
            .flat_map(|index| [self.transposed(index), self.inverted(index)])
            .flat_map(|form| [form, form.retrograde()])
            .collect()
    }

    /// How many *distinct* rows the forty-eight labelled forms produce.
    ///
    /// Forty-eight for a generic row. A row with an internal symmetry —
    /// one whose stabilizer is nontrivial — has fewer, which is why this is
    /// counted rather than asserted (OMT `110-row-properties.md`).
    pub(crate) fn forms(&self) -> u32 {
        let mut distinct: Vec<Self> = self.labelled_forms();
        distinct.sort_unstable();
        distinct.dedup();
        u32::try_from(distinct.len()).unwrap_or(LABELLED_FORMS)
    }

    /// The order of this row's stabilizer: how many of the forty-eight
    /// labelled operations send the row to itself.
    ///
    /// By the orbit–stabilizer theorem this times [`Row12::forms`] is always
    /// forty-eight, which is the accounting the language documentation asks
    /// to be kept honest.
    pub(crate) fn symmetries(&self) -> u32 {
        u32::try_from(self.labelled_forms().iter().filter(|form| *form == self).count()).unwrap_or(1)
    }

    /// This row with one function applied to each of its pitch classes.
    ///
    /// Private because it is only sound for a bijection of `pc12`: applying
    /// an arbitrary function would produce twelve pitch classes that are not
    /// a permutation, and the type would then be lying.
    fn mapped(&self, transform: impl Fn(Pc12) -> Pc12) -> Self {
        let mut mapped = self.0;
        for member in &mut mapped {
            *member = transform(*member);
        }
        Self(mapped)
    }
}

/// The order positions whose pitch class already appeared earlier.
///
/// Exact, and the first occurrence is not among them: in `0 1 0 …` it is
/// position two that repeats, because position zero is where that pitch
/// class belongs.
pub(crate) fn repeated_positions(pcs: &[Pc12]) -> Vec<u64> {
    let mut seen = PcSet12(0);
    let mut repeats = Vec::new();
    for (position, pc) in pcs.iter().enumerate() {
        if seen.contains(*pc) {
            repeats.push(position as u64);
        }
        seen = PcSet12(seen.0 | 1 << pc.number());
    }
    repeats
}

/// The pitch classes a sequence never names, ascending.
pub(crate) fn missing_classes(pcs: &[Pc12]) -> Vec<Pc12> {
    let present = PcSet12::of(pcs.iter().copied());
    Pc12::every().filter(|member| !present.contains(*member)).collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use crate::pitch::{Accidental, Letter};

    /// A row of pitch classes from their numbers, for readable fixtures.
    fn row(numbers: [u64; 12]) -> Row12 {
        let pcs: Vec<Pc12> = numbers.into_iter().map(Pc12::from_number).collect();
        Row12::checked(&pcs).expect("the fixture is a permutation")
    }

    #[test]
    fn transposition_and_inversion_compose_into_twenty_four_operations() {
        let mut distinct: Vec<Vec<Pc12>> = (0..12)
            .flat_map(|index| {
                [
                    Pc12::every().map(|pc| pc.transposed(index)).collect::<Vec<_>>(),
                    Pc12::every().map(|pc| pc.inverted(index)).collect::<Vec<_>>(),
                ]
            })
            .collect();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), 24, "the affine group on pc12 has 24 elements");
    }

    #[test]
    fn a_generic_row_has_forty_eight_forms() {
        let generic = row([0, 1, 4, 9, 5, 8, 3, 10, 2, 11, 6, 7]);
        assert_eq!(generic.forms(), 48);
        assert_eq!(generic.symmetries(), 1);
    }

    #[test]
    fn forms_times_symmetries_is_always_forty_eight() {
        let chromatic = row([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        let generic = row([0, 1, 4, 9, 5, 8, 3, 10, 2, 11, 6, 7]);
        for candidate in [chromatic, generic, chromatic.retrograde(), generic.inverted(5)] {
            assert_eq!(candidate.forms() * candidate.symmetries(), 48, "{candidate}");
        }
    }

    /// Every set of pitch classes there is, as a membership word.
    fn every_set() -> impl Iterator<Item = PcSet12> {
        (0u16..1 << CHROMA).map(PcSet12)
    }

    /// The documented normal-order convention, enumerated rather than
    /// derived: every rotation of the ascending members, keyed by the spans
    /// from the first member inward and then by the first member itself.
    ///
    /// A second implementation, written the other way round, so that
    /// agreement with [`PcSet12::normal_order`] is evidence about the
    /// convention rather than a restatement of one function.
    fn reference_normal_order(set: PcSet12) -> Vec<Pc12> {
        let ascending: Vec<Pc12> = set.members().collect();
        let mut best: Option<(Vec<u8>, u8, Vec<Pc12>)> = None;
        for start in 0..ascending.len() {
            let mut rotation = Vec::new();
            for step in 0..ascending.len() {
                let index = (start + step) % ascending.len();
                rotation.push(*ascending.get(index).expect("the index wrapped inside the vector"));
            }
            let first = rotation.first().expect("a rotation of a nonempty vector").number();
            let mut spans = Vec::new();
            for member in rotation.iter().skip(1).rev() {
                spans.push((member.number() + CHROMA - first) % CHROMA);
            }
            let key = (spans, first, rotation);
            if best.as_ref().is_none_or(|current| key < *current) {
                best = Some(key);
            }
        }
        best.map(|(_, _, rotation)| rotation).unwrap_or_default()
    }

    /// Prime form by brute force over all twenty-four affine images.
    ///
    /// Every `Tₙ` and `Iₙ` image is normalized to begin on zero through the
    /// reference normal order above, and the lowest-reading result wins.
    /// [`PcSet12::prime_form`] takes the shorter route — two normal orders,
    /// not twenty-four — so agreement is a real check.
    fn reference_prime_form(set: PcSet12) -> PcSet12 {
        let mut best: Option<Vec<Pc12>> = None;
        for index in 0..u64::from(CHROMA) {
            for image in [set.transposed(index), set.inverted(index)] {
                let normal = reference_normal_order(image);
                let first = normal.first().map_or(0, |member| u64::from(member.number()));
                let zeroed: Vec<Pc12> = normal
                    .iter()
                    .map(|member| Pc12::from_number(u64::from(member.number()) + u64::from(CHROMA) - first))
                    .collect();
                if best.as_ref().is_none_or(|current| zeroed < *current) {
                    best = Some(zeroed);
                }
            }
        }
        PcSet12::of(best.unwrap_or_default())
    }

    /// A set from its members' numbers.
    fn set(numbers: &[u64]) -> PcSet12 {
        PcSet12::of(numbers.iter().copied().map(Pc12::from_number))
    }

    #[test]
    fn the_quotient_agrees_with_the_chromatic_coordinate() {
        for letter in [
            Letter::C,
            Letter::D,
            Letter::E,
            Letter::F,
            Letter::G,
            Letter::A,
            Letter::B,
        ] {
            for alteration in -3..=3 {
                let spelled = PitchClass {
                    letter,
                    accidental: Accidental(alteration),
                };
                let chromatic = i64::from(letter.natural_semitone()) + i64::from(alteration);
                assert_eq!(
                    u8::try_from(chromatic.rem_euclid(12)).expect("a residue is small"),
                    Pc12::forgetting(spelled).number(),
                    "{spelled} forgets onto its chromatic coordinate"
                );
            }
        }
    }

    #[test]
    fn transposition_and_inversion_agree_with_modular_arithmetic() {
        for member in Pc12::every() {
            for index in 0..24u64 {
                let x = i64::from(member.number());
                let n = index as i64;
                assert_eq!(
                    member.transposed(index).number(),
                    u8::try_from((x + n).rem_euclid(12)).expect("a residue is small"),
                    "T{index} of {member}"
                );
                assert_eq!(
                    member.inverted(index).number(),
                    u8::try_from((n - x).rem_euclid(12)).expect("a residue is small"),
                    "I{index} of {member}"
                );
            }
        }
    }

    #[test]
    fn the_affine_operations_compose_identify_and_invert() {
        for member in Pc12::every() {
            assert_eq!(member.transposed(0), member, "T0 is the identity");
            for left in 0..12u64 {
                for right in 0..12u64 {
                    assert_eq!(
                        member.transposed(right).transposed(left),
                        member.transposed(left + right),
                        "transpositions add"
                    );
                    assert_eq!(
                        member.inverted(right).inverted(left),
                        member.transposed(left + 12 - right),
                        "two inversions make a transposition"
                    );
                    assert_eq!(
                        member.transposed(right).inverted(left),
                        member.inverted(left + 12 - right),
                        "an inversion after a transposition is an inversion"
                    );
                }
                assert_eq!(
                    member.inverted(left).inverted(left),
                    member,
                    "every inversion is its own inverse"
                );
            }
        }
    }

    #[test]
    fn reversal_is_an_involution_that_commutes_with_the_pitch_class_maps() {
        let series = row([0, 1, 4, 9, 5, 8, 3, 10, 2, 11, 6, 7]);
        assert_eq!(series.retrograde().retrograde(), series, "R is an involution");
        for index in 0..12u64 {
            assert_eq!(
                series.transposed(index).retrograde(),
                series.retrograde().transposed(index),
                "R commutes with T{index}"
            );
            assert_eq!(
                series.inverted(index).retrograde(),
                series.retrograde().inverted(index),
                "R commutes with I{index}"
            );
        }
    }

    #[test]
    fn a_sequence_that_is_not_a_row_is_refused_with_exact_positions() {
        let repeated: Vec<Pc12> = [0, 1, 2, 0, 4, 5, 6, 7, 8, 9, 10, 3]
            .into_iter()
            .map(Pc12::from_number)
            .collect();
        assert!(Row12::checked(&repeated).is_none(), "a repetition is not a row");
        assert_eq!(
            repeated_positions(&repeated),
            vec![3],
            "the second occurrence is the repeat"
        );
        assert_eq!(
            missing_classes(&repeated),
            vec![Pc12::from_number(11)],
            "eleven never arrived"
        );

        let short: Vec<Pc12> = (0..11).map(Pc12::from_number).collect();
        assert!(Row12::checked(&short).is_none(), "eleven entries are not a row");
        assert!(
            repeated_positions(&short).is_empty(),
            "nothing repeated; it is simply short"
        );

        let long: Vec<Pc12> = (0..13).map(Pc12::from_number).collect();
        assert!(Row12::checked(&long).is_none(), "thirteen entries are not a row");
    }

    #[test]
    fn every_row_orbit_and_stabilizer_multiply_to_forty_eight() {
        let seeds = [
            row([0, 1, 4, 9, 5, 8, 3, 10, 2, 11, 6, 7]),
            row([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
            row([0, 11, 3, 4, 8, 7, 9, 5, 6, 1, 2, 10]),
            row([7, 10, 2, 6, 9, 0, 4, 8, 11, 1, 3, 5]),
        ];
        for seed in seeds {
            for index in 0..12u64 {
                for candidate in [
                    seed.transposed(index),
                    seed.inverted(index),
                    seed.transposed(index).retrograde(),
                    seed.inverted(index).retrograde(),
                ] {
                    assert_eq!(
                        candidate.forms() * candidate.symmetries(),
                        LABELLED_FORMS,
                        "orbit times stabilizer is 48 for {candidate}"
                    );
                    assert_eq!(candidate.forms(), seed.forms(), "one orbit, one count");
                }
            }
        }
    }

    #[test]
    fn a_symmetric_row_has_fewer_than_forty_eight_forms() {
        let chromatic = row([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        assert_eq!(
            chromatic.symmetries(),
            2,
            "the retrograde inversion about eleven fixes it"
        );
        assert_eq!(
            chromatic.forms(),
            24,
            "so half of the labels name the other half's rows"
        );
    }

    #[test]
    fn the_matrix_is_twelve_transpositions_whose_columns_are_inversions() {
        let series = row([0, 1, 4, 9, 5, 8, 3, 10, 2, 11, 6, 7]);
        let matrix = series.matrix();
        assert_eq!(matrix.len(), 12);
        assert_eq!(matrix.first().copied(), Some(series), "row zero is the row as written");
        for candidate in &matrix {
            assert!(
                (0..12).any(|index| series.transposed(index) == *candidate),
                "every row is a transposition"
            );
        }
        let leftmost: Vec<Pc12> = matrix.iter().map(Row12::head).collect();
        assert_eq!(
            leftmost,
            series
                .inverted(u64::from(series.head().number()) * 2)
                .pcs()
                .collect::<Vec<_>>(),
            "the leftmost column reads the inversion about the head"
        );
        for position in 0..12usize {
            let column: Vec<Pc12> = matrix
                .iter()
                .map(|line| line.pcs().nth(position).expect("twelve order positions"))
                .collect();
            let expected = Row12::checked(&column).expect("a column is a row");
            assert!(
                (0..12).any(|index| series.inverted(index) == expected),
                "column {position} is an inversion"
            );
        }
    }

    #[test]
    fn normal_order_agrees_with_an_enumerated_reference() {
        for candidate in every_set() {
            assert_eq!(
                candidate.normal_order(),
                reference_normal_order(candidate),
                "normal order of {candidate}"
            );
        }
    }

    #[test]
    fn prime_form_agrees_with_a_brute_force_reference() {
        for candidate in every_set() {
            assert_eq!(
                candidate.prime_form(),
                reference_prime_form(candidate),
                "prime form of {candidate}"
            );
        }
    }

    #[test]
    fn a_set_class_survives_transposition_and_inversion() {
        for candidate in every_set() {
            let prime = candidate.prime_form();
            for index in 0..12u64 {
                assert_eq!(
                    candidate.transposed(index).prime_form(),
                    prime,
                    "T{index} of {candidate}"
                );
                assert_eq!(candidate.inverted(index).prime_form(), prime, "I{index} of {candidate}");
            }
        }
    }

    #[test]
    fn the_interval_class_vector_counts_every_pair_once() {
        for candidate in every_set() {
            let size = candidate.size();
            let pairs = size * size.saturating_sub(1) / 2;
            assert_eq!(
                candidate.interval_class_vector().iter().sum::<u32>(),
                pairs,
                "every pair of {candidate} lands in exactly one interval class"
            );
        }
    }

    #[test]
    fn the_literature_examples_come_out_as_written() {
        assert_eq!(set(&[0, 4, 7]).prime_form(), set(&[0, 3, 7]), "the major triad is 3-11");
        assert_eq!(
            set(&[0, 3, 7]).prime_form(),
            set(&[0, 3, 7]),
            "and so is the minor triad"
        );
        assert_eq!(set(&[0, 4, 7]).interval_class_vector(), [0, 0, 1, 1, 1, 0]);
        assert_eq!(set(&[0, 2, 4, 6, 8, 10]).interval_class_vector(), [0, 6, 0, 6, 0, 3]);
        assert_eq!(
            set(&[0, 2, 4, 5, 7, 9, 11]).interval_class_vector(),
            [2, 5, 4, 3, 6, 1],
            "the diatonic collection's vector"
        );
        assert_eq!(
            set(&[0, 2, 4, 5, 7, 9, 11]).prime_form(),
            set(&[0, 1, 3, 5, 6, 8, 10]),
            "the diatonic collection is 7-35"
        );
        assert_eq!(
            set(&[0, 5, 8]).normal_order(),
            vec![Pc12::from_number(5), Pc12::from_number(8), Pc12::from_number(0)],
            "the most compact rotation wraps through zero"
        );
    }

    #[test]
    fn a_spelling_needs_a_collection_and_may_not_exist_in_it() {
        let c_major = crate::scale::Scale::new(
            PitchClass {
                letter: Letter::C,
                accidental: Accidental(0),
            },
            crate::scale::Collection::named("major").expect("major is a collection"),
        );
        assert_eq!(
            Pc12::from_number(4).spelled(c_major).map(|spelled| spelled.to_string()),
            Some("e".to_owned())
        );
        assert_eq!(
            Pc12::from_number(1).spelled(c_major),
            None,
            "C major has no note of pitch class one, and this is not the place to invent one"
        );
    }

    #[test]
    fn enharmonic_spellings_forget_onto_one_pitch_class() {
        let sharp = PitchClass {
            letter: Letter::C,
            accidental: Accidental(1),
        };
        let flat = PitchClass {
            letter: Letter::D,
            accidental: Accidental(-1),
        };
        assert_eq!(Pc12::forgetting(sharp), Pc12::forgetting(flat));
        assert_ne!(sharp, flat, "forgetting is not injective, and spelling stays spelling");
    }
}
