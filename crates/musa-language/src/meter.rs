//! How a bar divides into the groups a player hears.
//!
//! This is one musical fact with two readers: the engraver beams a group
//! together, and the formatter puts a wider gap between groups. They live in
//! different crates, so the fact lives below both of them — a bar of 7/8 that
//! beamed as seven eighths and printed as 2+2+3 would be two answers to one
//! question, which is the kind of drift this repo's laws exist to catch.

/// How a bar of `numerator`/`denominator` divides into the groups a player
/// hears — the fact a beam draws and a beat group is spaced by.
///
/// Each group is its length counted in `1/denominator` units, so the groups
/// always sum to `numerator` and 7/8 answers `[2, 2, 3]`. Callers already hold
/// the denominator; counting in its units keeps the bottom of the graph free
/// of a rational-arithmetic dependency for one table.
///
/// The irregular meters are named because their grouping is a convention, not
/// a calculation: 7/8 is 2+2+3 and 5/8 is 3+2 in the Balkan repertoire that
/// asks for them. Compound meters (`6/8`, `9/8`, `12/8`) group in threes, and
/// everything else groups by the notated beat.
///
/// A numerator of zero has no groups, which is what `meter 0/4` deserves;
/// the compiler refuses it long before anything asks this.
pub fn beat_groups(numerator: u32, denominator: u32) -> Vec<u32> {
    match (numerator, denominator) {
        (5, 8 | 4) => vec![3, 2],
        (7, 8 | 4) => vec![2, 2, 3],
        _ if denominator == 8 && numerator > 3 && numerator.is_multiple_of(3) => {
            let threes = usize::try_from(numerator).unwrap_or(0).checked_div(3).unwrap_or(0);
            vec![3; threes]
        }
        _ => vec![1; usize::try_from(numerator).unwrap_or(0)],
    }
}

#[cfg(test)]
mod tests {
    use super::beat_groups;

    #[test]
    fn irregular_meters_group_the_way_players_count_them() {
        assert_eq!(beat_groups(7, 8), vec![2, 2, 3]);
        assert_eq!(beat_groups(5, 8), vec![3, 2]);
        assert_eq!(beat_groups(7, 4), vec![2, 2, 3]);
        assert_eq!(beat_groups(5, 4), vec![3, 2]);
    }

    #[test]
    fn compound_meters_group_in_threes() {
        assert_eq!(beat_groups(6, 8), vec![3, 3]);
        assert_eq!(beat_groups(9, 8), vec![3, 3, 3]);
        assert_eq!(beat_groups(12, 8), vec![3, 3, 3, 3]);
        // 3/8 is *not* compound: one bar, three beats, no grouping to make.
        assert_eq!(beat_groups(3, 8), vec![1, 1, 1]);
    }

    #[test]
    fn simple_meters_group_by_the_notated_beat() {
        assert_eq!(beat_groups(4, 4), vec![1, 1, 1, 1]);
        assert_eq!(beat_groups(3, 4), vec![1, 1, 1]);
        assert_eq!(beat_groups(2, 2), vec![1, 1]);
    }

    #[test]
    fn the_groups_always_sum_to_the_numerator() {
        for denominator in [1_u32, 2, 4, 8, 16] {
            for numerator in 0_u32..=16 {
                let total: u32 = beat_groups(numerator, denominator).iter().sum();
                assert_eq!(total, numerator, "{numerator}/{denominator}");
            }
        }
    }
}
