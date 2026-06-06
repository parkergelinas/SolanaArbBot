import type { Signal } from '@/lib/stream/types';

export type PumpSignalKind = 'launch' | 'curve' | 'other';

export function isPumpSignal(sig: Signal): boolean {
  return Boolean(sig.detail?.startsWith('pump_'));
}

export function pumpSignalKind(sig: Signal): PumpSignalKind {
  if (sig.detail?.startsWith('pump_launch')) return 'launch';
  if (sig.detail?.startsWith('pump_curve')) return 'curve';
  return 'other';
}

/** Human-readable one-liner for pump stream signals. */
export function formatPumpDetail(detail?: string): string {
  if (!detail) return '';
  if (!detail.startsWith('pump_')) return detail;

  const parts = detail.split(':').slice(1).join(':');
  const fields = Object.fromEntries(
    parts.split(';').map((kv) => {
      const [k, ...rest] = kv.split('=');
      return [k?.trim() ?? '', rest.join('=').trim()];
    }),
  );

  if (detail.startsWith('pump_launch')) {
    const liq = fields.liq_sol ? `${Number(fields.liq_sol).toFixed(2)} SOL` : 'new';
    return `Launch · ${liq}`;
  }
  if (detail.startsWith('pump_curve')) {
    const sym = fields.symbol ?? '?';
    const grad = fields.grad_pct ? `${fields.grad_pct}% grad` : '';
    const buys = fields.buys_m5 ? `${fields.buys_m5} buys/5m` : '';
    return [sym, grad, buys].filter(Boolean).join(' · ');
  }
  return detail;
}
