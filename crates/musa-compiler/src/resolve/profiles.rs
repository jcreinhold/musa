#![allow(clippy::arithmetic_side_effects)]
use musa_syntax::ast::{AstNode as _, DynamicRule, MarkRule, PerformanceDecl, ProfileDecl, SettingStmt};
use num_rational::Ratio;

use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::SourceSpan;
use musa_score::profile::{ArticulationRealization, PerformanceProfile, ProfileSet};
use musa_score::score::DynamicMark;

use super::{Resolver, parse_ratio, suggest, trimmed_span};

/// Read the `performance` block into a [`ProfileSet`]. Declarations only —
/// nothing here is applied until `lower_performance` (roadmap §6.4).
pub(crate) fn parse_profiles(resolver: &mut Resolver, performance: &PerformanceDecl) -> ProfileSet {
    let mut set = ProfileSet::default();
    for declaration in performance.profiles() {
        let name = declaration.name().unwrap_or_default();
        if set.declares(&name) {
            resolver.error(
                Code::DuplicateName,
                format!("profile `{name}` is declared twice"),
                trimmed_span(declaration.syntax()),
                "declared again here",
            );
            continue;
        }
        let mut profile = PerformanceProfile::named(&name);
        for rule in declaration.marks() {
            let written = rule.name().unwrap_or_default();
            let Some(mark) = musa_score::Mark::parse(&written) else {
                resolver.report(
                    Diagnostic::error(Code::UnknownWord, format!("`{written}` is not a mark"))
                        .at(trimmed_span(rule.syntax()), "unknown mark")
                        .help(suggest(&written, &musa_score::marks::names(), "marks")),
                );
                continue;
            };
            profile.set_mark(mark, mark_settings(resolver, &rule));
        }
        for rule in declaration.dynamics() {
            let written = rule.name().unwrap_or_default();
            let Some(mark) = DynamicMark::parse(&written) else {
                resolver.report(
                    Diagnostic::error(Code::UnknownWord, format!("`{written}` is not a dynamic marking"))
                        .at(trimmed_span(rule.syntax()), "unknown marking")
                        .help(suggest(&written, DynamicMark::NAMES, "markings")),
                );
                continue;
            };
            if let Some(amplitude) = dynamic_settings(resolver, &rule) {
                profile.set_dynamic(mark, amplitude);
            }
        }
        if let Some((groove, span)) = groove_of(resolver, &declaration) {
            if !groove.is_straight() {
                resolver.groove_rules.push((profile.name().to_owned(), span));
            }
            profile.set_groove(groove);
        }
        if let Some(grace) = grace_of(resolver, &declaration) {
            profile.set_grace(grace);
        }
        set.insert(profile);
    }
    set
}

/// The one grace reading a profile declares, if it declares one.
///
/// More than one is refused for the same reason two grooves are: a reading of
/// a grace note is a single decision, and two of them composed in written
/// order would mean nothing a performer could act on.
fn grace_of(resolver: &mut Resolver, declaration: &ProfileDecl) -> Option<musa_score::GracePolicy> {
    let rules = declaration.graces();
    let (first, rest) = rules.split_first()?;
    for extra in rest {
        resolver.error(
            Code::DuplicateName,
            "this profile has more than one grace rule",
            trimmed_span(extra.syntax()),
            "a reading plays a grace note one way",
        );
    }
    let mut policy = musa_score::GracePolicy::DEFAULT;
    for setting in first.settings() {
        let name = setting.name().unwrap_or_default();
        match name.as_str() {
            "steal" => {
                if let Some(steal) = beat_setting(resolver, &setting) {
                    if steal <= Ratio::ZERO {
                        resolver.error(
                            Code::OutOfRange,
                            "`steal` is not a length",
                            trimmed_span(setting.syntax()),
                            "zero or less",
                        );
                    } else {
                        policy.steal = steal;
                    }
                }
            }
            "from" => {
                if let Some(from) = steal_from(resolver, &setting) {
                    policy.from = from;
                }
            }
            other => resolver.report(
                Diagnostic::error(
                    Code::UnknownWord,
                    format!("a grace rule has no setting called `{other}`"),
                )
                .at(trimmed_span(setting.syntax()), "unknown setting")
                .help(suggest(other, &["steal", "from"], "settings")),
            ),
        }
    }
    Some(policy)
}

/// `from = principal;` or `from = previous;`.
fn steal_from(resolver: &mut Resolver, setting: &SettingStmt) -> Option<musa_score::StealFrom> {
    let written = setting.word().or_else(|| setting.value())?;
    match written.as_str() {
        "principal" => Some(musa_score::StealFrom::Principal),
        "previous" => Some(musa_score::StealFrom::Previous),
        other => {
            resolver.report(
                Diagnostic::error(Code::UnknownWord, format!("`{other}` is not a note to steal from"))
                    .at(trimmed_span(setting.syntax()), "unknown source")
                    .help(suggest(other, &["principal", "previous"], "sources"))
                    .note("a grace note takes its time from the note it leans on, or from the one before it"),
            );
            None
        }
    }
}

/// The one groove a profile declares, if it declares one.
///
/// More than one is refused rather than merged: a part has one beat, and two
/// grooves composed in written order would mean something no musician asked
/// for.
fn groove_of(resolver: &mut Resolver, declaration: &ProfileDecl) -> Option<(musa_score::Groove, SourceSpan)> {
    let rules = declaration.grooves();
    let (first, rest) = rules.split_first()?;
    for extra in rest {
        resolver.error(
            Code::DuplicateName,
            "this profile has more than one groove",
            trimmed_span(extra.syntax()),
            "a part has one beat",
        );
    }
    let written = first.name().unwrap_or_default();
    let Some(def) = musa_score::groove::lookup(&written) else {
        resolver.report(
            Diagnostic::error(Code::UnknownWord, format!("`{written}` is not a groove"))
                .at(trimmed_span(first.syntax()), "unknown groove")
                .help(suggest(&written, &musa_score::groove::names(), "grooves")),
        );
        return None;
    };
    let mut values: indexmap::IndexMap<String, Ratio<i64>> = indexmap::IndexMap::new();
    for setting in first.settings() {
        let name = setting.name().unwrap_or_default();
        if !def.params.contains(&name.as_str()) {
            resolver.report(
                Diagnostic::error(
                    Code::UnknownWord,
                    format!("`{}` has no setting called `{name}`", def.name),
                )
                .at(trimmed_span(setting.syntax()), "unknown setting")
                .help(suggest(&name, def.params, "settings")),
            );
            continue;
        }
        if let Some(value) = beat_setting(resolver, &setting) {
            values.insert(name, value);
        }
    }
    let mut required = |param: &str| match values.get(param) {
        Some(value) => Some(*value),
        None => {
            resolver.report(
                Diagnostic::error(Code::NotAValue, format!("`{}` needs a `{param}`", def.name))
                    .at(trimmed_span(first.syntax()), format!("no `{param}`"))
                    .help(example_of(def.name)),
            );
            None
        }
    };
    let groove = match def.name {
        "straight" => Some(musa_score::Groove::STRAIGHT),
        "swing" => musa_score::Groove::swing(required("ratio")?),
        "push" => musa_score::Groove::push(required("grid")?, required("by")?),
        other => {
            // `VOCABULARY` and this match are the same list. A row added to
            // one and not the other would be a groove the parser accepts and
            // the compiler quietly ignores, so it is reported rather than
            // dropped on the floor.
            resolver.error(
                Code::UnknownWord,
                format!("`{other}` is a groove musa does not know how to build"),
                trimmed_span(first.syntax()),
                "not implemented",
            );
            None
        }
    };
    if groove.is_none() {
        resolver.report(
            Diagnostic::error(
                Code::OutOfRange,
                format!("this `{}` cannot be laid over the beat", def.name),
            )
            .at(trimmed_span(first.syntax()), "out of range")
            .note("a groove displaces the beat inside a cell it never leaves: it may not move a note past its neighbour, and its cell must tile the whole note or the downbeat it fixes would drift from bar to bar")
            .help(example_of(def.name)),
        );
    }
    groove.map(|groove| (groove, trimmed_span(first.syntax())))
}

/// What a groove looks like written correctly, for the diagnostics above.
fn example_of(name: &str) -> &'static str {
    match name {
        "swing" => "write `groove swing { ratio = 2/3; }` — the first of each pair takes two thirds",
        "push" => "write `groove push { grid = 1/8; by = -1/64; }` — the offbeat eighths land early",
        _ => "write `groove straight {}`",
    }
}

/// A position or a length in whole notes, exact and possibly negative: a
/// swing ratio, a grid, or a displacement. Not a gate, so `0..=1` is not the
/// range, and not a time, so a unit is a category error.
fn beat_setting(resolver: &mut Resolver, setting: &SettingStmt) -> Option<Ratio<i64>> {
    let name = setting.name().unwrap_or_default();
    let span = trimmed_span(setting.syntax());
    if let Some(unit) = setting.unit() {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, format!("`{name}` does not take a unit"))
                .at(span, format!("drop the `{unit}`"))
                .note("a groove is written in beats, so it survives a tempo change"),
        );
        return None;
    }
    // A word reaches here too, so a setting given the wrong kind of value
    // is refused by name rather than dropped without a word (see `word`).
    let written = setting.value().or_else(|| setting.word())?;
    let (sign, magnitude) = written
        .strip_prefix('-')
        .map_or((Ratio::ONE, written.as_str()), |rest| (-Ratio::ONE, rest));
    let value = parse_ratio(magnitude).or_else(|| musa_score::profile::parse_decimal(magnitude));
    match value {
        Some(value) => Some(value * sign),
        None => {
            resolver.error(
                Code::NotAValue,
                format!("`{written}` is not a beat value"),
                span,
                "not a ratio",
            );
            None
        }
    }
}

/// `gate` (a ratio of the written value) and `attack` (a time).
fn mark_settings(resolver: &mut Resolver, rule: &MarkRule) -> ArticulationRealization {
    let mut realization = ArticulationRealization::NEUTRAL;
    for setting in rule.settings() {
        let name = setting.name().unwrap_or_default();
        match name.as_str() {
            "gate" => {
                if let Some(gate) = ratio_setting(resolver, &setting) {
                    realization.gate = gate;
                }
            }
            "attack" => {
                if let Some(attack) = time_setting(resolver, &setting) {
                    realization.attack = attack;
                }
            }
            "hold" => {
                if let Some(hold) = hold_setting(resolver, &setting) {
                    realization.hold = hold;
                }
            }
            other => resolver.report(
                Diagnostic::error(Code::UnknownWord, format!("a mark has no setting called `{other}`"))
                    .at(trimmed_span(setting.syntax()), "unknown setting")
                    .help(suggest(other, &["gate", "attack", "hold"], "settings")),
            ),
        }
    }
    realization
}

/// `amplitude` — abstract loudness, not decibels (§2).
fn dynamic_settings(resolver: &mut Resolver, rule: &DynamicRule) -> Option<Ratio<i64>> {
    let mut amplitude = None;
    for setting in rule.settings() {
        let name = setting.name().unwrap_or_default();
        if name == "amplitude" {
            amplitude = ratio_setting(resolver, &setting);
        } else {
            resolver.report(
                Diagnostic::error(Code::UnknownWord, format!("a dynamic has no setting called `{name}`"))
                    .at(trimmed_span(setting.syntax()), "unknown setting")
                    .help("a dynamic rule sets `amplitude`, and nothing else"),
            );
        }
    }
    if amplitude.is_none() {
        resolver.report(
            Diagnostic::error(Code::NotAValue, "this dynamic rule says nothing")
                .at(trimmed_span(rule.syntax()), "no amplitude")
                .help("give it an amplitude, like `amplitude = 0.6;`")
                .note("amplitude is abstract loudness between 0 and 1, not decibels"),
        );
    }
    amplitude
}

/// A multiplier of at least one: how much longer than written a note is held.
///
/// Not [`ratio_setting`], and the difference is the whole point of the
/// setting. A gate is a *fraction* of the written value and so lives in
/// `0..=1`; a hold is a *multiple* of it, and a fermata that lasts twice as
/// long is the ordinary case. Reusing the gate's reader would make `hold = 2`
/// — the one value anybody writes first — an error.
///
/// Written as a ratio or as a decimal, like a groove's beat: `hold = 2/1` and
/// `hold = 1.5` are both a person saying the same kind of thing.
fn hold_setting(resolver: &mut Resolver, setting: &SettingStmt) -> Option<Ratio<i64>> {
    let name = setting.name().unwrap_or_default();
    let span = trimmed_span(setting.syntax());
    if let Some(unit) = setting.unit() {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, format!("`{name}` does not take a unit"))
                .at(span, format!("drop the `{unit}`"))
                .note("a hold is a multiple of the written value, not a length of time: it survives a tempo change"),
        );
        return None;
    }
    // A word reaches here too, so a setting given the wrong kind of value
    // is refused by name rather than dropped without a word (see `word`).
    let written = setting.value().or_else(|| setting.word())?;
    let Some(value) = parse_ratio(&written).or_else(|| musa_score::profile::parse_decimal(&written)) else {
        resolver.error(
            Code::NotAValue,
            format!("`{written}` is not a hold"),
            span,
            "not a number",
        );
        return None;
    };
    if value < Ratio::ONE {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, format!("`{name}` is less than 1"))
                .at(span, "shorter than written")
                .note("a hold lengthens a note; to shorten one, write a `gate`"),
        );
        return None;
    }
    Some(value)
}

/// A unitless ratio in `0..=1`. A unit here is a category error: a gate is a
/// fraction of the written value, not a length of time.
fn ratio_setting(resolver: &mut Resolver, setting: &SettingStmt) -> Option<Ratio<i64>> {
    let name = setting.name().unwrap_or_default();
    let span = trimmed_span(setting.syntax());
    if let Some(unit) = setting.unit() {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, format!("`{name}` does not take a unit"))
                .at(span, format!("drop the `{unit}`"))
                .note(format!(
                    "`{name}` is a fraction of the written value, not a length of time"
                )),
        );
        return None;
    }
    // Both spellings, and a value that will not parse is reported rather than
    // dropped. `gate = 1/2` used to resolve to nothing at all — a half-length
    // staccato that compiled clean and performed at full length.
    // A word reaches here too, so a setting given the wrong kind of value
    // is refused by name rather than dropped without a word (see `word`).
    let written = setting.value().or_else(|| setting.word())?;
    let Some(value) = parse_ratio(&written).or_else(|| musa_score::profile::parse_decimal(&written)) else {
        resolver.error(
            Code::NotAValue,
            format!("`{written}` is not a fraction of the written value"),
            span,
            "not a number",
        );
        return None;
    };
    if value < Ratio::ZERO || value > Ratio::ONE {
        resolver.error(
            Code::OutOfRange,
            format!("`{name}` is outside 0 to 1"),
            span,
            "out of range",
        );
        return None;
    }
    Some(value)
}

/// A time in seconds, written with its unit (roadmap §7.2: units are syntax).
fn time_setting(resolver: &mut Resolver, setting: &SettingStmt) -> Option<Ratio<i64>> {
    let name = setting.name().unwrap_or_default();
    let span = trimmed_span(setting.syntax());
    let written = setting.value().or_else(|| setting.word())?;
    let Some(value) = musa_score::profile::parse_decimal(&written) else {
        resolver.error(
            Code::NotAValue,
            format!("`{written}` is not a length of time"),
            span,
            "not a number",
        );
        return None;
    };
    let seconds = match setting.unit().as_deref() {
        Some("ms") => value / 1000,
        Some("s") => value,
        _ => {
            resolver.report(
                Diagnostic::error(Code::OutOfRange, format!("`{name}` is a length of time"))
                    .at(span, "no unit here")
                    .help(format!("write `{name} = 30 ms` or `{name} = 0.03 s`")),
            );
            return None;
        }
    };
    if seconds < Ratio::ZERO {
        resolver.error(
            Code::OutOfRange,
            format!("`{name}` cannot be negative"),
            span,
            "below zero",
        );
        return None;
    }
    Some(seconds)
}
