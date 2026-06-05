import type { Metadata } from 'next';

import Navigation from '@/components/Navigation';
import Providers from '@/components/Providers';

import './globals.css';

export const metadata: Metadata = {
  title: 'Solana Arb — Control Plane',
  description: 'Market intelligence & arbitrage simulation dashboard',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  const wsUrl = process.env.NEXT_PUBLIC_WS_URL ?? 'ws://localhost:3001/ws';
  const intelUrl =
    process.env.NEXT_PUBLIC_INTELLIGENCE_URL ?? 'ws://localhost:8090/intelligence';

  return (
    <html lang="en">
      <body className="flex h-screen overflow-hidden bg-platform-bg text-slate-100">
        <Providers wsUrl={wsUrl} intelUrl={intelUrl}>
          <Navigation />
          <main className="app-main flex-1 flex flex-col min-h-0 overflow-hidden p-6">
            {children}
          </main>
        </Providers>
      </body>
    </html>
  );
}
