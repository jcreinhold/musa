#![allow(clippy::arithmetic_side_effects)]

use crate::origin::SourceSpan;

/// What kind of material a name was bound to.
///
/// Motifs and bars share one namespace because "material with a name" is one
/// idea, and a composer who mistypes a name should get one diagnostic that
/// knows about both. They differ only in how a diagnostic refers to them.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Material {
    Motif,
    Bar,
    /// Material a *performance* arranges rather than a composer reuses: it
    /// may be reordered by a `mobile` and repeated a chosen number of times.
    /// One namespace with the other two, so `use` reaches all three.
    Fragment,
}

impl Material {
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::Motif => "motif",
            Self::Bar => "bar",
            Self::Fragment => "fragment",
        }
    }
}

/// What kind of thing a recorded name names.
///
/// Values and functions share the elaboration-value namespace. Motifs, bars,
/// and fragments share the material namespace (see [`MaterialCx`]); parts,
/// voices, and patches are a namespace each. The kind is what a rename checks
/// a collision against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NameKind {
    /// An immutable elaboration `let` binding.
    Value,
    /// A named elaboration function.
    Function,
    /// A `motif` declaration.
    Motif,
    /// A named `bar`.
    Bar,
    /// A `fragment` a mobile arranges.
    Fragment,
    /// A `part` in the score.
    Part,
    /// A `voice` in a part. Voices are declared, never used by name — their
    /// entries are declaration-only by construction.
    Voice,
    /// A `patch` in the studio.
    Patch,
    /// A `signature` or a `structure`. Static structure: it names a group of
    /// declarations, never a value, and stops existing once the group has
    /// been checked.
    Module,
    /// A `template`. Named like a module and used like a function: a `make`
    /// site applies it, and what the site names is this declaration rather
    /// than the module the application produces.
    Template,
}

/// One named thing and everywhere it is spoken in the compiled document.
///
/// Spans are the *name tokens'* spans, not the statements': a rename rewrites
/// exactly these ranges and nothing around them. A name declared in an
/// imported library has no declaration here — its uses in the document are
/// recorded honestly, its declaration is `None`, and `external_declaration`
/// identifies its source. That makes navigation exact while keeping a
/// one-document rename from silently editing only half a name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameReference {
    /// The name as written.
    pub name: String,
    /// What it names.
    pub kind: NameKind,
    /// Where the declaration's name token is, when it is in this document.
    pub declaration: Option<SourceSpan>,
    /// Where an imported declaration is written. Bundled modules use a
    /// stable `musa-stdlib:` URI; ordinary imports retain their resolved path.
    pub external_declaration: Option<SourceLocation>,
    /// Every resolved use's name token, in the order the resolver met them.
    /// A name that does not resolve records nothing: no uses, no entry.
    pub uses: Vec<SourceSpan>,
}

/// A declaration in another source document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    /// Resolved path or virtual URI of the source document.
    pub uri: String,
    /// Byte range of the declaration's name in that document.
    pub span: SourceSpan,
}

/// What the resolver knows about names, for an editor to restate.
///
/// Two things, kept together because they are learned together and asked
/// together: where each name is spoken, and what each declaration says. The
/// resolver already knows every use's declaration at the moment it resolves
/// the name; this is that knowledge kept, not a second pass re-derived
/// afterwards. Entries are few — a piece names dozens of things — so a `Vec`
/// scanned linearly beats an index that has to be kept true.
#[derive(Default)]
pub(crate) struct ReferenceIndex {
    entries: Vec<NameReference>,
    docs: crate::docs::DocIndex,
}

impl ReferenceIndex {
    /// Move every place this index points at back into the composer's own text
    /// (`crate::expand`).
    ///
    /// A declaration in *another* document is left alone: its span is a
    /// position in that file's own coordinates, and this map describes only
    /// the document the phase rewrote.
    pub(crate) fn remap_spans(&mut self, map: &crate::expand::SourceMap) {
        for entry in &mut self.entries {
            entry.declaration = map.maybe(entry.declaration);
            for span in &mut entry.uses {
                *span = map.span(*span);
            }
        }
    }

    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Record what one checked declaration says about itself.
    pub(crate) fn document(&mut self, item: crate::docs::ItemDoc) {
        self.docs.document(item);
    }

    /// Every documented declaration, in checking order.
    pub(crate) fn items(&self) -> &[crate::docs::ItemDoc] {
        self.docs.items()
    }

    /// Record a declaration in the compiled document.
    ///
    /// A checked elaboration expression may have recorded a use before the
    /// structural walk registers its legacy declaration. In that case this
    /// completes the same entry; every other declaration remains new.
    pub(crate) fn declare(&mut self, kind: NameKind, name: &str, span: SourceSpan) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.kind == kind && entry.name == name && entry.declaration.is_none())
        {
            entry.declaration = Some(span);
            entry.external_declaration = None;
            return;
        }
        self.entries.push(NameReference {
            name: name.to_owned(),
            kind,
            declaration: Some(span),
            external_declaration: None,
            uses: Vec::new(),
        });
    }

    /// Record a declaration an import supplied.
    ///
    /// The declaration's span is in its own document's coordinates and the
    /// URI is the one a reader opens that document at — bundled modules carry
    /// their stable `musa-stdlib:` URI, an ordinary import its resolved path.
    /// Recording it here, where the walk meets the declaration, is what keeps
    /// a use from being paired with a foreign span read against this
    /// document's text: the `None` declaration is the record's way of saying
    /// "used here, spelled elsewhere".
    pub(crate) fn declare_external(&mut self, kind: NameKind, name: &str, uri: &str, span: SourceSpan) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.kind == kind && entry.name == name && entry.declaration.is_none())
        {
            entry.external_declaration = Some(SourceLocation {
                uri: uri.to_owned(),
                span,
            });
            return;
        }
        self.entries.push(NameReference {
            name: name.to_owned(),
            kind,
            declaration: None,
            external_declaration: Some(SourceLocation {
                uri: uri.to_owned(),
                span,
            }),
            uses: Vec::new(),
        });
    }

    /// Record one resolved use.
    ///
    /// The entry is found by kind and name — unique in every namespace that
    /// has uses (voices are declaration-only). A use whose declaration lives
    /// in an imported library has no entry yet: it is created with a `None`
    /// declaration, which is the record's way of saying "used here, spelled
    /// elsewhere".
    pub(crate) fn record_use(&mut self, kind: NameKind, name: &str, span: SourceSpan) {
        self.record_use_from(kind, name, span, None);
    }

    /// Record that `name` was spoken here, whatever kind of thing it names.
    ///
    /// The reading that lowers a written name into a core term knows a name was
    /// spoken and where; it does not know whether that name is a motif, a
    /// fragment, a `let`, or a `fn`, because the core resolves names and the
    /// reading writes them through. This index already knows — the structural
    /// walk declared every one of them before any body was read — so the kind is
    /// answered here rather than passed in. That is the whole difference from
    /// [`Self::record_use`], whose callers hold a symbol and can say.
    ///
    /// A name no declaration claims is silently not recorded: a local binder, a
    /// prelude constructor, and a builtin are all spoken names with nothing in
    /// this document to point at, and an entry for one would be a reference list
    /// full of `Nat.Succ`.
    pub(crate) fn speak(&mut self, name: &str, span: SourceSpan) {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.name == name && (entry.declaration.is_some() || entry.external_declaration.is_some()))
        else {
            return;
        };
        if !entry.uses.contains(&span) {
            entry.uses.push(span);
        }
    }

    /// Record one resolved use, retaining an imported declaration's source.
    ///
    /// A use is a span, so a span already held is the same use and is not
    /// recorded twice. Checking is not one pass: a document that makes a
    /// piece at its root reads its modules once for the arguments and again
    /// for the piece, and both readings see the same `make` site. Without
    /// this, a rename would emit the same text edit twice and a reference
    /// list would show one occurrence as two.
    pub(crate) fn record_use_from(
        &mut self,
        kind: NameKind,
        name: &str,
        span: SourceSpan,
        external_declaration: Option<SourceLocation>,
    ) {
        match self
            .entries
            .iter_mut()
            .find(|entry| entry.kind == kind && entry.name == name)
        {
            Some(entry) => {
                if !entry.uses.contains(&span) {
                    entry.uses.push(span);
                }
                if entry.external_declaration.is_none() {
                    entry.external_declaration = external_declaration;
                }
            }
            None => self.entries.push(NameReference {
                name: name.to_owned(),
                kind,
                declaration: None,
                external_declaration,
                uses: vec![span],
            }),
        }
    }

    /// Everything recorded, in the order it was first met.
    pub(crate) fn entries(&self) -> &[NameReference] {
        &self.entries
    }
}
