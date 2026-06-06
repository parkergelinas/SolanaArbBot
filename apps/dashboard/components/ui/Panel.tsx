'use client';

import type { ReactNode } from 'react';

interface PanelProps {
  title: string;
  subtitle?: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
  bodyClassName?: string;
  noPadding?: boolean;
}

export default function Panel({
  title,
  subtitle,
  action,
  children,
  className = '',
  bodyClassName = '',
  noPadding = false,
}: PanelProps) {
  return (
    <section
      className={`flex flex-col min-h-0 min-w-0 bg-ds-surface border-ds-border ${className}`}
    >
      <header className="flex items-center justify-between h-8 px-3 border-b border-ds-border shrink-0 gap-2">
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-[11px] font-medium uppercase tracking-[0.1em] text-ds-text-secondary truncate">
            {title}
          </span>
          {subtitle && (
            <span className="text-[11px] font-mono text-ds-text-muted truncate hidden sm:inline">
              {subtitle}
            </span>
          )}
        </div>
        {action && <div className="shrink-0">{action}</div>}
      </header>
      <div className={`flex-1 min-h-0 min-w-0 ${noPadding ? '' : ''} ${bodyClassName}`}>
        {children}
      </div>
    </section>
  );
}
