SYSTEM: Solana Market Intelligence + Arbitrage Simulation Engine (Rust)

Project overview: see [PROJECT_SUMMARY.md](PROJECT_SUMMARY.md#L1)

PURPOSE:
Real-time ingestion, normalization, pricing, graph modeling, and arbitrage route simulation across Solana DEX liquidity sources (Raydium AMM + Orca CLMM).

This system is a deterministic market model, NOT a trading bot.

------------------------------------------------------------
ARCHITECTURE OVERVIEW

stream
- WebSocket/RPC ingestion layer
- Emits MarketEvent
- Handles reconnect + backpressure

rpc_client
- Abstract Solana RPC interface
- Mock implementation for development
- Future: Yellowstone gRPC replacement

decoder
- Converts MarketEvent → PoolState
- Supports:
  - Raydium AMM pools
  - Orca CLMM pools
- Output: normalized PoolState

pricing
- Input: Vec<PoolState>
- Output: UnifiedPrice
- Responsibilities:
  - liquidity-weighted pricing
  - multi-DEX aggregation
  - deterministic computation

graph
- Token-level directed weighted graph
- Nodes: Token
- Edges: swap routes between tokens
- Supports multi-DEX edges

routing
- Arbitrage cycle detection (max depth 3)
- Path scoring:
  - liquidity
  - fees
  - price inefficiency

execution
- Simulation engine
- Models:
  - AMM constant product swaps
  - simplified CLMM ticks
- Outputs realized trade result

risk
- Filters invalid or unsafe routes
- Checks:
  - slippage threshold
  - liquidity thresholds
  - stability constraints

engine
- Orchestrator
- Runs full pipeline:
  stream → decoder → pricing → graph → routing → execution → risk

common
- Shared types:
  - Token
  - Pubkey wrapper
  - MarketEvent
  - PoolState
  - Error types

------------------------------------------------------------
DATA FLOW (STRICT)

MarketEvent
  → PoolState
    → UnifiedPrice
      → Graph update
        → Route candidates
          → Execution simulation
            → Risk filtering
              → OpportunitySignal (internal only)

------------------------------------------------------------
HOT PATH RULES

Applies to:
stream, decoder, pricing, graph, routing, execution

RULES:
- NO Arc<RwLock<>>
- NO blocking calls in async loops
- MINIMIZE allocations in loops
- USE crossbeam channels for eventing
- AVOID cloning large structs
- MUST be deterministic
- NO logging in tight loops (except sampling)

------------------------------------------------------------
CRATE DEPENDENCY RULES

engine → all crates
execution → routing + common
routing → graph + pricing + common
graph → common
pricing → decoder + common
decoder → common
stream → rpc_client + common
rpc_client → common
risk → execution + common

------------------------------------------------------------
STRICT BOUNDARIES

FORBIDDEN:
- cross-crate business logic leaks
- combining pricing + routing logic
- embedding execution logic in decoder
- modifying upstream crate responsibilities
- introducing new crates without explicit approval

------------------------------------------------------------
NON-GOALS (IMPORTANT)

- NO live trading execution
- NO MEV / Jito integration (future phase)
- NO on-chain transaction submission
- NO external infra dependencies in core logic
- NO database dependency in core pipeline

------------------------------------------------------------
DESIGN PRINCIPLES

- Deterministic computation over probabilistic inference
- Separation of concerns between ingestion, modeling, and simulation
- Minimal abstraction in hot paths
- Explicit data transformations between layers
- Prefer clarity over clever optimizations early

------------------------------------------------------------
PERFORMANCE GOALS (TARGETS ONLY)

- Ingestion: 50k–150k events/sec capable design
- Pricing: sub-millisecond per pool batch (target)
- Routing: bounded depth search (<3 hops)
- Execution simulation: deterministic O(n) per route

------------------------------------------------------------
FINAL OUTPUT OF SYSTEM

System emits:
- arbitrage opportunity signals (internal only)
- simulated profitability estimates
- route scores with risk filtering applied

NOT:
- actual trades
- blockchain transactions
- external execution

------------------------------------------------------------
VERSION PHILOSOPHY

This system evolves in phases:

Phase 1: ingestion + decoding + pricing
Phase 2: graph + routing
Phase 3: execution simulation + risk
Phase 4: optimization + MEV strategy layer (future)

------------------------------------------------------------