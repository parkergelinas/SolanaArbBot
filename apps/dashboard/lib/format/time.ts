/** Human-readable age since timestamp ms. */
export function timeAgo(tsMs: number, now = Date.now()): string {
  if (!tsMs || tsMs <= 0) return '—';
  const sec = Math.max(0, Math.floor((now - tsMs) / 1000));
  if (sec < 5) return 'just now';
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  return `${hr}h ago`;
}
