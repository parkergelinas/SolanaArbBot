'use client';

import { useMemo, useState } from 'react';

import TabBar from '@/components/ui/TabBar';
import OrderTicket from '@/components/trading/OrderTicket';
import Positions from '@/components/positions/Positions';
import SignalFeed from '@/components/terminal/SignalFeed';

type RailTab = 'signals' | 'trade' | 'portfolio';

export default function RightRail() {
  const [tab, setTab] = useState<RailTab>('trade');

  const tabs = useMemo(
    () => [
      { id: 'signals' as const, label: 'Signals' },
      { id: 'trade' as const, label: 'Trade' },
      { id: 'portfolio' as const, label: 'Portfolio' },
    ],
    [],
  );

  return (
    <aside className="flex flex-col min-h-0 bg-ds-surface overflow-hidden">
      <TabBar tabs={tabs} active={tab} onChange={setTab} />
      <div className="flex-1 min-h-0 overflow-hidden">
        {tab === 'signals' && <SignalFeed />}
        {tab === 'trade' && <OrderTicket />}
        {tab === 'portfolio' && <Positions embedded />}
      </div>
    </aside>
  );
}
