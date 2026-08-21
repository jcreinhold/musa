//! The lint pass: warnings for notation that is spelled
//! correctly and still misleads.
//!
//! The rules are the machine-checkable subset of `docs/rules/style-guide.md`, and
//! each diagnostic names its section there; the guide is the authority when a
//! rule is too coarse. Every rule travels as an ordinary `Warning` with an
//! ordinary `Fix` where deletion is certain, so every surface that shows
//! diagnostics shows lints with no new plumbing.
//!
//! One pass, not one per layer: the CST-only rules and the semantic rules
//! share the suppression helper and the emission path, and splitting them to
//! match where their *evidence* lives would duplicate both for no caller.
//! musa-syntax stays silent — it does not know what a name is for.
//!
//! The pass runs only on a piece that compiles (advice about a piece that
//! does not exist is noise), and every rule is silent on every file in
//! `examples/` — which is a law in `tests/lint_laws.rs`, not a hope.

use musa_syntax::ast::{AstNode, BarStmt, EnumDecl, ImplDecl, PieceDecl, RecordDecl, TraitDecl, VoiceItem};
use musa_syntax::{ParsedDocument, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::resolve::{NameKind, ReferenceIndex};
use crate::studio::StudioSpec;
use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::SourceSpan;

/// Lint a compiled piece: the reference index and the resolved studio are
/// the semantic evidence, the parse tree is the rest.
///
/// Returns warnings only, in source order per rule — the caller appends them
/// to the compilation's diagnostics.
pub(crate) fn lint(
    document: &ParsedDocument,
    piece: &PieceDecl,
    references: &ReferenceIndex,
    studio: &StudioSpec,
) -> Vec<Diagnostic> {
    let source = document.text();
    let mut lints = Vec::new();
    unused_material(document, source, references, &mut lints);
    unassigned_patch(document, source, references, studio, &mut lints);
    redundant_marking(piece, source, &mut lints);
    copied_bars(piece, &mut lints);
    redundant_name_prefix(document, &mut lints);
    duplicate_constraint(document, &mut lints);
    lints
}

/// Guide §6: a name is read after the thing it belongs to, so a name that
/// repeats it says it twice.
///
/// Four declarations, one rule. An inherent method is read after its receiver
/// (`d.duration_of()`), a trait method after its trait (`Eq.eq_equal`), a record
/// field after its record (`d.duration_beats`), and an enum case after its type
/// (`Decision::DecisionYes`). In every one the prefix carries information the
/// reader already has.
///
/// This is the shape `10-traits.md` §5's migration table invites: every
/// `duration_of` and `chord_root` in the builtin registry is a name from a
/// language with no receivers, and moving one across without dropping the prefix
/// writes the old shape in the new spelling.
///
/// A **trait instance** is not checked, and that is the rule rather than an
/// omission: an `impl Eq<T>`'s method names belong to the trait, and an impl
/// that renamed one would not be implementing it. Only an `impl` whose head is a
/// bare type name — `01-surface.md` §1.4's inherent block — is a place an author
/// chooses the name.
fn redundant_name_prefix(document: &ParsedDocument, lints: &mut Vec<Diagnostic>) {
    let root = document.syntax();
    let mut owned: Vec<(String, &'static str, Vec<SyntaxNode>)> = Vec::new();
    for block in root.descendants().filter_map(ImplDecl::cast) {
        let Some(head) = block.head().filter(|head| head.kind() == SyntaxKind::TypeName) else {
            continue;
        };
        let written = normalized(&head);
        if written.contains('<') || written.contains(':') {
            continue;
        }
        owned.push((written, "type", named(block.methods())));
    }
    for declared in root.descendants().filter_map(TraitDecl::cast) {
        let Some(name) = declared.name() else { continue };
        owned.push((name, "trait", named(declared.methods())));
    }
    for declared in root.descendants().filter_map(RecordDecl::cast) {
        let Some(name) = declared.name() else { continue };
        owned.push((name, "record", named(declared.fields())));
    }
    for declared in root.descendants().filter_map(EnumDecl::cast) {
        let Some(name) = declared.name() else { continue };
        owned.push((name, "enum", named(declared.cases())));
    }
    for (owner, what, items) in owned {
        for item in items {
            let Some(spelled) = written_name(&item) else { continue };
            let Some(rest) = shortened(&owner, spelled.text()) else {
                continue;
            };
            if suppressed(&item, Code::RedundantNamePrefix) {
                continue;
            }
            lints.push(
                Diagnostic::warning(
                    Code::RedundantNamePrefix,
                    format!("this name repeats the {what} it belongs to"),
                )
                .at(token_span(&spelled), "declared here")
                .help(format!(
                    "name it `{rest}` — a use already says `{owner}` before the name arrives"
                ))
                .note("docs/rules/style-guide.md §6: a name is read after the thing it belongs to"),
            );
        }
    }
}

/// The syntax nodes of a declaration's items, so that four kinds of item can
/// share one loop.
fn named<N: AstNode>(items: Vec<N>) -> Vec<SyntaxNode> {
    items.into_iter().map(|item| item.syntax().clone()).collect()
}

/// The item's own name, which is its first identifier.
///
/// The token rather than its text, because the report points at the name being
/// renamed: a node's range in this CST starts at its leading trivia, so a field
/// underlined by its node would underline the blank line above it too.
fn written_name(item: &SyntaxNode) -> Option<SyntaxToken> {
    item.children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|token| token.kind() == SyntaxKind::Identifier)
}

/// Exactly the span of one token.
fn token_span(token: &SyntaxToken) -> SourceSpan {
    let range = token.text_range();
    SourceSpan::new(u32::from(range.start()), u32::from(range.end()))
}

/// What is left of `name` after the prefix that repeats `owner`, or `None`
/// where there is no such prefix.
///
/// Two spellings, because two conventions meet here: a method or field is
/// `snake_case` and repeats `Duration` as `duration_`, an enum case is
/// `UpperCamelCase` and repeats it as `Duration`. A name that is *only* the
/// prefix is left alone — `Duration::Duration` is a different mistake, and a
/// rule that renamed it would have nothing to rename it to.
fn shortened(owner: &str, name: &str) -> Option<String> {
    let snake = format!("{}_", snake_case(owner));
    if let Some(rest) = name.strip_prefix(&snake) {
        return (!rest.is_empty()).then(|| rest.to_string());
    }
    let rest = name.strip_prefix(owner)?;
    rest.chars()
        .next()
        .filter(char::is_ascii_uppercase)
        .map(|_| rest.to_string())
}

/// `NoteName` → `note_name`, the spelling a function name would have.
fn snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len().saturating_add(4));
    for (position, letter) in name.chars().enumerate() {
        if letter.is_uppercase() && position > 0 {
            out.push('_');
        }
        out.extend(letter.to_lowercase());
    }
    out
}

/// Guide §4: a `where` clause that names one constraint twice.
///
/// Harmless to the elaborator — the second lookup answers what the first did,
/// since coherence makes one instance per key — and exactly the kind of
/// duplication §4 is about: it survives an edit that changes one copy, and then
/// the reader has two clauses to reconcile that were never two facts.
fn duplicate_constraint(document: &ParsedDocument, lints: &mut Vec<Diagnostic>) {
    for clause in document
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::WhereClause)
    {
        let mut seen: Vec<String> = Vec::new();
        for constraint in clause.children().filter(|node| node.kind() == SyntaxKind::Constraint) {
            let spelling = normalized(&constraint);
            if !seen.contains(&spelling) {
                seen.push(spelling);
                continue;
            }
            if suppressed(&constraint, Code::DuplicateConstraint) {
                continue;
            }
            lints.push(
                Diagnostic::warning(Code::DuplicateConstraint, "this constraint is already written")
                    .at(span_of(&constraint), "written twice")
                    .help("delete it — one constraint is one dictionary, however many times it is asked for")
                    .note("docs/rules/style-guide.md §4: say it once"),
            );
        }
    }
}

/// Guide §1: a `motif` or `fragment` declared in this document and never
/// used is a rumour of an abstraction. The reference index answers "nobody
/// speaks this name" directly.
///
/// A *named bar* is not checked: it plays where it stands, so its name is an
/// address (edit sites, provenance), not a promise of reuse — an unused one
/// costs the reader nothing.
fn unused_material(document: &ParsedDocument, source: &str, references: &ReferenceIndex, lints: &mut Vec<Diagnostic>) {
    for entry in references.entries() {
        let Some(declaration) = entry.declaration else { continue };
        if !matches!(entry.kind, NameKind::Motif | NameKind::Fragment) || !entry.uses.is_empty() {
            continue;
        }
        let what = match entry.kind {
            NameKind::Motif => "motif",
            NameKind::Fragment => "fragment",
            NameKind::Value
            | NameKind::Function
            | NameKind::Bar
            | NameKind::Part
            | NameKind::Voice
            | NameKind::Patch
            | NameKind::Module
            | NameKind::Template => continue,
        };
        let Some(statement) = statement_node(
            document,
            declaration.start,
            &[SyntaxKind::MotifDecl, SyntaxKind::FragmentDecl],
        ) else {
            continue;
        };
        if suppressed(&statement, Code::UnusedMaterial) {
            continue;
        }
        lints.push(
            Diagnostic::warning(Code::UnusedMaterial, format!("this {what} is never used"))
                .at(declaration, "declared here")
                .help(format!(
                    "use it, or delete it — a {what} nobody speaks reads as an abstraction that never landed"
                ))
                .note("docs/rules/style-guide.md §1: a name is a promise")
                .fix(format!("delete this {what}"), delete_lines(source, &statement), ""),
        );
    }
}

/// Guide §1, one layer down: a `patch` no `assign` speaks is a cable that
/// ends in the air. Patches in imported libraries are their own documents'
/// business; only declarations in this one are checked.
fn unassigned_patch(
    document: &ParsedDocument,
    source: &str,
    references: &ReferenceIndex,
    studio: &StudioSpec,
    lints: &mut Vec<Diagnostic>,
) {
    let assigned: Vec<&str> = studio.assignments().map(|(_part, patch)| patch).collect();
    for entry in references.entries() {
        let Some(declaration) = entry.declaration else { continue };
        if entry.kind != NameKind::Patch || assigned.contains(&entry.name.as_str()) {
            continue;
        }
        let Some(statement) = statement_node(document, declaration.start, &[SyntaxKind::PatchDecl]) else {
            continue;
        };
        if suppressed(&statement, Code::UnassignedPatch) {
            continue;
        }
        lints.push(
            Diagnostic::warning(Code::UnassignedPatch, "this patch realizes no part")
                .at(declaration, "declared here")
                .help("assign a part to it (`assign violin -> glass_pad;`), or delete it — as written it is wired to silence")
                .note("docs/rules/style-guide.md §1: a name is a promise")
                .fix("delete this patch", delete_lines(source, &statement), ""),
        );
    }
}

/// Guide §2: a tempo, meter, or key marking that states the value already in
/// force reads as an event and is none.
///
/// Comparison is by written text within one voice's own sequence, seeded
/// with the piece's header markings: consecutive identical markings are
/// certainly redundant, and everything harder to prove is let go — a ramp
/// clears the tempo record (the in-force clock afterwards is the arrival,
/// not the text), a `senza` block clears the meter record (it changes and
/// restores the meter without a statement), and nested blocks are not
/// walked. Under-reporting is the rule's way of never lying.
fn redundant_marking(piece: &PieceDecl, source: &str, lints: &mut Vec<Diagnostic>) {
    let seed = [
        piece
            .tempos()
            .first()
            .map(|tempo| ("tempo", normalized(tempo.syntax()))),
        piece.meter().map(|meter| ("meter", normalized(meter.syntax()))),
        piece.key().map(|key| ("key", normalized(key.syntax()))),
    ];
    for voice in voices_of(piece) {
        let mut in_force: Vec<(&str, String)> = seed.iter().flatten().cloned().collect();
        for item in voice.items() {
            if let VoiceItem::Tempo(tempo) = &item {
                // A ramp's aftermath is a clock the text cannot see.
                if tempo.over().is_some() {
                    in_force.retain(|(kind, _)| *kind != "tempo");
                } else {
                    check_marking("tempo", tempo.syntax(), source, &mut in_force, lints);
                }
            } else if let VoiceItem::Meter(meter) = &item {
                check_marking("meter", meter.syntax(), source, &mut in_force, lints);
            } else if let VoiceItem::Key(key) = &item {
                check_marking("key", key.syntax(), source, &mut in_force, lints);
            } else if let VoiceItem::Senza(_) = &item {
                // `senza` is an implicit `meter none` and an implicit restore.
                in_force.retain(|(kind, _)| *kind != "meter");
            }
        }
    }
}

/// One marking against the record: identical to what is in force is
/// certainly redundant — and a no-op, so the record does not move.
fn check_marking(
    kind: &'static str,
    node: &SyntaxNode,
    source: &str,
    in_force: &mut Vec<(&'static str, String)>,
    lints: &mut Vec<Diagnostic>,
) {
    let text = normalized(node);
    if in_force
        .iter()
        .any(|(seen, previous)| *seen == kind && *previous == text)
    {
        if !suppressed(node, Code::RedundantMarking) {
            lints.push(
                Diagnostic::warning(Code::RedundantMarking, format!("this {kind} marking changes nothing"))
                    .at(span_of(node), "states what is already in force")
                    .help("a marking is a change, written where it happens — a reassurance belongs in a comment")
                    .note("docs/rules/style-guide.md §2: a marking changes something")
                    .fix(format!("delete this {kind} marking"), delete_lines(source, node), ""),
            );
        }
        return;
    }
    in_force.retain(|(seen, _)| *seen != kind);
    in_force.push((kind, text));
}

/// Guide §4: three identical bars in one voice is a motif that has not been
/// named yet. Only the voice's own top-level bars compare — a bar inside a
/// `repeat` is honest repetition, and a bar inside any other block is
/// someone's argument — and named bars are already an abstraction.
fn copied_bars(piece: &PieceDecl, lints: &mut Vec<Diagnostic>) {
    const THREE_IS_A_MOTIF: usize = 3;
    for voice in voices_of(piece) {
        let bars: Vec<BarStmt> = voice
            .items()
            .into_iter()
            .filter_map(|item| {
                if let VoiceItem::Bar(bar) = &item
                    && bar.name().is_none()
                {
                    return Some(bar.clone());
                }
                None
            })
            .collect();
        let mut clusters: Vec<(String, Vec<BarStmt>)> = Vec::new();
        for bar in bars {
            let text = normalized(bar.syntax());
            match clusters.iter_mut().find(|(seen, _)| *seen == text) {
                Some((_, group)) => group.push(bar),
                None => clusters.push((text, vec![bar])),
            }
        }
        for (_, group) in clusters {
            if group.len() < THREE_IS_A_MOTIF {
                continue;
            }
            if group.iter().all(|bar| suppressed(bar.syntax(), Code::CopiedBars)) {
                continue;
            }
            let Some((original, copies)) = group.split_first() else {
                continue;
            };
            let mut diagnostic = Diagnostic::warning(
                Code::CopiedBars,
                format!("this bar is written out {} times, note for note", group.len()),
            )
            .at(span_of(original.syntax()), "the original")
            .help("say it once in a `motif` and `use` it — then one edit reaches every occurrence")
            .note("docs/rules/style-guide.md §4: say it once");
            for copy in copies {
                diagnostic = diagnostic.also(span_of(copy.syntax()), "a copy");
            }
            lints.push(diagnostic);
        }
    }
}

/// The voices of the piece, wherever the score puts them.
fn voices_of(piece: &PieceDecl) -> Vec<musa_syntax::ast::VoiceDecl> {
    piece
        .score()
        .into_iter()
        .flat_map(|score| score.parts())
        .flat_map(|part| part.voices())
        .collect()
}

/// The statement node enclosing a name token's offset, when it is one of
/// `kinds` — where a declaration-level lint's suppression lives and its
/// deletion reaches.
fn statement_node(document: &ParsedDocument, offset: u32, kinds: &[SyntaxKind]) -> Option<SyntaxNode> {
    document
        .syntax()
        .token_at_offset(offset.into())
        .find_map(|token| token.parent_ancestors().find(|node| kinds.contains(&node.kind())))
}

/// Whether a `// musa:allow(code, …)` comment stands directly above `node` —
/// the same attachment the formatter keeps (guide §5).
///
/// "Directly above" is a fact about the text, and the tree spells it two ways.
/// Most constructs swallow their leading trivia, so the waiver is among the
/// node's own first tokens. A bar and a grace group do not: they are written
/// on one line, and a comment eaten into the line would have nowhere to go, so
/// the parser leaves it outside as a preceding sibling. Both readings are the
/// same comment in the same place, and the composer who typed it should not
/// have to know which construct they were above.
fn suppressed(node: &SyntaxNode, code: Code) -> bool {
    let inside = node
        .children_with_tokens()
        .map_while(SyntaxElement::into_token)
        .take_while(|token| token.kind().is_trivia());
    let above = std::iter::successors(node.prev_sibling_or_token(), SyntaxElement::prev_sibling_or_token)
        .map_while(SyntaxElement::into_token)
        .take_while(|token| token.kind().is_trivia());
    inside.chain(above).any(|token| allows(token.text(), code))
}

/// Whether one comment waives one code: `musa:allow(code, …)` with the codes
/// comma-separated.
fn allows(comment: &str, code: Code) -> bool {
    const DIRECTIVE: &str = "musa:allow(";
    let Some(from) = comment.find(DIRECTIVE) else {
        return false;
    };
    let Some(rest) = comment.get(from.saturating_add(DIRECTIVE.len())..) else {
        return false;
    };
    let Some(until) = rest.find(')') else { return false };
    let Some(list) = rest.get(..until) else { return false };
    list.split(',').map(str::trim).any(|listed| listed == code.as_str())
}

/// A node's text with trivia removed — the identity a no-op marking or a
/// copied bar cannot hide from.
fn normalized(node: &SyntaxNode) -> String {
    node.descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| {
            !matches!(
                token.kind(),
                SyntaxKind::Whitespace | SyntaxKind::LineComment | SyntaxKind::BlockComment
            )
        })
        .map(|token| token.text().to_string())
        .collect()
}

/// The span of a node, as diagnostics take one.
fn span_of(node: &SyntaxNode) -> SourceSpan {
    let range = node.text_range();
    SourceSpan::new(u32::from(range.start()), u32::from(range.end()))
}

/// The whole lines a node's statement spans — leading indent and trailing
/// newline included, so deleting them leaves the file formatted.
///
/// From the first *significant* token, not the node's span: leading trivia
/// belongs to the node's range in this CST, and an attached comment is the
/// composer's prose, not the construct's — deleting a marking must not take
/// the comment above it with it.
fn delete_lines(source: &str, node: &SyntaxNode) -> SourceSpan {
    let first_significant = node
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|token| {
            !matches!(
                token.kind(),
                SyntaxKind::Whitespace | SyntaxKind::LineComment | SyntaxKind::BlockComment
            )
        });
    let Some(first) = first_significant else {
        return span_of(node);
    };
    let start: usize = first.text_range().start().into();
    let end: usize = node.text_range().end().into();
    let line_start = source
        .get(..start)
        .and_then(|head| head.rfind('\n'))
        .map_or(0, |newline| newline.saturating_add(1));
    let after = source
        .get(end..)
        .and_then(|tail| tail.find('\n'))
        .map_or(source.len(), |newline| end.saturating_add(newline).saturating_add(1));
    SourceSpan::new(line_start as u32, after as u32)
}

#[cfg(test)]
mod tests {
    use super::{duplicate_constraint, redundant_name_prefix};
    use musa_score::diagnose::{Code, Diagnostic};

    /// The two rules that read only the parse tree, exercised here rather than
    /// through `compile`.
    ///
    /// `trait`, `impl`, and `where` are surface the checker does not resolve
    /// yet — prompt 142 is the migration that connects them — so a fixture
    /// holding one cannot compile, and `lint_laws.rs` asserts its fixtures do.
    /// The rules themselves read the CST and nothing else, which is why they can
    /// be asked here and answered honestly.
    fn lints(body: &str) -> Vec<Diagnostic> {
        let source = format!("library {{\n{body}}}\n");
        let document = musa_syntax::parse(&source);
        assert!(
            document.errors().is_empty(),
            "fixture must parse: {:?}",
            document.errors()
        );
        let mut found = Vec::new();
        redundant_name_prefix(&document, &mut found);
        duplicate_constraint(&document, &mut found);
        found
    }

    fn codes(body: &str) -> Vec<Code> {
        lints(body).into_iter().map(|lint| lint.code).collect()
    }

    #[test]
    fn a_trait_method_does_not_repeat_its_trait() {
        assert_eq!(
            codes("trait Eq<A> {\n    fn eq_equal(x: A, y: A) -> Bool;\n}\n"),
            vec![Code::RedundantNamePrefix]
        );
        assert!(codes("trait Eq<A> {\n    fn equal(x: A, y: A) -> Bool;\n}\n").is_empty());
    }

    #[test]
    fn an_inherent_method_does_not_repeat_its_type() {
        assert_eq!(
            codes("impl Duration {\n    fn duration_of(n: Nat) -> Nat { n }\n}\n"),
            vec![Code::RedundantNamePrefix]
        );
        assert!(codes("impl Duration {\n    fn of(n: Nat) -> Nat { n }\n}\n").is_empty());
    }

    /// The instance's method names are the trait's, so renaming one would stop
    /// implementing it — which is why the rule reads the head before it reads
    /// the names.
    #[test]
    fn an_instance_is_left_alone() {
        assert!(
            codes("impl Eq<Duration> {\n    fn duration_equal(x: Nat, y: Nat) -> Nat { x }\n}\n").is_empty(),
            "an impl of a trait names its methods after the trait, not after this rule"
        );
    }

    #[test]
    fn a_constraint_is_written_once() {
        assert_eq!(
            codes("fn f<A>(x: A) -> A where Eq<A>, Eq<A> { x }\n"),
            vec![Code::DuplicateConstraint]
        );
        assert!(codes("fn f<A>(x: A) -> A where Eq<A>, Ord<A> { x }\n").is_empty());
    }
}
