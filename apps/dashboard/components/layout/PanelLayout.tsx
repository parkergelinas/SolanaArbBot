'use client';

import StatusBar from './StatusBar';
import TerminalTopBar from './TerminalTopBar';
import RightRail from './RightRail';
import Orderbook from '@/components/orderbook/Orderbook';
import TradeLog from '@/components/tradelog/TradeLog';
import Watchlist from '@/components/watchlist/Watchlist';
import ArbitragePanel from '@/components/terminal/ArbitragePanel';
import DataFeedsBar from '@/components/terminal/DataFeedsBar';
import DexScreenerChart from '@/components/terminal/DexScreenerChart';
import LatencyMonitor from '@/components/terminal/LatencyMonitor';
import MarketStatsBar from '@/components/terminal/MarketStatsBar';
import { isPaperTradingEnabled } from '@/lib/config/env';

export default function PanelLayout() {
  const showPaperBanner = isPaperTradingEnabled();

  return (
    <div className="terminal-shell terminal-desk min-w-[1280px] overflow-hidden flex-1 min-h-0">
      <TerminalTopBar />
      <DataFeedsBar />
      <LatencyMonitor />
      {showPaperBanner && (
        <div className="dry-run-banner">
          PAPER TRADING — Simulated fills only · prices from DexScreener + stream
        </div>
      )}

      <div className="terminal-grid-main min-h-0">
        <Watchlist />

        <div className="terminal-grid-center">
          <MarketStatsBar />
          <div className="min-h-0 overflow-hidden">
            <DexScreenerChart />
          </div>
          <div className="terminal-grid-bottom">
            <Orderbook />
            <ArbitragePanel />
          </div>
          <TradeLog />
        </div>

        <RightRail />
      </div>

      <StatusBar />
    </div>
  );
}
