/**
 * The byte↔character conversion (prompt 148): compiler spans are UTF-8
 * bytes, JS strings are UTF-16, and the two agree exactly when the source is
 * ASCII — which is why these tests are not.
 */

import { describe, expect, it } from "vitest";

import { byteToCharOffset, excerptFor } from "../src/errors";

describe("byteToCharOffset", () => {
  it("is the identity on ASCII", () => {
    expect(byteToCharOffset("hello world", 7)).toBe(7);
  });

  it("counts code points, not UTF-16 units, before the offset", () => {
    // "é" is 2 bytes / 1 UTF-16 unit; "𝄞" is 4 bytes / 2 UTF-16 units.
    expect(byteToCharOffset("éa", 2)).toBe(1); // after é
    expect(byteToCharOffset("éa", 3)).toBe(2); // after a
    expect(byteToCharOffset("𝄞b", 4)).toBe(2); // after 𝄞 (2 UTF-16 units)
    expect(byteToCharOffset("𝄞b", 5)).toBe(3);
  });

  it("stops inside a multi-byte character rather than past it", () => {
    expect(byteToCharOffset("éa", 1)).toBe(0); // mid-é: not yet past it
  });
});

describe("excerptFor", () => {
  it("extracts the offending line with the span inside it", () => {
    const source = 'piece "té" {\n    | c4/4 d4/4 e4/4 f4/4 f5/4\n}\n';
    // Bytes 24–29 cover "c4/4 " starting the bar... compute from the string:
    const charStart = source.indexOf("f5/4");
    const encoder = new TextEncoder();
    const byteStart = encoder.encode(source.slice(0, charStart)).length;
    const excerpt = excerptFor(source, { start: byteStart, end: byteStart + 4, text: "", primary: true });
    expect(excerpt.line).toBe("    | c4/4 d4/4 e4/4 f4/4 f5/4");
    expect(excerpt.lineNumber).toBe(2);
    expect(excerpt.line.slice(excerpt.spanStart, excerpt.spanEnd)).toBe("f5/4");
  });

  it("clamps a span that runs off the line", () => {
    const source = "one\ntwo\nthree\n";
    const excerpt = excerptFor(source, { start: 2, end: 12, text: "", primary: true });
    expect(excerpt.line).toBe("one");
    expect(excerpt.line.slice(excerpt.spanStart, excerpt.spanEnd)).toBe("e");
  });

  it("lands on the right text after multi-byte characters", () => {
    const source = 'tempo 1/4 = 104; ♩\n    | g4/2\n';
    const charStart = source.indexOf("g4/2");
    const encoder = new TextEncoder();
    const byteStart = encoder.encode(source.slice(0, charStart)).length;
    const excerpt = excerptFor(source, { start: byteStart, end: byteStart + 4, text: "", primary: true });
    expect(excerpt.line.slice(excerpt.spanStart, excerpt.spanEnd)).toBe("g4/2");
  });
});
