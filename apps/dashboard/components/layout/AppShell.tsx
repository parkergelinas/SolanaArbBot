'use client';

import { usePathname } from 'next/navigation';

import AppNav from './AppNav';

export default function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const isTerminal = pathname === '/terminal' || pathname.startsWith('/terminal/');

  return (
    <div className="flex flex-col flex-1 min-h-0 overflow-hidden bg-ds-base">
      <AppNav />
      <main
        className={`app-main flex flex-1 flex-col min-h-0 ${
          isTerminal ? 'overflow-hidden p-0' : 'overflow-y-auto p-4 terminal-scroll'
        }`}
      >
        {children}
      </main>
    </div>
  );
}
