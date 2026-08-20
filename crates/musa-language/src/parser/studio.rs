//! The studio graph: patches, buses, chains, stages, and performance profiles.

use super::engine::Parser;
use crate::SyntaxKind;

const PERFORMANCE_RECOVERY: &[SyntaxKind] = &[SyntaxKind::RBrace, SyntaxKind::ProfileKw];
const PROFILE_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::Semicolon,
    SyntaxKind::RBrace,
    SyntaxKind::MarkKw,
    SyntaxKind::DynamicKw,
    SyntaxKind::GrooveKw,
    SyntaxKind::GraceKw,
];
const STUDIO_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::Semicolon,
    SyntaxKind::RBrace,
    SyntaxKind::PatchKw,
    SyntaxKind::BusKw,
    SyntaxKind::ModulateKw,
    SyntaxKind::AssignKw,
    SyntaxKind::RouteKw,
    SyntaxKind::SendKw,
];

impl Parser<'_> {
    /// `studio { patch ... bus ... assign ... }` (roadmap §7.1).
    ///
    /// The studio never mentions notes: everything inside it names signals,
    /// patches, buses, and the bindings between them (§6.5).
    pub(super) fn studio_decl(&mut self) {
        self.start(SyntaxKind::StudioDecl);
        self.bump(); // studio
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`studio` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::PatchKw) {
                self.patch_decl();
            } else if self.at(SyntaxKind::BusKw) {
                self.bus_decl();
            } else if self.at(SyntaxKind::ModulateKw) {
                self.modulate_stmt();
            } else if self.at(SyntaxKind::AssignKw) {
                self.binding_stmt(SyntaxKind::AssignStmt, "a part name", "a patch name");
            } else if self.at(SyntaxKind::RouteKw) {
                self.binding_stmt(SyntaxKind::RouteStmt, "a source name", "a destination");
            } else if self.at(SyntaxKind::SendKw) {
                self.send_stmt();
            } else if self.at(SyntaxKind::Identifier) {
                self.signal_binding();
            } else {
                self.expected("`patch`, `bus`, `modulate`, `assign`, `route`, `send`, or a signal binding");
                self.recover(STUDIO_RECOVERY);
            }
        }
        self.finish();
    }

    /// `patch <name> { <signal bindings and chains> }`
    pub(super) fn patch_decl(&mut self) {
        self.start(SyntaxKind::PatchDecl);
        self.bump(); // patch
        self.expect(SyntaxKind::Identifier, "a patch name");
        self.expect(SyntaxKind::LBrace, "`{`");
        self.chain_body("patch");
        self.finish();
    }

    /// `bus <name> { <chains> }`
    pub(super) fn bus_decl(&mut self) {
        self.start(SyntaxKind::BusDecl);
        self.bump(); // bus
        self.expect(SyntaxKind::Identifier, "a bus name");
        self.expect(SyntaxKind::LBrace, "`{`");
        self.chain_body("bus");
        self.finish();
    }

    /// The shared body of a patch or bus: named signals and bare chains, in
    /// any order, until the closing brace.
    pub(super) fn chain_body(&mut self, what: &str) {
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed(&format!("`{what}` block")) {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::Identifier) && self.nth_significant(1) == Some(SyntaxKind::Equals) {
                self.signal_binding();
            } else if self.at_any(&[SyntaxKind::Identifier, SyntaxKind::OutputKw]) {
                self.chain_stmt();
            } else {
                self.expected("a signal binding or a signal chain");
                self.recover(STUDIO_RECOVERY);
            }
        }
    }

    /// `<name> = <chain>;`
    pub(super) fn signal_binding(&mut self) {
        self.start(SyntaxKind::SignalBinding);
        self.bump(); // name
        self.expect(SyntaxKind::Equals, "`=`");
        self.signal_chain();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `<chain>;` — unnamed, so its value is the enclosing patch or bus's.
    pub(super) fn chain_stmt(&mut self) {
        self.start(SyntaxKind::ChainStmt);
        self.signal_chain();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `<stage> |> <stage> |> ...`
    ///
    /// `|>` is left-associative and the only operator in the language, so the
    /// "expression parser" §10.2 anticipates is this loop: a precedence table
    /// would be machinery with one entry.
    pub(super) fn signal_chain(&mut self) {
        self.start(SyntaxKind::SignalChain);
        self.stage();
        while self.at(SyntaxKind::PipeForward) {
            self.bump();
            self.stage();
        }
        self.finish();
    }

    /// One stage: a construction `name(args)`, or a bare name (another
    /// signal, or the `output` terminal).
    pub(super) fn stage(&mut self) {
        if self.at(SyntaxKind::OutputKw) || self.at(SyntaxKind::MasterKw) {
            self.start(SyntaxKind::NameRef);
            self.bump();
            self.finish();
        // `scale` is a processor here and a musical collection everywhere
        // else. The studio vocabulary is deliberately made of identifiers so
        // it can grow without the lexer, and this is the one word
        // the score side also needed; the stage accepts the keyword token so
        // that a signal can still be scaled.
        } else if self.at_any(&[SyntaxKind::Identifier, SyntaxKind::ScaleKw]) {
            if self.nth_significant(1) == Some(SyntaxKind::LParen) {
                self.call_expr();
            } else {
                self.start(SyntaxKind::NameRef);
                self.bump();
                self.finish();
            }
        } else {
            self.expected("a processor, a signal name, or `output`");
            self.recover(STUDIO_RECOVERY);
        }
    }

    /// `assign <a> -> <b>;` and `route <a> -> <b>;` — the same shape with
    /// different names on each side, so one production serves both.
    pub(super) fn binding_stmt(&mut self, kind: SyntaxKind, source: &str, destination: &str) {
        self.start(kind);
        self.bump(); // assign / route
        self.expect(SyntaxKind::Identifier, source);
        self.expect(SyntaxKind::Arrow, "`->`");
        if self.at(SyntaxKind::MasterKw) {
            self.bump();
        } else {
            self.expect(SyntaxKind::Identifier, destination);
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `send <source> -> <bus> at <gain> dB;`
    pub(super) fn send_stmt(&mut self) {
        self.start(SyntaxKind::SendStmt);
        self.bump(); // send
        self.expect(SyntaxKind::Identifier, "a source name");
        self.expect(SyntaxKind::Arrow, "`->`");
        self.expect(SyntaxKind::Identifier, "a bus name");
        self.expect(SyntaxKind::AtKw, "`at`");
        self.value();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `performance { profile ... }`
    pub(super) fn performance_decl(&mut self) {
        self.start(SyntaxKind::PerformanceDecl);
        self.bump(); // performance
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`performance` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::ProfileKw) {
                self.profile_decl();
            } else {
                self.expected("a `profile` declaration");
                self.recover(PERFORMANCE_RECOVERY);
            }
        }
        self.finish();
    }

    /// `profile name { mark ... dynamic ... }`
    pub(super) fn profile_decl(&mut self) {
        self.start(SyntaxKind::ProfileDecl);
        self.bump(); // profile
        self.expect(SyntaxKind::Identifier, "a profile name");
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`profile` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::MarkKw) {
                self.rule(SyntaxKind::MarkRule, "a mark name");
            } else if self.at(SyntaxKind::DynamicKw) {
                self.rule(SyntaxKind::DynamicRule, "a dynamic marking");
            } else if self.at(SyntaxKind::GrooveKw) {
                self.rule(SyntaxKind::GrooveRule, "a groove name");
            } else if self.at(SyntaxKind::GraceKw) {
                self.grace_rule();
            } else {
                self.expected("a `mark`, `dynamic`, `groove`, or `grace` rule");
                self.recover(PROFILE_RECOVERY);
            }
        }
        self.finish();
    }

    /// `mark|dynamic|groove <name> { <setting>* }` — one shape, three heads.
    pub(super) fn rule(&mut self, kind: SyntaxKind, what: &str) {
        self.start(kind);
        self.bump(); // mark | dynamic | groove
        self.expect(SyntaxKind::Identifier, what);
        self.settings_block();
        self.finish();
    }

    /// `grace { <setting>* }` — the same block with no name in front.
    ///
    /// Nameless because a profile has one reading of a grace note, not a
    /// vocabulary of them: `groove` is named because the name chooses the
    /// shape, and there is no such choice here.
    pub(super) fn grace_rule(&mut self) {
        self.start(SyntaxKind::GraceRule);
        self.bump(); // grace
        self.settings_block();
        self.finish();
    }

    /// `{ <setting>* }` — the body every profile rule shares.
    pub(super) fn settings_block(&mut self) {
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("rule block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::Identifier) {
                self.setting_stmt();
            } else {
                self.expected("a setting such as `gate = 0.55;`");
                self.recover(&[SyntaxKind::Semicolon, SyntaxKind::RBrace]);
            }
        }
    }

    /// `<name> = [-]<number> [unit];` — the unit is the value's, not the
    /// setting's, so `attack = 8 ms;` and `attack = 0.008 s;` both parse.
    ///
    /// A rational is admitted beside a decimal because some settings are
    /// musical rather than numeric: `ratio = 2/3` is a swing, and writing it
    /// `0.667` would put an approximation where §4 wants an exact one. The
    /// sign is a separate token, so `by = -1/64` is a rational with a minus
    /// in front rather than a third number syntax.
    pub(super) fn setting_stmt(&mut self) {
        self.start(SyntaxKind::SettingStmt);
        self.bump(); // setting name
        self.expect(SyntaxKind::Equals, "`=`");
        if self.at(SyntaxKind::Minus) {
            self.bump();
        }
        // A word is a value too: some settings are a choice between named
        // readings rather than a quantity — `from = principal;`. Which of the
        // two a given setting takes is the reader's answer, not the grammar's,
        // so both parse here and the profile says which it wanted.
        if self.at_any(&[
            SyntaxKind::Float,
            SyntaxKind::Integer,
            SyntaxKind::Rational,
            SyntaxKind::Identifier,
        ]) {
            self.bump();
        } else {
            self.expected("a number or a word");
        }
        if self.at_any(&[SyntaxKind::UnitMs, SyntaxKind::UnitS]) {
            self.bump();
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }
}
