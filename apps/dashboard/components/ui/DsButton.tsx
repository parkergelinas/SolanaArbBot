import type { ButtonHTMLAttributes, ReactNode } from 'react';

type DsButtonVariant = 'default' | 'green' | 'red' | 'blue' | 'ghost';

const VARIANT: Record<DsButtonVariant, string> = {
  default:
    'bg-ds-elevated/50 border-ds-border text-ds-text-secondary hover:text-ds-text-primary hover:border-ds-blue/30',
  green: 'bg-ds-green/10 border-ds-green/30 text-ds-green hover:bg-ds-green/20',
  red: 'bg-ds-red/10 border-ds-red/30 text-ds-red hover:bg-ds-red/20',
  blue: 'bg-ds-blue/10 border-ds-blue/30 text-ds-blue hover:bg-ds-blue/20',
  ghost: 'bg-transparent border-transparent text-ds-text-muted hover:text-ds-text-primary hover:bg-ds-elevated/50',
};

interface DsButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: DsButtonVariant;
  children: ReactNode;
  compact?: boolean;
}

export default function DsButton({
  variant = 'default',
  compact,
  className = '',
  children,
  ...props
}: DsButtonProps) {
  return (
    <button
      type="button"
      className={`inline-flex items-center justify-center font-medium rounded-terminal border transition-colors disabled:opacity-40 disabled:cursor-not-allowed ${
        compact ? 'px-2 py-0.5 text-[10px]' : 'px-3 py-1.5 text-[11px]'
      } ${VARIANT[variant]} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
}
