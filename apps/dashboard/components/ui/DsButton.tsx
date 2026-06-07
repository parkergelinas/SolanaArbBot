import type { ButtonHTMLAttributes, ReactNode } from 'react';

type DsButtonVariant = 'default' | 'green' | 'red' | 'blue' | 'ghost' | 'amber';
type DsButtonSize    = 'xs' | 'sm' | 'md' | 'lg';

const VARIANT: Record<DsButtonVariant, string> = {
  default: 'bg-[var(--bg-elevated)] border-[var(--bg-border)] text-[color:var(--text-secondary)] hover:text-[color:var(--text-primary)] hover:border-[color-mix(in_srgb,var(--blue)_30%,var(--bg-border))]',
  green:   'bg-[color-mix(in_srgb,var(--green)_10%,transparent)] border-[color-mix(in_srgb,var(--green)_30%,var(--bg-border))] text-[color:var(--green)] hover:bg-[color-mix(in_srgb,var(--green)_18%,transparent)]',
  red:     'bg-[color-mix(in_srgb,var(--red)_8%,transparent)] border-[color-mix(in_srgb,var(--red)_28%,var(--bg-border))] text-[color:var(--red)] hover:bg-[color-mix(in_srgb,var(--red)_15%,transparent)]',
  blue:    'bg-[color-mix(in_srgb,var(--blue)_10%,transparent)] border-[color-mix(in_srgb,var(--blue)_30%,var(--bg-border))] text-[color:var(--blue)] hover:bg-[color-mix(in_srgb,var(--blue)_18%,transparent)]',
  amber:   'bg-[color-mix(in_srgb,var(--amber)_8%,transparent)] border-[color-mix(in_srgb,var(--amber)_28%,var(--bg-border))] text-[color:var(--amber)] hover:bg-[color-mix(in_srgb,var(--amber)_15%,transparent)]',
  ghost:   'bg-transparent border-transparent text-[color:var(--text-muted)] hover:text-[color:var(--text-primary)] hover:bg-[var(--bg-elevated)]',
};

const SIZE: Record<DsButtonSize, string> = {
  xs: 'px-1.5 h-5   text-[9px]  gap-1',
  sm: 'px-2   h-6   text-[10px] gap-1.5',
  md: 'px-3   h-7   text-[11px] gap-1.5',
  lg: 'px-4   h-8   text-[12px] gap-2',
};

interface DsButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: DsButtonVariant;
  size?: DsButtonSize;
  loading?: boolean;
  children: ReactNode;
  /** @deprecated use size="sm" instead */
  compact?: boolean;
}

export default function DsButton({
  variant = 'default',
  size,
  loading = false,
  compact,
  className = '',
  children,
  disabled,
  ...props
}: DsButtonProps) {
  const resolvedSize: DsButtonSize = size ?? (compact ? 'sm' : 'md');

  return (
    <button
      type="button"
      disabled={disabled || loading}
      className={`inline-flex items-center justify-center font-medium rounded-sm border transition-colors duration-75 disabled:opacity-40 disabled:cursor-not-allowed active:scale-[0.97] ${
        SIZE[resolvedSize]
      } ${VARIANT[variant]} ${className}`}
      {...props}
    >
      {loading ? (
        <>
          <svg
            className="animate-spin"
            width="10" height="10" viewBox="0 0 24 24" fill="none"
            stroke="currentColor" strokeWidth="2.5"
            aria-hidden
          >
            <path d="M21 12a9 9 0 1 1-6.219-8.56" />
          </svg>
          <span className="opacity-70">…</span>
        </>
      ) : (
        children
      )}
    </button>
  );
}
