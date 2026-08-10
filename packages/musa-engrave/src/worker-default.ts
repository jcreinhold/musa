/**
 * The default way the worker arrives: a module worker beside this module,
 * resolved by the bundler of whoever depends on us. Its own module so a
 * build that supplies the worker another way (the CDN build's Blob) can
 * alias this file away and keep the unused worker asset out of its output.
 */
export function defaultWorker(): Worker {
  return new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
}
