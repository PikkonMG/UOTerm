/** The waits between tries to reach UOTerm again, for a link or a request. The last one repeats. */
export const BACKOFF_MS = [250, 500, 1000, 2000, 4000];
export const LONGEST_BACKOFF_MS = BACKOFF_MS[BACKOFF_MS.length - 1];

/** The wait before retry number `retry` (0 for the first retry). */
export function backoffWait(retry: number): number {
  return BACKOFF_MS[retry] ?? LONGEST_BACKOFF_MS;
}
