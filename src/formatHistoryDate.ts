/** Format unix-seconds `at` from history entries for display. */
export function formatHistoryDate(at: string): string {
  const secs = Number.parseInt(at, 10);
  if (!Number.isFinite(secs) || secs <= 0) {
    return at;
  }
  return new Date(secs * 1000).toLocaleString();
}
