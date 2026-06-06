'use client';

import { useCallback, useEffect, useMemo, type ReactNode } from 'react';
import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react';
import { WalletModalProvider } from '@solana/wallet-adapter-react-ui';
import { PhantomWalletAdapter } from '@solana/wallet-adapter-phantom';
import { SolflareWalletAdapter } from '@solana/wallet-adapter-solflare';

import { useNetworkStore } from '@/stores/networkStore';

import '@solana/wallet-adapter-react-ui/styles.css';

export default function SolanaWalletProvider({ children }: { children: ReactNode }) {
  const cluster = useNetworkStore((s) => s.cluster);
  const rpcUrl = useNetworkStore((s) => s.rpcUrl);
  const hydrate = useNetworkStore((s) => s.hydrate);

  useEffect(() => {
    hydrate();
  }, [hydrate]);

  const wallets = useMemo(
    () => [new PhantomWalletAdapter(), new SolflareWalletAdapter()],
    [],
  );

  const onError = useCallback((error: Error) => {
    console.error('[wallet]', error.message);
  }, []);

  return (
    <ConnectionProvider key={cluster} endpoint={rpcUrl} config={{ commitment: 'confirmed' }}>
      <WalletProvider wallets={wallets} autoConnect onError={onError}>
        <WalletModalProvider>{children}</WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
