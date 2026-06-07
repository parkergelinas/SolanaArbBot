'use client';

import type { ReactNode } from 'react';
import { motion } from 'framer-motion';
import { cn } from '@/lib/cn';

interface PanelProps {
  title?: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
  compact?: boolean;
  flush?: boolean;
  /** Subtle left-edge accent bar colour (Tailwind bg class). */
  accent?: string;
  animate?: boolean;
}

export function Panel({
  title,
  action,
  children,
  className = '',
  compact,
  flush,
  accent,
  animate = true,
}: PanelProps) {
  const bodyPad = flush ? '' : compact ? 'p-2' : 'p-4';

  return (
    <motion.section
      initial={animate ? { opacity: 0, y: 6 } : false}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.22, ease: 'easeOut' }}
      className={cn(
        'relative bg-ds-surface border border-ds-border rounded-terminal overflow-hidden flex flex-col min-h-0',
        className,
      )}
    >
      {accent && (
        <span className={cn('absolute inset-y-0 left-0 w-[2px]', accent)} />
      )}
      {title && (
        <div
          className={cn(
            'flex items-center justify-between border-b border-ds-border shrink-0',
            compact ? 'px-3 py-2' : 'px-4 py-2.5',
          )}
        >
          <span className="text-[10px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">
            {title}
          </span>
          {action}
        </div>
      )}
      {flush ? children : <div className={bodyPad}>{children}</div>}
    </motion.section>
  );
}
