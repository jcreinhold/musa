//! Signature help: what the call being written takes, and which argument the
//! caret is on.
//!
//! Two vocabularies answer, and both are authoritative. A call to a declared
//! name is answered from the session's item facts — the parameters the checker
//! lowered, with the defaults it will apply — so an editor cannot show a
//! signature the compiler disagrees with. A claim inside `assert` is answered
//! from the compiler's own closed claim registry, which is the only place that
//! knows a claim exists.
//!
//! The *call* is read from the lossless tree by
//! [`features::call`](crate::features::call), and must be: signature help is
//! asked for while the call is half-written, when there is no valid compile to
//! describe it. No expression is evaluated and no type is inferred here.

use lsp_types::{
    Documentation, MarkupContent, MarkupKind, ParameterInformation, ParameterLabel, Position, SignatureHelp,
    SignatureInformation,
};

use crate::workspace::Document;

/// The signature of the call the caret is inside, when it names something.
pub(crate) fn signature_help(document: &Document, position: Position) -> Option<SignatureHelp> {
    let byte = document.lines().byte(position);
    let snapshot = document.snapshot();
    let parsed = musa_syntax::parse(snapshot.source());
    let call = super::call::at(&parsed.syntax(), byte)?;
    let signature = declared(&snapshot, &call.name).or_else(|| claimed(&call.name))?;
    // Clamped to the signature's own length: a caller who wrote one comma too
    // many is past the end, and highlighting a parameter that does not exist
    // would answer a question the signature cannot.
    let active = call.argument.min(signature.parameters.as_ref().map_or(0, Vec::len));
    Some(SignatureHelp {
        // One signature: a name means one declaration in Musa's flat
        // namespace, so a list to choose from would be a list of one.
        active_parameter: Some(u32::try_from(active).unwrap_or(0)),
        signatures: vec![signature],
        active_signature: Some(0),
    })
}

/// The signature of a name the document declares or imports.
fn declared(snapshot: &musa_project::ProjectSnapshot<'_>, name: &str) -> Option<SignatureInformation> {
    let item = super::items::named(snapshot, name)?;
    Some(SignatureInformation {
        label: item.signature.clone(),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: super::items::markdown(item),
        })),
        parameters: Some(
            item.parameters
                .iter()
                .map(|parameter| ParameterInformation {
                    label: ParameterLabel::Simple(parameter.label.clone()),
                    documentation: parameter.ty.distinction.as_ref().map(|distinction| {
                        Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!("`{}` — {distinction}", parameter.ty.name),
                        })
                    }),
                })
                .collect(),
        ),
        active_parameter: None,
    })
}

/// The signature of one of the six claims an `assert` may write.
fn claimed(name: &str) -> Option<SignatureInformation> {
    let claim = musa_project::assertion_claims().find(|claim| claim.name == name)?;
    Some(SignatureInformation {
        label: claim.signature.clone(),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("**{}** — *claim*\n\n{}.", claim.name, claim.checks),
        })),
        parameters: Some(
            claim
                .parameters
                .iter()
                .map(|parameter| ParameterInformation {
                    label: ParameterLabel::Simple((*parameter).to_owned()),
                    documentation: None,
                })
                .collect(),
        ),
        active_parameter: None,
    })
}
