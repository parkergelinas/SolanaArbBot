# Alpha Engine

Fuses wallet intelligence, market microstructure, arb signals, and liquidity into execution-ready alpha signals.

## Architecture

```
Wallet Intelligence → Market Microstructure → Arb Signals
         ↓
Feature Fusion Engine (DashMap per token)
         ↓
Alpha Scoring Model
         ↓
Signal Gate (rules)
         ↓
Priority Queue (TTL + dedup)
         ↓
TradeSignal → execution-engine
```

## FusedFeatureVector

| Field | Source |
|-------|--------|
| wallet_smart_money_score | WalletScore.score × confidence, decay if idle >30s |
| momentum_strength | MarketSignal.momentum + microstructure + volume_spike |
| liquidity_conditions | pool liquidity / MIN_LIQUIDITY, spread penalty |
| arbitrage_opportunity_strength | ArbSignal.spread_pct × confidence |
| volume_spike_norm | capped volume_spike / 3 |

TTL: 5s stale eviction.

## Scoring

```
alpha_score = w1*wallet + w2*momentum + w3*arb + w4*volume_spike + w5*liquidity
```

Default weights: 0.30, 0.20, 0.25, 0.15, 0.10 (env `ALPHA_W1`..`ALPHA_W5`).

Strategy routing:
- arb_strength > 0.6 → `arbitrage_capture`
- wallet_score > 0.7 → `whale_copy_trade`
- momentum > 0.65 && volume_spike > 0.5 → `sniper_entry`
- momentum > 0.5 → `momentum_follow`

## Signal Gates

| Gate | Env | Default |
|------|-----|---------|
| min alpha score | `ALPHA_MIN_SCORE` | 0.75 |
| min confidence | `ALPHA_MIN_CONFIDENCE` | 0.65 |
| min liquidity | `ALPHA_MIN_LIQUIDITY` | 0.30 |
| cooldown | `SIGNAL_COOLDOWN_SECS` | 10 |

Reject reasons: low score, low confidence, low liquidity, cooldown, conflicting signals.

## Priority Queue

| Env | Default |
|-----|---------|
| `SIGNAL_QUEUE_MAX` | 100 |
| `SIGNAL_QUEUE_TTL_MS` | 5000 |
| `SIGNAL_DRAIN_PER_TICK` | 5 |
| `ALPHA_TICK_INTERVAL_MS` | 40 |

## TradeSignal (canonical)

```rust
struct TradeSignal {
    token: String,
    direction: String,  // "long" | "short"
    size_usd: f64,
    confidence: f64,
    expected_edge: f64,
    strategy: String,
}
```

## Run

```bash
# Terminal 1 — intelligence
cargo run -p intelligence-api

# Terminal 2 — arb detection
cargo run -p arb-engine

# Terminal 3 — alpha fusion
ALPHA_MOCK_MODE=true cargo run -p alpha-engine

# Terminal 4 — execution
cargo run -p execution-engine
```

## Tests

```bash
cargo test -p alpha-engine
```
