import type { Metadata } from 'next';
import './globals.css';
import Navigation from '@/components/Navigation';
import { WebSocketProvider } from '@/components/WebSocketProvider';

export const metadata: Metadata = {
  title: 'Solana Arb — Control Plane',
  description: 'Market intelligence & arbitrage simulation dashboard',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  const wsUrl = process.env.NEXT_PUBLIC_WS_URL ?? 'ws://localhost:3001/ws';

  return (
    <html lang="en">
      <body className="flex h-screen overflow-hidden bg-platform-bg text-slate-100">
        <WebSocketProvider url={wsUrl}>
          <Navigation />
          <main className="app-main flex-1 flex flex-col min-h-0 overflow-hidden p-6">{children}</main>
        </WebSocketProvider>
      </body>
    </html>
  );
}
