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
  import { closeBrackets } from "@codemirror/autocomplete";
  import { defaultKeymap, indentWithTab } from "@codemirror/commands";
  import { bracketMatching, foldGutter, foldKeymap, syntaxHighlighting } from "@codemirror/language";
  import { setDiagnostics } from "@codemirror/lint";
  import {
    Compartment,
    EditorState,
    StateEffect,
    StateField,
    type ChangeSpec,
    type Extension,
  } from "@codemirror/state";
  import {
    Decoration,
    EditorView,
    drawSelection,
    dropCursor,
    highlightActiveLine,
    highlightActiveLineGutter,
    keymap,
    lineNumbers,
    type DecorationSet,
  } from "@codemirror/view";

  import { untrack } from "svelte";

  import { musa, musaHighlighting } from "../lang-musa";
  import type { Reveal } from "../state/reveal";
  import type { Diagnostic, Span } from "../state/snapshot";

  let {
    source,
    diagnostics = [],
    editable = true,
    highlight = [],
    reveal = null,
    onedit,
    oncaret,
  }: {
    source: string;
    /** The compiler's own, never recomputed here (`05-states.md` §5). */
    diagnostics?: Diagnostic[];
    editable?: boolean;
    /** Spans to mark: the provenance of what is on the page. */
    highlight?: Span[];
    /** A place to put the caret, once, when it changes. */
    reveal?: Reveal | null;
    onedit?: (source: string) => void;
    /** Where the caret is now, so the score can follow it. */
    oncaret?: (offset: number) => void;
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
              .filter((span) => span.end > span.start && span.end <= transaction.newDoc.length)
              .map((span) => marked.range(span.start, span.end)),
            true,
          );
        }
      }
      return current.map(transaction.changes);
    },
    provide: (field) => EditorView.decorations.from(field),
  });

  const writable = new Compartment();

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
    ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--ink)" },
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
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
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
    ".cm-tooltip": {
      backgroundColor: "var(--leaf)",
      border: "1px solid var(--rule)",
      color: "var(--ink)",
      fontFamily: "var(--f-ui)",
      fontSize: "var(--t-small-size)",
    },
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
    return globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches ? 0 : 1200;
  }

  function extensions(): Extension[] {
    return [
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
      marks,
      appearance,
      writable.of(EditorState.readOnly.of(!editable)),
      EditorView.lineWrapping,
      // The editable surface is the field, and it says what it is: every
      // test and every screen reader finds the source by this name.
      EditorView.contentAttributes.of({ "aria-label": "Source", spellcheck: "false" }),
      EditorView.updateListener.of((update) => {
        if (update.docChanged && !echoing) onedit?.(update.state.doc.toString());
        if (update.selectionSet) oncaret?.(update.state.selection.main.head);
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
    while (tail < limit - start && from[from.length - 1 - tail] === to[to.length - 1 - tail]) {
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
    view?.dispatch({ effects: writable.reconfigure(EditorState.readOnly.of(!editable)) });
  });

  $effect(() => {
    const spans = highlight;
    view?.dispatch({ effects: setMarks.of(spans) });
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
            to: Math.min(Math.max(diagnostic.span?.end ?? 0, (diagnostic.span?.start ?? 0) + 1), length),
            severity: diagnostic.severity === "error" ? ("error" as const) : ("warning" as const),
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
