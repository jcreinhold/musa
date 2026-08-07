/**
 * The law the keyboard map exists to keep: one list, no drift
 * (`03-interaction.md` §3, §6).
 *
 * The native menu is generated from the Rust registry; the palette and the
 * keyboard sheet are generated from `commands/map.ts`. If those two disagree
 * about what a command is called or what key runs it, the application tells
 * the composer two different things — so they are compared here rather than
 * by eye.
 */

import { describe, expect, it } from "vitest";

import { COMMANDS, REGISTERED, commandFor, matches, spell } from "../../src/lib/commands/map";

/** A keystroke, as the handler receives it. */
function press(key: string, modifiers: Partial<KeyboardEvent> = {}): KeyboardEvent {
  return {
    key,
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    ...modifiers,
  } as KeyboardEvent;
}

describe("the map and the registry", () => {
  it("gives every available registered command an implementation", () => {
    const missing = REGISTERED.filter(
      (entry) => entry.available && !COMMANDS.some((command) => command.id === entry.id),
    );
    expect(missing.map((entry) => entry.id)).toEqual([]);
  });

  it("uses the registry's own words and bindings", () => {
    for (const entry of REGISTERED) {
      const command = COMMANDS.find((candidate) => candidate.id === entry.id);
      if (!command) continue;
      expect(command.title, entry.id).toBe(entry.title);
      expect(command.accelerator, entry.id).toBe(entry.accelerator);
    }
  });

  it("binds no key twice within a scope", () => {
    const bound = COMMANDS.filter((command) => command.accelerator !== null);
    const seen = new Map<string, string>();
    for (const command of bound) {
      const key = `${command.scope}:${command.accelerator ?? ""}`;
      expect(seen.get(key), `${command.id} and ${seen.get(key) ?? ""}`).toBeUndefined();
      seen.set(key, command.id);
    }
  });

  it("names every command in sentence case", () => {
    for (const command of COMMANDS) {
      expect(command.title, command.id).toMatch(/^[A-Z][^A-Z]/);
    }
  });
});

describe("matching a keystroke", () => {
  it("reads a chord in Tauri's spelling", () => {
    expect(matches("CmdOrCtrl+K", press("k", { metaKey: true }))).toBe(true);
    expect(matches("CmdOrCtrl+K", press("k"))).toBe(false);
    expect(matches("Alt+ArrowLeft", press("ArrowLeft", { altKey: true }))).toBe(true);
    expect(matches("ArrowLeft", press("ArrowLeft", { altKey: true }))).toBe(false);
  });

  it("treats a shifted character as the character it produces", () => {
    expect(matches("?", press("?", { shiftKey: true }))).toBe(true);
  });

  it("keeps the unmodified keys inside the score", () => {
    expect(commandFor(press("ArrowRight"), "score")?.id).toBe("score.next");
    expect(commandFor(press("ArrowRight"), "global")).toBeUndefined();
    expect(commandFor(press("k", { metaKey: true }), "global")?.id).toBe("view.palette");
  });

  it("distinguishes play from play-from-the-selection", () => {
    expect(commandFor(press(" "), "score")?.id).toBe("transport.play");
    expect(commandFor(press(" ", { shiftKey: true }), "score")?.id).toBe("transport.play.selection");
  });
});

describe("spelling a binding", () => {
  it("sets modifiers as glyphs, run together", () => {
    expect(spell("CmdOrCtrl+K")).toBe("⌘K");
    expect(spell("Alt+ArrowLeft")).toBe("⌥←");
    expect(spell("Shift+Space")).toBe("⇧Space");
    expect(spell(null)).toBe("");
  });
});
