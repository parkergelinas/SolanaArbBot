'use client';

import type { ReactNode } from 'react';

import { IntelligenceProvider } from './IntelligenceProvider';
import { WebSocketProvider } from './WebSocketProvider';
import NetworkBootstrap from './wallet/NetworkBootstrap';
import SolanaWalletProvider from './wallet/SolanaWalletProvider';

export default function Providers({
  wsUrl,
  intelUrl,
  children,
}: {
  wsUrl: string;
  intelUrl: string;
  children: ReactNode;
}) {
  return (
    <SolanaWalletProvider>
      <NetworkBootstrap />
      <WebSocketProvider url={wsUrl}>
        <IntelligenceProvider url={intelUrl}>{children}</IntelligenceProvider>
      </WebSocketProvider>
    </SolanaWalletProvider>
  );
}
