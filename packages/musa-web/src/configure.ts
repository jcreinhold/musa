/**
 * Configuration for `@musa/web`. Set it before the first `parse`/`render`;
 * the MathJax config-before-load convention, as a function rather than as a
 * global (prompt 141 adds the script-tag form).
 */

export interface WebConfig {
  /**
   * Where the musa wasm module is served from. Defaults to the `wasm/`
   * directory beside the package's module — right for anything serving the
   * package as-is; set it when a bundler relocates assets.
   */
  wasmUrl?: string;
}

let config: WebConfig = {};

export function configure(next: WebConfig): void {
  config = { ...config, ...next };
}

/** The configuration as it stands. Internal; the surface is `configure`. */
export function getConfig(): WebConfig {
  return config;
}
