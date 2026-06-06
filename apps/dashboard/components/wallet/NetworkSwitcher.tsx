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
      className={`flex items-center gap-1.5 rounded-lg border font-medium mono transition-colors disabled:opacity-50 ${
        compact
          ? 'px-2 py-1 text-[9px]'
          : 'px-2.5 py-1.5 text-[10px]'
      } ${
        isDevnet
          ? 'border-amber-500/40 text-amber-400 bg-amber-500/10 hover:bg-amber-500/15'
          : 'border-emerald-500/40 text-emerald-400 bg-emerald-500/10 hover:bg-emerald-500/15'
      }`}
    >
      <span
        className={`w-1.5 h-1.5 rounded-full ${isDevnet ? 'bg-amber-400' : 'bg-emerald-400'}`}
      />
      {clusterLabel(cluster)}
    </button>
  );
}
