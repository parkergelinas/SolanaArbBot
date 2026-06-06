'use client';

import { useCallback, useMemo, type ReactNode } from 'react';
import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react';
import { WalletModalProvider } from '@solana/wallet-adapter-react-ui';
import { PhantomWalletAdapter } from '@solana/wallet-adapter-phantom';
import { SolflareWalletAdapter } from '@solana/wallet-adapter-solflare';
import { clusterApiUrl } from '@solana/web3.js';

import '@solana/wallet-adapter-react-ui/styles.css';

function resolveNetwork(): 'devnet' | 'mainnet-beta' | 'testnet' {
  const raw = process.env.NEXT_PUBLIC_SOLANA_NETWORK ?? 'mainnet-beta';
  if (raw === 'mainnet' || raw === 'mainnet-beta') return 'mainnet-beta';
  if (raw === 'testnet') return 'testnet';
  return 'devnet';
}

const NETWORK = resolveNetwork();

export default function SolanaWalletProvider({ children }: { children: ReactNode }) {
  const endpoint = useMemo(() => {
    if (process.env.NEXT_PUBLIC_SOLANA_RPC) {
      return process.env.NEXT_PUBLIC_SOLANA_RPC;
    }
    return clusterApiUrl(NETWORK);
  }, []);

  const wallets = useMemo(
    () => [new PhantomWalletAdapter(), new SolflareWalletAdapter()],
    [],
  );

  const onError = useCallback((error: Error) => {
    console.error('[wallet]', error.message);
  }, []);

  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect onError={onError}>
        <WalletModalProvider>{children}</WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
