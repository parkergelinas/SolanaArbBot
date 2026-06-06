import type { ReactNode } from 'react';

type DsBadgeTone = 'default' | 'green' | 'red' | 'amber' | 'blue' | 'muted';

const TONE: Record<DsBadgeTone, string> = {
  default: 'border-ds-border text-ds-text-secondary bg-ds-elevated/40',
  green: 'border-ds-green/30 text-ds-green bg-ds-green/8',
  red: 'border-ds-red/30 text-ds-red bg-ds-red/8',
  amber: 'border-ds-amber/30 text-ds-amber bg-ds-amber/8',
  blue: 'border-ds-blue/30 text-ds-blue bg-ds-blue/8',
  muted: 'border-ds-border text-ds-text-muted bg-ds-elevated/30',
};

interface DsBadgeProps {
  children: ReactNode;
  tone?: DsBadgeTone;
  className?: string;
  mono?: boolean;
}

export default function DsBadge({ children, tone = 'default', className = '', mono }: DsBadgeProps) {
  return (
    <span
      className={`inline-flex items-center gap-1 px-1.5 py-px rounded-terminal border text-[10px] uppercase tracking-wider ${
        mono ? 'font-mono normal-case tracking-normal' : 'font-medium'
      } ${TONE[tone]} ${className}`}
    >
      {children}
    </span>
  );
}
