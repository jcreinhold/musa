/**
 * The CDN build aliases `worker-default.ts` to this stub: the worker is
 * Blob-inlined by `cdn-entry.ts`, so the default construction is unreachable
 * — and must not emit a worker asset nobody will fetch.
 */
export function defaultWorker(): Worker {
  throw new Error(
    "@musa/web CDN build: the worker is inlined; the engraver factory must supply it",
  );
}
