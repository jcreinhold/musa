import { defineConfig } from "vitest/config";

/**
 * The sanitizer's contract runs in node against real Verovio output — the
 * sanitizer and the option table are plain modules and are tested as such.
 */
export default defineConfig({
  test: {
    include: ["tests/**/*.test.ts"],
    environment: "node",
  },
});
