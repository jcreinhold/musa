/**
 * Terms, as the source column asks about them
 * (`docs/interface/08-elaboration.md` §§1–2).
 *
 * Pure lookups over facts the core computed: which name covers an offset,
 * which declaration that name reaches, where it is used. Nothing here parses
 * musa, infers a type, or words a sentence — the signature, the summary, and
 * the distinction line are all written in the source and carried on the wire
 * (`03-interaction.md` §7).
 */

import type { NameFacts, Span, TermFacts, TermSite } from "./snapshot";

/**
 * The part of a snapshot these lookups need.
 *
 * Structural rather than `ProjectSnapshot`, so the source editor can be given
 * two lists and answered — a component that had to be handed a whole snapshot
 * to say what a word means would be a component only a running session can
 * exercise.
 */
export interface Vocabulary {
  terms: TermFacts[];
  names: NameFacts[];
}

/** Whether `span` covers `offset`, counting both ends as inside. */
function covers(span: Span, offset: number): boolean {
  return offset >= span.start && offset <= span.end;
}

/**
 * The resolved name written at `offset`, if one is.
 *
 * Declarations count as well as uses: hovering the name in `motif sigh()`
 * asks the same question as hovering `sigh()` further down, and answering one
 * and not the other would be a distinction only the resolver can see.
 */
export function nameAt(known: Vocabulary, offset: number): NameFacts | undefined {
  return known.names.find(
    (name) =>
      (name.declaration !== null && covers(name.declaration, offset)) ||
      name.uses.some((use) => covers(use, offset)),
  );
}

/**
 * The declaration a name reaches.
 *
 * Matched on name *and* kind: a part and a motif may share a word, and the
 * pair is the identity a reference carries.
 */
export function termOf(known: Vocabulary, name: NameFacts): TermFacts | undefined {
  return known.terms.find((term) => term.name === name.name && term.kind === name.kind);
}

/** The declaration written at `offset`, when the name there has one. */
export function termAt(known: Vocabulary, offset: number): TermFacts | undefined {
  const name = nameAt(known, offset);
  return name && termOf(known, name);
}

/**
 * Where the name at `offset` is written, as a place the interface can go.
 *
 * A declaration in the open document is a span to put the caret at; one in a
 * library module is the module and the opaque handle that opens it. Undefined
 * when nothing is written there, or when the compiler never resolved it — a
 * guess would be the interface having a theory of the language.
 */
export function definitionAt(known: Vocabulary, offset: number): TermSite | undefined {
  const name = nameAt(known, offset);
  if (!name) return undefined;
  if (name.declaration) return { where: "open", span: name.declaration };
  return termOf(known, name)?.site;
}

/**
 * Every resolved use of the name at `offset`, declaration included.
 *
 * Uses, not text matches: two different `root`s are two names, and the
 * resolver is the only thing that knows which is which.
 */
export function usesAt(known: Vocabulary, offset: number): Span[] {
  const name = nameAt(known, offset);
  if (!name) return [];
  return name.declaration ? [name.declaration, ...name.uses] : [...name.uses];
}

/**
 * The declarations a completion list offers, in the order the core listed
 * them.
 *
 * Not filtered, not ranked, not re-worded: the list is the core's, and a
 * frontend that scored it would be deciding which term is most musical.
 * Deprecated declarations stay in — the reader is told what to write instead
 * rather than shown a gap where a name they can see in their own file was.
 */
export function completions(known: Vocabulary | null | undefined): TermFacts[] {
  return known?.terms ?? [];
}
