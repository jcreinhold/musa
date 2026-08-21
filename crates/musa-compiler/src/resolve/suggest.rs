#![allow(clippy::arithmetic_side_effects)]

use crate::diagnose::nearest;

/// "did you mean", or the list, or nothing.
///
/// One suggestion when one candidate stands out; otherwise the vocabulary
/// itself, which for a closed set is short enough to print and is what the
/// reader actually needs. A list of more than six is neither, and says so.
pub(crate) fn suggest(written: &str, known: &[&str], plural: &str) -> String {
    if let Some(near) = nearest(written, known.iter().copied()) {
        return format!("did you mean `{near}`?");
    }
    if known.is_empty() {
        return format!("this piece declares no {plural}");
    }
    if known.len() > 6 {
        return format!("run `musa explain unknown-word` for the {plural} musa reads");
    }
    let quoted: Vec<String> = known.iter().map(|name| format!("`{name}`")).collect();
    format!("musa reads {}", quoted.join(", "))
}

/// The same, for a name the *composer* chose rather than a word musa knows.
///
/// The difference is whose vocabulary is at fault. `musa reads f, mf, p` is
/// the right answer for a misspelled dynamic and the wrong one for a
/// misspelled motif, where the list is the piece's own.
pub(crate) fn suggest_name(written: &str, known: &[&str]) -> String {
    if let Some(near) = nearest(written, known.iter().copied()) {
        return format!("did you mean `{near}`?");
    }
    if known.is_empty() {
        return "this piece declares no motifs and no bars".to_owned();
    }
    if known.len() > 6 {
        return "check the spelling against the declaration".to_owned();
    }
    let quoted: Vec<String> = known.iter().map(|name| format!("`{name}`")).collect();
    format!("this piece declares {}", quoted.join(", "))
}
