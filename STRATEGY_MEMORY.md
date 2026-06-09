# Solana Arb Bot — Strategy Memory

## ~~Strategy A: WETH/USDC Route Divergence Arb~~ (RETIRED — NOT SOLANA-NATIVE)
**Status:** ❌ Retired. WETH is a Wormhole-bridged token (not native Solana chain). Per user
decision 2026-06-07: only native Solana ecosystem tokens are permitted. WETH and WBTC
have been removed from `solana-mint-map.ts`. The $1.18/trade paper result was driven by
WETH's high notional — not relevant to the native-only strategy.

### Parameters
| Setting | Value |
|---------|-------|
| Pair | WETH/USDC |
| Trade size | 0.35 WETH (~$840 notional at $2,400/WETH) |
| Min profit gate | $0.08 |
| Slippage | 20 bps |
| Scan interval | 5s |
| Pairs/scan | 3 |

### Paper Session Results (6hr, ~t+110min completed)
| Metric | Value |
|--------|-------|
| Avg/trade | $1.18–$1.20 |
| Sustained rate | 150–155/hr |
| Projected hourly P&L | ~$179/hr |
| Projected 6hr P&L | ~$1,080 |
| Loss rate | 0% |
| Execution haircut (real vs expected) | ~15% |

### How It Works
- Compares "restricted" (single-hop, Raydium 0.25% fee) vs "unrestricted" (multi-hop, Orca 0.05% + venue inefficiency) Jupiter routing
- Route divergence of 30–80 bps on WETH/USDC generates $1–$2.40/trade at $840 notional
- Completely bypasses Jupiter in paper mode (DexScreener prices + stochastic simulator)

### Capital Requirement
- **Minimum:** 0.35 WETH ($840) + ~0.1 SOL for gas
- **Not viable under $200 notional** — fixed costs eat the edge

### Bugs Found & Fixed
1. Jupiter quote bypass (paper mode was calling Jupiter, getting 429, silently blocking simulator)
2. Min-profit hard stop (below_min_profit was excluded from risk gate — now enforced)
3. Token quality bypass (passesMarketFilter checks stale quality data after 60s — paper mode now skips)
4. fetchVerifiedTokens now uses fetchWithBackoff (shared 429 tracking)

---

## Strategy B: SOL-Native Micro Route Divergence Arb (LOW CAPITAL ENTRY)
**Status:** Designed. Not yet paper-tested. Implement next.

### Concept
Same route-divergence mechanism as Strategy A but using SOL/USDC as the primary pair.
Trade full 0.9 SOL per tick (keep 0.1 SOL as gas buffer). Profits accumulate as extra SOL
from the round-trip (put in 0.9 SOL, get back 0.9 + profit SOL).

### Parameters
| Setting | Value |
|---------|-------|
| Primary pair | SOL/USDC |
| Secondary pairs | SOL/mSOL, SOL/USDT, SOL/BONK |
| Trade size | 0.9 SOL (keep 0.1 SOL for gas) |
| Notional at $65/SOL | ~$58.50 |
| Min profit gate | $0.02 (lower gate for smaller notional) |
| Slippage | 20 bps |

### Expected Performance (unvalidated)
| Metric | Estimate |
|--------|----------|
| Divergence if sim holds | 30–80 bps |
| Profit/trade at $58.50 notional | ~$0.02–$0.05 |
| Rate | ~150/hr |
| Hourly P&L | ~$3–$7/hr |
| Time to double (1→2 SOL) | ~10–20 hrs |

### Compounding Path (Solana-native only)
```
Start:   1.0 SOL ($65)
Step 1:  Trade SOL/USDC + SOL/mSOL + BONK/USDC → accumulate profits
Step 2:  Reinvest profits → compound trade size upward
Step 3:  Target: 10 SOL ($650) → larger notional = larger profit/trade
Step 4:  Target: 50 SOL ($3,250) → $0.10–$0.20/trade range → ~$15–$30/hr
Note:    No bridge to a non-native strategy — native all the way up
```

### Key Unknown
Real Jupiter divergence on SOL/USDC — the live-quotes session (currently running)
will measure this. If divergence < 5 bps on SOL/USDC, Strategy B is not viable
and a different approach is needed.

---

---

## Strategy C: Cross-DEX Arb (Raydium vs Orca)
**Status:** Validated. Real spreads confirmed: core pairs 1–20 bps, pump pairs 50–500 bps (unconfirmed).
**Key finding:** SOL/USDC = 0.25 bps. Core pairs have no real edge at Stage 1 capital. Pump pairs are the only viable source at 1–3 SOL.
**Config:** `.env.cross-dex` — `BOT_ENABLE_CROSS_DEX_ARB=1`
**TX cost floor:** ~$0.065/trade — minimum profitable trade = $0.065 net

---

## Strategy D: Adaptive Arb (ACTIVE — capital-building strategy)
**Status:** Built. Paper validation running.
**Config:** `.env.adaptive`

### How it adapts
| Stage | Capital | Trade size | Min spread | Min profit | Notes |
|-------|---------|-----------|-----------|-----------|-------|
| 1 | 1–3 SOL | 0.5 SOL | 100 bps | $0.01 | Pump pairs only |
| 2 | 3–10 SOL | capital×50% | 60 bps | $0.03 | Pump + LST |
| 3 | 10–30 SOL | capital×60% | 35 bps | $0.08 | Core pairs viable |
| 4 | 30+ SOL | capital×70% | 25 bps | $0.15 | Full suite |

### Edge sources
1. **Pump.fun graduated tokens** (first 24–48h on Raydium) — 50–500 bps, DexScreener discovery
2. **LST de-peg** (mSOL/jitoSOL/bSOL) — 2–15 bps, reliable, low-risk
3. **Cross-DEX core pairs** — 1–20 bps, only viable at Stage 3+

### Capital persistence
`CapitalTracker` stores balance in SQLite, survives restarts, compounds automatically.

### Live execution
Requires `JITO_ENABLED=1` for atomic two-tx bundle (buy on cheaperDex + sell on dearerDex).
Without Jito: paper mode only (gap risk between legs in live mode).

### Honest expected trajectory
```
Start:  1.0 SOL ($65)
Month 1: $0.01–$0.10/trade, 2–10 trades/day (pump pairs only) → +$0.02–$1/day
Month 2: Balance may be 1.5–3 SOL → Stage 2 parameters unlock
Month 3: 3–5 SOL → core pairs start generating $0.03–$0.15/trade
Month 6: 10+ SOL → Stage 3, $0.08–$0.50/trade, consistent daily P&L
```
Timeline compresses sharply if pump pair spreads are wide and frequent.

---

## Live-Quotes Session
**Started:** 2026-06-07 09:09:39
**Mode:** BOT_PAPER_MODE=1 + BOT_LIVE_QUOTES=1
**Purpose:** Measure real Jupiter route divergence before committing capital
**API Key:** Stored in .env.live-quotes (never log or display)
**Pairs/scan:** 10 (bumped from 3 after API key added)
**Watch for:** divergenceBps in trade notifications — target >15 bps for viable edge
