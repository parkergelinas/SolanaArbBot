/** HTTP base for stream-api REST (scanners, health) derived from WS URL. */

export function streamHttpBase(): string {
  const explicit = process.env.NEXT_PUBLIC_STREAM_HTTP_URL?.replace(/\/$/, '');
  if (explicit) return explicit;

  const ws =
    typeof process !== 'undefined' && process.env.NEXT_PUBLIC_STREAM_URL
      ? process.env.NEXT_PUBLIC_STREAM_URL
      : 'ws://localhost:8080/stream';

  try {
    const http = ws
      .replace(/^wss:\/\//i, 'https://')
      .replace(/^ws:\/\//i, 'http://');
    const u = new URL(http);
    u.pathname = '';
    u.search = '';
    u.hash = '';
    return u.origin;
  } catch {
    return 'http://localhost:8080';
  }
}
