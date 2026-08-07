/**
 * The session store's contracts.
 *
 * These are the rules the interface depends on and the ones that are easiest
 * to break by accident: one compile per pause, a late answer never overwrites
 * a newer one, and a source with problems keeps the last valid score on
 * screen (`05-states.md` §4).
 */

import { beforeEach, describe, expect, it, vi } from "vitest";

import { SETTLE_MS, Session, type Link } from "../../src/lib/session/session.svelte";
import fixture from "../../fixtures/glass-mountain.snapshot.json";
import type { ProjectSnapshot } from "../../src/lib/state/snapshot";

const VALID = fixture as unknown as ProjectSnapshot;

/** A snapshot of `source` at `revision`, compiling or not, as the core would. */
function snapshotOf(source: string, revision: number, compiles = true): ProjectSnapshot {
  return {
    ...VALID,
    source,
    revision,
    compiles,
    // The core keeps the last valid artifacts when the source does not
    // compile; that is exactly what the interface leans on.
    diagnostics: compiles ? [] : [{ severity: "error", message: "expected `}`", span: null }],
  };
}

interface Recorder extends Link {
  applied: string[];
  answer: (source: string) => ProjectSnapshot;
}

function recorder(): Recorder {
  let revision = VALID.revision;
  const link: Recorder = {
    applied: [],
    answer: (source) => snapshotOf(source, ++revision),
    openProject: vi.fn(async () => VALID),
    newProject: vi.fn(async () => VALID),
    apply: vi.fn(async (command) => {
      const source = command.kind === "setSource" ? command.source : VALID.source;
      link.applied.push(source);
      return link.answer(source);
    }),
    transport: vi.fn(async () => VALID),
    exportTo: vi.fn(async () => ({ path: "/tmp/out.mei" })),
    snapshot: vi.fn(async () => VALID),
    on: vi.fn(async () => () => {}),
    askToOpen: vi.fn(async () => "/tmp/piece.musa"),
    askToSave: vi.fn(async () => "/tmp/out.mei"),
  };
  return link;
}

describe("editing", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  it("compiles once for a burst of typing, not once per keystroke", async () => {
    const link = recorder();
    const session = new Session(link);
    session.receive(VALID);

    for (const text of ["p", "pi", "pie", "piec", "piece"]) session.edit(text);
    expect(link.applied).toEqual([]);

    await vi.advanceTimersByTimeAsync(SETTLE_MS);
    expect(link.applied).toEqual(["piece"]);
  });

  it("shows the draft immediately, and the document once it answers", async () => {
    const link = recorder();
    const session = new Session(link);
    session.receive(VALID);

    session.edit("piece x");
    expect(session.text).toBe("piece x");

    await vi.advanceTimersByTimeAsync(SETTLE_MS);
    expect(session.draft).toBeNull();
    expect(session.text).toBe("piece x");
  });

  it("does not compile text the document already has", async () => {
    const link = recorder();
    const session = new Session(link);
    session.receive(VALID);

    session.edit(VALID.source);
    await vi.advanceTimersByTimeAsync(SETTLE_MS);
    expect(link.applied).toEqual([]);
  });
});

describe("supersession", () => {
  it("ignores a compile that finished after a newer one", async () => {
    const link = recorder();
    const session = new Session(link);
    session.receive(VALID);

    // The first compile answers late and with an older revision — the shape
    // of a slow compile the user has already typed past.
    const slow = snapshotOf("first", VALID.revision + 1);
    const quick = snapshotOf("second", VALID.revision + 2);

    session.receive(quick);
    session.receive(slow);
    expect(session.snapshot?.source).toBe("second");
  });
});

describe("the stale revision", () => {
  it("keeps the last valid score and says how far behind it is", () => {
    const session = new Session(recorder());
    session.receive(VALID);
    const engraved = session.snapshot?.mei;

    session.receive(snapshotOf("piece {", VALID.revision + 1, false));

    expect(session.stale).toBe(true);
    expect(session.snapshot?.mei).toBe(engraved);
    expect(session.shownRevision).toBe(VALID.scoreRevision);
    expect(session.snapshot?.diagnostics).toHaveLength(1);
  });

  it("opens the drawer the first time, and not again after the user closes it", () => {
    const session = new Session(recorder());
    session.receive(VALID);
    expect(session.drawerOpen).toBe(false);

    session.receive(snapshotOf("piece {", VALID.revision + 1, false));
    expect(session.drawerOpen).toBe(true);

    session.drawerOpen = false;
    session.receive(snapshotOf("piece {{", VALID.revision + 2, false));
    expect(session.drawerOpen).toBe(false);
  });
});

describe("without a shell", () => {
  it("renders a seeded snapshot and refuses every command", async () => {
    const session = new Session(null);
    session.snapshot = VALID;

    expect(session.live).toBe(false);
    await session.save();
    await session.play();
    expect(session.notice).toBeNull();
    expect(session.snapshot).toBe(VALID);
  });
});

describe("results and failures", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  it("names the file an export wrote, and stops saying so after three seconds", async () => {
    const link = recorder();
    const session = new Session(link);
    session.receive(VALID);

    await session.exportTo("mei");
    expect(session.notice).toEqual({ tone: "result", message: "Exported out.mei." });

    await vi.advanceTimersByTimeAsync(3000);
    expect(session.notice).toBeNull();
  });

  it("keeps a failure on screen until something supersedes it", async () => {
    const link = recorder();
    link.apply = vi.fn(async () => {
      throw { kind: "nothing", message: "nothing to undo" };
    });
    const session = new Session(link);
    session.receive(VALID);

    await session.undo();
    expect(session.notice).toEqual({ tone: "failure", message: "nothing to undo" });

    await vi.advanceTimersByTimeAsync(10_000);
    expect(session.notice?.tone).toBe("failure");
  });
});
