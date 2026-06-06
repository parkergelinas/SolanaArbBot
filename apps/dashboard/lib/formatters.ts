/** Terminal formatters — all numeric/date formatting in one place. */

const MINUS = '\u2212';

export function formatPrice(price: number, decimals?: number): string {
  if (!Number.isFinite(price) || price === 0) return '—';
  if (decimals != null) return price.toFixed(decimals);
  if (price >= 10_000) return price.toFixed(0);
  if (price >= 100) return price.toFixed(2);
  if (price >= 1) return price.toFixed(4);
  if (price >= 0.0001) return price.toFixed(6);
  return price.toExponential(2);
}

export function formatPnL(value: number, opts?: { signed?: boolean }): string {
  if (!Number.isFinite(value)) return '—';
  const signed = opts?.signed ?? true;
  const abs = Math.abs(value);
  let body: string;
  if (abs >= 1_000_000) body = `$${(abs / 1_000_000).toFixed(2)}M`;
  else if (abs >= 1_000) body = `$${abs.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  else if (abs >= 1) body = `$${abs.toFixed(2)}`;
  else body = `$${abs.toFixed(4)}`;

  if (!signed) return body;
  if (value > 0) return `+${body}`;
  if (value < 0) return `${MINUS}${body}`;
  return body;
}

export function formatSize(size: number): string {
  if (!Number.isFinite(size)) return '—';
  const abs = Math.abs(size);
  if (abs >= 1_000_000) return `${(size / 1_000_000).toFixed(3)}`;
  if (abs >= 1_000) return `${(size / 1_000).toFixed(3)}`;
  if (abs >= 1) return size.toFixed(3);
  if (abs >= 0.01) return size.toFixed(4);
  return size.toExponential(2);
}

export function formatVolume(vol: number): string {
  if (!Number.isFinite(vol) || vol <= 0) return '—';
  if (vol >= 1_000_000_000) return `${(vol / 1_000_000_000).toFixed(1)}B`;
  if (vol >= 1_000_000) return `${Math.round(vol / 1_000_000)}M`;
  if (vol >= 1_000) return `${(vol / 1_000).toFixed(1)}K`;
  return vol.toFixed(0);
}

export function formatChangePct(pct: number): string {
  if (!Number.isFinite(pct)) return '—';
  if (pct > 0) return `+${pct.toFixed(1)}%`;
  if (pct < 0) return `${MINUS}${Math.abs(pct).toFixed(1)}%`;
  return '0.0%';
}

export function formatSpread(spread: number, mid: number): { abs: string; pct: string; pctValue: number } {
  const pctValue = mid > 0 ? (spread / mid) * 100 : 0;
  return {
    abs: spread.toFixed(2),
    pct: `${pctValue.toFixed(2)}%`,
    pctValue,
  };
}

export function formatTime(tsMs: number): string {
  if (!tsMs || tsMs <= 0) return '—';
  const d = new Date(tsMs);
  const h = String(d.getHours()).padStart(2, '0');
  const m = String(d.getMinutes()).padStart(2, '0');
  const s = String(d.getSeconds()).padStart(2, '0');
  const ms = String(d.getMilliseconds()).padStart(3, '0');
  return `${h}:${m}:${s}.${ms}`;
}

export function formatUtcClock(date = new Date()): string {
  return date.toISOString().slice(11, 19) + ' UTC';
}
