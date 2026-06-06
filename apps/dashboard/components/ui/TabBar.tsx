'use client';

interface TabBarProps<T extends string> {
  tabs: { id: T; label: string; count?: number }[];
  active: T;
  onChange: (id: T) => void;
}

export default function TabBar<T extends string>({ tabs, active, onChange }: TabBarProps<T>) {
  return (
    <div className="flex border-b border-ds-border shrink-0">
      {tabs.map((tab) => {
        const isActive = tab.id === active;
        return (
          <button
            key={tab.id}
            type="button"
            onClick={() => onChange(tab.id)}
            className={`flex-1 min-h-[44px] lg:min-h-0 lg:h-8 text-[11px] font-medium uppercase tracking-[0.08em] transition-colors border-b-2 -mb-px ${
              isActive
                ? 'text-ds-text-primary border-ds-blue bg-ds-elevated/50'
                : 'text-ds-text-secondary border-transparent hover:text-ds-text-primary hover:bg-ds-elevated/30'
            }`}
          >
            {tab.label}
            {tab.count != null && tab.count > 0 && (
              <span className="ml-1 font-mono text-ds-text-muted">{tab.count}</span>
            )}
          </button>
        );
      })}
    </div>
  );
}
