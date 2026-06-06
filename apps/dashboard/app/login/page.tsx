'use client';

import { useCallback, useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { useWallet } from '@solana/wallet-adapter-react';
import type { WalletName } from '@solana/wallet-adapter-base';

import { useAuthStore } from '@/stores/authStore';

export default function LoginPage() {
  const router = useRouter();
  const { publicKey, wallet, wallets, connected, connecting, select, signMessage } = useWallet();
  const { authenticate, isAuthenticated, isLoading, error } = useAuthStore();
  const [walletMenuOpen, setWalletMenuOpen] = useState(false);

  useEffect(() => {
    if (isAuthenticated) {
      router.replace('/');
    }
  }, [isAuthenticated, router]);

  const handleSelectWallet = useCallback(
    (name: WalletName) => {
      select(name);
      setWalletMenuOpen(false);
    },
    [select],
  );

  const handleSign = useCallback(async () => {
    if (!signMessage || !publicKey) return;
    await authenticate(signMessage, publicKey.toBase58());
  }, [authenticate, signMessage, publicKey]);

  const installedWallets = wallets.filter(
    (w) => w.readyState === 'Installed' || w.readyState === 'Loadable',
  );
  const displayWallets = installedWallets.length > 0 ? installedWallets : wallets;

  return (
    <div className="flex items-center justify-center min-h-[100dvh] bg-ds-base px-4">
      <div className="w-full max-w-sm">
        {/* Header */}
        <div className="mb-8 text-center">
          <div className="inline-flex items-center gap-2 mb-3">
            <span className="w-2 h-2 rounded-full bg-ds-green animate-pulse" />
            <span className="text-[10px] uppercase tracking-[0.15em] text-ds-text-muted font-medium">
              Solana Arb
            </span>
          </div>
          <h1 className="text-lg font-semibold text-ds-text-primary tracking-tight">
            Wallet Authentication
          </h1>
          <p className="text-[11px] text-ds-text-muted mt-1">
            Connect an approved wallet to access the terminal
          </p>
        </div>

        {/* Card */}
        <div className="bg-ds-surface border border-ds-border rounded-terminal p-5 space-y-4">
          {/* Step 1 — Connect */}
          <div>
            <p className="text-[9px] uppercase tracking-[0.12em] text-ds-text-muted mb-2">
              Step 1 — Connect wallet
            </p>
            {connected && publicKey ? (
              <div className="flex items-center gap-2 px-3 py-2 bg-ds-elevated/60 border border-ds-green/20 rounded-terminal">
                <span className="w-1.5 h-1.5 rounded-full bg-ds-green shrink-0" />
                <span className="text-[11px] font-mono text-ds-text-primary truncate">
                  {publicKey.toBase58()}
                </span>
              </div>
            ) : (
              <div className="relative">
                <button
                  type="button"
                  disabled={connecting}
                  onClick={() => setWalletMenuOpen((v) => !v)}
                  className="w-full flex items-center justify-between px-3 py-2.5 bg-ds-elevated/40 border border-ds-border rounded-terminal text-[11px] text-ds-text-secondary hover:text-ds-text-primary hover:border-ds-blue/40 transition-colors disabled:opacity-50"
                >
                  <span>{connecting ? 'Connecting…' : 'Select wallet'}</span>
                  <span className="text-[9px] text-ds-text-muted">{walletMenuOpen ? '▴' : '▾'}</span>
                </button>

                {walletMenuOpen && (
                  <div className="absolute top-[calc(100%+4px)] left-0 right-0 z-50 bg-ds-surface border border-ds-border rounded-terminal shadow-lg shadow-black/40 py-1">
                    {displayWallets.map(({ adapter, readyState }) => {
                      const missing = readyState === 'NotDetected';
                      return (
                        <button
                          key={adapter.name}
                          type="button"
                          disabled={missing}
                          onClick={() => handleSelectWallet(adapter.name)}
                          className="w-full text-left px-3 py-2 hover:bg-ds-elevated/70 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                        >
                          <span className="block text-[11px] font-medium text-ds-text-primary">
                            {adapter.name}
                          </span>
                          <span className="block text-[9px] text-ds-text-muted mt-0.5">
                            {missing ? 'Not detected — install extension' : 'Solana wallet'}
                          </span>
                        </button>
                      );
                    })}
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Step 2 — Sign */}
          <div>
            <p className="text-[9px] uppercase tracking-[0.12em] text-ds-text-muted mb-2">
              Step 2 — Sign to authenticate
            </p>
            <button
              type="button"
              disabled={!connected || isLoading}
              onClick={handleSign}
              className="w-full px-3 py-2.5 bg-ds-blue/10 border border-ds-blue/30 text-ds-blue text-[11px] font-medium rounded-terminal hover:bg-ds-blue/20 hover:border-ds-blue/50 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
            >
              {isLoading ? 'Waiting for signature…' : 'Sign message'}
            </button>
          </div>

          {/* Error */}
          {error && (
            <div className="px-3 py-2 bg-ds-red/10 border border-ds-red/20 rounded-terminal">
              <p className="text-[11px] text-ds-red">{error}</p>
            </div>
          )}
        </div>

        {/* Footer note */}
        <p className="text-center text-[9px] text-ds-text-muted mt-4">
          Only approved wallets can sign in. No transaction is initiated.
        </p>
      </div>
    </div>
  );
}
