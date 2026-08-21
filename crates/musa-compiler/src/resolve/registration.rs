#![allow(clippy::arithmetic_side_effects)]
use musa_language::SyntaxKind;
use musa_language::ast::{AstNode as _, FrontMatterRole, PieceDecl};

use crate::diagnose::{Code, Diagnostic};
use crate::origin::SourceSpan;
use crate::profile::ProfileSet;
use crate::score::ScoreSnapshot;

use super::{NameKind, Resolver, parse_meter, parse_profiles, span_of, token_span, trimmed_span};

/// Resolve the piece's `studio` block, if it has one.
///
/// Shared by both semantic paths: the studio says nothing about notes, so
/// there is nothing here for the two elaborations to disagree about, and one
/// implementation is one fewer place for them to drift.
pub(crate) fn lower_studio(
    resolver: &mut Resolver,
    piece: &PieceDecl,
    snapshot: &ScoreSnapshot,
    imported: &[musa_language::ast::StudioDecl],
) -> crate::studio::StudioSpec {
    let studio = piece.studio();
    if studio.is_none() && imported.is_empty() {
        return crate::studio::StudioSpec::default();
    }
    let parts: Vec<String> = snapshot
        .parts()
        .iter()
        .map(|(_, part)| part.name().to_string())
        .collect();
    crate::studio::resolve(
        studio.as_ref(),
        imported,
        &parts,
        &mut resolver.references,
        &mut resolver.diagnostics,
    )
}

/// Tempo, meter, key — plus registration of motif declarations (expansion
/// is `expand.rs`'s).
pub(crate) fn lower_header(resolver: &mut Resolver, piece: &PieceDecl, snapshot: &mut ScoreSnapshot) {
    snapshot.set_title(piece.name().unwrap_or_default());
    // A second tempo in the header would leave two answers to "how fast does
    // this piece start" — the one thing a header may not do. The reading
    // itself happens in `elaborate.rs`, where the marking enters the timeline.
    for extra in piece.tempos().iter().skip(1) {
        resolver.report(
            Diagnostic::error(Code::Misplaced, "this piece already says how fast it starts")
                .at(trimmed_span(extra.syntax()), "a second starting tempo")
                .help("write the change in the voice that reaches it, like a meter or a key change")
                .note("a piece has one starting tempo; every later one is a place in the music"),
        );
    }
    // The meter the piece opens in, which is what `crate::elaborate` seeds the
    // barlines with before it folds in the changes the score states. Read here
    // and not complained about here: the same statement becomes the piece's
    // meter fact in `crate::lower::piece`, which refuses one it cannot read, and
    // two readings of one line saying so twice is two sentences about one
    // mistake.
    if let Some(meter) = piece.meter()
        && let Some(written) = parse_meter(&meter)
    {
        resolver.meter = written;
        resolver.meter_written = true;
    }
    lower_front_matter(resolver, piece, snapshot);
    if let Some(performance) = piece.performance() {
        let profiles = parse_profiles(resolver, &performance);
        merge_profiles(resolver, snapshot, &profiles, span_of(performance.syntax()), None);
    }
    register_motifs(resolver, snapshot, &piece.motifs(), None);
    register_fragments(resolver, snapshot, &piece.fragments(), None);
}

/// `composer`, `arranger`, `subtitle`, `copyright`.
///
/// Written twice is an error rather than a silent last-wins: a page whose
/// composer depends on which line the engraver read is worse than a page that
/// refuses to compile.
fn lower_front_matter(resolver: &mut Resolver, piece: &PieceDecl, snapshot: &mut ScoreSnapshot) {
    for statement in piece.front_matter() {
        let Some(role) = statement.role() else { continue };
        let text = statement.text().unwrap_or_default();
        let slot = match role {
            FrontMatterRole::Subtitle => &mut snapshot.front_matter_mut().subtitle,
            FrontMatterRole::Composer => &mut snapshot.front_matter_mut().composer,
            FrontMatterRole::Arranger => &mut snapshot.front_matter_mut().arranger,
            FrontMatterRole::Copyright => &mut snapshot.front_matter_mut().copyright,
        };
        if slot.is_some() {
            let word = match role {
                FrontMatterRole::Subtitle => "subtitle",
                FrontMatterRole::Composer => "composer",
                FrontMatterRole::Arranger => "arranger",
                FrontMatterRole::Copyright => "copyright",
            };
            resolver.error(
                Code::Misplaced,
                format!("this piece already names a {word}"),
                span_of(statement.syntax()),
                format!("a second {word}"),
            );
            continue;
        }
        *slot = Some(text);
    }
}

/// Register a set of motif declarations, refusing to shadow.
///
/// `from` names the library a declaration was imported from; `None` is the
/// piece's own. Two motifs with one name is always an error — an imported
/// name that quietly loses to a local one would make a piece sound different
/// depending on what it imported.
pub(crate) fn register_motifs(
    resolver: &mut Resolver,
    snapshot: &mut ScoreSnapshot,
    motifs: &[musa_language::ast::MotifDecl],
    from: Option<&str>,
) {
    for motif in motifs {
        let name = motif.name().unwrap_or_default();
        let span = trimmed_span(motif.syntax());
        if refuses_to_shadow(resolver, snapshot, &name, span, from) {
            continue;
        }
        if from.is_none()
            && !name.is_empty()
            && let Some(name_span) = token_span(motif.syntax(), SyntaxKind::Identifier)
        {
            resolver.references.declare(NameKind::Motif, &name, name_span);
        }
        snapshot.push_motif(crate::score::MotifDeclaration {
            name: name.clone(),
            span,
        });
        resolver.motifs.insert(name);
    }
}

/// Register the fragment declarations, refusing to shadow.
///
/// Separate from [`register_motifs`] only because the AST nodes differ: a
/// fragment takes no parameters, so it is not a motif whose parameter list
/// happens to be empty — it is material with nothing to substitute into.
pub(crate) fn register_fragments(
    resolver: &mut Resolver,
    snapshot: &mut ScoreSnapshot,
    fragments: &[musa_language::ast::FragmentDecl],
    from: Option<&str>,
) {
    for fragment in fragments {
        let name = fragment.name().unwrap_or_default();
        let span = trimmed_span(fragment.syntax());
        if refuses_to_shadow(resolver, snapshot, &name, span, from) {
            continue;
        }
        if from.is_none()
            && !name.is_empty()
            && let Some(name_span) = token_span(fragment.syntax(), SyntaxKind::Identifier)
        {
            resolver.references.declare(NameKind::Fragment, &name, name_span);
        }
        snapshot.push_motif(crate::score::MotifDeclaration {
            name: name.clone(),
            span,
        });
        resolver.motifs.insert(name);
    }
}

/// Register every named `bar` in the score, in source order.
///
/// A bar is declared where it sounds, which is inside a voice, so this walks
/// the score rather than reading a list off the piece. Registration happens
/// before any voice is elaborated for the same reason a motif's does: a name
/// has to exist before the thing that plays it is read. Whether a `use` is
/// *allowed* to reach a given bar is a separate question, and one the spans
/// answer — see [`MotifDef::span`].
pub(crate) fn register_bars(
    resolver: &mut Resolver,
    snapshot: &mut ScoreSnapshot,
    score: &musa_language::ast::ScoreDecl,
) {
    for bar in named_bars(score) {
        let Some(name) = bar.name() else { continue };
        let span = trimmed_span(bar.syntax());
        if refuses_to_shadow(resolver, snapshot, &name, span, None) {
            continue;
        }
        if let Some(name_span) = token_span(bar.syntax(), SyntaxKind::Identifier) {
            resolver.references.declare(NameKind::Bar, &name, name_span);
        }
        snapshot.push_motif(crate::score::MotifDeclaration {
            name: name.clone(),
            span,
        });
        resolver.motifs.insert(name);
    }
}

/// Every named bar in the score, outermost first and in source order.
fn named_bars(score: &musa_language::ast::ScoreDecl) -> Vec<musa_language::ast::BarStmt> {
    let mut found = Vec::new();
    for node in score.syntax().descendants() {
        if node.kind() == SyntaxKind::BarStmt
            && let Some(bar) = musa_language::ast::BarStmt::cast(node)
            && bar.name().is_some()
        {
            found.push(bar);
        }
    }
    found
}

/// Report a name that is already taken, and say where by.
///
/// Two declarations of one name is always an error — an imported name that
/// quietly lost to a local one would make a piece sound different depending on
/// what it imported — and a bar shares the rule because it shares the
/// namespace.
fn refuses_to_shadow(
    resolver: &mut Resolver,
    snapshot: &ScoreSnapshot,
    name: &str,
    span: SourceSpan,
    from: Option<&str>,
) -> bool {
    if !resolver.motifs.contains(name) {
        return false;
    }
    let first = snapshot
        .motifs()
        .iter()
        .find(|declared| declared.name == name)
        .map(|declared| declared.span);
    resolver.report(
        Diagnostic::error(
            Code::DuplicateName,
            match from {
                Some(path) => format!("`{path}` also declares `{name}`"),
                None => format!("`{name}` is declared twice"),
            },
        )
        .at(span, "declared again here")
        .maybe_also(first, "first declared here")
        .help("rename one of them, or delete this declaration")
        .note("musa has no shadowing: a name means one thing everywhere the piece can see it"),
    );
    true
}

/// Merge a `performance` block's profiles into the snapshot, refusing to
/// shadow for the same reason motifs do.
pub(crate) fn merge_profiles(
    resolver: &mut Resolver,
    snapshot: &mut ScoreSnapshot,
    profiles: &ProfileSet,
    span: SourceSpan,
    from: Option<&str>,
) {
    let names: Vec<String> = profiles.names().map(str::to_owned).collect();
    for name in names {
        if snapshot.profiles().declares(&name) {
            resolver.report(
                Diagnostic::error(
                    Code::DuplicateName,
                    match from {
                        Some(path) => format!("`{path}` also declares profile `{name}`"),
                        None => format!("profile `{name}` is declared twice"),
                    },
                )
                .at(span, "declared again here")
                .help("rename one of them, or delete this declaration"),
            );
            continue;
        }
        if let Some(profile) = profiles.get(&name) {
            snapshot.profiles_mut().insert(profile.clone());
        }
    }
}
