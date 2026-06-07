'use client';

import { motion } from 'framer-motion';
import { cn } from '@/lib/cn';
import { AnimatedNumber } from './animated-number';

export interface StatCardProps {
  label: string;
  value: number | string | null;
  format?: (n: number) => string;
  accent?: 'green' | 'red' | 'blue' | 'amber' | 'default';
  trend?: 'up' | 'down' | 'neutral';
  loading?: boolean;
  className?: string;
}

const accentMap = {
  green:   'text-ds-green',
  red:     'text-ds-red',
  blue:    'text-ds-blue',
  amber:   'text-ds-amber',
  default: 'text-ds-text-primary',
};

export function StatCard({
  label,
  value,
  format,
  accent = 'default',
  trend,
  loading,
  className,
}: StatCardProps) {
  const colorClass = accentMap[accent];

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.25, ease: 'easeOut' }}
      className={cn(
        'relative bg-ds-surface border border-ds-border rounded-terminal px-3 py-2.5',
        'hover:border-ds-text-muted/30 transition-colors duration-200',
        className,
      )}
    >
      {/* Accent glow on green/red */}
      {(accent === 'green' || accent === 'red') && (
        <span
          className={cn(
            'absolute inset-x-0 top-0 h-[1px] rounded-t-terminal',
            accent === 'green' ? 'bg-ds-green/40' : 'bg-ds-red/40',
          )}
        />
      )}

      <p className="text-[9px] uppercase tracking-[0.12em] text-ds-text-muted mb-1 truncate">
        {label}
      </p>

      {loading || value === null ? (
        <div className="h-[22px] w-20 bg-ds-elevated rounded animate-pulse" />
      ) : typeof value === 'number' ? (
        <AnimatedNumber
          value={value}
          format={format}
          className={cn('text-[15px] font-semibold font-mono tabular-nums block', colorClass)}
        />
      ) : (
        <motion.span
          key={String(value)}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.2 }}
          className={cn('text-[15px] font-semibold font-mono tabular-nums block', colorClass)}
        >
          {value}
        </motion.span>
      )}

      {trend && (
        <span
          className={cn(
            'absolute top-2 right-2 text-[9px]',
            trend === 'up' ? 'text-ds-green' : trend === 'down' ? 'text-ds-red' : 'text-ds-text-muted',
          )}
        >
          {trend === 'up' ? '▲' : trend === 'down' ? '▼' : '●'}
        </span>
      )}
    </motion.div>
  );
}
