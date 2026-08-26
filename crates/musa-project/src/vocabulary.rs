//! The checked edition-pinned studio vocabulary shared by application readers.
//!
//! Compilation and exact decoding happen once per process. The cached value is
//! an opaque projection of ordinary bundled Musa source; it is not a second
//! construction API and carries the complete checked source identity.

use std::sync::OnceLock;

static VOCABULARY: OnceLock<Result<musa_dsp::StudioVocabulary, String>> = OnceLock::new();
static INSTRUMENTS: OnceLock<Result<musa_dsp::InstrumentContracts, String>> = OnceLock::new();

/// Read the standard studio vocabulary compiled from `std::sound::catalogue`.
///
/// # Errors
///
/// Returns a stable description when the bundled declaration tree fails to
/// check or its exact artifact disagrees with the consumer schema. Such a
/// failure is a build defect and is held so repeated editor requests do not
/// repeatedly compile it.
pub fn standard_studio_vocabulary() -> Result<&'static musa_dsp::StudioVocabulary, &'static str> {
    VOCABULARY
        .get_or_init(|| {
            let checked = musa_compiler::checked_standard_studio_vocabulary().map_err(|diagnostics| {
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            })?;
            musa_dsp::decode_studio_vocabulary(&checked).map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(String::as_str)
}

/// Read the standard instrument contracts projected from checked Musa source.
///
/// The projection is cached for tooling and preparation readers. It exposes
/// no constructor, default, or inference rule; source remains the declaration
/// and exact identity.
///
/// # Errors
///
/// Returns a stable description when the bundled declarations fail to check
/// or the exact artifact disagrees with the read-only consumer schema.
pub fn standard_instrument_contracts() -> Result<&'static musa_dsp::InstrumentContracts, &'static str> {
    INSTRUMENTS
        .get_or_init(|| {
            let checked = musa_compiler::checked_standard_instruments().map_err(|diagnostics| {
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            })?;
            musa_dsp::decode_instrument_contracts(&checked).map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(String::as_str)
}

/// Format an exact studio quantity for musician-facing facts and help.
/// Terminating rationals use decimals; all others retain exact fraction form.
#[must_use]
pub fn format_studio_ratio(value: num_rational::Ratio<i64>) -> String {
    let denominator = *value.denom();
    let mut reduced = denominator;
    while reduced % 2 == 0 {
        reduced /= 2;
    }
    while reduced % 5 == 0 {
        reduced /= 5;
    }
    if reduced != 1 {
        return value.to_string();
    }
    let numerator = i128::from(*value.numer());
    let denominator = i128::from(denominator);
    let negative = numerator < 0;
    let numerator = numerator.abs();
    let Some(whole) = numerator.checked_div(denominator) else {
        return value.to_string();
    };
    let Some(mut remainder) = numerator.checked_rem(denominator) else {
        return value.to_string();
    };
    if remainder == 0 {
        return format!("{}{whole}", if negative { "-" } else { "" });
    }
    let mut fraction = String::new();
    while remainder != 0 {
        let Some(scaled) = remainder.checked_mul(10) else {
            return value.to_string();
        };
        let Some(digit) = scaled.checked_div(denominator) else {
            return value.to_string();
        };
        fraction.push(char::from_digit(u32::try_from(digit).unwrap_or(0), 10).unwrap_or('0'));
        let Some(next) = scaled.checked_rem(denominator) else {
            return value.to_string();
        };
        remainder = next;
    }
    format!("{}{}.{}", if negative { "-" } else { "" }, whole, fraction)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use std::fmt::Write as _;

    use super::*;

    fn reference() -> Result<String, String> {
        let vocabulary = standard_studio_vocabulary().map_err(str::to_owned)?;
        musa_dsp::check_studio_vocabulary(vocabulary).map_err(|error| error.to_string())?;
        let mut out = String::from(
            "# Studio vocabulary\n\nThis page is generated from the checked ordinary Musa value `std::sound::catalogue::studio_vocabulary`. Source declarations own names, documentation, exact written domains, defaults, ranges, and examples; the native registry contributes only checked primitive support facts. Production studio values and source-owned defaults are declared by `std::sound::production`.\n\n",
        );
        for processor in vocabulary.processors() {
            let primitives = processor
                .primitives()
                .iter()
                .map(|primitive| format!("{}@{}", primitive.id(), primitive.version()))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(
                out,
                "## `{}`\n\n{} {}\n\n- Signature: `{}`\n- Role: {}\n- Origin: `std::sound::catalogue`\n- Schema: version {}\n- Registered primitive support: `{}`\n- Example: `{}`\n",
                processor.name(),
                processor.summary(),
                processor.note(),
                processor.signature(),
                processor.role().label(),
                vocabulary.schema_version(),
                if primitives.is_empty() {
                    "source composition"
                } else {
                    &primitives
                },
                processor.example(),
            );
            let ports = processor
                .ports()
                .iter()
                .map(|ports| {
                    let inputs = if ports.inputs().is_empty() {
                        "()".to_owned()
                    } else {
                        ports
                            .inputs()
                            .iter()
                            .map(|port| port.label())
                            .collect::<Vec<_>>()
                            .join(" × ")
                    };
                    format!("{inputs} -> {}", ports.output().label())
                })
                .collect::<Vec<_>>()
                .join(" or ");
            let _ = writeln!(out, "- Ports: `{ports}`\n");
            if !processor.parameters().is_empty() {
                out.push_str(
                    "| Parameter | Meaning | Unit | Default | Written range |\n| --- | --- | --- | ---: | ---: |\n",
                );
                for parameter in processor.parameters() {
                    let _ = writeln!(
                        out,
                        "| `{}` | {} | `{}` | {} | {}–{} |",
                        parameter.name(),
                        parameter.summary(),
                        parameter.default().unit().spelling().unwrap_or("Ratio"),
                        format_studio_ratio(*parameter.default().magnitude()),
                        format_studio_ratio(*parameter.minimum().magnitude()),
                        format_studio_ratio(*parameter.maximum().magnitude()),
                    );
                }
                out.push('\n');
            }
        }
        out.push_str("## Studio concepts\n\n");
        for term in vocabulary.terms() {
            let _ = writeln!(
                out,
                "### `{}`\n\n{} {}\n\n- Shape: `{}`\n- Origin: `std::sound::catalogue`\n- Example: `{}`\n",
                term.spelling(),
                term.summary(),
                term.note(),
                term.signature(),
                term.example(),
            );
        }
        out.push_str("## Standard instruments\n\nThese public contracts are projected from the checked `std::sound::instrument::standard_instruments` value. Private machine bodies remain implementation facts.\n\n");
        let instruments = standard_instrument_contracts().map_err(str::to_owned)?;
        for instrument in instruments.declarations() {
            let channels = match instrument.channels() {
                1 => "mono".to_owned(),
                2 => "stereo".to_owned(),
                channels => format!("{channels} channels"),
            };
            let _ = writeln!(
                out,
                "### `{}`\n\n{}\n\n- Signature: `{}`\n- Output: {channels}\n- Origin: `std::sound::instrument`\n",
                instrument.declaration_id(),
                instrument.summary(),
                instrument.name(),
            );
            if !instrument.controls().is_empty() {
                out.push_str("| Exposed control | Kind | Rate | Source default |\n| --- | --- | --- | --- |\n");
                for control in instrument.controls() {
                    let default = control.default_ratio().map_or_else(
                        || control.default_symbol().unwrap_or("source value").to_owned(),
                        format_studio_ratio,
                    );
                    let _ = writeln!(
                        out,
                        "| `{}::{}` | `{}` | `{}` | `{default}` |",
                        control.namespace(),
                        control.name(),
                        control.kind(),
                        control.update_rate(),
                    );
                }
                out.push('\n');
            }
            if !instrument.techniques().is_empty() {
                out.push_str("Techniques:\n\n");
                for technique in instrument.techniques() {
                    let fallback = if technique.notation_only_warning() {
                        "notation-only warning"
                    } else {
                        "required"
                    };
                    let _ = writeln!(out, "- `{}::{}` — {fallback}", technique.namespace(), technique.name());
                }
                out.push('\n');
            }
        }
        out.push_str("## Foreign-format support\n\nForeign formats are registered host adapters into the source-declared sample-map contract. This table is generated from the same adapter facts editor hover reads; it is not a Musa surface catalogue.\n\n");
        for support in crate::format_supports() {
            let _ = writeln!(
                out,
                "### `{}` — {}\n\n{}\n\n- Checked result: `{}`\n- Specification: <{}>\n- Accepted names/categories: {}\n- Named refusal classes: {}\n",
                support.adapter,
                support.format,
                support.summary,
                support.result,
                support.specification,
                support
                    .accepted_names
                    .iter()
                    .map(|name| format!("`{name}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
                support.refused.join("; "),
            );
        }
        Ok(out)
    }

    #[test]
    fn checked_in_studio_reference_is_derived_from_executable_source() {
        let generated = reference().expect("the checked studio vocabulary renders");
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/book/src/reference/studio-vocabulary.md"
        );
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(path, &generated).expect("write studio vocabulary reference");
        }
        assert_eq!(
            generated,
            include_str!("../../../docs/book/src/reference/studio-vocabulary.md"),
            "re-run the project vocabulary test with UPDATE_FIXTURES=1"
        );
    }
}
