/**
 * The selection model of `docs/interface/03-interaction.md` §1, and the small
 * amount of state that goes with it.
 *
 * Selection is by `EventId` — never by index, never by DOM node — so it
 * survives re-engraving (`02-engraving.md` §6). Everything this module knows
 * about the music it read from the snapshot; it computes nothing musical
 * (§7). The caret is declared here because it is a first-class state of the
 * model, and is constructed from prompt 25 onward when entry exists.
 */

import type { EventFacts, OccurrenceFacts, ProjectSnapshot, Span } from "./snapshot";

export type Selection =
  | { kind: "none" }
  | { kind: "caret"; part: string; voice: string; before: string | "end" }
  | { kind: "event"; events: string[] }
  | { kind: "range"; part: string; voice: string; from: string; to: string };

/** The events of one voice, in the order the core listed them. */
function voiceEvents(snapshot: ProjectSnapshot | null, part: string, voice: string): EventFacts[] {
  return (snapshot?.score?.events ?? []).filter(
    (event) => event.part === part && event.voice === voice,
  );
}

export class Workspace {
  /**
   * Read on demand rather than copied, because the session replaces the
   * snapshot on every revision and a selection that outlived its score would
   * describe notes that are no longer there.
   */
  readonly #read: () => ProjectSnapshot | null;
  selection = $state<Selection>({ kind: "none" });
  hovered = $state<string | null>(null);

  /**
   * Set when a re-render moved the selection because the event it was on no
   * longer exists. The inspector says so rather than silently describing a
   * different note (`02-engraving.md` §6).
   */
  adrift = $state<string | null>(null);

  /** The events as they were before the last reconcile, for the fallback. */
  #previous: EventFacts[] = [];

  constructor(read: () => ProjectSnapshot | null) {
    this.#read = read;
  }

  /** The score everything here is read against, or null before one is open. */
  get snapshot(): ProjectSnapshot | null {
    return this.#read();
  }

  /** Every event id the selection covers, in score order. */
  get selected(): string[] {
    const selection = this.selection;
    switch (selection.kind) {
      case "event":
        return selection.events;
      case "range": {
        const events = voiceEvents(this.snapshot, selection.part, selection.voice);
        const from = events.findIndex((event) => event.id === selection.from);
        const to = events.findIndex((event) => event.id === selection.to);
        if (from < 0 || to < 0) return [];
        return events.slice(Math.min(from, to), Math.max(from, to) + 1).map((event) => event.id);
      }
      case "none":
      case "caret":
        return [];
    }
  }

  /**
   * The event the interface is working near: the first of the selection, or —
   * when nothing is selected — the first of the score, so that entry, the
   * active voice, and the position readout always have somewhere to start.
   */
  get focused(): EventFacts | undefined {
    const [first] = this.selected;
    const events = this.snapshot?.score?.events ?? [];
    return first === undefined ? events[0] : events.find((event) => event.id === first);
  }

  /**
   * The event the composer actually chose, which is a different question.
   *
   * [`focused`](#focused) answers "where is work happening" and always has an
   * answer; this answers "what is selected" and is allowed not to. The
   * inspector reads this one, because describing a note nobody picked is how
   * an interface comes to have no empty state at all — and the empty state is
   * where the piece's own facts live (prompt 54).
   */
  get chosen(): EventFacts | undefined {
    return this.selected.length > 0 ? this.focused : undefined;
  }

  /** Active part and voice are part of the selection, not a separate mode. */
  get active(): { part: string; voice: string } | undefined {
    const event = this.focused;
    return event && { part: event.part, voice: event.voice };
  }

  /**
   * Click a note. Applied by the frontend in the same frame from the clicked
   * element's `xml:id` — the one place the frontend acts before the core
   * answers, and safe because it changes nothing semantic (§2).
   */
  select(id: string, extend = false): void {
    this.adrift = null;
    const events = this.snapshot?.score?.events ?? [];
    const [anchorId] = this.selected;
    const anchor = events.find((candidate) => candidate.id === anchorId);
    const event = events.find((candidate) => candidate.id === id);
    if (!event) return;
    if (extend && anchor && anchor.part === event.part && anchor.voice === event.voice) {
      this.selection = {
        kind: "range",
        part: event.part,
        voice: event.voice,
        from: anchor.id,
        to: event.id,
      };
      return;
    }
    this.selection = { kind: "event", events: [event.id] };
  }

  /** Click a voice in the left margin: active voice, caret at its start. */
  selectVoice(part: string, voice: string): void {
    this.adrift = null;
    const [first] = voiceEvents(this.snapshot, part, voice);
    this.selection = first
      ? { kind: "event", events: [first.id] }
      : { kind: "caret", part, voice, before: "end" };
  }

  clear(): void {
    this.selection = { kind: "none" };
    this.adrift = null;
  }

  /**
   * The voice the keyboard is working in: the selection's, or the score's
   * first. Navigation always has somewhere to start, so the first arrow key
   * on a freshly opened piece selects its first note rather than nothing.
   */
  get #voice(): { part: string; voice: string } | undefined {
    const selection = this.selection;
    if (selection.kind === "caret" || selection.kind === "range") {
      return { part: selection.part, voice: selection.voice };
    }
    const event = this.focused;
    return event && { part: event.part, voice: event.voice };
  }

  /** Where in its voice the working position is: the anchor event's index. */
  #at(events: EventFacts[]): number {
    const selection = this.selection;
    const id =
      selection.kind === "caret" ? selection.before : (this.selected[0] ?? this.focused?.id);
    if (id === undefined || id === "end") return events.length - 1;
    return events.findIndex((event) => event.id === id);
  }

  /** Select an event by id without disturbing anything else. */
  #land(event: EventFacts | undefined): void {
    if (!event) return;
    this.adrift = null;
    this.selection = { kind: "event", events: [event.id] };
  }

  /** `←` `→`: the previous or next event in the active voice (§3). */
  step(by: number): void {
    const active = this.#voice;
    if (!active) return;
    const events = voiceEvents(this.snapshot, active.part, active.voice);
    const at = this.#at(events);
    const next = Math.min(Math.max(at + by, 0), events.length - 1);
    this.#land(events[at < 0 ? 0 : next]);
  }

  /**
   * `⇧←` `⇧→`: grow the selection by one event, in the active voice.
   *
   * The keyboard's half of the horizontal drag (`03-interaction.md` §2): the
   * anchor stays where the selection started and the far end moves, so
   * pressing the other arrow shrinks the range rather than starting a new one.
   */
  stretch(by: number): void {
    const active = this.#voice;
    if (!active) return;
    const events = voiceEvents(this.snapshot, active.part, active.voice);
    const selection = this.selection;
    const chosen = this.selected;
    const anchorId = selection.kind === "range" ? selection.from : chosen[0];
    const farId = selection.kind === "range" ? selection.to : chosen[0];
    const anchor = events.findIndex((event) => event.id === anchorId);
    const far = events.findIndex((event) => event.id === farId);
    // Nothing to grow from is a move: the first ⇧→ on an empty selection
    // selects a note, which is what the bare arrow would have done.
    if (anchor < 0 || far < 0) {
      this.step(by);
      return;
    }
    const from = events[anchor];
    const to = events[Math.min(Math.max(far + by, 0), events.length - 1)];
    if (!from || !to) return;
    this.adrift = null;
    this.selection =
      from.id === to.id
        ? { kind: "event", events: [from.id] }
        : { kind: "range", part: active.part, voice: active.voice, from: from.id, to: to.id };
  }

  /** `Home` `End`: the first or last event in the active voice. */
  edge(which: "first" | "last"): void {
    const active = this.#voice;
    if (!active) return;
    const events = voiceEvents(this.snapshot, active.part, active.voice);
    this.#land(which === "first" ? events[0] : events[events.length - 1]);
  }

  /**
   * `⌥←` `⌥→`: the previous or next bar.
   *
   * Bar numbers come from the core; this walks to the first event of the
   * nearest bar in that direction, which is what a composer means by "next
   * bar" when the bar has no note on its downbeat in this voice.
   */
  bar(by: number): void {
    const active = this.#voice;
    if (!active) return;
    const events = voiceEvents(this.snapshot, active.part, active.voice);
    const here = events[Math.max(this.#at(events), 0)];
    if (!here) return;
    const wanted = by < 0 ? [...events].reverse() : events;
    this.#land(wanted.find((event) => (by < 0 ? event.bar < here.bar : event.bar > here.bar)));
  }

  /** `↑` `↓`: the previous or next voice of the active part, in staff order. */
  voice(by: number): void {
    const active = this.#voice;
    const part = this.snapshot?.score?.parts.find((candidate) => candidate.name === active?.part);
    if (!active || !part) return;
    const at = part.voices.findIndex((voice) => voice.name === active.voice);
    const next = part.voices[Math.min(Math.max(at + by, 0), part.voices.length - 1)];
    if (next) this.selectVoice(part.name, next.name);
  }

  /** `Tab`: the next part, wrapping — the score is a loop, not a list. */
  part(by: number): void {
    const parts = this.snapshot?.score?.parts ?? [];
    if (parts.length === 0) return;
    const at = parts.findIndex((candidate) => candidate.name === this.#voice?.part);
    const next = parts[(((at + by) % parts.length) + parts.length) % parts.length];
    const voice = next?.voices[0];
    if (next && voice) this.selectVoice(next.name, voice.name);
  }

  /**
   * Put the caret before an event, or at the end of its voice (§1).
   *
   * The caret is a state of the selection rather than a mode, so placing it
   * is how "between notes" is expressed — including in an empty voice, where
   * there is no note to select instead.
   */
  placeCaret(part: string, voice: string, before: string | "end"): void {
    this.adrift = null;
    this.selection = { kind: "caret", part, voice, before };
  }

  /**
   * Select everything the focused event's origin, cut at `depth` segments,
   * produced in the same voice (`04-provenance.md` §3).
   *
   * Clicking `transform ▸ use` in the inspector is how a composer asks "what
   * else did this make?" — and the answer is a selection, so the score shows
   * it rather than describing it. One voice only, per §1: the selection model
   * has no cross-voice state to put the rest in.
   */
  selectOrigin(depth: number): void {
    const here = this.focused;
    if (!here || depth <= 0) return;
    // The innermost segment *is* the occurrence, and the core already listed
    // exactly what it produced — across voices and staves, which a path
    // prefix within one voice cannot reach (`04-provenance.md` §2).
    if (depth >= here.origin.path.length && this.selectOccurrence(here.origin.occurrence)) return;
    const prefix = here.origin.path.slice(0, depth);
    const kin = voiceEvents(this.snapshot, here.part, here.voice).filter((event) =>
      prefix.every((segment, index) => event.origin.path[index] === segment),
    );
    if (kin.length === 0) return;
    this.adrift = null;
    this.selection = { kind: "event", events: kin.map((event) => event.id) };
  }

  /** Every expansion that ran in this score, as the core listed them. */
  get occurrences(): OccurrenceFacts[] {
    return this.snapshot?.score?.occurrences ?? [];
  }

  /** The occurrence with this id, or undefined. */
  occurrence(id: string | null): OccurrenceFacts | undefined {
    return id === null ? undefined : this.occurrences.find((candidate) => candidate.id === id);
  }

  /**
   * Select everything one expansion produced (`04-provenance.md` §2): the unit
   * a composer wants to operate on, and the one prompt 25's edit-definition
   * path acts through. Reports whether there was an occurrence to select.
   */
  selectOccurrence(id: string | null): boolean {
    const occurrence = this.occurrence(id);
    if (!occurrence || occurrence.events.length === 0) return false;
    this.adrift = null;
    this.selection = { kind: "event", events: [...occurrence.events] };
    return true;
  }

  /** The expansion that produced an event, when one did. */
  occurrenceOf(eventId: string | null): OccurrenceFacts | undefined {
    const event = (this.snapshot?.score?.events ?? []).find(
      (candidate) => candidate.id === eventId,
    );
    return this.occurrence(event?.origin.occurrence ?? null);
  }

  /** The expansion the selection landed in — one, never several. */
  get selectedOccurrence(): OccurrenceFacts | undefined {
    return this.occurrenceOf(this.selected[0] ?? null);
  }

  /** The expansion the pointer is over, which Origin view traces. */
  get hoveredOccurrence(): OccurrenceFacts | undefined {
    return this.occurrenceOf(this.hovered);
  }

  /**
   * The spans the source marks right now: where the expansion in view is
   * declared, and where it was used.
   *
   * `held` is Origin view: hover only counts while the lens is held, or the
   * marks would chase the pointer around the page (`04-provenance.md` §2).
   */
  originSpans(held: boolean): Span[] {
    const occurrence = (held ? this.hoveredOccurrence : undefined) ?? this.selectedOccurrence;
    if (!occurrence) return [];
    return [occurrence.declaration, occurrence.useSite].filter((span) => span !== null);
  }

  /**
   * What the source marks: where the music in view is written.
   *
   * The chosen note's own text, and — when it came from a motif — the
   * declaration and the `use` that produced it. Marking the note itself is
   * how the link reads in the other direction: choosing a note on the page
   * shows it in the text without taking the keyboard off the page.
   */
  sourceSpans(held: boolean): Span[] {
    const chosen = this.focused?.origin.span;
    const origins = this.originSpans(held);
    return chosen ? [...origins, chosen] : origins;
  }

  /**
   * The events a diagnostic points at: those whose source text encloses it.
   *
   * Spans come from the core on both sides, so this is a containment test, not
   * a parse — the frontend still computes nothing musical (§7).
   */
  eventsForSpan(span: Span | null): string[] {
    if (!span) return [];
    return (this.snapshot?.score?.events ?? [])
      .filter(
        (event) => event.origin.span.start <= span.start && span.start < event.origin.span.end,
      )
      .map((event) => event.id);
  }

  /**
   * Where the caret is drawn, as an event and a side of it.
   *
   * The engraving has notes, not gaps, so "before this note" and "after the
   * last one" are the only two things a caret can be pinned to. A voice with
   * no events yet has nowhere to draw it, and says so with `null` rather than
   * putting a hairline at the origin.
   */
  get caretAt(): { id: string; side: "before" | "after" } | null {
    const selection = this.selection;
    if (selection.kind !== "caret") return null;
    if (selection.before !== "end") return { id: selection.before, side: "before" };
    const events = voiceEvents(this.snapshot, selection.part, selection.voice);
    const last = events[events.length - 1];
    return last ? { id: last.id, side: "after" } : null;
  }

  /**
   * Carry the selection across a re-render (`02-engraving.md` §6).
   *
   * Selection is by id, so it usually survives untouched. When the selected
   * event is gone — the user deleted the note they were on — it moves to the
   * nearest surviving neighbour in the same voice, in the order the score had
   * before the edit, and says that it moved.
   */
  reconcile(): void {
    const events = this.snapshot?.score?.events ?? [];
    const previous = this.#previous;
    this.#previous = events;

    const [first] = this.selected;
    if (first === undefined) return;
    if (events.some((event) => event.id === first)) {
      this.adrift = null;
      return;
    }

    const gone = previous.find((event) => event.id === first);
    const surviving = gone
      ? previous.filter(
          (event) =>
            event.part === gone.part &&
            event.voice === gone.voice &&
            events.some((kept) => kept.id === event.id),
        )
      : [];
    const at = gone ? previous.indexOf(gone) : -1;
    const nearest = surviving.reduce<EventFacts | undefined>((best, candidate) => {
      if (!best) return candidate;
      const distance = (event: EventFacts) => Math.abs(previous.indexOf(event) - at);
      return distance(candidate) < distance(best) ? candidate : best;
    }, undefined);

    if (nearest) {
      this.selection = { kind: "event", events: [nearest.id] };
      this.adrift = "The note this described is gone";
    } else {
      this.selection = { kind: "none" };
      this.adrift = null;
    }
  }

  /**
   * The accessible name for an event: a musical sentence, built from the
   * snapshot (`03-interaction.md` §5). A score reader that speaks musical
   * facts is better than one that speaks `path element 4712`.
   */
  describe(event: EventFacts): string {
    const what =
      event.kind === "rest"
        ? `${event.durationSpelling} rest`
        : `${event.pitches.join(" ")} ${event.durationSpelling}`;
    const where = `${event.part}, ${event.voice}, bar ${event.bar} beat ${event.beat.numerator}`;
    const origin = event.origin.generated
      ? `, generated from ${event.origin.path[event.origin.path.length - 1] ?? "an expansion"}`
      : "";
    return `${what}, ${where}${origin}`;
  }
}
