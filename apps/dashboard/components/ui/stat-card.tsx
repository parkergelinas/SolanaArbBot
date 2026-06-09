'use client';

import { motion } from 'framer-motion';
import { cn } from '@/lib/cn';
import { AnimatedNumber } from './animated-number';

// ── Format helpers ────────────────────────────────────────────────────────────

const PRESETS: Record<string, (n: number) => string> = {
  usd:     (n) => '$' + Math.abs(n).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 }),
  integer: (n) => Math.abs(n).toLocaleString('en-US', { maximumFractionDigits: 0 }),
  percent: (n) => Math.abs(n).toFixed(1) + '%',
  sol:     (n) => Math.abs(n).toFixed(4) + ' SOL',
};

function resolveFormat(
  fmt: string | ((n: number) => string) | undefined,
  value: number,
  signed: boolean,
): string {
  const fn = typeof fmt === 'function' ? fmt : (PRESETS[fmt ?? ''] ?? ((n) => String(n)));
  const formatted = fn(value);
  if (!signed) return formatted;
  return value > 0 ? `+${formatted}` : value < 0 ? `−${formatted}` : formatted;
}

// ── Color resolution ──────────────────────────────────────────────────────────

// Accepts either legacy preset keys ("green", "red", …) or raw CSS values ("var(--green)", "#aaa")
const PRESET_COLORS: Record<string, string> = {
  green:   'var(--green)',
  red:     'var(--red)',
  blue:    'var(--blue)',
  amber:   'var(--amber)',
  default: 'var(--text-primary)',
};

function resolveColor(accent: string | undefined): string {
  if (!accent) return 'var(--text-primary)';
  return PRESET_COLORS[accent] ?? accent;
}

// ── Props ─────────────────────────────────────────────────────────────────────

export interface StatCardProps {
  label: string;
  value: number | string | null;
  /**
   * Either a named preset ("usd" | "integer" | "percent" | "sol") or a
   * `(n: number) => string` formatter function.  Ignored when `value` is a string.
   */
  format?: string | ((n: number) => string);
  /**
   * Accent color — accepts either a preset name ("green" | "red" | "blue" | "amber")
   * or any CSS color string ("var(--green)", "#00ff00", "hsl(…)", …).
   */
  accent?: string;
  /** Prefix positive values with '+' and negatives with '−'. */
  signed?: boolean;
  trend?: 'up' | 'down' | 'neutral';
  /** Small badge text rendered in the top-right corner. */
  badge?: string;
  loading?: boolean;
  className?: string;
}

export default function StatCard({
  label,
  value,
  format,
  accent,
  signed = false,
  trend,
  badge,
  loading,
  className,
}: StatCardProps) {
  const color       = resolveColor(accent);
  const isAccented  = !!accent && accent !== 'default';
  const isGreen     = accent === 'green'   || accent === 'var(--green)';
  const isRed       = accent === 'red'     || accent === 'var(--red)';

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.22, ease: 'easeOut' }}
      className={cn(
        'relative bg-[var(--bg-surface)] border border-[var(--bg-border)] rounded-sm px-3 py-2.5',
        'hover:border-[color-mix(in_srgb,var(--text-muted)_25%,var(--bg-border))] transition-colors duration-150',
        className,
      )}
    >
      {/* Top accent line */}
      {isGreen && (
        <span className="absolute inset-x-0 top-0 h-[2px] rounded-t-sm bg-[color-mix(in_srgb,var(--green)_45%,transparent)]" />
      )}
      {isRed && (
        <span className="absolute inset-x-0 top-0 h-[2px] rounded-t-sm bg-[color-mix(in_srgb,var(--red)_45%,transparent)]" />
      )}

      {/* Label */}
      <p className="text-[9px] uppercase tracking-[0.12em] text-[color:var(--text-muted)] mb-1 truncate pr-5">
        {label}
      </p>

      {/* Badge */}
      {badge && (
        <span className="absolute top-2 right-2 text-[8px] font-mono px-1 py-0.5 rounded-sm border border-[var(--bg-border)] text-[color:var(--text-muted)]">
          {badge}
        </span>
      )}

      {/* Value */}
      {loading || value === null ? (
        <div className="h-[22px] w-20 bg-[var(--bg-elevated)] rounded skeleton" />
      ) : typeof value === 'number' ? (
        <AnimatedNumber
          value={value}
          format={(n) => resolveFormat(format, n, signed)}
          className="text-[15px] font-semibold font-mono tabular-nums block"
          style={{ color }}
        />
      ) : (
        <motion.span
          key={String(value)}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.18 }}
          className="text-[15px] font-semibold font-mono tabular-nums block"
          style={{ color }}
        >
          {value}
        </motion.span>
      )}

      {/* Trend indicator */}
      {trend && (
        <span
          className="absolute bottom-2 right-2 text-[9px]"
          style={{
            color: trend === 'up' ? 'var(--green)' : trend === 'down' ? 'var(--red)' : 'var(--text-muted)',
          }}
        >
          {trend === 'up' ? '▲' : trend === 'down' ? '▼' : '●'}
        </span>
      )}
    </motion.div>
  );
}

// Named export alias for callers that import `{ StatCard }`
export { StatCard };
