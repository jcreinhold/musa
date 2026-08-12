<script lang="ts">
  /**
   * The source, as an editor rather than a text box (roadmap §14.1, §14.4).
   *
   * CodeMirror 6 because its model is transactions over immutable state,
   * which is the same shape as the project's own: a change is a transaction,
   * the document is canonical, and there is no second mutable copy anywhere
   * in the path. That is also why the editor keeps no history of its own —
   * `⌘Z` is the project's undo, over revisions, and two undo stacks over one
   * document is one too many.
   *
   * Everything it shows about the music came from the core: diagnostics are
   * the compiler's, highlighted spans are provenance, and the caret's meaning
   * is reported outward rather than interpreted here (`03-interaction.md` §7).
   */
  import {
    autocompletion,
    closeBrackets,
    type CompletionSource,
  } from "@codemirror/autocomplete";
  import { defaultKeymap, indentWithTab } from "@codemirror/commands";
  import {
    bracketMatching,
    foldGutter,
    foldKeymap,
    syntaxHighlighting,
  } from "@codemirror/language";
  import { setDiagnostics } from "@codemirror/lint";
  import {
    Compartment,
    EditorState,
    RangeSet,
    StateEffect,
    StateField,
    type ChangeSpec,
    type Extension,
    type Range,
    type Text,
  } from "@codemirror/state";
  import {
    Decoration,
    EditorView,
    GutterMarker,
    drawSelection,
    dropCursor,
    gutterLineClass,
    highlightActiveLine,
    highlightActiveLineGutter,
    keymap,
    lineNumbers,
    WidgetType,
    hoverTooltip,
    type DecorationSet,
  } from "@codemirror/view";

  import { untrack } from "svelte";

  import {
    musa,
    musaHighlighting,
    docParts,
    keywordDoc,
    proseRuns,
    type KeywordDoc,
  } from "../lang-musa";
  import type { Reveal } from "../state/reveal";
  import type {
    Diagnostic,
    NameFacts,
    Span,
    TermFacts,
  } from "../state/snapshot";
  import { termAt } from "../state/terms";
  import { modal as modalKeymap, serve } from "./vim";

  let {
    source,
    diagnostics = [],
    editable = true,
    highlight = [],
    focus = null,
    sounding = [],
    candidate = null,
    reveal = null,
    modal = false,
    onedit,
    oncaret,
    onpoint,
    onundo,
    onredo,
    onsave,
    terms = [],
    names = [],
    onlibrary,
  }: {
    source: string;
    /** The compiler's own, never recomputed here (`05-states.md` §5). */
    diagnostics?: Diagnostic[];
    editable?: boolean;
    /** Spans to mark: the provenance of what is on the page. */
    highlight?: Span[];
    /**
     * The focus, in the text: the line that placed the music, and
     * — when it is elsewhere — the statement that spells it.
     */
    focus?: { definition: Span | null; place: Span | null } | null;
    /**
     * The statements that made the music on the page in view. Their lines take
     * a tick in the gutter: the difference between a line that sounds and a
     * line that is scaffolding, which is the shape of a musa file at a glance.
     */
    sounding?: Span[];
    /**
     * The token a live pointer gesture would replace, and what it would put
     * there. Shown in the text, in the file it will be written
     * into, before anything is committed.
     */
    candidate?: { start: number; end: number; text: string } | null;
    /** A place to put the caret, once, when it changes. */
    reveal?: Reveal | null;
    /** Vim mode: the composer's preference, not the document's. */
    modal?: boolean;
    onedit?: (source: string) => void;
    /** Where the caret is now, so the score can follow it. */
    oncaret?: (offset: number) => void;
    /** Where the pointer is in the text, or null when it is not in it. */
    onpoint?: (line: { from: number; to: number } | null) => void;
    /**
     * What `u`, `⌃r`, and `:w` do — the project's undo, redo, and save.
     *
     * They are props rather than anything the editor owns because the whole
     * point is that they are *not* the editor's: vim's own history would be a
     * second stack over one document, which is the thing this editor exists
     * to not have.
     */
    onundo?: () => void;
    onredo?: () => void;
    onsave?: () => void;
    /**
     * Every declaration in scope, and every resolved name, from the last
     * valid compile (`08-elaboration.md` §2). Both are the core's: what a
     * term means, where it is declared, and which uses are the same name are
     * all answers only the resolver has.
     */
    terms?: TermFacts[];
    names?: NameFacts[];
    /** Follow a term into a bundled module, by the handle it carries. */
    onlibrary?: (uri: string, start: number, end: number) => void;
  } = $props();

  /** The provenance marks, carried in the editor's own state. */
  const setMarks = StateEffect.define<readonly Span[]>();
  const marked = Decoration.mark({ class: "cm-musa-origin" });
  const marks = StateField.define<DecorationSet>({
    create: () => Decoration.none,
    update(current, transaction) {
      for (const effect of transaction.effects) {
        if (effect.is(setMarks)) {
          return Decoration.set(
            effect.value
              .filter(
                (span) =>
                  span.end > span.start &&
                  span.end <= transaction.newDoc.length,
              )
              .map((span) => marked.range(span.start, span.end)),
            true,
          );
        }
      }
      return current.map(transaction.changes);
    },
    provide: (field) => EditorView.decorations.from(field),
  });

  /**
   * The focus, carried in the editor's own state.
   *
   * One effect for all three marks, because they are one answer: the statement
   * that spells the focused music, the line that placed it, and the lines that
   * made what is on the page. Dispatching them separately would let the source
   * show half of a focus.
   */
  interface Focused {
    definition: Span | null;
    place: Span | null;
    sounding: readonly Span[];
  }
  const setFocus = StateEffect.define<Focused>();

  /** The hairline under the statement that spells the focused note. */
  const spelling = Decoration.mark({ class: "cm-musa-focus" });
  const focusMarks = StateField.define<DecorationSet>({
    create: () => Decoration.none,
    update(current, transaction) {
      for (const effect of transaction.effects) {
        if (!effect.is(setFocus)) continue;
        const span = effect.value.definition;
        const fits =
          span &&
          span.end > span.start &&
          span.end <= transaction.newDoc.length;
        return fits && span
          ? Decoration.set([spelling.range(span.start, span.end)])
          : Decoration.none;
      }
      return current.map(transaction.changes);
    },
    provide: (field) => EditorView.decorations.from(field),
  });

  /**
   * The candidate a live gesture would write.
   *
   * Drawn as a replacement over the token, not as an edit to the document:
   * the text is untouched until the pointer comes up, so `Esc` costs nothing
   * and undo has one step rather than one per pixel.
   */
  const setCandidate = StateEffect.define<{
    start: number;
    end: number;
    text: string;
  } | null>();

  class Candidate extends WidgetType {
    readonly #text: string;

    constructor(text: string) {
      super();
      this.#text = text;
    }

    override eq(other: Candidate): boolean {
      return other.#text === this.#text;
    }

    override toDOM(): HTMLElement {
      const span = document.createElement("span");
      span.className = "cm-musa-candidate";
      span.textContent = this.#text;
      return span;
    }
  }

  const candidateMark = StateField.define<DecorationSet>({
    create: () => Decoration.none,
    update(current, transaction) {
      for (const effect of transaction.effects) {
        if (!effect.is(setCandidate)) continue;
        const write = effect.value;
        if (
          !write ||
          write.end > transaction.newDoc.length ||
          write.end < write.start
        ) {
          return Decoration.none;
        }
        return Decoration.set([
          Decoration.replace({ widget: new Candidate(write.text) }).range(
            write.start,
            write.end,
          ),
        ]);
      }
      return current.map(transaction.changes);
    },
    provide: (field) => EditorView.decorations.from(field),
  });

  /** A gutter class, which is all a `GutterMarker` has to be to mark a line. */
  class LineClass extends GutterMarker {
    override elementClass: string;

    constructor(elementClass: string) {
      super();
      this.elementClass = elementClass;
    }
  }
  const PLACED = new LineClass("cm-musa-focus-line");
  const SOUNDS = new LineClass("cm-musa-sounds");

  /** The starts of the lines these spans are on, deduplicated and in order. */
  function lineStarts(doc: Text, spans: readonly Span[]): number[] {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- local set inside a pure helper, not state
    const starts = new Set<number>();
    for (const span of spans) {
      if (span.start < 0 || span.start > doc.length) continue;
      starts.add(doc.lineAt(span.start).from);
    }
    return [...starts].sort((left, right) => left - right);
  }

  const gutterMarks = StateField.define<RangeSet<GutterMarker>>({
    create: () => RangeSet.empty,
    update(current, transaction) {
      for (const effect of transaction.effects) {
        if (!effect.is(setFocus)) continue;
        const doc = transaction.newDoc;
        const { place, sounding } = effect.value;
        const ranges: Range<GutterMarker>[] = [
          ...lineStarts(doc, sounding).map((at) => SOUNDS.range(at)),
          ...lineStarts(doc, place ? [place] : []).map((at) =>
            PLACED.range(at),
          ),
        ];
        return RangeSet.of(ranges, true);
      }
      return current.map(transaction.changes);
    },
    provide: (field) => gutterLineClass.from(field),
  });

  const writable = new Compartment();
  const modality = new Compartment();

  // `u`, `⌃r`, and `:w` are the project's, not the editor's (see `./vim`).
  $effect(() => {
    serve({
      undo: () => onundo?.(),
      redo: () => onredo?.(),
      save: () => onsave?.(),
    });
  });

  /**
   * The look. Set here rather than in a stylesheet because CodeMirror owns
   * the elements; every value is a design token, so the source follows the
   * theme like everything else.
   */
  const appearance = EditorView.theme({
    "&": {
      backgroundColor: "transparent",
      color: "var(--ink)",
      fontFamily: "var(--f-mono)",
      fontSize: "var(--t-body-size)",
      height: "100%",
    },
    ".cm-content": { padding: "0", fontVariantLigatures: "none" },
    ".cm-scroller": { fontFamily: "var(--f-mono)", lineHeight: "1.65" },
    "&.cm-focused": { outline: "none" },
    ".cm-gutters": {
      backgroundColor: "transparent",
      border: "0",
      color: "var(--ink-muted)",
      paddingRight: "var(--s-2)",
    },
    // Where you are is said twice, both times typographically: the line the
    // caret is on takes its number in full ink rather than the muted ink every
    // other number is set in, and the caret itself is drawn below. No band, no
    // fill — a highlighted row behind the text would be a second wash arguing
    // with the provenance wash that means something.
    ".cm-activeLine": { backgroundColor: "transparent" },
    ".cm-activeLineGutter": {
      backgroundColor: "transparent",
      color: "var(--ink)",
    },
    /*
     * The focus. Three marks can now be true of one line at once —
     * selected, provenance, focused — so each is a different *shape*: the
     * selection is a wash, provenance is a wash, and the focus is a number in
     * `--plate` and a hairline under the text. Nothing here moves; a focus
     * appears and disappears (`01-visual-language.md` §6).
     */
    ".cm-musa-focus-line": { color: "var(--plate)" },
    ".cm-musa-focus": { borderBottom: "1px solid var(--plate)" },
    /*
     * The candidate a live gesture would write, standing where
     * the token it would replace stands. `--plate` because it is not in the
     * file yet: the hue that already means *derived, or live*.
     */
    ".cm-musa-candidate": {
      color: "var(--plate)",
      borderBottom: "1px solid var(--plate)",
    },
    /*
     * And the quiet permanent one: a line that made music on the page in view
     * gets a tick beside its number. Not a hue and not a count — just the
     * difference between a line that sounds and a line that is scaffolding.
     * Only in the number gutter, or the fold gutter would tick it again.
     */
    ".cm-lineNumbers .cm-musa-sounds": { position: "relative" },
    ".cm-lineNumbers .cm-musa-sounds::after": {
      content: '""',
      position: "absolute",
      right: "0",
      top: "calc(50% - 0.5px)",
      width: "3px",
      height: "1px",
      backgroundColor: "var(--rule)",
    },
    /*
     * The caret, in `--plate`.
     *
     * That hue is the one the design language already gives to "where the
     * keyboard is and what is live" — the focus ring, the playhead hairline
     * (`01-visual-language.md` §2). The caret is both of those things for the
     * text, and a caret that took `--ink` would be a hairline of body-text
     * colour inside a field of body text: findable only by hunting. Two
     * pixels, because one is a hairline and hairlines are for rules.
     */
    ".cm-cursor, .cm-dropCursor": {
      borderLeftColor: "var(--plate)",
      borderLeftWidth: "2px",
      marginLeft: "-1px",
    },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection":
      {
        backgroundColor: "var(--plate-wash)",
      },
    /*
     * Provenance, in the one hue that ever means it. Inside the mark the hue
     * is the ground, so the type goes back to full ink: `--plate` on
     * `--plate-wash` is the same colour twice and reads at 3.9:1, and the
     * muted greys the highlighter uses for identifiers and punctuation are no
     * better. Marked text is the text this screen is *about* — setting it in
     * the darkest ink is what it deserves anyway.
     */
    ".cm-musa-origin": { backgroundColor: "var(--plate-wash)" },
    ".cm-musa-origin, .cm-musa-origin span": { color: "var(--ink)" },
    ".cm-lintRange-error": {
      backgroundImage: "none",
      borderBottom: "1px solid var(--chalk)",
    },
    ".cm-lintRange-warning": {
      backgroundImage: "none",
      borderBottom: "1px dotted var(--chalk)",
    },
    ".cm-lint-marker-error": { content: "none" },
    /*
     * Vim's own status line, in the interface's type rather than the
     * package's. It is a caption about the editor, not text in the document,
     * so it takes the caption face and the muted ink — and one hairline above
     * it, which is the only rule the source column has ever needed.
     */
    ".cm-vim-panel": {
      backgroundColor: "transparent",
      borderTop: "1px solid var(--rule)",
      color: "var(--ink-muted)",
      fontFamily: "var(--f-mono)",
      fontSize: "var(--t-small-size)",
      padding: "var(--s-1) 0",
    },
    ".cm-vim-panel input": {
      backgroundColor: "transparent",
      border: "0",
      color: "var(--ink)",
      font: "inherit",
      outline: "none",
      width: "100%",
    },
    ".cm-tooltip": {
      backgroundColor: "var(--leaf)",
      border: "1px solid var(--rule)",
      color: "var(--ink)",
      fontFamily: "var(--f-ui)",
      fontSize: "var(--t-small-size)",
    },
    /*
     * The keyword's own documentation, hovered. A tooltip is a
     * leaf the size of a thought: prose in the interface's face, the example
     * in the editor's, and one hairline between what is said and what is
     * shown — the same rule the prose column uses for a footnote.
     */
    ".cm-musa-keyword-doc": {
      maxWidth: "42ch",
      padding: "var(--s-3) var(--s-4)",
    },
    ".cm-musa-keyword-doc p": { margin: "0" },
    ".cm-musa-keyword-doc .head em": {
      color: "var(--ink-muted)",
      fontStyle: "normal",
    },
    ".cm-musa-keyword-doc code": { fontFamily: "var(--f-mono)" },
    ".cm-musa-keyword-doc pre": {
      borderTop: "1px solid var(--rule)",
      fontFamily: "var(--f-mono)",
      margin: "var(--s-2) 0 0",
      paddingTop: "var(--s-2)",
      whiteSpace: "pre-wrap",
    },
    /*
     * A term's own documentation, in the same leaf at the same size. The
     * signature leads because it is what the composer is checking; everything
     * under it is the declaration's own words (`08-elaboration.md` §2).
     */
    ".cm-musa-term-doc": {
      maxWidth: "48ch",
      padding: "var(--s-3) var(--s-4)",
    },
    ".cm-musa-term-doc p": { margin: "var(--s-2) 0 0" },
    ".cm-musa-term-doc .signature": {
      fontFamily: "var(--f-mono)",
      margin: "0",
      whiteSpace: "pre-wrap",
    },
    ".cm-musa-term-doc .deprecated": { color: "var(--chalk)" },
    ".cm-musa-term-doc ul": {
      margin: "var(--s-2) 0 0",
      paddingLeft: "var(--s-4)",
    },
    ".cm-musa-term-doc code": { fontFamily: "var(--f-mono)" },
    ".cm-musa-term-doc summary": {
      color: "var(--ink-muted)",
      cursor: "pointer",
      marginTop: "var(--s-2)",
    },
    ".cm-musa-term-doc .site": {
      background: "none",
      border: "0",
      borderTop: "1px solid var(--rule)",
      color: "var(--ink-muted)",
      cursor: "pointer",
      display: "block",
      font: "inherit",
      marginTop: "var(--s-2)",
      padding: "var(--s-2) 0 0",
      textAlign: "left",
      width: "100%",
    },
    ".cm-musa-term-doc .site:hover": { color: "var(--ink)" },
  });

  let host = $state<HTMLElement | undefined>();
  let view: EditorView | undefined;
  /** True while the editor is applying a document the core sent back. */
  let echoing = false;
  /** The last reveal acted on, so one request moves the caret once. */
  let revealed: Reveal | null = null;

  /**
   * How the caret blinks, in milliseconds — or not at all.
   *
   * A blinking caret is a platform convention rather than one of the three
   * things this application animates (`01-visual-language.md` §6), and it is
   * the signal every reader already knows. Under `prefers-reduced-motion` it
   * stops blinking and stays lit, which is the reduction that loses nothing:
   * the caret is still exactly where it was.
   */
  function blinkRate(): number {
    return globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches
      ? 0
      : 1200;
  }

  function extensions(): Extension[] {
    return [
      // First, because the package asks for it: an extension listed earlier
      // wins the key, and a modal editor whose `d` is the default keymap's is
      // not modal.
      modality.of(modal ? modalKeymap() : []),
      lineNumbers(),
      foldGutter({ markerDOM: marker }),
      highlightActiveLine(),
      highlightActiveLineGutter(),
      // The caret and the selection are drawn by the editor rather than by the
      // platform, so both answer to the theme above: a native caret is the
      // browser's colour, and on a graphite sheet the browser's colour is
      // black on near-black.
      drawSelection({ cursorBlinkRate: blinkRate() }),
      dropCursor(),
      bracketMatching(),
      closeBrackets(),
      keymap.of([...defaultKeymap, ...foldKeymap, indentWithTab]),
      musa(),
      syntaxHighlighting(musaHighlighting),
      keywordDocs,
      termDocs,
      autocompletion({ override: [termCompletions], activateOnTyping: false }),
      marks,
      focusMarks,
      gutterMarks,
      candidateMark,
      appearance,
      writable.of(EditorState.readOnly.of(!editable)),
      EditorView.lineWrapping,
      // The editable surface is the field, and it says what it is: every
      // test and every screen reader finds the source by this name.
      EditorView.contentAttributes.of({
        "aria-label": "Source",
        spellcheck: "false",
      }),
      EditorView.updateListener.of((update) => {
        if (update.docChanged && !echoing)
          onedit?.(update.state.doc.toString());
        if (update.selectionSet) oncaret?.(update.state.selection.main.head);
      }),
      // Which line the pointer is on, so the page can mark what that line
      // wrote. `posAtCoords` and `lineAt` are the editor answering questions
      // about its own document; nothing musical is computed here.
      EditorView.domEventHandlers({
        mousemove(event, view) {
          const at = view.posAtCoords({ x: event.clientX, y: event.clientY });
          const line = at === null ? null : view.state.doc.lineAt(at);
          onpoint?.(line && { from: line.from, to: line.to });
          return false;
        },
        mouseleave() {
          onpoint?.(null);
          return false;
        },
      }),
    ];
  }

  /** The fold arrow, as a mark rather than a widget with a border. */
  function marker(open: boolean): HTMLElement {
    const span = document.createElement("span");
    span.textContent = open ? "▾" : "▸";
    span.style.color = "var(--ink-muted)";
    return span;
  }

  /*
   * The keyword under the pointer teaches. The words are the
   * language's own — generated out of `keywords.rs`, so the tooltip cannot
   * drift from what the lexer and the language server say — and this file
   * only sets them: prose in the interface's face, the example in the
   * editor's, one hairline between.
   */
  const LETTER = /[a-z]/;

  /** The word at `pos`, when it is a keyword the language documents. */
  function keywordAt(view: EditorView, pos: number) {
    const line = view.state.doc.lineAt(pos);
    const text = line.text;
    let start = pos - line.from;
    let end = start;
    while (start > 0 && LETTER.test(text.charAt(start - 1))) start -= 1;
    while (end < text.length && LETTER.test(text.charAt(end))) end += 1;
    const doc = keywordDoc(text.slice(start, end));
    return doc ? { from: line.from + start, to: line.from + end, doc } : null;
  }

  /** The doc, set: the word and its summary, the prose, the example. */
  function renderKeywordDoc(doc: KeywordDoc): HTMLElement {
    const dom = document.createElement("div");
    dom.className = "cm-musa-keyword-doc";

    const head = document.createElement("p");
    head.className = "head";
    const word = document.createElement("strong");
    word.textContent = doc.spelling;
    const summary = document.createElement("em");
    summary.textContent = doc.summary;
    head.append(word, document.createTextNode(" — "), summary);

    const { prose, example } = docParts(doc.doc);
    const body = document.createElement("p");
    body.className = "prose";
    for (const run of proseRuns(prose)) {
      if (run.code) {
        const code = document.createElement("code");
        code.textContent = run.text;
        body.append(code);
      } else {
        body.append(document.createTextNode(run.text));
      }
    }
    dom.append(head, body);

    if (example !== null) {
      const pre = document.createElement("pre");
      pre.textContent = example;
      dom.append(pre);
    }
    return dom;
  }

  const keywordDocs = hoverTooltip((view, pos) => {
    const found = keywordAt(view, pos);
    if (!found) return null;
    return {
      pos: found.from,
      end: found.to,
      above: true,
      create: () => ({ dom: renderKeywordDoc(found.doc) }),
    };
  });

  /*
   * A term teaches too, and the words are the source's own: the signature as
   * it is written, the comment above the declaration, and — behind a
   * disclosure — what only a language implementor wants
   * (`08-elaboration.md` §1). Nothing here is inferred; a name the compiler
   * did not resolve has no tooltip, because a guess would be the interface
   * having a theory of the language.
   */

  /** What a declaration says about itself, set: the two lines, then the rest. */
  function renderTerm(term: TermFacts): HTMLElement {
    const dom = document.createElement("div");
    dom.className = "cm-musa-term-doc";

    const signature = document.createElement("pre");
    signature.className = "signature";
    signature.textContent = term.signature;
    dom.append(signature);

    if (term.summary !== null) {
      const summary = document.createElement("p");
      summary.className = "summary";
      summary.textContent = term.summary;
      dom.append(summary);
    }

    // A deprecation is not detail: it is the one thing a reader must act on,
    // so it stays above the fold and says what to write instead.
    if (term.deprecation !== null) {
      const deprecated = document.createElement("p");
      deprecated.className = "deprecated";
      deprecated.textContent = `Deprecated — ${term.deprecation}`;
      dom.append(deprecated);
    }

    if (term.parameters.length > 0) {
      const list = document.createElement("ul");
      list.className = "parameters";
      for (const parameter of term.parameters) {
        const item = document.createElement("li");
        const label = document.createElement("code");
        label.textContent = parameter.label;
        item.append(label);
        if (parameter.default !== null) {
          item.append(document.createTextNode(" — may be left out"));
        }
        list.append(item);
      }
      dom.append(list);
    }

    const detail = details(term);
    if (detail.length > 0) {
      const disclosure = document.createElement("details");
      const summary = document.createElement("summary");
      summary.textContent = "language detail";
      disclosure.append(summary);
      for (const line of detail) {
        const paragraph = document.createElement("p");
        paragraph.textContent = line;
        disclosure.append(paragraph);
      }
      dom.append(disclosure);
    }

    // A term the composer did not declare was declared somewhere, and that
    // somewhere is readable (`08-elaboration.md` §3). The line says where
    // before it offers to go there, so the offer is not the only way to learn
    // it — `⌘⇧D` reaches the same module from the keyboard.
    if (term.site.where === "library") {
      const { uri, start, end } = term.site;
      const open = document.createElement("button");
      open.type = "button";
      open.className = "site";
      open.textContent = `Declared in ${uri} — read it`;
      open.addEventListener("click", () => onlibrary?.(uri, start, end));
      dom.append(open);
    }
    return dom;
  }

  /** What the disclosure holds: types as types, and where the text lives. */
  function details(term: TermFacts): string[] {
    const lines: string[] = [];
    if (term.result) {
      lines.push(
        term.result.distinction === null
          ? term.result.name
          : `${term.result.name} — ${term.result.distinction}`,
      );
    }
    for (const parameter of term.parameters) {
      if (parameter.ty.distinction !== null) {
        lines.push(
          `${parameter.name}: ${parameter.ty.name} — ${parameter.ty.distinction}`,
        );
      }
    }
    return lines;
  }

  const termDocs = hoverTooltip((view, pos) => {
    const term = termAt({ terms, names }, pos);
    if (!term) return null;
    const word = view.state.wordAt(pos);
    return {
      pos: word?.from ?? pos,
      end: word?.to ?? pos,
      above: true,
      create: () => ({ dom: renderTerm(term) }),
    };
  });

  /**
   * The completion list: the declarations in scope, in the order the core
   * listed them, each with its signature beside it and its summary under it.
   *
   * Explicit only. A list that opened itself on every letter would put a
   * popup over the music a composer is typing beside; `⌃Space` asks for it,
   * which is what the keyboard sheet says.
   */
  const termCompletions: CompletionSource = (context) => {
    const word = context.matchBefore(/[A-Za-z_][A-Za-z0-9_]*/);
    if (!word && !context.explicit) return null;
    return {
      from: word?.from ?? context.pos,
      options: terms.map((term) => ({
        label: term.name,
        detail: term.signature,
        info: term.summary ?? undefined,
        type:
          term.kind === "function" || term.kind === "motif"
            ? "function"
            : "variable",
      })),
    };
  };

  // Built once, for the life of the element. The document and the settings
  // are read untracked on purpose: an editor that was torn down and rebuilt
  // whenever its text changed would throw away the caret, the focus, and any
  // keystroke that arrived while it was doing it. Everything that can change
  // reaches the live editor as a transaction below.
  $effect(() => {
    if (!host) return;
    const parent = host;
    const editor = untrack(
      () => new EditorView({ doc: source, extensions: extensions(), parent }),
    );
    // A scrolling region has to be reachable from the keyboard, and
    // CodeMirror marks its scroller `tabindex="-1"`. Making it a tab stop
    // that hands focus straight to the text is what makes "Tab into the
    // source" mean "put the caret in the source" — reaching it and being able
    // to type in it are the same act.
    editor.scrollDOM.setAttribute("tabindex", "0");
    editor.scrollDOM.addEventListener("focus", (event) => {
      // Only from outside: focus leaving the text must not be pulled back in.
      const from = (event as FocusEvent).relatedTarget;
      if (from instanceof Node && editor.dom.contains(from)) return;
      editor.focus();
    });
    view = editor;
    return () => {
      view = undefined;
      editor.destroy();
    };
  });

  /**
   * The smallest change that turns one string into another, at an offset.
   *
   * Prefix and suffix in common are left alone; what is between them is the
   * change. Two texts that differ only in their indentation therefore differ
   * only in their indentation as far as the editor is concerned.
   */
  function difference(from: string, to: string, at = 0): ChangeSpec {
    const limit = Math.min(from.length, to.length);
    let start = 0;
    while (start < limit && from[start] === to[start]) start += 1;
    let tail = 0;
    while (
      tail < limit - start &&
      from[from.length - 1 - tail] === to[to.length - 1 - tail]
    ) {
      tail += 1;
    }
    return {
      from: at + start,
      to: at + from.length - tail,
      insert: to.slice(start, to.length - tail),
    };
  }

  /**
   * The core's answer, as the change it is.
   *
   * Replacing the whole document would move the caret to the start, drop the
   * scroll, and collapse the selection — which is exactly what "formatting
   * preserves the selection" forbids. So the two texts are compared line by
   * line when they have the same number of lines, which is the case for every
   * re-indentation, and a word the composer had hold of keeps its position
   * through the change. When the shape changed too much for that, one
   * difference over the whole text is still smaller than a replacement.
   */
  function changesFor(from: string, to: string): ChangeSpec {
    const was = from.split("\n");
    const now = to.split("\n");
    if (was.length !== now.length) return difference(from, to);
    const changes: ChangeSpec[] = [];
    let at = 0;
    for (const [index, line] of was.entries()) {
      const next = now[index] ?? "";
      if (line !== next) changes.push(difference(line, next, at));
      at += line.length + 1;
    }
    return changes;
  }

  // The document follows the core: a snapshot that came back with different
  // text — a format, an undo, a structured edit — is applied as the change it
  // is, so the caret, the scroll, and the marks all survive it.
  $effect(() => {
    const next = source;
    const editor = view;
    if (!editor || editor.state.doc.toString() === next) return;
    echoing = true;
    editor.dispatch({ changes: changesFor(editor.state.doc.toString(), next) });
    echoing = false;
  });

  $effect(() => {
    view?.dispatch({
      effects: writable.reconfigure(EditorState.readOnly.of(!editable)),
    });
  });

  // Turning the mode on and off is a reconfiguration, not a rebuild: the
  // caret, the scroll, and the text are the document's and survive it.
  $effect(() => {
    const on = modal;
    view?.dispatch({ effects: modality.reconfigure(on ? modalKeymap() : []) });
  });

  $effect(() => {
    const spans = highlight;
    view?.dispatch({ effects: setMarks.of(spans) });
  });

  // The focus and the page's own lines, as one dispatch: they are one answer.
  $effect(() => {
    const at = focus;
    const lines = sounding;
    view?.dispatch({
      effects: setFocus.of({
        definition: at?.definition ?? null,
        place: at?.place ?? null,
        sounding: lines,
      }),
    });
  });

  $effect(() => {
    const write = candidate;
    view?.dispatch({ effects: setCandidate.of(write) });
  });

  // Diagnostics arrive as the compiler's own list, with the compiler's own
  // words; the gutter and the underline are two views of that one list.
  $effect(() => {
    const editor = view;
    if (!editor) return;
    const length = editor.state.doc.length;
    editor.dispatch(
      setDiagnostics(
        editor.state,
        diagnostics
          .filter((diagnostic) => diagnostic.span !== null)
          .map((diagnostic) => ({
            from: Math.min(diagnostic.span?.start ?? 0, length),
            to: Math.min(
              Math.max(
                diagnostic.span?.end ?? 0,
                (diagnostic.span?.start ?? 0) + 1,
              ),
              length,
            ),
            severity:
              diagnostic.severity === "error"
                ? ("error" as const)
                : ("warning" as const),
            message: diagnostic.message,
          })),
      ),
    );
  });

  // Revealing is a place, not a scroll position: the caret goes there and the
  // line is brought into view (`05-states.md` §5).
  $effect(() => {
    const place = reveal;
    const editor = view;
    // Once per request, and only for a span the text actually has: a place
    // past the end of the document is not a place, and moving the caret to
    // one would take the composer somewhere for no reason.
    if (!place || place === revealed || !editor) return;
    revealed = place;
    if (place.span.start > editor.state.doc.length) return;
    const from = place.span.start;
    const to = Math.min(place.span.end, editor.state.doc.length);
    editor.dispatch({
      selection: { anchor: from, head: to },
      effects: EditorView.scrollIntoView(from, { y: "center" }),
    });
    if (place.focus) editor.focus();
  });
</script>

<div class="editor" bind:this={host}></div>

<style>
  .editor {
    flex: 1;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
  }
</style>
