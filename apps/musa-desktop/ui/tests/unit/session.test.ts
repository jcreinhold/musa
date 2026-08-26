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
    diagnostics: compiles
      ? []
      : [
          {
            severity: "error",
            code: "syntax",
            message: "missing `}`",
            labels: [],
            help: null,
            note: null,
            fixes: [],
            causes: [],
            span: null,
          },
        ],
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
    showPiece: vi.fn(async () => VALID),
    saveAll: vi.fn(async () => VALID),
    askToOpenProject: vi.fn(async () => null),
    apply: vi.fn(async (command) => {
      const source = command.kind === "setSource" ? command.source : VALID.source;
      link.applied.push(source);
      return link.answer(source);
    }),
    editImpact: vi.fn(async () => ({
      generated: false,
      motif: null,
      occurrence: null,
      occurrences: 0,
      events: [],
      specializable: false,
      writes: [],
    })),
    barlineRewrite: vi.fn(async () => ({
      status: "rewrite" as const,
      revision: revision as unknown as bigint,
      inserted: 2,
      summary: "Insert 2 bar lines in 2 measures",
    })),
    transport: vi.fn(async () => VALID),
    exportTo: vi.fn(async () => ({ path: "/tmp/out.mei" })),
    snapshot: vi.fn(async () => VALID),
    on: vi.fn(async () => () => {}),
    askToOpen: vi.fn(async () => "/tmp/piece.musa"),
    askToSave: vi.fn(async () => "/tmp/out.mei"),
    listenToMidi: vi.fn(async () => VALID),
    selectMidiInput: vi.fn(async () => VALID),
    startMidiCapture: vi.fn(async () => VALID),
    stopMidiCapture: vi.fn(async () => VALID),
    keepRecentMidi: vi.fn(async () => VALID),
    clearRecentMidi: vi.fn(async () => VALID),
    setRecentMidi: vi.fn(async () => VALID),
    analyze: vi.fn(async (kind: string) => ({
      revision,
      kind,
      method: "counts what it sees",
      profile: null,
      assumptions: [],
      findings: [],
    })),
    libraryDocument: vi.fn(async (uri: string) => ({
      uri,
      name: "core",
      text: "let identity_nat = ...\n",
      span: null,
    })),
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

  it("previews the project-owned bar count and applies that revision", async () => {
    const link = recorder();
    const session = new Session(link);
    session.receive(VALID);

    await session.previewBarlines();
    expect(session.barlinePreview?.summary).toBe("Insert 2 bar lines in 2 measures");
    await session.insertBarlines();

    expect(link.apply).toHaveBeenCalledWith({
      kind: "insertBarlines",
      revision: VALID.revision as unknown as bigint,
    });
    expect(session.notice?.message).toBe("Insert 2 bar lines in 2 measures.");
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

/**
 * A revision counts within one piece, and every piece starts at zero — so
 * "older" is a question that only has an answer inside a document. Reading
 * the two the wrong way round is why File → Open used to do nothing at all
 * once the piece on screen had been edited.
 */
describe("opening another piece", () => {
  /** A different document, at its own first revision. */
  function other(source: string, compiles = true): ProjectSnapshot {
    return { ...snapshotOf(source, 0, compiles), document: VALID.document + 1 };
  }

  it("takes the new piece, whose revisions start again at zero", async () => {
    const link = recorder();
    const opened = other('piece "Second" {}');
    link.openProject = vi.fn(async () => opened);
    const session = new Session(link);
    session.receive(snapshotOf('piece "First" {}', 12));

    await session.open("/tmp/second.musa");
    expect(session.snapshot?.source).toBe(opened.source);
  });

  it("leaves the previous piece's draft behind", async () => {
    const link = recorder();
    link.openProject = vi.fn(async () => other('piece "Second" {}'));
    const session = new Session(link);
    session.receive(VALID);
    session.edit("half a thought");
    expect(session.draft).toBe("half a thought");

    await session.open("/tmp/second.musa");
    expect(session.draft).toBeNull();
    expect(session.text).toBe('piece "Second" {}');
  });

  it("ignores an answer about the piece that was closed", async () => {
    const link = recorder();
    link.openProject = vi.fn(async () => other('piece "Second" {}'));
    const session = new Session(link);
    session.receive(VALID);

    await session.open("/tmp/second.musa");
    // A compile of the first piece, still in flight when the second opened.
    session.receive(snapshotOf('piece "First" {}', VALID.revision + 9));
    expect(session.snapshot?.source).toBe('piece "Second" {}');
  });

  it("shows the source again for a second piece that does not compile", async () => {
    const link = recorder();
    link.openProject = vi.fn(async () => other("piece {", false));
    const session = new Session(link);
    session.receive(VALID);
    session.receive(snapshotOf("piece {", VALID.revision + 1, false));
    session.sourceOpen = false;

    await session.open("/tmp/second.musa");
    expect(session.sourceOpen).toBe(true);
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

  it("shows the source the first time, and not again after the user hides it", () => {
    const session = new Session(recorder());
    session.receive(VALID);
    expect(session.sourceOpen).toBe(false);

    session.receive(snapshotOf("piece {", VALID.revision + 1, false));
    expect(session.sourceOpen).toBe(true);

    session.sourceOpen = false;
    session.receive(snapshotOf("piece {{", VALID.revision + 2, false));
    expect(session.sourceOpen).toBe(false);
  });
});

describe("asking the score a question", () => {
  it("says which reading is in flight, and stops saying so when it lands", async () => {
    const link = recorder();
    const session = new Session(link);
    session.receive(VALID);

    const asked = session.analyze("cadences");
    expect(session.reading).toBe("cadences");
    await asked;
    expect(session.reading).toBeNull();
    expect(session.report?.kind).toBe("cadences");
  });

  /*
   * A reader shown zero findings would conclude the music is clean. So a
   * refusal leaves the last reading where it was and says what went wrong
   * (`08-elaboration.md` §8).
   */
  it("keeps the last reading when the next one is refused", async () => {
    const link = recorder();
    const session = new Session(link);
    session.receive(VALID);
    await session.analyze("cadences");

    link.analyze = vi.fn(async () => {
      throw { kind: "backend", message: "no valid score" };
    });
    await session.analyze("tonal");
    expect(session.report?.kind).toBe("cadences");
    expect(session.notice).toEqual({
      tone: "failure",
      message: "no valid score",
    });
  });

  /*
   * A reading names events and spans in the piece it read. Shown beside a
   * different piece it would point at the wrong notes, which is the same error
   * as carrying a selection across a new performance.
   */
  it("forgets a reading of the piece that was closed", async () => {
    const link = recorder();
    link.openProject = vi.fn(async () => ({
      ...VALID,
      document: VALID.document + 1,
      revision: 0,
    }));
    const session = new Session(link);
    session.receive(VALID);
    await session.analyze("cadences");
    expect(session.report).not.toBeNull();

    await session.open("/tmp/second.musa");
    expect(session.report).toBeNull();
    expect(session.reading).toBeNull();
  });

  /*
   * An edit does not: the reading is still of a score the composer can see, and
   * it carries the revision it read so the panel can say it is behind. Throwing
   * it away on a keystroke would make analysis unusable while typing.
   */
  it("keeps a reading across an edit, tagged with the score it read", async () => {
    const link = recorder();
    const session = new Session(link);
    session.receive(VALID);
    await session.analyze("cadences");
    const read = session.report?.revision;

    session.receive(snapshotOf('piece "Later" {}', VALID.revision + 1));
    expect(session.report?.revision).toBe(read);
    expect(session.snapshot?.revision).toBeGreaterThan(read ?? 0);
  });
});

describe("opening a bundled module", () => {
  it("holds it beside the piece, and gives it back", async () => {
    const link = recorder();
    const session = new Session(link);
    session.receive(VALID);

    await session.openLibrary("musa-stdlib:/std/core.musa", 254, 266);
    expect(session.library?.name).toBe("core");
    expect(link.libraryDocument).toHaveBeenCalledWith("musa-stdlib:/std/core.musa", 254, 266);
    // The piece is still open behind it: a module is not a document that
    // replaced the composer's own.
    expect(session.snapshot).toBe(VALID);

    session.closeLibrary();
    expect(session.library).toBeNull();
  });

  it("says so when the module is not one the compiler bundles", async () => {
    const link = recorder();
    link.libraryDocument = vi.fn(async () => {
      throw { kind: "backend", message: "`std::nowhere` is not bundled" };
    });
    const session = new Session(link);
    session.receive(VALID);

    await session.openLibrary("musa-stdlib:/std/nowhere.musa");
    expect(session.library).toBeNull();
    expect(session.notice?.tone).toBe("failure");
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
    expect(session.notice).toEqual({
      tone: "result",
      message: "Exported out.mei.",
    });

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
    expect(session.notice).toEqual({
      tone: "failure",
      message: "nothing to undo",
    });

    await vi.advanceTimersByTimeAsync(10_000);
    expect(session.notice?.tone).toBe("failure");
  });
});
