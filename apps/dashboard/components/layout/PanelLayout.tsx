'use client';

import { useState } from 'react';

import StatusBar from './StatusBar';
import TerminalMobileHeader from './TerminalMobileHeader';
import TerminalMobileNav, { type TerminalMobilePanel } from './TerminalMobileNav';
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
import QuickTradeBar from '@/components/terminal/QuickTradeBar';
import MobileSheet from '@/components/ui/MobileSheet';
import TabBar from '@/components/ui/TabBar';
import { isPaperTradingEnabled } from '@/lib/config/env';

type LogTab = 'fills' | 'arb';

export default function PanelLayout() {
  const showPaperBanner = isPaperTradingEnabled();
  const [mobilePanel, setMobilePanel] = useState<TerminalMobilePanel>('chart');
  const [bookOpen, setBookOpen] = useState(false);
  const [logTab, setLogTab] = useState<LogTab>('fills');

  return (
    <div className="terminal-shell terminal-desk overflow-hidden flex-1 min-h-0 w-full max-w-[100vw]">
      <TerminalMobileHeader />

      <div className="hidden lg:block">
        <TerminalTopBar />
      </div>

      <DataFeedsBar />
      <div className="hidden lg:block">
        <LatencyMonitor />
      </div>

      {showPaperBanner && (
        <div className="dry-run-banner px-2 text-center leading-tight">
          <span className="lg:hidden">Paper trading — simulated fills</span>
          <span className="hidden lg:inline">
            PAPER TRADING — Simulated fills only · prices from DexScreener + stream
          </span>
        </div>
      )}

      {/* Desktop layout */}
      <div className="hidden lg:grid terminal-grid-main min-h-0">
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

      {/* Mobile layout */}
      <div className="lg:hidden flex flex-1 flex-col min-h-0 overflow-hidden">
        {mobilePanel === 'chart' && (
          <>
            <div className="flex flex-col flex-1 min-h-0 overflow-hidden">
              <DexScreenerChart compact />
            </div>
            <QuickTradeBar onOpenBook={() => setBookOpen(true)} />
          </>
        )}

        {mobilePanel === 'markets' && (
          <div className="flex flex-1 min-h-0 overflow-hidden w-full">
            <Watchlist mobile onAfterSelect={() => setMobilePanel('chart')} />
          </div>
        )}

        {mobilePanel === 'trade' && (
          <div className="flex flex-1 min-h-0 overflow-hidden w-full">
            <RightRail />
          </div>
        )}

        {mobilePanel === 'log' && (
          <div className="flex flex-col flex-1 min-h-0 overflow-hidden">
            <TabBar
              tabs={[
                { id: 'fills' as const, label: 'Fills' },
                { id: 'arb' as const, label: 'Arb' },
              ]}
              active={logTab}
              onChange={setLogTab}
            />
            <div className="flex-1 min-h-0 overflow-hidden">
              {logTab === 'fills' ? <TradeLog /> : <ArbitragePanel />}
            </div>
          </div>
        )}

        <TerminalMobileNav active={mobilePanel} onChange={setMobilePanel} />
      </div>

      <StatusBar compactOnMobile />

      <MobileSheet open={bookOpen} onClose={() => setBookOpen(false)} title="Order book" maxHeight="75dvh">
        <div className="h-[min(65dvh,520px)]">
          <Orderbook />
        </div>
      </MobileSheet>
    </div>
  );
}
