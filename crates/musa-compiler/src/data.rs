//! Nominal data a library declares for itself.
//!
//! The declarations themselves now live in `musa-core`'s family registry;
//! what remains here is the identity a `Type::Nominal` variant still carries.
//!
//! Nothing here is public. A declaration's identity is *build-local*: it is
//! derived from where the declaration resolved, never from a digest of the
//! text, so editing a comment above a declaration does not make a new type and
//! two packages that both declare `Motive` do not share one.

/// The name of the fold a declaration generates: `motive_fold` for `Motive`,
/// `chord_shape_fold` for `ChordShape`.
///
/// The shape is `nat_fold`, `option_fold`, `list_fold_from_end` — the
/// catamorphisms the language already had — because a generated fold *is* one
/// of those, and a reader who knows one should not have to learn a second
/// convention. A generated fold has no direction to name: a case sees its
/// group-member fields already folded, one constructor layer at a time, which
/// is what "replaces one constructor layer" means. Only `list` has two names,
/// because only `list` nests its constructors against its element order.
pub(crate) fn fold_name(ty: &str) -> String {
    let mut out = String::with_capacity(ty.len().saturating_add(6));
    for (index, character) in ty.chars().enumerate() {
        if character.is_uppercase() && index > 0 {
            out.push('_');
        }
        out.extend(character.to_lowercase());
    }
    out.push_str("_fold");
    out
}
