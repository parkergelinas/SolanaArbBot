'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import type { WalletName } from '@solana/wallet-adapter-base';
import type { PublicKey } from '@solana/web3.js';

const WALLET_LABELS: Record<string, { title: string; subtitle: string }> = {
  Phantom: { title: 'Phantom', subtitle: 'Solana extension' },
  Trust: { title: 'Trust Wallet', subtitle: 'Extension or mobile' },
};

function shortenAddress(key: PublicKey): string {
  const s = key.toBase58();
  return `${s.slice(0, 4)}…${s.slice(-4)}`;
}

export default function WalletConnectButton() {
  const { publicKey, wallet, disconnect, connecting, connected, select, wallets } = useWallet();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onPointerDown = (event: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener('mousedown', onPointerDown);
    return () => document.removeEventListener('mousedown', onPointerDown);
  }, []);

  useEffect(() => {
    if (!connected) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false);
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [connected]);

  const handleSelect = useCallback(
    async (walletName: WalletName) => {
      select(walletName);
      setOpen(false);
    },
    [select],
  );

  const handleDisconnect = useCallback(async () => {
    await disconnect();
    setOpen(false);
  }, [disconnect]);

  const installedWallets = wallets.filter((w) => w.readyState === 'Installed' || w.readyState === 'Loadable');
  const availableWallets = installedWallets.length > 0 ? installedWallets : wallets;

  const walletLabel = wallet?.adapter.name
    ? (WALLET_LABELS[wallet.adapter.name]?.title ?? wallet.adapter.name)
    : 'Wallet';

  return (
    <div ref={rootRef} className="relative">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        disabled={connecting}
        className={`inline-flex items-center gap-1.5 h-7 px-2.5 text-[11px] font-medium rounded-terminal border transition-colors ${
          connected
            ? 'bg-ds-elevated/60 border-ds-green/30 text-ds-green hover:bg-ds-green/10'
            : 'bg-ds-surface border-ds-border text-ds-text-secondary hover:text-ds-text-primary hover:border-ds-blue/40 hover:bg-ds-elevated/50'
        } disabled:opacity-50`}
        aria-expanded={open}
        aria-haspopup="menu"
      >
        {connected && publicKey ? (
          <>
            <span className="w-1.5 h-1.5 rounded-full bg-ds-green shrink-0" />
            <span className="font-mono tabular-nums">{shortenAddress(publicKey)}</span>
          </>
        ) : (
          <span>{connecting ? 'Connecting…' : 'Connect Wallet'}</span>
        )}
        <span className="text-ds-text-muted text-[9px]">{open ? '▴' : '▾'}</span>
      </button>

      {open && (
        <div
          role="menu"
          className="absolute right-0 top-[calc(100%+4px)] z-[10001] min-w-[13.5rem] py-1 bg-ds-surface border border-ds-border rounded-terminal shadow-lg shadow-black/40"
        >
          {connected && publicKey ? (
            <>
              <div className="px-3 py-2 border-b border-ds-border">
                <p className="text-[9px] uppercase tracking-[0.12em] text-ds-text-muted">Connected</p>
                <p className="text-[11px] font-medium text-ds-text-primary mt-0.5">{walletLabel}</p>
                <p className="text-[10px] font-mono text-ds-text-secondary mt-0.5">{publicKey.toBase58()}</p>
              </div>
              <button
                type="button"
                role="menuitem"
                onClick={handleDisconnect}
                className="w-full text-left px-3 py-2 text-[11px] text-ds-red hover:bg-ds-red/10 transition-colors"
              >
                Disconnect
              </button>
            </>
          ) : (
            <>
              <p className="px-3 py-1.5 text-[9px] uppercase tracking-[0.12em] text-ds-text-muted border-b border-ds-border">
                Select wallet
              </p>
              {availableWallets.map(({ adapter, readyState }) => {
                const meta = WALLET_LABELS[adapter.name] ?? {
                  title: adapter.name,
                  subtitle: 'Solana wallet',
                };
                const missing = readyState === 'NotDetected';
                return (
                  <button
                    key={adapter.name}
                    type="button"
                    role="menuitem"
                    disabled={missing}
                    onClick={() => handleSelect(adapter.name)}
                    className="w-full text-left px-3 py-2 hover:bg-ds-elevated/70 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                  >
                    <span className="block text-[11px] font-medium text-ds-text-primary">{meta.title}</span>
                    <span className="block text-[9px] text-ds-text-muted mt-0.5">
                      {missing ? 'Not detected — install extension' : meta.subtitle}
                    </span>
                  </button>
                );
              })}
            </>
          )}
        </div>
      )}
    </div>
  );
}
