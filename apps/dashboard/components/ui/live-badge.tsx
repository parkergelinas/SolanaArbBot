'use client';

import { motion, AnimatePresence } from 'framer-motion';
import { cn } from '@/lib/cn';

export type LiveBadgeStatus = 'live' | 'degraded' | 'offline' | 'idle';

const STATUS_CONFIG = {
  live:     { dot: 'bg-ds-green',  label: 'Live',     text: 'text-ds-green',  border: 'border-ds-green/30',  bg: 'bg-ds-green/5',  pulse: true  },
  degraded: { dot: 'bg-ds-amber',  label: 'Degraded', text: 'text-ds-amber',  border: 'border-ds-amber/30',  bg: 'bg-ds-amber/5',  pulse: true  },
  offline:  { dot: 'bg-ds-red',    label: 'Offline',  text: 'text-ds-red',    border: 'border-ds-red/30',    bg: 'bg-ds-red/5',    pulse: false },
  idle:     { dot: 'bg-ds-text-muted', label: 'Idle', text: 'text-ds-text-muted', border: 'border-ds-border', bg: 'bg-transparent', pulse: false },
};

interface LiveBadgeProps {
  status: LiveBadgeStatus;
  label?: string;
  className?: string;
}

export function LiveBadge({ status, label, className }: LiveBadgeProps) {
  const cfg = STATUS_CONFIG[status];
  return (
    <motion.span
      layout
      className={cn(
        'inline-flex items-center gap-1.5 px-2 py-0.5 rounded-terminal border text-[10px] font-mono uppercase',
        cfg.text, cfg.border, cfg.bg,
        className,
      )}
    >
      <span className={cn('w-1.5 h-1.5 rounded-full shrink-0', cfg.dot, cfg.pulse && 'live-pulse')} />
      {label ?? cfg.label}
    </motion.span>
  );
}

interface PulsingDotProps {
  ok: boolean;
  className?: string;
}

export function PulsingDot({ ok, className }: PulsingDotProps) {
  return (
    <AnimatePresence mode="wait">
      <motion.span
        key={String(ok)}
        initial={{ scale: 0.6, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        exit={{ scale: 0.6, opacity: 0 }}
        transition={{ duration: 0.18 }}
        className={cn(
          'w-1.5 h-1.5 rounded-full shrink-0',
          ok ? 'bg-ds-green live-pulse' : 'bg-ds-text-muted',
          className,
        )}
      />
    </AnimatePresence>
  );
}
