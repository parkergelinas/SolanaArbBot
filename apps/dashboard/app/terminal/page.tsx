'use client';

import ArbitragePanel from '@/components/terminal/ArbitragePanel';
import DexScreenerChart from '@/components/terminal/DexScreenerChart';
import DepthPanel from '@/components/terminal/DepthPanel';
import LatencyMonitor from '@/components/terminal/LatencyMonitor';
import MarketStatsBar from '@/components/terminal/MarketStatsBar';
import SignalPanel from '@/components/terminal/SignalPanel';
import TerminalHeader from '@/components/terminal/TerminalHeader';
import VirtualizedSwapFeed from '@/components/terminal/VirtualizedSwapFeed';
import Watchlist from '@/components/terminal/Watchlist';

/**
 * Personal trading terminal — TradingView / Bloomberg-inspired layout.
 * Data: live stream-api WS when available; demo simulator fills gaps offline.
 */
export default function TerminalPage() {
  return (
    <div className="terminal-shell flex flex-col flex-1 min-h-0 overflow-hidden">
      <TerminalHeader />
      <LatencyMonitor />

      <div className="flex flex-1 min-h-0 overflow-hidden">
        <Watchlist />

        <div className="flex flex-col flex-1 min-w-0 min-h-0 overflow-hidden">
          <MarketStatsBar />
          <div className="flex flex-1 min-h-0 overflow-hidden">
            <DexScreenerChart />
            <div className="w-44 xl:w-52 flex-shrink-0 flex flex-col min-h-0 overflow-hidden border-l border-terminal-border">
              <DepthPanel />
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
