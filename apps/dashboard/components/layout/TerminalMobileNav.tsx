'use client';

export type TerminalMobilePanel = 'chart' | 'markets' | 'trade' | 'log';

const TABS: { id: TerminalMobilePanel; label: string }[] = [
  { id: 'chart', label: 'Chart' },
  { id: 'markets', label: 'Markets' },
  { id: 'trade', label: 'Trade' },
  { id: 'log', label: 'Log' },
];

interface TerminalMobileNavProps {
  active: TerminalMobilePanel;
  onChange: (panel: TerminalMobilePanel) => void;
}

export default function TerminalMobileNav({ active, onChange }: TerminalMobileNavProps) {
  return (
    <nav
      className="lg:hidden grid grid-cols-4 shrink-0 border-t border-ds-border bg-ds-elevated/40"
      aria-label="Terminal panels"
    >
      {TABS.map(({ id, label }) => {
        const isActive = active === id;
        return (
          <button
            key={id}
            type="button"
            onClick={() => onChange(id)}
            className={`touch-target py-2.5 text-[11px] font-semibold uppercase tracking-[0.08em] transition-colors border-t-2 ${
              isActive
                ? 'text-ds-blue border-ds-blue bg-ds-surface'
                : 'text-ds-text-secondary border-transparent hover:text-ds-text-primary hover:bg-ds-surface/50'
            }`}
            aria-current={isActive ? 'page' : undefined}
          >
            {label}
          </button>
        );
      })}
    </nav>
  );
}
