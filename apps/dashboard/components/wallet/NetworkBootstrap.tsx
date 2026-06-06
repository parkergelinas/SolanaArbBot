'use client';

import { useEffect } from 'react';

import { usePaperStore } from '@/stores/paperStore';
import { useNetworkStore } from '@/stores/networkStore';

/** Hydrate persisted cluster + paper portfolio on mount. */
export default function NetworkBootstrap() {
  const hydrate = useNetworkStore((s) => s.hydrate);
  const cluster = useNetworkStore((s) => s.cluster);
  const initForCluster = usePaperStore((s) => s.initForCluster);

  useEffect(() => {
    hydrate();
  }, [hydrate]);

  useEffect(() => {
    initForCluster(cluster);
  }, [cluster, initForCluster]);

  return null;
}
