'use client';

import type { ReactNode } from 'react';

interface PageHeaderProps {
  title: string;
  description?: string;
  actions?: ReactNode;
}

export function PageHeader({ title, description, actions }: PageHeaderProps) {
  return (
    <header className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <h1 className="text-[18px] font-semibold text-ds-text-primary tracking-tight">{title}</h1>
        {description && (
          <p className="text-ds-text-secondary text-[13px] mt-0.5">{description}</p>
        )}
      </div>
      {actions && <div className="flex items-center gap-2 flex-wrap shrink-0">{actions}</div>}
    </header>
  );
}

interface CompactPageHeaderProps {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
}

/** Signals / terminal desk compact header row. */
export function CompactPageHeader({ title, subtitle, actions }: CompactPageHeaderProps) {
  return (
    <header className="flex flex-wrap items-center gap-x-4 gap-y-2 shrink-0 pb-2 border-b border-ds-border">
      <div className="flex items-baseline gap-3 min-w-0">
        <h1 className="text-[15px] font-semibold tracking-[0.06em] text-ds-text-primary uppercase">
          {title}
        </h1>
        {subtitle && (
          <p className="hidden sm:inline text-[11px] text-ds-text-muted truncate">{subtitle}</p>
        )}
      </div>
      {actions && (
        <div className="flex items-center gap-2 flex-wrap ml-auto [&_button]:min-h-[40px] [&_button]:sm:min-h-0">
          {actions}
        </div>
      )}
    </header>
  );
}

interface PageShellProps {
  children: ReactNode;
  className?: string;
  desk?: boolean;
}

export function PageShell({ children, className = '', desk }: PageShellProps) {
  const base = desk
    ? 'flex flex-col gap-2 page-desk-height max-w-[100rem] mx-auto w-full pb-2 md:pb-4'
    : 'flex flex-col gap-3 max-w-[90rem] pb-4 md:pb-6';
  return <div className={`${base} ${className}`}>{children}</div>;
}

interface DsPanelProps {
  title?: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
  compact?: boolean;
  flush?: boolean;
}

export function DsPanel({ title, action, children, className = '', compact, flush }: DsPanelProps) {
  const bodyClass = flush ? '' : compact ? 'p-2' : 'p-4';

  return (
    <section
      className={`bg-ds-surface border border-ds-border rounded-terminal overflow-hidden flex flex-col min-h-0 ${className}`}
    >
      {title && (
        <div
          className={`flex items-center justify-between border-b border-ds-border shrink-0 ${
            compact ? 'px-3 py-2' : 'px-4 py-2.5'
          }`}
        >
          <span className="text-[10px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">
            {title}
          </span>
          {action}
        </div>
      )}
      {flush ? children : <div className={bodyClass}>{children}</div>}
    </section>
  );
}

interface DsStatPillProps {
  label: string;
  value: string | number;
  accent?: string;
}

export function DsStatPill({ label, value, accent }: DsStatPillProps) {
  return (
    <div className="bg-ds-surface border border-ds-border rounded-terminal px-3 py-2.5 min-w-[6.5rem]">
      <p className="text-[9px] uppercase tracking-[0.12em] text-ds-text-muted mb-0.5">{label}</p>
      <p
        className="text-[15px] font-semibold font-mono tabular-nums"
        style={{ color: accent ?? 'var(--text-primary, #e8ecf4)' }}
      >
        {value}
      </p>
    </div>
  );
}
