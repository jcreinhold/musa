//! Whole-file shapes: the root preamble, piece and library documents, imports, `make`/templates, and front matter.

use super::engine::{Parser, RootShape};
use crate::{SyntaxError, SyntaxKind};

/// What may name a module inside an import path.
///
/// A module may be called after a type or a domain — `pitch`, `scale`,
/// `harmony` — and the lexer writes the keyword token wherever the word
/// appears. The path position is what makes the word a name. `list` and
/// `option` left this list when they stopped being keywords: the types are
/// `List` and `Option`, and the files that hold them are ordinary names.
pub(crate) const MODULE_NAME: &[SyntaxKind] = &[
    SyntaxKind::Identifier,
    SyntaxKind::HarmonyKw,
    SyntaxKind::PitchKw,
    SyntaxKind::ScaleKw,
];

/// Declaration keywords that bound error recovery at each level.
const PIECE_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::Semicolon,
    SyntaxKind::RBrace,
    SyntaxKind::UseKw,
    SyntaxKind::TempoKw,
    SyntaxKind::MeterKw,
    SyntaxKind::KeyKw,
    SyntaxKind::SubtitleKw,
    SyntaxKind::ComposerKw,
    SyntaxKind::ArrangerKw,
    SyntaxKind::CopyrightKw,
    SyntaxKind::MotifKw,
    SyntaxKind::FragmentKw,
    SyntaxKind::ScoreKw,
    SyntaxKind::PerformanceKw,
    SyntaxKind::StudioKw,
    SyntaxKind::LetKw,
    SyntaxKind::FnKw,
    SyntaxKind::DataKw,
    SyntaxKind::RecordKw,
    SyntaxKind::EnumKw,
    SyntaxKind::TraitKw,
    SyntaxKind::ImplKw,
];

/// What ends a broken declaration at the file's lexical root.
const ROOT_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::Semicolon,
    SyntaxKind::TemplateKw,
    SyntaxKind::MakeKw,
    SyntaxKind::SignatureKw,
    SyntaxKind::StructureKw,
    SyntaxKind::DataKw,
    SyntaxKind::RecordKw,
    SyntaxKind::EnumKw,
    SyntaxKind::TraitKw,
    SyntaxKind::ImplKw,
    SyntaxKind::PieceKw,
    SyntaxKind::LibraryKw,
];

/// The four front-matter keywords, which open statements of one shape.
const FRONT_MATTER: &[SyntaxKind] = &[
    SyntaxKind::SubtitleKw,
    SyntaxKind::ComposerKw,
    SyntaxKind::ArrangerKw,
    SyntaxKind::CopyrightKw,
];

impl Parser<'_> {
    /// Whatever stands before the file's piece or library: imports, values,
    /// functions, and templates.
    ///
    /// These are the file's lexical root, and the only scope a template body
    /// reads besides its own parameters. Nothing here is the file's
    /// declaration — the piece or library that follows is.
    pub(super) fn root_preamble(&mut self) -> RootShape {
        let mut shape = RootShape::default();
        loop {
            if self.at_any(&[SyntaxKind::ImportKw, SyntaxKind::UseKw]) {
                self.import_stmt();
            } else if self.at(SyntaxKind::ModKw) {
                self.mod_decl();
                shape.declares_modules = true;
            } else if self.opens(SyntaxKind::LetKw) {
                self.let_decl();
            } else if self.opens(SyntaxKind::FnKw) {
                self.fn_decl();
            } else if self.at(SyntaxKind::TemplateKw) {
                self.template_decl();
            } else if self.at(SyntaxKind::SignatureKw) {
                self.signature_decl();
            } else if self.at_type_decl() {
                self.type_decl();
            } else if self.at_trait_or_impl() {
                self.trait_or_impl();
            } else if self.opens_any(&[SyntaxKind::StructureKw, SyntaxKind::ModuleKw]) {
                self.structure_decl();
            } else if self.at(SyntaxKind::PrivateKw) {
                self.misplaced_private();
            } else if self.at(SyntaxKind::MakeKw) {
                // Every `make` a file's root writes is read here, whether it
                // makes a module or the piece itself. Which one is the piece
                // is a question about what the names mean, and the parser
                // does not know what anything means.
                self.make_stmt();
                shape.made = true;
            } else {
                break;
            }
        }
        shape
    }

    /// `mod tonal;` — one child of a package's module tree.
    ///
    /// A name and nothing else: what the name reaches is a question about the
    /// package's files, which the parser has none of. The name is read from
    /// [`MODULE_NAME`] for the same reason an import path is — `mod list;` is
    /// the module namespace, where `list` is a name and not a type.
    pub(super) fn mod_decl(&mut self) {
        self.start(SyntaxKind::ModDecl);
        self.bump(); // `mod`
        if self.at_any(MODULE_NAME) {
            self.bump();
        } else {
            self.expected("a module name");
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `piece "name" { ... }`, or `piece study(k: key) "Study" { ... }` for
    /// the piece a `template` parameterizes.
    pub(super) fn piece_decl(&mut self) {
        self.start(SyntaxKind::PieceDecl);
        self.expect(SyntaxKind::PieceKw, "`piece`");
        // A template's piece is named twice: once as the template, in code,
        // and once as the piece, on the page. The identifier is the first.
        if self.at(SyntaxKind::Identifier) {
            self.bump();
            self.param_list();
        }
        self.expect(SyntaxKind::String, "a piece name");
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`piece` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at_any(&[SyntaxKind::ImportKw, SyntaxKind::UseKw]) {
                self.import_stmt();
            } else if self.at(SyntaxKind::TempoKw) {
                self.tempo_stmt();
            } else if self.at(SyntaxKind::MeterKw) {
                self.meter_stmt();
            } else if self.at(SyntaxKind::KeyKw) {
                self.key_stmt();
            } else if self.at_any(FRONT_MATTER) {
                self.front_matter_stmt();
            } else if self.at(SyntaxKind::MotifKw) {
                self.motif_decl();
            } else if self.at(SyntaxKind::FragmentKw) {
                self.fragment_decl();
            } else if self.opens(SyntaxKind::LetKw) {
                self.let_decl();
            } else if self.opens(SyntaxKind::FnKw) {
                self.fn_decl();
            } else if self.at_type_decl() {
                self.type_decl();
            } else if self.at_trait_or_impl() {
                self.trait_or_impl();
            } else if self.at(SyntaxKind::PrivateKw) {
                self.misplaced_private();
            } else if self.at(SyntaxKind::ScoreKw) {
                self.score_decl();
            } else if self.at(SyntaxKind::PerformanceKw) {
                self.performance_decl();
            } else if self.at(SyntaxKind::StudioKw) {
                self.studio_decl();
            } else {
                self.expected_with_help(
                    "a declaration",
                    "a piece holds imports, value/function declarations, score declarations, performance, and studio",
                );
                self.recover(PIECE_RECOVERY);
            }
        }
        self.finish();
    }

    /// `composer "Name";` — one of the four front-matter statements.
    ///
    /// All four have the same shape, and which one this is lives in the
    /// keyword token the node opens with. What each means on the page is the
    /// engraver's business, not the parser's.
    pub(super) fn front_matter_stmt(&mut self) {
        self.start(SyntaxKind::FrontMatterStmt);
        self.bump(); // the keyword
        self.expect(SyntaxKind::String, "a quoted line of front matter");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `tempo <beat> = <bpm>;` — beat is a name (`quarter`) or fraction.
    /// `tempo 1/4 = 92;`, `tempo 1/4 = 132 "Allegro vivace";`, `tempo "Andante";`
    ///
    /// The third form is the one that shapes the grammar. A metronome mark and
    /// a tempo word are two different things a composer writes — often one
    /// without the other — so the number is optional, the word is optional,
    /// and a statement with neither is the error.
    pub(super) fn tempo_stmt(&mut self) {
        self.start(SyntaxKind::TempoStmt);
        self.bump(); // tempo
        // The word may lead (`tempo "Andante";`) or trail
        // (`tempo 1/4 = 132 "Allegro";`) — wherever it can be read aloud in
        // the order it is written. One or the other, never both.
        let leads = self.at(SyntaxKind::String);
        if leads {
            self.bump();
        } else {
            if self.at_any(&[SyntaxKind::Identifier, SyntaxKind::Rational]) {
                self.bump();
            } else {
                self.expected("a beat unit (`quarter` or `1/4`) or a tempo word in quotes");
            }
            self.expect(SyntaxKind::Equals, "`=`");
            self.expect(SyntaxKind::Integer, "a tempo in bpm");
            // `to 60` — where a gradual change arrives, in the same beat
            // unit. `to` rather than a keyword of its own, for the reason
            // `crescendo to f` reads: arriving somewhere is one idea.
            if self.at(SyntaxKind::ToKw) {
                self.bump();
                self.expect(SyntaxKind::Integer, "the tempo the change arrives at");
            }
        }
        // `over 4/1` — how far a gradual change reaches.
        if self.at(SyntaxKind::OverKw) {
            self.bump();
            self.expect(SyntaxKind::Rational, "how far the change reaches, like `4/1`");
        }
        if !leads && self.at(SyntaxKind::String) {
            self.bump();
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `library { ... }` — shared declarations, importable by a piece.
    pub(super) fn library_decl(&mut self) {
        self.start(SyntaxKind::LibraryDecl);
        self.bump(); // library
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`library` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at_any(&[SyntaxKind::ImportKw, SyntaxKind::UseKw]) {
                self.import_stmt();
            } else if self.at(SyntaxKind::MotifKw) {
                self.motif_decl();
            } else if self.at(SyntaxKind::FragmentKw) {
                self.fragment_decl();
            } else if self.opens(SyntaxKind::LetKw) {
                self.let_decl();
            } else if self.opens(SyntaxKind::FnKw) {
                self.fn_decl();
            } else if self.at(SyntaxKind::PerformanceKw) {
                self.performance_decl();
            } else if self.at(SyntaxKind::StudioKw) {
                self.studio_decl();
            } else if self.at(SyntaxKind::SignatureKw) {
                self.signature_decl();
            } else if self.at_type_decl() {
                self.type_decl();
            } else if self.at_trait_or_impl() {
                self.trait_or_impl();
            } else if self.opens_any(&[SyntaxKind::StructureKw, SyntaxKind::ModuleKw]) {
                self.structure_decl();
            } else if self.at(SyntaxKind::PrivateKw) {
                self.misplaced_private();
            } else if self.at(SyntaxKind::TemplateKw)
                && matches!(
                    self.nth_significant(1),
                    Some(SyntaxKind::StructureKw | SyntaxKind::ModuleKw)
                )
            {
                // A piece or a voice belongs to a piece, so the one template
                // a library may hold is the one that makes a module.
                self.template_decl();
            } else if self.at(SyntaxKind::MakeKw) {
                self.make_stmt();
            } else {
                // A library holds what can be shared. Music belongs to a
                // piece, which is why `score` is not in this list.
                self.expected_with_help(
                    "a declaration",
                    "a library holds imports, reusable values/functions, material, signatures, modules, performance, \
                     and studio declarations",
                );
                self.recover(PIECE_RECOVERY);
            }
        }
        self.finish();
    }

    /// `import "../library/motifs.musa";` or `import std::core as core;`
    ///
    /// Nothing is told apart by lookahead any more: `import` is one statement
    /// and `use` is the other, which is the whole point of there being two
    /// words. The old spelling is still read here so that a file written
    /// against it parses into the same shape and gets one located complaint
    /// instead of a cascade.
    pub(super) fn import_stmt(&mut self) {
        self.start(SyntaxKind::ImportStmt);
        if self.at(SyntaxKind::UseKw) {
            self.moved_to_import();
        }
        self.bump(); // `import`, or the `use` that should have been one
        // `import syntax <package path> as <name>;` — the header form that
        // says which package reads a region. It is a *different statement*
        // from an ordinary import and not a modifier on one: an ordinary
        // import cannot change syntax, and the word is here so that reading
        // the header tells you whether it can.
        let syntax = self.at(SyntaxKind::SyntaxKw);
        if syntax {
            self.bump();
        }
        if self.at(SyntaxKind::String) {
            self.bump();
        } else {
            self.expect(SyntaxKind::Identifier, "a relative path in quotes, or `std::module`");
            // A path nests as far as the package's module tree does, and the
            // parser counts no segments: how deep `std::tonal::harmony` is
            // is a fact about `stdlib/`, not about the grammar.
            loop {
                self.expect(SyntaxKind::Colon, "`::`");
                self.expect(SyntaxKind::Colon, "`::`");
                // A module may be named after a type or a domain keyword:
                // `pitch`, `list`, `option`, `scale`, `harmony`. The path
                // position is what makes the word a module name.
                if self.at_any(MODULE_NAME) {
                    self.bump();
                } else {
                    self.expected("a module name");
                    break;
                }
                if !self.at(SyntaxKind::Colon) {
                    break;
                }
            }
        }
        if self.at(SyntaxKind::AsKw) {
            self.bump();
            self.expect(SyntaxKind::Identifier, "a name for the imported module");
        } else if syntax {
            // A region is written by name, so a syntax import that names
            // nothing has nothing a region could say.
            self.expected("`as <name>` — a region names its adapter by this name");
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `syntax staff { ... }` — one named, delimited adapter region.
    ///
    /// What is inside is read by the fixed lexer and grouper and by nothing
    /// else: the parser forms a node per matched delimiter pair and takes
    /// every other token as it comes. It assigns no meaning, because the
    /// meaning is whatever the adapter the header named answers with.
    pub(super) fn syntax_region(&mut self) {
        self.start(SyntaxKind::SyntaxRegion);
        self.bump(); // syntax
        self.expect(SyntaxKind::Identifier, "the name a syntax import gave the adapter");
        if self.at(SyntaxKind::LBrace) {
            self.raw_group();
        } else {
            self.expected("`{` — a region is delimited");
        }
        self.finish();
    }

    /// The fixed grouper: one node per matched delimiter pair, and no other
    /// rule at all.
    pub(super) fn raw_group(&mut self) {
        let close = match self.current() {
            Some(SyntaxKind::LBracket) => SyntaxKind::RBracket,
            Some(SyntaxKind::LParen) => SyntaxKind::RParen,
            _ => SyntaxKind::RBrace,
        };
        // Trivia before the opening delimiter belongs to whatever came before,
        // not to the group: a group whose first token is a space is a group the
        // reader cannot tell is delimited.
        self.eat_trivia();
        self.start(SyntaxKind::SyntaxGroup);
        self.bump();
        loop {
            match self.current() {
                None => {
                    self.expected("the delimiter that closes this region");
                    break;
                }
                Some(kind) if kind == close => {
                    self.bump();
                    break;
                }
                Some(SyntaxKind::LBrace | SyntaxKind::LBracket | SyntaxKind::LParen) => self.raw_group(),
                Some(_) => self.bump(),
            }
        }
        self.finish();
    }

    /// The one complaint a file written against the old spelling gets.
    ///
    /// Located at the word itself and carrying the word that replaces it, so
    /// the migration is an accepted fix rather than a search. Both meanings
    /// are named because the reader's next question is which `use` this was:
    /// the splices in their score are not affected and should not be touched.
    pub(super) fn moved_to_import(&mut self) {
        if self.cascading() {
            return;
        }
        let Some(token) = self.significant() else {
            return;
        };
        self.errors.push(
            SyntaxError::new(
                token.range,
                "`use` no longer imports",
                "this brings in a file, so it is an `import`",
            )
            .with_help("`use` writes out a motif where it stands; `import` brings another file's names into this one")
            .with_fix("write `import`", "import"),
        );
    }

    /// The one complaint a file written against the old spelling gets.
    ///
    /// Located at the word itself and carrying the word that replaces it, so
    /// the migration is an accepted fix rather than a search. `module` is
    /// still a word in this language — it names a node of a package's tree —
    /// which is exactly why the thing it used to declare needed its own.
    pub(super) fn moved_to_structure(&mut self) {
        if self.cascading() {
            return;
        }
        let Some(token) = self.significant() else {
            return;
        };
        self.errors.push(
            SyntaxError::new(
                token.range,
                "`module` no longer declares one",
                "this provides a signature, so it is a `structure`",
            )
            .with_help(
                "a `module` is a node of a package's tree, declared with `mod`; a `structure` provides a `signature`",
            )
            .with_fix("write `structure`", "structure"),
        );
    }

    /// `make study(key g major) as study_in_g;` — one instance site.
    pub(super) fn make_stmt(&mut self) {
        self.start(SyntaxKind::MakeStmt);
        self.bump(); // make
        self.expect(SyntaxKind::Identifier, "a template name");
        if self.at(SyntaxKind::LParen) {
            self.expr_arg_list();
        } else {
            self.expected("`(`");
        }
        self.expect(SyntaxKind::AsKw, "`as`");
        self.expect(SyntaxKind::Identifier, "a name for what is made");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `template piece study(k: key) "Study" { ... }`, or the same for a
    /// voice.
    ///
    /// The node wraps an ordinary [`SyntaxKind::PieceDecl`] or
    /// [`SyntaxKind::VoiceDecl`] carrying a parameter list, so a template
    /// body is read by exactly the accessors a written-out declaration is.
    /// The only thing `template` adds is the word that says the declaration
    /// is a pattern rather than a thing.
    pub(super) fn template_decl(&mut self) {
        self.start(SyntaxKind::TemplateDecl);
        self.bump(); // template
        if self.at(SyntaxKind::PieceKw) {
            self.piece_decl();
        } else if self.at(SyntaxKind::VoiceKw) {
            self.voice_decl();
        } else if self.at_any(&[SyntaxKind::StructureKw, SyntaxKind::ModuleKw]) {
            self.structure_decl();
        } else {
            self.expected_with_help(
                "`piece`, `voice`, or `structure`",
                "a template parameterizes a declaration, and those are the three kinds it may parameterize",
            );
            self.recover(ROOT_RECOVERY);
        }
        self.finish();
    }
}
