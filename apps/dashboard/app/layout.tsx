import type { Metadata, Viewport } from 'next';

import AppShell from '@/components/layout/AppShell';
import Providers from '@/components/Providers';

import './globals.css';

export const metadata: Metadata = {
  title: 'Solana Arb — Trading Terminal',
  description: 'Live market intelligence and arbitrage terminal for Solana',
  appleWebApp: {
    capable: true,
    statusBarStyle: 'black-translucent',
    title: 'SolArb',
  },
  formatDetection: {
    telephone: false,
  },
};

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
  viewportFit: 'cover',
  themeColor: '#0a0a0b',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  const wsUrl = process.env.NEXT_PUBLIC_WS_URL ?? 'ws://localhost:3001/ws';
  const intelUrl =
    process.env.NEXT_PUBLIC_INTELLIGENCE_URL ?? 'ws://localhost:8090/intelligence';

  return (
    <html lang="en">
      <head>
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
        <link
          href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600&display=swap"
          rel="stylesheet"
        />
      </head>
      <body className="flex h-[100dvh] overflow-hidden bg-ds-base text-ds-text-primary antialiased">
        <Providers wsUrl={wsUrl} intelUrl={intelUrl}>
          <AppShell>{children}</AppShell>
        </Providers>
      </body>
    </html>
  );
}
