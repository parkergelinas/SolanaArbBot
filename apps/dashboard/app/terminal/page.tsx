'use client';

import TerminalHeader from '@/components/terminal/TerminalHeader';
import LatencyMonitor from '@/components/terminal/LatencyMonitor';
import TokenTable from '@/components/terminal/TokenTable';
import CandleChart from '@/components/terminal/CandleChart';
import ArbitragePanel from '@/components/terminal/ArbitragePanel';
import VirtualizedSwapFeed from '@/components/terminal/VirtualizedSwapFeed';
import SignalPanel from '@/components/terminal/SignalPanel';

export default function TerminalPage() {
  return (
    <div className="terminal-shell flex flex-col flex-1 min-h-0 overflow-hidden">
      <TerminalHeader />
      <LatencyMonitor />
      <TokenTable />
      <div className="flex flex-1 min-h-0 overflow-hidden">
        <div className="flex flex-col flex-1 min-w-0 min-h-0 overflow-hidden">
          <div className="flex flex-1 min-h-0 overflow-hidden">
            <CandleChart />
            <div className="w-48 flex-shrink-0 flex flex-col min-h-0 overflow-hidden">
              <ArbitragePanel />
            </div>
          </div>
          <VirtualizedSwapFeed />
        </div>
        <SignalPanel />
      </div>
    </div>
  );
}
