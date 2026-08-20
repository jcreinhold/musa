//! See `ast` module docs; the items parsed in this family.

use super::children;
use super::rule_wrapper;
use super::token_text;
use super::wrapper;
use crate::SyntaxKind;
use crate::language::{SyntaxElement, SyntaxNode};

/// `performance { profile violin { ... } }` — how marks are realized.
pub struct PerformanceDecl(SyntaxNode);
wrapper!(PerformanceDecl, SyntaxKind::PerformanceDecl);

impl PerformanceDecl {
    /// The declared profiles, in source order.
    pub fn profiles(&self) -> Vec<ProfileDecl> {
        children(&self.0)
    }
}

/// `profile violin { mark staccato { ... } dynamic p { ... } }`
pub struct ProfileDecl(SyntaxNode);
wrapper!(ProfileDecl, SyntaxKind::ProfileDecl);

impl ProfileDecl {
    /// The profile name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The mark rules, in source order.
    pub fn marks(&self) -> Vec<MarkRule> {
        children(&self.0)
    }

    /// The dynamic rules, in source order.
    pub fn dynamics(&self) -> Vec<DynamicRule> {
        children(&self.0)
    }

    /// The groove rules, in source order. At most one is meaningful — a part
    /// has one beat — and the compiler says so when there are more.
    pub fn grooves(&self) -> Vec<GrooveRule> {
        children(&self.0)
    }

    /// The grace rules, in source order. At most one is meaningful, for the
    /// same reason a groove is.
    pub fn graces(&self) -> Vec<GraceRule> {
        children(&self.0)
    }
}

rule_wrapper!(MarkRule, SyntaxKind::MarkRule, "mark");
rule_wrapper!(DynamicRule, SyntaxKind::DynamicRule, "dynamic");

rule_wrapper!(GrooveRule, SyntaxKind::GrooveRule, "groove");

/// `grace { steal = 1/16; from = principal; }` inside a profile.
///
/// No `name`: a profile has one reading of a grace note, so there is nothing
/// for a name to choose between.
pub struct GraceRule(SyntaxNode);

wrapper!(GraceRule, SyntaxKind::GraceRule);

impl GraceRule {
    /// The rule's settings, in source order.
    pub fn settings(&self) -> Vec<SettingStmt> {
        children(&self.0)
    }
}

/// `gate = 0.55;`, `attack = 8 ms;`
pub struct SettingStmt(SyntaxNode);
wrapper!(SettingStmt, SyntaxKind::SettingStmt);

impl SettingStmt {
    /// The setting name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The written number with its sign, without its unit.
    ///
    /// The minus is a token of its own, so it is put back here rather than
    /// left for each reader to look for: a value is one string either way.
    pub fn value(&self) -> Option<String> {
        let magnitude = token_text(&self.0, SyntaxKind::Float)
            .or_else(|| token_text(&self.0, SyntaxKind::Integer))
            .or_else(|| token_text(&self.0, SyntaxKind::Rational))?;
        Some(match token_text(&self.0, SyntaxKind::Minus) {
            Some(_) => format!("-{magnitude}"),
            None => magnitude,
        })
    }

    /// The written word, for a setting whose value is a named reading rather
    /// than a quantity (`from = principal;`).
    ///
    /// Read as the identifier *after* the `=`, because the setting's own name
    /// is an identifier too and is the one before it.
    pub fn word(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .skip_while(|token| token.kind() != SyntaxKind::Equals)
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .map(|token| token.text().to_string())
    }

    /// The unit written after the number (`ms`, `s`), if any.
    pub fn unit(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::UnitMs).or_else(|| token_text(&self.0, SyntaxKind::UnitS))
    }
}
