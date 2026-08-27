//! The written-source spelling of measured durations (prompt 205ca).
//!
//! Prompt 205c composed the proposal but could only preview a voice whose
//! every written end and gap was a binary subdivision; everything else was a
//! declared deferral. This module completes the written half: it turns each
//! measured duration into its exact written form — the largest binary division
//! it equals, a dotted division when it is 3/2 of a binary one, a
//! `tuplet 3/2 { … }` group for ternary divisions, a tied chain when no single
//! form spells it, and `rest/…` for silence between written ends — and cuts
//! the result into bars that each measure exactly what the destination meter
//! says, splitting a note that crosses a bar line into tied halves the way a
//! copyist does.
//!
//! The spelling decisions are position-free and therefore deterministic:
//! one duration has one spelling wherever it falls, and the only position
//! that enters is the bar line, which the compilation law forces. Ternary
//! runs are bracketed per realignment span (one quarter on the 24-tick
//! grid): a 3-layer against the meter's binary layer realigns with it at each
//! quarter, and the bracket follows the realignment (Open Music Theory,
//! `118-metrical-dissonance.md`, grouping dissonance). Off-beat binary notes
//! stay where they were measured — a displaced layer is written displaced,
//! never renotated onto the beat (same chapter, displacement dissonance).
//!
//! A duration this vocabulary cannot spell exactly (an odd off-grid length, a
//! tie out of a ternary position, a tuplet group that crosses a bar line) is a
//! deferral carrying the note's index: the proposal declares the loss rather
//! than emitting a source that rounds it.

// The `roles`/`pages` vectors are built by `enumerate()` over the same slice
// every index came from and remain position-aligned by construction; reading
// them by index cannot leave the slice. Tick arithmetic is bounded by the
// bar-fit checks it feeds.
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::indexing_slicing)]

/// One note's placement request in a voice: where it starts, how long its
/// written end is, what it is called, and whether it belongs to the grace tier.
pub(crate) struct WrittenEntry {
    /// The entry's global proposal-note index, reported on deferral.
    pub index: usize,
    /// The measured onset, in grid ticks (24 per quarter).
    pub onset_ticks: u32,
    /// The measured written end, in grid ticks.
    pub duration_ticks: u32,
    /// The spelled pitch, e.g. `c#4`.
    pub pitch: String,
    /// Grace tier: too short for an ordinary notehead, so it leans on the next
    /// note and contributes no ticks of its own. A grace with no later
    /// ordinary note in the voice cannot lean on anything and must spell its
    /// own duration like any other note.
    pub grace: bool,
}

/// Every value one note or rest can spell on its own: a binary subdivision or
/// a dotted one (3/2 of a binary), as `(ticks, denominator, dotted)`, **largest
/// first** — the greedy spellings depend on that order, so keep it sorted.
/// Ticks are whole notes over 96: 24 per quarter.
const VALUES: [(u32, u32, bool); 11] = [
    (144, 1, true), // dotted whole
    (96, 1, false), // whole
    (72, 2, true),  // dotted half
    (48, 2, false), // half
    (36, 4, true),  // dotted quarter
    (24, 4, false), // quarter
    (18, 8, true),  // dotted eighth
    (12, 8, false), // eighth
    (9, 16, true),  // dotted sixteenth
    (6, 16, false), // sixteenth
    (3, 32, false), // thirty-second, the grid unit
];

/// Every duration a single `tuplet 3/2` member can spell: a binary value
/// scaled by 2/3, as `(ticks, denominator)`, largest first.
const MEMBERS: [(u32, u32); 6] = [
    (64, 1), // whole × 2/3
    (32, 2), // half × 2/3
    (16, 4), // quarter × 2/3
    (8, 8),  // eighth × 2/3
    (4, 16), // sixteenth × 2/3
    (2, 32), // thirty-second × 2/3
];

/// The realignment span of a 3-layer against the binary meter on this grid:
/// one quarter. A ternary run is bracketed per quarter boundary.
const REALIGNMENT_TICKS: u32 = 24;

/// The smallest value the table spells: the grid unit (a thirty-second at 24
/// ticks per quarter). Every multiple of it has a greedy spelling.
const GRID_UNIT: u32 = 3;

/// One spelled stretch of a voice, before bars are cut.
enum Atom {
    /// A note of the given total ticks, split into tied pieces as needed.
    Note { index: usize, pitch: String, ticks: u32 },
    /// Silence of the given total ticks, split as needed. `blame` is the
    /// index of the note the gap precedes, reported if it cannot be placed.
    Rest { ticks: u32, blame: usize },
    /// Silence spelled inside one `tuplet 3/2` group; cannot be split.
    TupletRest { ticks: u32, blame: usize },
    /// A bracketed ternary run; cannot cross a bar line. `blame` is the
    /// first member's note index, for deferral reporting.
    TupletNotes {
        blame: usize,
        members: Vec<(String, u32)>,
        ticks: u32,
    },
    /// A grace group: zero ticks, standing at the next note's page onset.
    Grace { pitches: Vec<String> },
}

/// Why one voice entry is placed the way it is: a leaning grace stands on a
/// later ordinary note and contributes no ticks; everything else — including
/// a grace with nothing to lean on — spells its own duration like a note.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    LeaningGrace,
    Note,
}

/// Classify every entry once: a grace with a strictly later non-grace entry
/// leans on that note; a grace at the end of the voice leans on nothing and
/// must spell its own duration.
fn roles(entries: &[WrittenEntry]) -> Vec<Role> {
    let mut roles = vec![Role::Note; entries.len()];
    let mut ordinary_seen = false;
    for (at, entry) in entries.iter().enumerate().rev() {
        if entry.grace {
            if ordinary_seen {
                roles[at] = Role::LeaningGrace;
            }
        } else {
            ordinary_seen = true;
        }
    }
    roles
}

/// Spell one voice's entries into bar bodies (tokens without the `|`).
///
/// `Err` carries the global note index of the first entry the written
/// vocabulary cannot place exactly; the caller declares that loss and ships no
/// source rather than one that rounds it. A zero-length bar is not a meter any
/// note can be placed against, so it defers the same way rather than silently
/// skipping the voice.
pub(crate) fn spell_voice(entries: &[WrittenEntry], bar_ticks: u32) -> Result<Vec<String>, usize> {
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    if bar_ticks == 0 {
        return Err(entries[0].index);
    }
    let atoms = atoms(entries)?;
    let trailing_blame = entries[entries.len() - 1].index;
    assemble(&atoms, bar_ticks, trailing_blame)
}

/// The page onset of each entry: graces donate their measured time to the
/// principal they lean on, so a note's page onset pulls back by the total
/// duration of the grace run immediately before it. Page entries for leaning
/// graces are unused (the grace stands on its principal's page onset).
fn page_onsets(entries: &[WrittenEntry], roles: &[Role]) -> Vec<u32> {
    let mut pages = vec![0_u32; entries.len()];
    let mut pending_grace = 0_u32;
    for (at, entry) in entries.iter().enumerate() {
        match roles[at] {
            Role::LeaningGrace => pending_grace = pending_grace.saturating_add(entry.duration_ticks),
            Role::Note => {
                pages[at] = entry.onset_ticks.saturating_sub(pending_grace);
                pending_grace = 0;
            }
        }
    }
    pages
}

/// Lay one voice's entries out as atoms: gap rests, graces, ordinary notes,
/// and bracketed ternary runs.
fn atoms(entries: &[WrittenEntry]) -> Result<Vec<Atom>, usize> {
    let roles = roles(entries);
    let pages = page_onsets(entries, &roles);
    let mut atoms = Vec::new();
    let mut run: Vec<(String, u32)> = Vec::new();
    let mut run_blame = 0_usize;
    let mut run_ticks = 0_u32;
    let mut run_end = 0_u32;
    let mut cursor = 0_u32;
    let mut index = 0_usize;

    while index < entries.len() {
        let entry = &entries[index];
        let page = pages[index];

        if roles[index] == Role::LeaningGrace {
            close_run(&mut atoms, &mut run, &mut run_ticks, run_blame);
            // The whole grace run leans on the next ordinary note; gather it,
            // then fill the gap up to that note's page onset and stand there
            // for free.
            let mut pitches = Vec::new();
            while index < entries.len() && roles[index] == Role::LeaningGrace {
                pitches.push(entries[index].pitch.clone());
                index += 1;
            }
            let principal = pages[index];
            if principal < cursor {
                return Err(entry.index);
            }
            push_gap(&mut atoms, principal.saturating_sub(cursor), entries[index].index).ok_or(entries[index].index)?;
            atoms.push(Atom::Grace { pitches });
            cursor = principal;
            continue;
        }

        if page < cursor {
            // Overlap within one voice is not a written form; defer here.
            return Err(entry.index);
        }
        if page > cursor {
            close_run(&mut atoms, &mut run, &mut run_ticks, run_blame);
            push_gap(&mut atoms, page - cursor, entry.index).ok_or(entry.index)?;
        }

        if let Some(denominator) = member_denominator(entry.duration_ticks) {
            // A ternary division: continue the open bracket when the run is
            // contiguous and this onset is not a realignment point; otherwise
            // close it and open a new one.
            if !run.is_empty() && page == run_end && !page.is_multiple_of(REALIGNMENT_TICKS) {
                run.push((entry.pitch.clone(), denominator));
                run_ticks += entry.duration_ticks;
            } else {
                close_run(&mut atoms, &mut run, &mut run_ticks, run_blame);
                run = vec![(entry.pitch.clone(), denominator)];
                run_blame = entry.index;
                run_ticks = entry.duration_ticks;
            }
            run_end = page.saturating_add(entry.duration_ticks);
        } else {
            close_run(&mut atoms, &mut run, &mut run_ticks, run_blame);
            atoms.push(Atom::Note {
                index: entry.index,
                pitch: entry.pitch.clone(),
                ticks: entry.duration_ticks,
            });
        }
        cursor = page.saturating_add(entry.duration_ticks);
        index += 1;
    }
    close_run(&mut atoms, &mut run, &mut run_ticks, run_blame);
    Ok(atoms)
}

/// Close the open ternary bracket into an atom, if one is open.
fn close_run(atoms: &mut Vec<Atom>, run: &mut Vec<(String, u32)>, run_ticks: &mut u32, run_blame: usize) {
    if !run.is_empty() {
        atoms.push(Atom::TupletNotes {
            blame: run_blame,
            members: std::mem::take(run),
            ticks: *run_ticks,
        });
        *run_ticks = 0;
    }
}

/// Decompose a gap into rest atoms: the binary/dotted greedy, then — when an
/// off-grid remainder of one or two ticks survives — fold the smallest greedy
/// pieces back into it until the tail is an even length one `tuplet 3/2` rest
/// group spells exactly. `None` when even that cannot place the gap (an odd
/// off-grid length). `blame` names the note the gap precedes, for deferral.
fn push_gap(atoms: &mut Vec<Atom>, gap: u32, blame: usize) -> Option<()> {
    let mut ticks = Vec::new();
    let mut remaining = gap;
    while remaining >= GRID_UNIT {
        let value = *VALUES.iter().find(|(value, _, _)| *value <= remaining)?;
        ticks.push(value.0);
        remaining -= value.0;
    }
    if remaining == 0 {
        for tick in ticks {
            atoms.push(Atom::Rest { ticks: tick, blame });
        }
        return Some(());
    }
    // remaining ∈ {1, 2}: fold pieces back until the tail is an even length a
    // tuplet rest group spells (at least a sixteenth member, 4 ticks, unless
    // the whole gap was smaller than that).
    let mut tail = remaining;
    while (tail == 1 || !tail.is_multiple_of(2) || tail < 4) && !ticks.is_empty() {
        tail += ticks.pop()?;
    }
    if tail < 2 || !tail.is_multiple_of(2) {
        return None;
    }
    for tick in ticks {
        atoms.push(Atom::Rest { ticks: tick, blame });
    }
    atoms.push(Atom::TupletRest { ticks: tail, blame });
    Some(())
}

/// The tuplet member denominator a tick length spells, when it is a single
/// member value (a binary length scaled by 2/3).
fn member_denominator(ticks: u32) -> Option<u32> {
    MEMBERS
        .iter()
        .find(|(member, _)| *member == ticks)
        .map(|(_, denominator)| *denominator)
}

/// The greedy tied chain of binary/dotted values summing exactly to `ticks`;
/// `None` unless `ticks` is a multiple of the grid unit.
fn spell_chain(ticks: u32) -> Option<Vec<(u32, bool)>> {
    if ticks == 0 {
        return Some(Vec::new());
    }
    if !ticks.is_multiple_of(GRID_UNIT) {
        return None;
    }
    let mut pieces = Vec::new();
    let mut remaining = ticks;
    while remaining > 0 {
        let (tick, denominator, dotted) = *VALUES.iter().find(|(tick, _, _)| *tick <= remaining)?;
        pieces.push((denominator, dotted));
        remaining -= tick;
    }
    Some(pieces)
}

/// Cut atoms into bars that each measure exactly `bar_ticks`, splitting notes
/// into tied halves and rests into re-spelled parts at the bar lines. `Err`
/// carries the note index of the atom a split landed on when the vocabulary
/// cannot follow it (an off-grid in-bar part, or a tuplet atom crossing the
/// line); `trailing_blame` names the note whose unfilled bar could not close.
fn assemble(atoms: &[Atom], bar_ticks: u32, trailing_blame: usize) -> Result<Vec<String>, usize> {
    let mut bars = Vec::new();
    let mut tokens: Vec<String> = Vec::new();
    let mut used = 0_u32;

    for atom in atoms {
        match atom {
            Atom::Grace { pitches } => {
                if used == bar_ticks {
                    flush(&mut bars, &mut tokens, &mut used);
                }
                tokens.push(format!("grace {{ {} }}", pitches.join(" ")));
            }
            Atom::TupletNotes { blame, members, ticks } => {
                if used.saturating_add(*ticks) > bar_ticks {
                    return Err(*blame);
                }
                let body = members
                    .iter()
                    .map(|(pitch, denominator)| format!("{pitch}/{denominator}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                tokens.push(format!("tuplet 3/2 {{ {body} }}"));
                used += ticks;
            }
            Atom::TupletRest { ticks, blame } => {
                if used.saturating_add(*ticks) > bar_ticks {
                    return Err(*blame);
                }
                tuplet_rest_tokens(*ticks, &mut tokens).ok_or(*blame)?;
                used += ticks;
            }
            Atom::Rest { ticks, blame } => {
                let mut remaining = *ticks;
                while remaining > 0 {
                    if used == bar_ticks {
                        flush(&mut bars, &mut tokens, &mut used);
                    }
                    let part = remaining.min(bar_ticks - used);
                    rest_tokens(part, &mut tokens).ok_or(*blame)?;
                    used += part;
                    remaining -= part;
                }
            }
            Atom::Note { index, pitch, ticks } => {
                let mut remaining = *ticks;
                while remaining > 0 {
                    if used == bar_ticks {
                        flush(&mut bars, &mut tokens, &mut used);
                    }
                    let part = remaining.min(bar_ticks - used);
                    note_tokens(pitch, part, remaining > part, &mut tokens).ok_or(*index)?;
                    used += part;
                    remaining -= part;
                }
            }
        }
    }

    // Fill the last bar so it measures what the meter says.
    if used < bar_ticks {
        let mut fill = Vec::new();
        push_gap(&mut fill, bar_ticks - used, trailing_blame).ok_or(trailing_blame)?;
        for atom in fill {
            match atom {
                Atom::Rest { ticks, blame } => {
                    rest_tokens(ticks, &mut tokens).ok_or(blame)?;
                    used += ticks;
                }
                Atom::TupletRest { ticks, blame } => {
                    tuplet_rest_tokens(ticks, &mut tokens).ok_or(blame)?;
                    used += ticks;
                }
                Atom::Note { .. } | Atom::TupletNotes { .. } | Atom::Grace { .. } => return Err(trailing_blame),
            }
        }
    }
    flush(&mut bars, &mut tokens, &mut used);
    Ok(bars)
}

/// Close the current bar, if it holds anything.
fn flush(bars: &mut Vec<String>, tokens: &mut Vec<String>, used: &mut u32) {
    if !tokens.is_empty() {
        bars.push(tokens.join(" "));
        tokens.clear();
    }
    *used = 0;
}

/// Append one note's tied chain, ending in `~` when the note continues.
fn note_tokens(pitch: &str, ticks: u32, tied_onward: bool, tokens: &mut Vec<String>) -> Option<()> {
    let chain = spell_chain(ticks)?;
    let last = chain.len().saturating_sub(1);
    for (piece, (denominator, dotted)) in chain.iter().enumerate() {
        let dot = if *dotted { "." } else { "" };
        tokens.push(format!("{pitch}/{denominator}{dot}"));
        if piece < last || tied_onward {
            tokens.push("~".to_owned());
        }
    }
    Some(())
}

/// Append one stretch of rests.
fn rest_tokens(ticks: u32, tokens: &mut Vec<String>) -> Option<()> {
    for (denominator, dotted) in spell_chain(ticks)? {
        let dot = if dotted { "." } else { "" };
        tokens.push(format!("rest/{denominator}{dot}"));
    }
    Some(())
}

/// Append one `tuplet 3/2` rest group spelling `ticks` exactly.
fn tuplet_rest_tokens(ticks: u32, tokens: &mut Vec<String>) -> Option<()> {
    let mut members = Vec::new();
    let mut remaining = ticks;
    for (tick, denominator) in MEMBERS {
        while remaining >= tick {
            remaining -= tick;
            members.push(format!("rest/{denominator}"));
        }
    }
    if remaining != 0 {
        return None;
    }
    tokens.push(format!("tuplet 3/2 {{ {} }}", members.join(" ")));
    Some(())
}

#[cfg(test)]
mod laws {
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    use super::*;

    fn entry(index: usize, onset: u32, duration: u32, pitch: &str) -> WrittenEntry {
        WrittenEntry {
            index,
            onset_ticks: onset,
            duration_ticks: duration,
            pitch: pitch.to_owned(),
            grace: duration < 5,
        }
    }

    fn line(entries: &[WrittenEntry]) -> Vec<String> {
        spell_voice(entries, 96).expect("the spelling must place these")
    }

    #[test]
    fn the_value_table_is_sorted_and_grounded_on_the_grid_unit() {
        // The greedy spellings read the table largest-first and stop at the
        // grid unit; a mis-sorted entry (a sixteenth before its dotted elder,
        // say) would silently decompose the dotted value into a tie.
        assert_eq!(VALUES[VALUES.len() - 1].0, GRID_UNIT);
        for pair in VALUES.windows(2) {
            assert!(pair[0].0 > pair[1].0, "VALUES must be strictly descending");
        }
        assert!(MEMBERS[MEMBERS.len() - 1].0 == 2, "the member table ends at its atom");
        for pair in MEMBERS.windows(2) {
            assert!(pair[0].0 > pair[1].0, "MEMBERS must be strictly descending");
        }
    }

    #[test]
    fn a_dotted_sixteenth_is_one_notehead_not_a_tie() {
        // 9 ticks is the dotted sixteenth the pedal corpus spells; the greedy
        // must take it as one value.
        let bars = line(&[entry(0, 0, 9, "c4"), entry(1, 9, 9, "d4")]);
        assert_eq!(bars[0], "c4/16. d4/16. rest/2. rest/16");
    }

    #[test]
    fn one_duration_has_one_spelling_wherever_it_falls() {
        // 30 ticks is a quarter tied to a sixteenth, and 18 a dotted eighth,
        // at every in-bar onset they can occupy without crossing the line.
        for onset in [0, 12, 24, 36] {
            let bars = line(&[entry(0, onset, 30, "c4"), entry(1, onset + 30, 18, "d4")]);
            assert!(bars[0].contains("c4/4 ~ c4/16 d4/8."), "onset {onset}: {}", bars[0]);
        }
    }

    #[test]
    fn ternary_runs_bracket_per_realignment_quarter() {
        let mut entries: Vec<WrittenEntry> = [0, 8, 16, 24, 32]
            .iter()
            .enumerate()
            .map(|(index, &onset)| entry(index, onset, 8, "c4"))
            .collect();
        entries.push(entry(5, 40, 24, "a4"));
        assert_eq!(
            line(&entries),
            vec!["tuplet 3/2 { c4/8 c4/8 c4/8 } tuplet 3/2 { c4/8 c4/8 } a4/4 rest/4 tuplet 3/2 { rest/8 }".to_owned(),]
        );
    }

    #[test]
    fn a_note_crossing_the_bar_line_is_two_tied_halves() {
        assert_eq!(
            line(&[entry(0, 84, 24, "a4")]),
            vec!["rest/2. rest/8 a4/8 ~".to_owned(), "a4/8 rest/2. rest/8".to_owned()]
        );
    }

    #[test]
    fn gaps_spell_greedily_and_fold_to_a_tuplet_tail() {
        // A 32-tick gap is rest/4 plus one triplet-eighth rest, never a wrong
        // binary sum.
        let entries = vec![entry(0, 0, 24, "c4"), entry(1, 56, 8, "e4"), entry(2, 64, 32, "g4")];
        let bars = line(&entries);
        assert!(bars[0].contains("rest/4 tuplet 3/2 { rest/8 }"), "{}", bars[0]);
    }

    #[test]
    fn an_odd_off_grid_length_is_a_deferral_not_a_rounding() {
        assert_eq!(spell_voice(&[entry(0, 0, 5, "c4")], 96), Err(0));
        assert_eq!(spell_voice(&[entry(0, 0, 7, "c4")], 96), Err(0));
    }

    #[test]
    fn graces_stand_on_their_principal_and_take_no_time() {
        let entries = vec![entry(0, 0, 24, "c4"), entry(1, 24, 3, "d4"), entry(2, 27, 24, "e4")];
        assert_eq!(line(&entries), vec!["c4/4 grace { d4 } e4/4 rest/2".to_owned()]);
    }

    #[test]
    fn a_trailing_grace_spells_its_own_duration() {
        // Nothing follows to lean on, so the 3-tick grace becomes a plain
        // thirty-second and the 4-tick one a single tuplet member.
        assert_eq!(
            line(&[entry(0, 0, 24, "c4"), entry(1, 24, 3, "d4")]),
            vec!["c4/4 d4/32 rest/2 rest/8. rest/32".to_owned()]
        );
        assert_eq!(
            line(&[entry(0, 0, 24, "c4"), entry(1, 24, 4, "d4")]),
            vec!["c4/4 tuplet 3/2 { d4/16 } rest/2 tuplet 3/2 { rest/4 rest/16 }".to_owned()]
        );
    }

    #[test]
    fn spelling_is_deterministic() {
        let entries: Vec<WrittenEntry> = [0, 8, 16, 24, 32, 40]
            .iter()
            .enumerate()
            .map(|(index, &onset)| entry(index, onset, if onset == 40 { 24 } else { 8 }, "c4"))
            .collect();
        assert_eq!(spell_voice(&entries, 96), spell_voice(&entries, 96));
    }
}
