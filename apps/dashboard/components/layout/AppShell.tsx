'use client';

import { usePathname } from 'next/navigation';

import { AppMobileNav, AppTopNav } from './AppNav';

export default function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const isTerminal = pathname === '/terminal' || pathname.startsWith('/terminal/');

  return (
    <div
      className={`flex flex-col flex-1 min-h-0 overflow-hidden bg-ds-base ${
        isTerminal ? 'terminal-app-shell' : ''
      }`}
    >
      <AppTopNav hideOnMobileTerminal={isTerminal} />
      <main
        className={`app-main flex flex-1 flex-col min-h-0 ${
          isTerminal
            ? 'overflow-hidden p-0'
            : 'page-desk-contrast overflow-y-auto p-3 sm:p-4 terminal-scroll overscroll-y-contain'
        }`}
      >
        {children}
      </main>
      <AppMobileNav hideOnTerminal={isTerminal} />
    </div>
  );
}
