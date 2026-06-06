/** Client-safe env helpers — production defaults favor real data only. */

/** Demo simulator: opt-in only (`NEXT_PUBLIC_TERMINAL_DEMO=1`). Default OFF. */
export function isTerminalDemoEnabled(): boolean {
  return process.env.NEXT_PUBLIC_TERMINAL_DEMO === '1';
}

/** Paper trading panel (dry-run fills). Default ON; set `0` to hide. */
export function isPaperTradingEnabled(): boolean {
  return process.env.NEXT_PUBLIC_PAPER_TRADING !== '0';
}

export function defaultSolanaNetwork(): 'mainnet-beta' | 'devnet' {
  const raw = process.env.NEXT_PUBLIC_SOLANA_NETWORK ?? 'devnet';
  return raw === 'mainnet-beta' ? 'mainnet-beta' : 'devnet';
}
