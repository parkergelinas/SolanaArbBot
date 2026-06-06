'use client';

import type { ReactNode } from 'react';
import { useEffect } from 'react';

interface MobileSheetProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  /** 0–1 fraction of viewport height */
  maxHeight?: string;
}

export default function MobileSheet({
  open,
  onClose,
  title,
  children,
  maxHeight = '72dvh',
}: MobileSheetProps) {
  useEffect(() => {
    if (!open) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', onKey);
    return () => {
      document.body.style.overflow = prev;
      document.removeEventListener('keydown', onKey);
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    <>
      <button
        type="button"
        className="lg:hidden fixed inset-0 z-[9990] bg-black/55 backdrop-blur-[1px]"
        aria-label="Close panel"
        onClick={onClose}
      />
      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className="lg:hidden fixed inset-x-0 bottom-0 z-[9991] flex flex-col rounded-t-md border-t border-ds-border bg-ds-surface shadow-2xl shadow-black/50"
        style={{ maxHeight, paddingBottom: 'var(--safe-bottom)' }}
      >
        <div className="flex items-center justify-between px-4 py-3 border-b border-ds-border shrink-0">
          <span className="text-[12px] font-semibold uppercase tracking-[0.1em] text-ds-text-primary">
            {title}
          </span>
          <button
            type="button"
            onClick={onClose}
            className="touch-target flex items-center justify-center px-3 text-[11px] font-medium text-ds-text-secondary hover:text-ds-text-primary"
          >
            Close
          </button>
        </div>
        <div className="flex-1 min-h-0 overflow-hidden">{children}</div>
      </div>
    </>
  );
}
