/**
 * How a workstation bundle is arranged for reading.
 *
 * The rule these hold to is the one the Bundle sheet rests on: the interface
 * arranges what the project wrote and computes nothing. So what there is to
 * test is arrangement — which heading a file is filed under, that no file the
 * project wrote goes missing, and that the losses are gathered by class
 * rather than repeated.
 */

import { describe, expect, it } from "vitest";

import { PROFILES, byKind, folder, groups, hasLosses, size, summary } from "../../src/lib/state/bundle";
import type { DawReportDto } from "../../src/lib/session/generated/DawReportDto";

function file(path: string, bytes = 1024): { path: string; bytes: number; sha256: string } {
  return { path, bytes, sha256: "0".repeat(64) };
}

function report(partial: Partial<DawReportDto> = {}): DawReportDto {
  return {
    version: 1,
    profile: "logic",
    destination: "/Users/composer/Music/glass mountain for a workstation",
    files: [
      file("score.mid", 800),
      file("performance.mid", 900),
      file("score.musicxml", 4096),
      file("audio/mix.wav", 2_000_000),
      file("audio/parts/violin.wav", 1_000_000),
      file("audio/returns/hall.wav", 1_000_000),
      file("musa-manifest.json", 3000),
    ],
    losses: [],
    ...partial,
  };
}

describe("the file list", () => {
  it("files each artifact under what it is for", () => {
    expect(groups(report()).map((group) => [group.title, group.files.map((each) => each.path)])).toEqual([
      ["Notation and MIDI", ["score.mid", "performance.mid", "score.musicxml"]],
      ["Mix", ["audio/mix.wav"]],
      ["Stems", ["audio/parts/violin.wav", "audio/returns/hall.wav"]],
      ["Record", ["musa-manifest.json"]],
    ]);
  });

  it("loses no file the project wrote, whatever it is called", () => {
    const unknown = report({ files: [file("score.mid"), file("something-new.txt")] });
    const listed = groups(unknown).flatMap((group) => group.files.map((each) => each.path));
    expect(listed).toEqual(["score.mid", "something-new.txt"]);
  });

  it("shows no heading with nothing under it", () => {
    // A GarageBand bundle has no notation file; a piece with no studio has
    // no stems. Neither is a heading over an empty list.
    expect(groups(report({ files: [file("score.mid")] })).map((group) => group.title)).toEqual(["Notation and MIDI"]);
  });

  it("says a size a person reads", () => {
    expect(size(800)).toBe("800 bytes");
    expect(size(4096)).toBe("4 kB");
    expect(size(2_000_000)).toBe("1.9 MB");
  });
});

describe("what the bundle could not carry", () => {
  it("gathers by class, because a kind is a class and not an incident", () => {
    const many = report({
      losses: [
        { kind: "notation", message: "a" },
        { kind: "tuning", message: "b" },
        { kind: "notation", message: "c" },
      ],
    });
    expect(byKind(many)).toEqual([
      { kind: "notation", messages: ["a", "c"] },
      { kind: "tuning", messages: ["b"] },
    ]);
  });

  it("knows when there is nothing to weigh", () => {
    expect(hasLosses(report())).toBe(false);
    expect(hasLosses(null)).toBe(false);
    expect(hasLosses(report({ losses: [{ kind: "tuning", message: "b" }] }))).toBe(true);
  });
});

describe("what the sheet says", () => {
  it("names the folder and counts what is in it", () => {
    expect(summary(report())).toBe("7 files in glass mountain for a workstation.");
    expect(summary(report({ losses: [{ kind: "tuning", message: "b" }] }))).toBe(
      "7 files in glass mountain for a workstation, with 1 thing the formats cannot carry.",
    );
  });

  it("suggests a folder named after the piece, without its extension", () => {
    expect(folder("glass-mountain.musa")).toBe("glass-mountain for a workstation");
    expect(folder(null)).toBe("piece for a workstation");
    expect(folder("  ")).toBe("piece for a workstation");
  });

  it("offers both workstations and says what each one gets", () => {
    expect(PROFILES.map((profile) => profile.value)).toEqual(["logic", "garageBand"]);
    expect(PROFILES.every((profile) => profile.note.length > 0)).toBe(true);
  });
});
