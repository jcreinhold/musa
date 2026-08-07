/**
 * Every command, its binding, and what it does — in one file
 * (`docs/interface/03-interaction.md` §3).
 *
 * One list, three readers: the key handler, the command palette (§6), and the
 * keyboard sheet. Bindings and their documentation cannot drift because there
 * is nowhere for them to drift *to*. The native menu is generated from the
 * Rust registry, and a test asserts that every command in that registry
 * appears here with the same words and the same binding.
 *
 * Bindings are spelled in Tauri's accelerator vocabulary — `CmdOrCtrl+K`,
 * `Alt+ArrowLeft` — for the same reason: the menu already speaks it, and one
 * spelling means one parser and one place to be wrong.
 */

import registry from "../session/generated/commands.json";
import type { CommandDescriptor } from "../session/generated/CommandDescriptor";
import type { Session } from "../session/session.svelte";
import type { ThemeChoice } from "../session/theme.svelte";
import type { Workspace } from "../state/selection.svelte";

/** The commands the native menu shows, generated from the Rust registry. */
export const REGISTERED = registry as readonly CommandDescriptor[];

/** Where a command belongs in the palette and the keyboard sheet. */
export type Group = "File" | "Edit" | "Score" | "Transport" | "View" | "Help";

/** The parts of the interface a command can reach. */
export interface Surface {
  session: Session;
  theme: ThemeChoice;
  /** Absent on the launch screen, where there is no score to navigate. */
  workspace?: Workspace;
  zoom(by: number): void;
  resetZoom(): void;
  /** Cycle the follow mode: off → page → continuous. */
  follow(): void;
  /** Loop the selected range, or stop looping. */
  loop(): void;
  /** Pin Origin view, for anyone who cannot hold a key while pointing. */
  origin(): void;
  /** Turn note entry on or off (prompt 25). */
  entry(): void;
  /** Lift the selected notes into a motif, naming it inline (prompt 25). */
  extract(): void;
  palette(open: boolean): void;
  keys(open: boolean): void;
  /** Clear the selection, or — with nothing selected — close the drawer. */
  escape(): void;
}

export interface Command {
  id: string;
  /** What the user sees, in the same words as the result it reports. */
  title: string;
  group: Group;
  /** Tauri accelerator spelling, or null for a mouse-only command. */
  accelerator: string | null;
  /**
   * Where the binding applies. `score` bindings are the unmodified keys —
   * arrows, `Tab`, `Home` — which belong to the score pane and must not fire
   * while the composer is typing in the drawer or the palette.
   */
  scope: "global" | "score";
  run(surface: Surface): void;
}

/** The registry's words for a command, so the two cannot disagree. */
function registered(id: string): Pick<CommandDescriptor, "title" | "accelerator"> {
  const found = REGISTERED.find((command) => command.id === id);
  return { title: found?.title ?? id, accelerator: found?.accelerator ?? null };
}

function command(id: string, group: Group, run: (surface: Surface) => void): Command {
  return { id, group, run, scope: "global", ...registered(id) };
}

function own(
  id: string,
  title: string,
  group: Group,
  accelerator: string | null,
  run: (surface: Surface) => void,
  scope: Command["scope"] = group === "Score" ? "score" : "global",
): Command {
  return { id, title, group, accelerator, run, scope };
}

/**
 * The map.
 *
 * Order is the order the palette and the sheet show: what a piece is (File),
 * what you change (Edit), where you are (Score), what you hear (Transport),
 * what you see (View).
 */
export const COMMANDS: readonly Command[] = [
  command("file.new", "File", ({ session }) => void session.create()),
  command("file.open", "File", ({ session }) => void session.open()),
  command("file.save", "File", ({ session }) => void session.save()),
  command("file.export.mei", "File", ({ session }) => void session.exportTo("mei")),
  command("file.export.lilypond", "File", ({ session }) => void session.exportTo("lilyPond")),
  command("file.export.wav", "File", ({ session }) => void session.exportTo("wav")),

  command("edit.undo", "Edit", ({ session }) => void session.undo()),
  command("edit.redo", "Edit", ({ session }) => void session.redo()),
  command("edit.format", "Edit", ({ session }) => void session.format()),

  own("score.previous", "Previous note", "Score", "ArrowLeft", ({ workspace }) =>
    workspace?.step(-1),
  ),
  own("score.next", "Next note", "Score", "ArrowRight", ({ workspace }) => workspace?.step(1)),
  own("score.voice.up", "Voice above", "Score", "ArrowUp", ({ workspace }) => workspace?.voice(-1)),
  own("score.voice.down", "Voice below", "Score", "ArrowDown", ({ workspace }) =>
    workspace?.voice(1),
  ),
  own("score.bar.previous", "Previous bar", "Score", "Alt+ArrowLeft", ({ workspace }) =>
    workspace?.bar(-1),
  ),
  own("score.bar.next", "Next bar", "Score", "Alt+ArrowRight", ({ workspace }) =>
    workspace?.bar(1),
  ),
  own("score.first", "First note of the voice", "Score", "Home", ({ workspace }) =>
    workspace?.edge("first"),
  ),
  own("score.last", "Last note of the voice", "Score", "End", ({ workspace }) =>
    workspace?.edge("last"),
  ),
  own("score.part.next", "Next part", "Score", "Tab", ({ workspace }) => workspace?.part(1)),
  // Note entry is a mode because the unmodified letters already belong to the
  // navigation map: `f` follows and `l` loops, so a bare `f` cannot also be
  // the note F. `N` is how a composer says "the letters are notes now", and it
  // is the key every notation editor they have used binds it to.
  own("score.entry", "Note entry", "Score", "N", (surface) => surface.entry()),
  // Extraction is the composer noticing they have written the same idea
  // twice; `M` for motif, and the name is asked for in the margin rather than
  // in a dialog that would take the notes off the screen.
  own("score.extract", "Extract a motif", "Score", "M", (surface) => surface.extract()),
  // Escape is the one Score command that is global: giving up is answered
  // wherever the composer happens to be, including the drawer.
  own(
    "score.clear",
    "Clear the selection",
    "Score",
    "Escape",
    (surface) => surface.escape(),
    "global",
  ),

  own(
    "transport.play",
    "Play or pause",
    "Transport",
    "Space",
    ({ session }) => void session.toggle(),
  ),
  own(
    "transport.play.selection",
    "Play from the selection",
    "Transport",
    "Shift+Space",
    (surface) => void surface.session.playFrom(surface.workspace?.focused?.onsetFrames ?? 0),
  ),
  own(
    "transport.stop",
    "Stop and return to the start",
    "Transport",
    "Enter",
    ({ session }) => void session.stop(),
  ),
  own("transport.loop", "Loop the selection", "Transport", "L", (surface) => surface.loop()),
  own("transport.follow", "Follow the playhead", "Transport", "F", (surface) => surface.follow()),

  // The lens itself is held rather than run (`04-provenance.md` §2), so what
  // the map carries is the pin: the same view, kept, for anyone who cannot
  // hold a key and work the pointer at once.
  own("view.origin", "Pin Origin view (hold O)", "View", "Shift+O", (surface) => surface.origin()),

  command("view.zoom.out", "View", (surface) => surface.zoom(-1)),
  command("view.zoom.in", "View", (surface) => surface.zoom(1)),
  command("view.zoom.reset", "View", (surface) => surface.resetZoom()),
  command("view.drawer", "View", ({ session }) => (session.drawerOpen = !session.drawerOpen)),
  command("view.theme", "View", ({ theme }) => theme.toggle()),
  command("view.palette", "View", (surface) => surface.palette(true)),

  command("help.keys", "Help", (surface) => surface.keys(true)),
];

/** Run a command by id. Unknown ids do nothing. */
export function dispatch(id: string, surface: Surface): void {
  COMMANDS.find((candidate) => candidate.id === id)?.run(surface);
}

interface Chord {
  mod: boolean;
  shift: boolean;
  alt: boolean;
  key: string;
}

function parse(accelerator: string): Chord {
  const parts = accelerator.split("+");
  const key = parts[parts.length - 1] ?? "";
  return {
    mod: parts.includes("CmdOrCtrl"),
    shift: parts.includes("Shift"),
    alt: parts.includes("Alt"),
    key: key.toLowerCase(),
  };
}

/**
 * Whether a keystroke is this binding.
 *
 * `event.key` rather than `event.code`, so a binding follows the keyboard
 * layout the composer actually types on. `?` arrives as a shifted `/` on most
 * layouts, so a binding written `?` matches the character, not the chord.
 */
export function matches(accelerator: string, event: KeyboardEvent): boolean {
  const chord = parse(accelerator);
  const pressed = event.key === " " ? "space" : event.key.toLowerCase();
  if (pressed !== chord.key) return false;
  if (chord.mod !== (event.metaKey || event.ctrlKey)) return false;
  if (chord.alt !== event.altKey) return false;
  if (chord.shift) return event.shiftKey;
  // `?` is a shifted character on most layouts, so a binding written `?`
  // matches the character the composer produced rather than a chord.
  const typedWithShift = chord.key.length === 1 && !/[a-z0-9]/.test(chord.key);
  return !event.shiftKey || typedWithShift;
}

/**
 * The command a keystroke means, or undefined.
 *
 * `within` is where the keystroke happened: `score` when the score pane has
 * focus, `global` anywhere else. Unmodified keys belong to the score, so that
 * typing an `f` in the drawer is an `f` and not a follow-mode toggle.
 */
export function commandFor(
  event: KeyboardEvent,
  within: "global" | "score" = "global",
): Command | undefined {
  return COMMANDS.find(
    (candidate) =>
      candidate.accelerator !== null &&
      (candidate.scope === "global" || within === "score") &&
      matches(candidate.accelerator, event),
  );
}

const SYMBOLS: Record<string, string> = {
  CmdOrCtrl: "⌘",
  Shift: "⇧",
  Alt: "⌥",
  ArrowLeft: "←",
  ArrowRight: "→",
  ArrowUp: "↑",
  ArrowDown: "↓",
  Enter: "↩",
  Escape: "Esc",
  Space: "Space",
};

/**
 * A binding as the sheet and the palette print it.
 *
 * Modifiers are set as their glyphs and run together — `⇧Space`, `⌥←` — which
 * is how a musician reading a manual expects to see them, and how the OS
 * prints them in its own menus.
 */
export function spell(accelerator: string | null): string {
  if (accelerator === null) return "";
  const parts = accelerator.split("+");
  return parts.map((part) => SYMBOLS[part] ?? part).join("");
}
