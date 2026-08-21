//! Type-like declarations: signature, data, record, enum, trait, impl, and the structure of their members.

use super::engine::Parser;
use crate::{SyntaxError, SyntaxKind};

impl Parser<'_> {
    /// `signature TonalContext { let tonic: key; }` — the members a module
    /// must provide, each a `let` with its definition left out.
    pub(super) fn signature_decl(&mut self) {
        self.start(SyntaxKind::SignatureDecl);
        self.bump(); // signature
        self.expect(SyntaxKind::Identifier, "a signature name");
        self.expect(SyntaxKind::LBrace, "`{`");
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.at(SyntaxKind::LetKw) {
                self.signature_member();
            } else if self.at(SyntaxKind::DataKw) {
                self.data_member();
            } else {
                self.expected("`let`, `data`, or `}`");
                self.recover(&[SyntaxKind::LetKw, SyntaxKind::DataKw, SyntaxKind::RBrace]);
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `data Motive { Silence, Sounded(pitch: Pitch, held: Duration), }` —
    /// one finite nominal declaration.
    ///
    /// A variant with no fields writes no parentheses, because there are no
    /// fields to name; a variant with fields names every one of them, because
    /// a field a reader cannot name is a field the record case could not
    /// project.
    pub(super) fn data_decl(&mut self) {
        self.start(SyntaxKind::DataDecl);
        self.visibility();
        self.bump(); // data
        self.expect(SyntaxKind::Identifier, "a type name");
        if self.at(SyntaxKind::Less) {
            self.type_params();
        }
        if self.at(SyntaxKind::LParen) {
            self.index_param();
        }
        self.expect(SyntaxKind::LBrace, "`{`");
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.at(SyntaxKind::Identifier) {
                self.data_variant();
                if self.at(SyntaxKind::Comma) {
                    self.bump();
                }
            } else {
                self.expected("a constructor name, or `}`");
                self.recover(&[SyntaxKind::Identifier, SyntaxKind::RBrace]);
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `(n: Nat)` after a declaration's name — the index it carries
    /// (`02-core-calculus.md` §1.5).
    ///
    /// Exactly one binder, because a type carries exactly one index: the
    /// elaborated form holds a single index term and `Pc(12)` writes a single
    /// expression, so a comma here would parse a telescope nothing can
    /// represent. A second binder is left to `expect` to report against the
    /// `)` it was looking for, which points at the comma the author wrote.
    ///
    /// The binder's type is an ordinary type expression rather than one of
    /// §1.5's three sorts spelled out. Which types may stand here is a question
    /// about what a *sort* is — whether its values can be read as numbers — and
    /// that is registered, not syntactic; a grammar that listed three names
    /// would be a fourth place obliged to agree with the other three.
    pub(super) fn index_param(&mut self) {
        self.start(SyntaxKind::IndexParam);
        self.bump(); // `(`
        self.expect(SyntaxKind::Identifier, "an index name");
        self.expect(SyntaxKind::Colon, "`:`");
        self.type_expr();
        self.expect(SyntaxKind::RParen, "`)`");
        self.finish();
    }

    /// Whether a type declaration opens here, `private` or not.
    pub(super) fn at_type_decl(&self) -> bool {
        self.opens_any(&[SyntaxKind::DataKw, SyntaxKind::RecordKw, SyntaxKind::EnumKw])
    }

    /// Whether a `trait` or an `impl` opens here, `private` or not.
    ///
    /// Separate from [`Self::at_type_decl`] rather than folded into it,
    /// because the two do not stand in the same places: `01-surface.md` §1's
    /// `document` production admits a trait and an impl at a file's root, in a
    /// piece, and in a library, and a structure's body holds bindings and
    /// functions only. An instance declared inside a sealed structure would be
    /// a coherence question — the table `10-traits.md` §2 keeps is one table
    /// for the whole program — and admitting the syntax would be asking it.
    pub(super) fn at_trait_or_impl(&self) -> bool {
        self.opens_any(&[SyntaxKind::TraitKw, SyntaxKind::ImplKw])
    }

    /// A `private` standing in front of something it cannot mark.
    ///
    /// The word marks a *declaration*, and an `import`, a `make`, or a `mod` is
    /// not one: there would be nothing for it to hide. Reported here rather
    /// than let through, because the alternative is a dispatcher falling off
    /// the end of its chain and complaining that a declaration was expected —
    /// at a position where one *was* written, with the actual mistake one word
    /// to the left.
    pub(super) fn misplaced_private(&mut self) {
        if !self.cascading()
            && let Some(token) = self.significant()
        {
            self.errors.push(
                SyntaxError::new(
                    token.range,
                    "`private` does not mark this",
                    "only a declaration can be private",
                )
                .with_help(
                    "`private` stands before `let`, `fn`, `record`, `enum`, `data`, `trait`, `impl`, or `structure`",
                ),
            );
        }
        self.bump(); // the marker, so the next dispatch sees what follows it
    }

    /// A `private` on a member of a structure, which its signature already
    /// hides.
    ///
    /// The two mechanisms do not overlap and do not conflict: a structure seals
    /// by *listing* — the signature is the interface, and everything else is
    /// already private to the structure — while a module hides by *marking*. A
    /// marker that means nothing is worth saying so, because a reader who wrote
    /// one believes it is doing something.
    pub(super) fn sealed_already(&mut self) {
        if !self.cascading()
            && let Some(token) = self.significant()
        {
            self.errors.push(
                SyntaxError::new(
                    token.range,
                    "this is already private",
                    "a structure's signature is its interface",
                )
                .with_help(
                    "a member the signature does not list is private to the structure, so the marker adds nothing",
                )
                .with_fix("remove `private`", ""),
            );
        }
    }

    /// Whichever of `data`, `record`, and `enum` opens here.
    ///
    /// One dispatch for the three because they stand in exactly the same
    /// places: a type is declared at a file's root, in a piece, in a library,
    /// and in a structure, and which of the three words opens it changes what
    /// the type *is* rather than where it may be written.
    pub(super) fn type_decl(&mut self) {
        if self.opens(SyntaxKind::RecordKw) {
            self.record_decl();
        } else if self.opens(SyntaxKind::EnumKw) {
            self.enum_decl();
        } else {
            self.data_decl();
        }
    }

    /// `record Pending { read: Reading; dots: Dots; }` — named fields, and
    /// nothing else.
    ///
    /// Fields end in `;` rather than `,` because a field declaration is a
    /// declaration and every other one in this language ends in `;`. An enum's
    /// *cases* are comma-separated, which is the visible difference between
    /// reading a record and reading a sum.
    pub(super) fn record_decl(&mut self) {
        self.start(SyntaxKind::RecordDecl);
        self.visibility();
        self.bump(); // record
        self.expect(SyntaxKind::Identifier, "a type name");
        if self.at(SyntaxKind::Less) {
            self.type_params();
        }
        self.where_clause();
        self.expect(SyntaxKind::LBrace, "`{`");
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.at(SyntaxKind::Identifier) {
                self.field_decl();
            } else {
                self.expected("a field, such as `dots: Dots;`");
                self.recover(&[SyntaxKind::Identifier, SyntaxKind::RBrace]);
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `read: Reading;` — one declared field.
    pub(super) fn field_decl(&mut self) {
        self.start(SyntaxKind::FieldDecl);
        self.bump(); // the field's name
        self.expect(SyntaxKind::Colon, "`:`");
        self.type_expr();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `enum Reading<A> { Done(A), Refused { at: NodePath, why: Text }, }` —
    /// a nominal sum whose cases live in its namespace.
    ///
    /// A case may carry nothing, a positional list of types, or named fields.
    /// The positional form names types and not fields, which is the difference
    /// from `data`: `data` made every constructor argument a projectable field,
    /// and §1.3 does not, because a case with fields worth naming is written in
    /// the named form where the names are the interface.
    pub(super) fn enum_decl(&mut self) {
        self.start(SyntaxKind::EnumDecl);
        self.visibility();
        self.bump(); // enum
        self.expect(SyntaxKind::Identifier, "a type name");
        if self.at(SyntaxKind::Less) {
            self.type_params();
        }
        self.where_clause();
        self.expect(SyntaxKind::LBrace, "`{`");
        // An enum with no cases at all is admitted, and deliberately: `enum
        // Empty {}` is the type with no closed inhabitant, which is what
        // `P -> Empty` needs to say *not P* (§1.3).
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.opens(SyntaxKind::Identifier) {
                self.enum_case();
                if self.at(SyntaxKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            } else {
                self.expected("a case name, or `}`");
                self.recover(&[SyntaxKind::Identifier, SyntaxKind::RBrace]);
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// One case of an enum: empty, positional, or named.
    pub(super) fn enum_case(&mut self) {
        self.start(SyntaxKind::EnumCase);
        self.visibility();
        self.bump(); // the case's name
        if self.at(SyntaxKind::LParen) {
            self.bump();
            while !self.at(SyntaxKind::RParen) && self.current().is_some() {
                self.type_expr();
                if self.at(SyntaxKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
            self.expect(SyntaxKind::RParen, "`)`");
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
                if self.at(SyntaxKind::Identifier) {
                    self.field_decl();
                } else {
                    self.expected("a field, such as `why: Text;`");
                    self.recover(&[SyntaxKind::Identifier, SyntaxKind::RBrace]);
                }
            }
            self.expect(SyntaxKind::RBrace, "`}`");
        }
        self.finish();
    }

    /// Whichever of `trait` and `impl` opens here.
    ///
    /// One dispatch for the two because they stand in the same places and are
    /// halves of one mechanism: a trait declares the record and an impl
    /// declares a value of it (`10-traits.md` §1).
    pub(super) fn trait_or_impl(&mut self) {
        if self.opens(SyntaxKind::TraitKw) {
            self.trait_decl();
        } else {
            self.impl_decl();
        }
    }

    /// `trait Eq<A> where Storable<A> { fn equal(x: A, y: A) -> Bool; }`
    ///
    /// A trait's items are ordinary function declarations, and a `;` where the
    /// body would be is how one says *required*: `10-traits.md` §1 makes a
    /// trait a dependent record, so a method with a body is a field with a
    /// default and a method without one is a field an instance must fill. They
    /// share a node kind with every other function for that reason — asking
    /// whether a method is required is asking whether it has a body, which is
    /// one child lookup rather than a second declaration form to read.
    pub(super) fn trait_decl(&mut self) {
        self.start(SyntaxKind::TraitDecl);
        self.visibility();
        self.bump(); // trait
        self.expect(SyntaxKind::Identifier, "a trait name");
        // Written and not optional: a trait with no parameter has nothing to
        // dispatch on, which `10-traits.md` §9 refuses by name.
        if self.at(SyntaxKind::Less) {
            self.type_params();
        } else {
            self.expected("a type parameter, such as `<A>` — a trait dispatches on one");
        }
        self.where_clause();
        self.expect(SyntaxKind::LBrace, "`{`");
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.at(SyntaxKind::FnKw) {
                self.trait_method();
            } else {
                self.expected("a method, such as `fn equal(x: A, y: A) -> Bool;`");
                self.recover(&[SyntaxKind::FnKw, SyntaxKind::RBrace]);
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `impl<A> Eq<List<A>> where Eq<A> { fn equal(x, y) { … } }`, and
    /// `impl Duration { … }` — the same declaration read two ways.
    ///
    /// The head is parsed as a type, because that is the one shape both forms
    /// have: `Eq<List<A>>` and `Duration` are the same syntax, and which of
    /// them is a constraint and which a type namespace is decided by what the
    /// head *name* resolves to (`10-traits.md` §6), which the parser has no
    /// way to know and no business guessing.
    pub(super) fn impl_decl(&mut self) {
        self.start(SyntaxKind::ImplDecl);
        self.visibility();
        self.bump(); // impl
        if self.at(SyntaxKind::Less) {
            self.type_params();
        }
        self.type_expr();
        self.where_clause();
        self.expect(SyntaxKind::LBrace, "`{`");
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.opens(SyntaxKind::FnKw) {
                self.fn_decl();
            } else {
                self.expected("a function, or `}`");
                self.recover(&[SyntaxKind::FnKw, SyntaxKind::PrivateKw, SyntaxKind::RBrace]);
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `where Eq<A>, Ord<B>` — the constraints a declaration carries, if it
    /// wrote any.
    ///
    /// Written in one place for every declaration that takes one, because
    /// `01-surface.md` §1 gives functions, records, enums, traits, and impls
    /// the same clause: a constraint means the same thing wherever it is
    /// written, and reading it in five places would be five chances to let one
    /// of them drift.
    pub(super) fn where_clause(&mut self) {
        if !self.at(SyntaxKind::WhereKw) {
            return;
        }
        self.start(SyntaxKind::WhereClause);
        self.bump(); // where
        loop {
            self.start(SyntaxKind::Constraint);
            self.type_expr();
            self.finish();
            if !self.at(SyntaxKind::Comma) {
                break;
            }
            self.bump();
        }
        self.finish();
    }

    /// `Sounded(pitch: Pitch, held: Duration)` — one constructor.
    pub(super) fn data_variant(&mut self) {
        self.start(SyntaxKind::DataVariant);
        self.bump(); // the constructor's name
        if self.at(SyntaxKind::LParen) {
            self.bump();
            while !self.at(SyntaxKind::RParen) && self.current().is_some() {
                if self.at(SyntaxKind::Identifier) {
                    self.start(SyntaxKind::DataField);
                    self.bump();
                    self.expect(SyntaxKind::Colon, "`:`");
                    self.type_expr();
                    self.finish();
                    if self.at(SyntaxKind::Comma) {
                        self.bump();
                    }
                } else {
                    self.expected("a field name, or `)`");
                    self.recover(&[SyntaxKind::Identifier, SyntaxKind::RParen]);
                    break;
                }
            }
            self.expect(SyntaxKind::RParen, "`)`");
        }
        self.finish();
    }

    /// `data Motive;` — one signature member naming a type and not its
    /// constructors.
    pub(super) fn data_member(&mut self) {
        self.start(SyntaxKind::DataMember);
        self.bump(); // data
        self.expect(SyntaxKind::Identifier, "a type name");
        if self.at(SyntaxKind::Less) {
            self.type_params();
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `let tonic: key;` — one signature member.
    pub(super) fn signature_member(&mut self) {
        self.start(SyntaxKind::SignatureMember);
        self.bump(); // let
        self.expect(SyntaxKind::Identifier, "a member name");
        self.expect(SyntaxKind::Colon, "`:`");
        self.type_expr();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `structure CMajor : TonalContext { ... }`, or the same with a
    /// parameter list for the structure a `template` parameterizes.
    pub(super) fn structure_decl(&mut self) {
        self.start(SyntaxKind::StructureDecl);
        self.visibility();
        if self.at(SyntaxKind::ModuleKw) {
            self.moved_to_structure();
        }
        self.bump(); // `structure`, or the `module` that should have been one
        self.expect(SyntaxKind::Identifier, "a structure name");
        if self.at(SyntaxKind::LParen) {
            self.param_list();
        }
        self.expect(SyntaxKind::Colon, "`:`");
        self.expect(SyntaxKind::Identifier, "the signature this structure provides");
        self.expect(SyntaxKind::LBrace, "`{`");
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.at(SyntaxKind::PrivateKw) {
                self.sealed_already();
            }
            if self.opens(SyntaxKind::LetKw) {
                self.let_decl();
            } else if self.opens(SyntaxKind::FnKw) {
                self.fn_decl();
            } else if self.at_type_decl() {
                self.type_decl();
            } else {
                self.expected("`let`, `fn`, `data`, `record`, `enum`, or `}`");
                self.recover(&[
                    SyntaxKind::LetKw,
                    SyntaxKind::FnKw,
                    SyntaxKind::DataKw,
                    SyntaxKind::RecordKw,
                    SyntaxKind::EnumKw,
                    SyntaxKind::RBrace,
                ]);
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }
}
