'use client';

import { useState } from 'react';

export default function MethodologyTip({
  title,
  body,
}: {
  title: string;
  body: string;
}) {
  const [open, setOpen] = useState(false);

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="text-[9px] font-mono uppercase tracking-wider text-ds-text-muted hover:text-ds-blue transition-colors"
        title={title}
      >
        ? method
      </button>
      {open && (
        <div className="absolute right-0 top-full mt-1 z-20 w-56 p-2.5 rounded-terminal border border-ds-border bg-ds-elevated shadow-lg text-[10px] text-ds-text-secondary leading-relaxed">
          <p className="font-semibold text-ds-text-primary mb-1">{title}</p>
          <p>{body}</p>
        </div>
      )}
    </div>
  );
}
