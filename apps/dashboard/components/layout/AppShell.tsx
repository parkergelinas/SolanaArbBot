'use client';

import AppNav from './AppNav';

export default function AppShell({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex flex-col flex-1 min-h-0 overflow-hidden bg-ds-base">
      <AppNav />
      <main className="app-main flex flex-1 flex-col min-h-0 overflow-y-auto p-4 terminal-scroll">
        {children}
      </main>
    </div>
  );
}
