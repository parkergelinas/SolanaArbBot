'use client';

import { usePathname } from 'next/navigation';

import AppNav from './AppNav';

export default function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const isTerminal = pathname.startsWith('/terminal');

  return (
    <div className="flex flex-col flex-1 min-h-0 overflow-hidden">
      <AppNav />
      <main
        className={
          isTerminal
            ? 'flex flex-col flex-1 min-h-0 overflow-hidden'
            : 'app-main flex-1 flex flex-col min-h-0 overflow-y-auto p-6'
        }
      >
        {children}
      </main>
    </div>
  );
}
