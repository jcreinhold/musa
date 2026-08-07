/**
 * One place where a command id becomes something happening.
 *
 * The list itself is generated from the Rust registry
 * (`03-interaction.md` §6), so the native menu, this table, and — from prompt
 * 23 — the palette and the keyboard sheet cannot disagree about what exists
 * or what it is called. What each id *does* is here, because it is the
 * interface's half of the answer.
 */

import registry from "./generated/commands.json";
import type { CommandDescriptor } from "./generated/CommandDescriptor";
import type { Session } from "./session.svelte";
import type { ThemeChoice } from "./theme.svelte";

export const COMMANDS = registry as readonly CommandDescriptor[];

/** The parts of the interface a command can reach. */
export interface Surface {
  session: Session;
  theme: ThemeChoice;
  zoom(by: number): void;
  resetZoom(): void;
}

const ACTIONS: Record<string, (surface: Surface) => void> = {
  "file.new": ({ session }) => void session.create(),
  "file.open": ({ session }) => void session.open(),
  "file.save": ({ session }) => void session.save(),
  "file.export.mei": ({ session }) => void session.exportTo("mei"),
  "file.export.lilypond": ({ session }) => void session.exportTo("lilyPond"),
  "file.export.wav": ({ session }) => void session.exportTo("wav"),
  "edit.undo": ({ session }) => void session.undo(),
  "edit.redo": ({ session }) => void session.redo(),
  "edit.format": ({ session }) => void session.format(),
  "view.zoom.out": (surface) => surface.zoom(-1),
  "view.zoom.in": (surface) => surface.zoom(1),
  "view.zoom.reset": (surface) => surface.resetZoom(),
  "view.drawer": ({ session }) => (session.drawerOpen = !session.drawerOpen),
  "view.theme": ({ theme }) => theme.toggle(),
};

/** Run a command by id. Unknown or unimplemented ids do nothing. */
export function dispatch(id: string, surface: Surface): void {
  ACTIONS[id]?.(surface);
}
