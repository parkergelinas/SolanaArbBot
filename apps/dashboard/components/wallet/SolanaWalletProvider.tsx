'use client';

import { useCallback, useEffect, useMemo, type ReactNode } from 'react';
import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react';
import { PhantomWalletAdapter } from '@solana/wallet-adapter-phantom';
import { TrustWalletAdapter } from '@solana/wallet-adapter-trust';

import { useNetworkStore } from '@/stores/networkStore';

export default function SolanaWalletProvider({ children }: { children: ReactNode }) {
  const cluster = useNetworkStore((s) => s.cluster);
  const rpcUrl = useNetworkStore((s) => s.rpcUrl);
  const hydrate = useNetworkStore((s) => s.hydrate);

  useEffect(() => {
    hydrate();
  }, [hydrate]);

  const wallets = useMemo(
    () => [new PhantomWalletAdapter(), new TrustWalletAdapter()],
    [],
  );

  const onError = useCallback((error: Error) => {
    console.error('[wallet]', error.message);
  }, []);

  return (
    <ConnectionProvider key={cluster} endpoint={rpcUrl} config={{ commitment: 'confirmed' }}>
      <WalletProvider wallets={wallets} autoConnect onError={onError}>
        {children}
      </WalletProvider>
    </ConnectionProvider>
  );
}
