import type { ReactNode } from 'react';

interface DsTableProps {
  children: ReactNode;
  className?: string;
}

export function DsTable({ children, className = '' }: DsTableProps) {
  return (
    <div className={`flex-1 min-h-0 overflow-auto terminal-scroll mobile-table-scroll ${className}`}>
      <table className="ds-table w-full text-[11px]">{children}</table>
    </div>
  );
}

export function DsTableHead({ children }: { children: ReactNode }) {
  return (
    <thead className="sticky top-0 z-10 bg-ds-elevated border-b border-ds-border">{children}</thead>
  );
}

export function DsTh({
  children,
  align = 'left',
  className = '',
}: {
  children: ReactNode;
  align?: 'left' | 'right' | 'center';
  className?: string;
}) {
  return (
    <th
      className={`px-2 py-1.5 text-[9px] uppercase tracking-[0.12em] font-medium text-ds-text-muted ${
        align === 'right' ? 'text-right' : align === 'center' ? 'text-center' : 'text-left'
      } ${className}`}
    >
      {children}
    </th>
  );
}

export function DsTd({
  children,
  align = 'left',
  className = '',
  mono,
  title,
}: {
  children: ReactNode;
  align?: 'left' | 'right' | 'center';
  className?: string;
  mono?: boolean;
  title?: string;
}) {
  return (
    <td
      title={title}
      className={`px-2 py-1.5 border-b border-ds-border/50 ${
        mono ? 'font-mono tabular-nums' : ''
      } ${
        align === 'right' ? 'text-right' : align === 'center' ? 'text-center' : 'text-left'
      } ${className}`}
    >
      {children}
    </td>
  );
}
