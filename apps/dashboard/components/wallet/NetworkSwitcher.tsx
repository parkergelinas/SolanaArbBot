'use client';

import { useWallet } from '@solana/wallet-adapter-react';

import { clusterLabel } from '@/stores/networkStore';
import { useNetworkStore } from '@/stores/networkStore';
import { usePaperStore } from '@/stores/paperStore';

export default function NetworkSwitcher({ compact = false }: { compact?: boolean }) {
  const cluster = useNetworkStore((s) => s.cluster);
  const setCluster = useNetworkStore((s) => s.setCluster);
  const switching = useNetworkStore((s) => s.switching);
  const initForCluster = usePaperStore((s) => s.initForCluster);
  const { disconnect } = useWallet();

  const toggle = () => {
    const next = cluster === 'devnet' ? 'mainnet-beta' : 'devnet';
    disconnect().catch(() => undefined);
    setCluster(next);
    initForCluster(next);
  };

  const isDevnet = cluster === 'devnet';

  return (
    <button
      type="button"
      onClick={toggle}
      disabled={switching}
      title={`Switch to ${isDevnet ? 'mainnet' : 'devnet'}`}
      className={`flex items-center gap-1 rounded-terminal border font-medium font-mono transition-colors disabled:opacity-50 min-h-[44px] sm:min-h-0 ${
        compact
          ? 'px-1.5 sm:px-2 py-1 text-[8px] sm:text-[9px]'
          : 'px-2.5 py-1.5 text-[10px]'
      } ${
        isDevnet
          ? 'border-ds-amber/40 text-ds-amber bg-ds-amber/8 hover:bg-ds-amber/12'
          : 'border-ds-green/40 text-ds-green bg-ds-green/8 hover:bg-ds-green/12'
      }`}
    >
      <span
        className={`w-1.5 h-1.5 rounded-full ${isDevnet ? 'bg-amber-400' : 'bg-emerald-400'}`}
      />
      <span className="sm:hidden">{isDevnet ? 'DEV' : 'MAIN'}</span>
      <span className="hidden sm:inline">{clusterLabel(cluster)}</span>
    </button>
  );
}
