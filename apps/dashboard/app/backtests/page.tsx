'use client';

import { DsPanel, PageHeader, PageShell } from '@/components/layout/PageShell';

// Backtest results viewer — populated by the backtesting engine once wired.
// Currently shows an empty state with the expected schema.

interface BacktestResult {
  id:              string;
  name:            string;
  start_date:      string;
  end_date:        string;
  total_return:    number;
  sharpe:          number;
  max_drawdown:    number;
  win_rate:        number;
  total_trades:    number;
}

const MOCK_RESULTS: BacktestResult[] = [];

export default function BacktestsPage() {
  const results = MOCK_RESULTS;

  return (
    <PageShell className="max-w-6xl">
      <PageHeader
        title="Backtests"
        description="Historical simulation results from the backtesting engine"
      />

      {results.length === 0 ? (
        <DsPanel className="border-dashed !p-12 text-center space-y-3">
          <p className="text-slate-400 text-sm">No backtest results yet.</p>
          <p className="text-slate-500 text-xs max-w-sm mx-auto">
            Run a backtest from the CLI or wire the backtesting engine to the event bus.
            Results will appear here automatically.
          </p>
          <code className="block mt-4 text-xs bg-ds-elevated text-ds-green px-4 py-2 rounded-terminal font-mono">
            cargo run -p backtester-app -- --hours 24 --interval 10
          </code>
        </DsPanel>
      ) : (
        <div className="grid gap-4">
          {results.map(r => (
            <div key={r.id} className="bg-slate-800 border border-slate-700 rounded-xl p-5">
              <div className="flex items-center justify-between mb-4">
                <div>
                  <h3 className="text-slate-100 font-medium">{r.name}</h3>
                  <p className="text-slate-500 text-xs mt-0.5">{r.start_date} → {r.end_date}</p>
                </div>
                <span className={`text-2xl font-semibold mono ${r.total_return >= 0 ? 'text-green-400' : 'text-red-400'}`}>
                  {r.total_return >= 0 ? '+' : ''}{(r.total_return * 100).toFixed(2)}%
                </span>
              </div>
              <div className="grid grid-cols-4 gap-3 text-center">
                {[
                  { label: 'Sharpe', value: r.sharpe.toFixed(2) },
                  { label: 'Max DD', value: `${(r.max_drawdown * 100).toFixed(1)}%` },
                  { label: 'Win Rate', value: `${(r.win_rate * 100).toFixed(1)}%` },
                  { label: 'Trades', value: r.total_trades.toLocaleString() },
                ].map(({ label, value }) => (
                  <div key={label} className="bg-slate-900/50 rounded-lg p-2">
                    <p className="text-slate-500 text-xs">{label}</p>
                    <p className="text-slate-200 font-medium mono text-sm mt-0.5">{value}</p>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </PageShell>
  );
}
