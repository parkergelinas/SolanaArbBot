// Realistic mock data for dry-run / disconnected state

export const MOCK_ORDERBOOK = {
  pair: "SOL/USDC",
  asks: [
    { price: 184.23, size: 12.45, depth: 0.85 },
    { price: 184.21, size: 8.2, depth: 0.6 },
    { price: 184.19, size: 4.1, depth: 0.35 },
    { price: 184.17, size: 6.8, depth: 0.5 },
    { price: 184.15, size: 3.2, depth: 0.25 },
  ],
  bids: [
    { price: 184.13, size: 5.3, depth: 0.4 },
    { price: 184.11, size: 9.8, depth: 0.72 },
    { price: 184.09, size: 14.2, depth: 1.0 },
    { price: 184.07, size: 7.6, depth: 0.56 },
    { price: 184.05, size: 2.4, depth: 0.18 },
  ],
  spread: 0.1,
  spreadPct: 0.0054,
};

export const MOCK_WATCHLIST = [
  { symbol: "SOL/U", price: 184.21, change: 2.14, volume: 142e6 },
  { symbol: "JUP", price: 0.8821, change: 5.2, volume: 28e6 },
  { symbol: "BONK", price: 0.000021, change: -1.1, volume: 4e6 },
  { symbol: "WIF", price: 1.42, change: 3.8, volume: 19e6 },
  { symbol: "PYTH", price: 0.31, change: -0.8, volume: 8e6 },
];

export const MOCK_POSITIONS = [
  { pair: "SOL/USDC", size: 12.4, entry: 184.1, pnl: 14.22, strategy: "ARB" },
  { pair: "JUP/SOL", size: 8.0, entry: 0.89, pnl: -3.1, strategy: "COPY" },
];

export const MOCK_TRADES = Array.from({ length: 20 }, (_, i) => ({
  id: i,
  ts: Date.now() - i * 11_000,
  strategy: i % 3 === 0 ? "SNIPER" : i % 2 === 0 ? "COPY" : "ARB",
  action: i % 3 === 0 ? "SELL" : "BUY",
  pair: i % 4 === 0 ? "JUP/SOL" : "SOL/USDC",
  size: +(Math.random() * 10 + 1).toFixed(2),
  price: +(184 + Math.random() * 0.5).toFixed(4),
  pnl: i % 3 === 2 ? null : +(Math.random() * 4 - 1).toFixed(2),
  status: i === 0 ? "pending" : i === 1 ? "reverted" : "filled",
}));

export const MOCK_STATUS = {
  connected: true,
  slot: 284_119_442,
  latencyMs: 47,
  jitoOk: true,
  dryRun: true,
  strategy: "ARB",
  solBalance: 10.42,
};
