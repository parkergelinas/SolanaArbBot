import type { Metadata } from 'next';
import './globals.css';
import StreamBootstrap from '@/components/StreamBootstrap';

export const metadata: Metadata = {
  title: 'Sol Intel — Real-Time Whale Tracking',
  description: 'Solana streaming market intelligence terminal',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className="h-screen overflow-hidden">
        <StreamBootstrap />
        {children}
      </body>
    </html>
  );
}
